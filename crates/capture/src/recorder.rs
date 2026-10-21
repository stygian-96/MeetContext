//! Records each track on its own thread into 30-second, 16 kHz mono WAV chunks.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const SAMPLE_RATE: u32 = 16_000;
const CHUNK_SAMPLES: u64 = 30 * SAMPLE_RATE as u64;
/// WASAPI loopback delivers nothing while no sound plays, so a track that falls this far
/// behind the wall clock is filled with silence to stay in sync with the other track.
const PAD_SLACK: u64 = SAMPLE_RATE as u64 / 4;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum Track {
    Mic,
    System,
}

impl Track {
    pub fn as_str(self) -> &'static str {
        match self {
            Track::Mic => "mic",
            Track::System => "system",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub track: Track,
    pub seq: u32,
    pub start_ms: i64,
    pub end_ms: i64,
    pub path: PathBuf,
    pub bytes: u64,
}

pub type OnChunk = Arc<dyn Fn(Chunk) + Send + Sync>;

#[derive(Serialize, specta::Type)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, specta::Type)]
pub struct Devices {
    pub mics: Vec<AudioDevice>,
    /// Meeting (system) audio capture; Windows only for now.
    pub system_audio: bool,
}

pub fn devices() -> Devices {
    let mics = cpal::default_host()
        .input_devices()
        .map(|ds| {
            ds.filter_map(|d| Some(AudioDevice { id: d.id().ok()?.to_string(), name: d.description().ok()?.name().into() }))
                .collect()
        })
        .unwrap_or_default();
    // shortcut: no macOS meeting audio yet; needs ScreenCaptureKit, add when testing on a Mac.
    Devices { mics, system_audio: cfg!(windows) }
}

/// Downsamples to 16 kHz mono and writes rolling WAV chunks.
struct ChunkWriter {
    dir: PathBuf,
    track: Track,
    ratio: f64,
    phase: f64,
    sum: f32,
    n: u32,
    last: f32,
    written: u64,
    chunk_start: u64,
    seq: u32,
    wav: Option<(hound::WavWriter<BufWriter<File>>, PathBuf)>,
    sq: f32,
    sq_n: u32,
    level: Arc<AtomicU32>,
    on_chunk: OnChunk,
}

impl ChunkWriter {
    fn new(dir: &Path, track: Track, input_rate: u32, level: Arc<AtomicU32>, on_chunk: OnChunk) -> Self {
        Self {
            dir: dir.into(),
            track,
            ratio: f64::from(input_rate) / f64::from(SAMPLE_RATE),
            phase: 0.0,
            sum: 0.0,
            n: 0,
            last: 0.0,
            written: 0,
            chunk_start: 0,
            seq: 0,
            wav: None,
            sq: 0.0,
            sq_n: 0,
            level,
            on_chunk,
        }
    }

    // shortcut: box-filter resampling (slight aliasing), use a proper resampler if transcription suffers.
    fn push(&mut self, input: &[f32]) -> Result<(), String> {
        for &s in input {
            self.sum += s;
            self.n += 1;
            self.phase += 1.0;
            while self.phase >= self.ratio {
                self.phase -= self.ratio;
                if self.n > 0 {
                    self.last = self.sum / self.n as f32;
                    (self.sum, self.n) = (0.0, 0);
                }
                self.emit(self.last)?;
            }
        }
        Ok(())
    }

    fn pad_to(&mut self, expected: u64) -> Result<(), String> {
        while self.written + PAD_SLACK < expected {
            self.emit(0.0)?;
        }
        Ok(())
    }

    fn emit(&mut self, v: f32) -> Result<(), String> {
        if self.wav.is_none() {
            let path = self.dir.join(format!("{}_{:04}.wav", self.track.as_str(), self.seq));
            let spec = hound::WavSpec { channels: 1, sample_rate: SAMPLE_RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
            self.wav = Some((hound::WavWriter::create(&path, spec).map_err(|e| e.to_string())?, path));
        }
        let (wav, _) = self.wav.as_mut().unwrap();
        wav.write_sample((v.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16).map_err(|e| e.to_string())?;
        self.written += 1;
        self.sq += v * v;
        self.sq_n += 1;
        if self.sq_n == SAMPLE_RATE / 10 {
            self.level.store((self.sq / self.sq_n as f32).sqrt().to_bits(), Ordering::Relaxed);
            (self.sq, self.sq_n) = (0.0, 0);
        }
        if self.written - self.chunk_start == CHUNK_SAMPLES {
            self.close()?;
        }
        Ok(())
    }

    fn close(&mut self) -> Result<(), String> {
        let Some((wav, path)) = self.wav.take() else { return Ok(()) };
        wav.finalize().map_err(|e| e.to_string())?;
        let bytes = fs::metadata(&path).map_err(|e| e.to_string())?.len();
        let ms = |samples: u64| (samples * 1000 / u64::from(SAMPLE_RATE)) as i64;
        (self.on_chunk)(Chunk { track: self.track, seq: self.seq, start_ms: ms(self.chunk_start), end_ms: ms(self.written), path, bytes });
        self.seq += 1;
        self.chunk_start = self.written;
        Ok(())
    }
}

pub struct Recorder {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<Result<(), String>>>,
    pub started: Instant,
    pub mic_level: Arc<AtomicU32>,
    pub system_level: Arc<AtomicU32>,
}

pub fn level(l: &AtomicU32) -> f32 {
    f32::from_bits(l.load(Ordering::Relaxed))
}

impl Recorder {
    /// Starts every track; fails (and stops any started track) if a device cannot be opened.
    pub fn start(dir: &Path, mic_id: Option<&str>, system_audio: bool, on_chunk: OnChunk) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let mut rec = Recorder {
            stop: Arc::default(),
            threads: vec![],
            started: Instant::now(),
            mic_level: Arc::default(),
            system_level: Arc::default(),
        };
        let mut tracks = vec![(Track::Mic, rec.mic_level.clone())];
        if system_audio {
            tracks.push((Track::System, rec.system_level.clone()));
        }
        for (track, level) in tracks {
            let (ready_tx, ready_rx) = mpsc::channel();
            let (dir, mic_id, stop, started, on_chunk) =
                (dir.to_path_buf(), mic_id.map(String::from), rec.stop.clone(), rec.started, on_chunk.clone());
            rec.threads.push(std::thread::spawn(move || {
                record_track(&dir, track, mic_id.as_deref(), &stop, started, level, on_chunk, ready_tx)
            }));
            if let Err(e) = ready_rx.recv().unwrap_or_else(|_| Err("audio thread exited".into())) {
                let _ = rec.stop();
                return Err(e);
            }
        }
        Ok(rec)
    }

    /// Stops all tracks and writes their last partial chunks.
    pub fn stop(self) -> Result<(), String> {
        self.stop.store(true, Ordering::Relaxed);
        // Join every track before reporting, so no track is left writing after a failure.
        let results: Vec<_> =
            self.threads.into_iter().map(|t| t.join().unwrap_or_else(|_| Err("audio thread panicked".into()))).collect();
        results.into_iter().collect()
    }
}

/// Live mic level before a capture starts (the New capture dialog's meter); records nothing.
pub struct MicMonitor {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    pub level: Arc<AtomicU32>,
}

impl MicMonitor {
    pub fn start(mic_id: Option<&str>) -> Result<Self, String> {
        let (stop, level) = (Arc::<AtomicBool>::default(), Arc::<AtomicU32>::default());
        let (ready_tx, ready_rx) = mpsc::channel();
        let (s, l, id) = (stop.clone(), level.clone(), mic_id.map(String::from));
        let thread = std::thread::spawn(move || {
            let (_stream, rx, _) = match open_stream(Track::Mic, id.as_deref()) {
                Ok(x) => {
                    let _ = ready_tx.send(Ok(()));
                    x
                }
                Err(e) => return drop(ready_tx.send(Err(e))),
            };
            while !s.load(Ordering::Relaxed) {
                if let Ok(buf) = rx.recv_timeout(Duration::from_millis(100)) {
                    let rms = (buf.iter().map(|v| v * v).sum::<f32>() / buf.len().max(1) as f32).sqrt();
                    l.store(rms.to_bits(), Ordering::Relaxed);
                }
            }
        });
        ready_rx.recv().unwrap_or_else(|_| Err("audio thread exited".into()))?;
        Ok(Self { stop, thread: Some(thread), level })
    }
}

impl Drop for MicMonitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn record_track(
    dir: &Path,
    track: Track,
    mic_id: Option<&str>,
    stop: &AtomicBool,
    started: Instant,
    level: Arc<AtomicU32>,
    on_chunk: OnChunk,
    ready: mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    let (stream, rx, rate) = match open_stream(track, mic_id) {
        Ok(s) => s,
        Err(e) => {
            let _ = ready.send(Err(e.clone()));
            return Err(e);
        }
    };
    let _ = ready.send(Ok(()));
    let mut writer = ChunkWriter::new(dir, track, rate, level, on_chunk);
    while !stop.load(Ordering::Relaxed) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(buf) => writer.push(&buf)?,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        writer.pad_to(started.elapsed().as_millis() as u64 * u64::from(SAMPLE_RATE) / 1000)?;
    }
    drop(stream);
    writer.close()
}

fn open_stream(track: Track, mic_id: Option<&str>) -> Result<(cpal::Stream, mpsc::Receiver<Vec<f32>>, u32), String> {
    let host = cpal::default_host();
    let (device, config) = match track {
        Track::Mic => {
            let device = match mic_id {
                Some(id) => host.device_by_id(&id.parse().map_err(|_| format!("bad device id {id}"))?),
                None => host.default_input_device(),
            }
            .ok_or("microphone not found")?;
            let config = device.default_input_config().map_err(|e| format!("microphone: {e}"))?;
            (device, config)
        }
        // On Windows an input stream on an output device records it in loopback mode.
        Track::System => {
            let device = host.default_output_device().ok_or("no audio output device")?;
            let config = device.default_output_config().map_err(|e| format!("meeting audio: {e}"))?;
            (device, config)
        }
    };
    let channels = usize::from(config.channels());
    let (tx, rx) = mpsc::channel();
    let on_error = move |e| eprintln!("{} stream error: {e}", track.as_str());
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            config.config(),
            move |data: &[f32], _: &_| drop(tx.send(downmix(data, channels, |s| s))),
            on_error,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            config.config(),
            move |data: &[i16], _: &_| drop(tx.send(downmix(data, channels, |s| f32::from(s) / 32768.0))),
            on_error,
            None,
        ),
        other => return Err(format!("unsupported sample format {other:?}")),
    }
    .map_err(|e| format!("{}: {e}", track.as_str()))?;
    stream.play().map_err(|e| format!("{}: {e}", track.as_str()))?;
    Ok((stream, rx, config.sample_rate()))
}

fn downmix<T: Copy>(data: &[T], channels: usize, to_f32: impl Fn(T) -> f32) -> Vec<f32> {
    data.chunks(channels).map(|frame| frame.iter().map(|&s| to_f32(s)).sum::<f32>() / channels as f32).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn writer(dir: &Path, rate: u32) -> (ChunkWriter, Arc<Mutex<Vec<Chunk>>>) {
        let chunks = Arc::new(Mutex::new(vec![]));
        let sink = chunks.clone();
        let w = ChunkWriter::new(dir, Track::Mic, rate, Arc::default(), Arc::new(move |c| sink.lock().unwrap().push(c)));
        (w, chunks)
    }

    fn wav_len(path: &Path) -> u32 {
        hound::WavReader::open(path).unwrap().len()
    }

    #[test]
    fn rolls_30s_chunks_at_16khz() {
        let dir = std::env::temp_dir().join(format!("mc-capture-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (mut w, chunks) = writer(&dir, 48_000);
        w.push(&vec![0.25; 48_000 * 31]).unwrap();
        w.close().unwrap();
        let chunks = chunks.lock().unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!((chunks[0].start_ms, chunks[0].end_ms), (0, 30_000));
        assert_eq!((chunks[1].start_ms, chunks[1].end_ms), (30_000, 31_000));
        assert_eq!(wav_len(&chunks[0].path), 30 * SAMPLE_RATE);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resamples_odd_rates_and_pads_silence() {
        let dir = std::env::temp_dir().join(format!("mc-capture-test2-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (mut w, chunks) = writer(&dir, 44_100);
        w.push(&vec![0.1; 44_100]).unwrap(); // 1 s; the last sample may round into the next push
        assert!(w.written.abs_diff(u64::from(SAMPLE_RATE)) <= 1, "{}", w.written);
        w.pad_to(3 * u64::from(SAMPLE_RATE)).unwrap(); // the device went quiet for 2 s
        assert_eq!(w.written + PAD_SLACK, 3 * u64::from(SAMPLE_RATE));
        let (mut up, _) = writer(&dir, 8_000);
        up.push(&vec![0.1; 8_000]).unwrap();
        assert!(up.written.abs_diff(u64::from(SAMPLE_RATE)) <= 1, "{}", up.written);
        w.close().unwrap();
        assert_eq!(chunks.lock().unwrap().len(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Uses the real default mic and speakers: `cargo test -p capture -- --ignored`.
    #[test]
    #[ignore]
    fn records_real_devices() {
        let dir = std::env::temp_dir().join(format!("mc-capture-real-{}", std::process::id()));
        let chunks = Arc::new(Mutex::new(vec![]));
        let sink = chunks.clone();
        let rec = Recorder::start(&dir, None, cfg!(windows), Arc::new(move |c| sink.lock().unwrap().push(c))).unwrap();
        std::thread::sleep(Duration::from_secs(2));
        // Tracks are aligned to the capture start, which includes the time devices took to open.
        let expected = rec.started.elapsed().as_secs_f32();
        rec.stop().unwrap();
        let chunks = chunks.lock().unwrap();
        assert_eq!(chunks.len(), if cfg!(windows) { 2 } else { 1 });
        for c in chunks.iter() {
            let secs = wav_len(&c.path) as f32 / SAMPLE_RATE as f32;
            println!("{:?}: {secs:.2}s, {} bytes", c.track, c.bytes);
            assert!((secs - expected).abs() < 0.4, "{:?} recorded {secs}s, expected {expected}s", c.track);
        }
        fs::remove_dir_all(&dir).unwrap();
    }
}
