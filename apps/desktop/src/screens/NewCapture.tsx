import { useEffect, useState } from "react";
import { commands, type Devices } from "../bindings";
import { Alert, Button, Checkbox, Dialog, Field, LevelMeter } from "../ui";

/** The New capture dialog; shows a live mic meter while open. */
export function NewCapture({ onClose }: { onClose: () => void }) {
  const [devices, setDevices] = useState<Devices>();
  const [name, setName] = useState("");
  const [mic, setMic] = useState<string>();
  const [systemAudio, setSystemAudio] = useState(true);
  const [consent, setConsent] = useState(false);
  const [level, setLevel] = useState(0);
  const [error, setError] = useState("");

  useEffect(() => {
    commands.captureDevices().then((d) => (setDevices(d), setSystemAudio(d.system_audio)));
    commands.settingsGet("mic_device_id").then((r) => setMic(r.status === "ok" ? r.data ?? "" : ""));
  }, []);

  // Live meter for the chosen mic; restarts when the mic changes.
  useEffect(() => {
    if (mic === undefined) return;
    setError("");
    commands.micMonitorStart(mic || null).then((r) => r.status === "error" && setError(r.error.message));
    const timer = setInterval(() => commands.micMonitorLevel().then((l) => setLevel(l ?? 0)), 150);
    return () => {
      clearInterval(timer);
      commands.micMonitorStop();
    };
  }, [mic]);

  async function start() {
    await commands.settingsSet("mic_device_id", mic ?? "");
    const title = name.trim() || `Meeting ${new Date().toLocaleString()}`;
    const r = await commands.captureStart(title, mic || null, systemAudio, consent);
    r.status === "ok" ? onClose() : setError(r.error.message);
  }

  const footer = (
    <>
      <span className="ui-muted" style={{ flex: 1, fontSize: 12.5, lineHeight: 1.45 }}>
        When you start, this window gets out of the way and a small capture bar floats on top of your meeting until you stop.
      </span>
      <Button onClick={onClose}>Cancel</Button>
      <Button variant="record" disabled={!consent} onClick={start}>
        <span style={{ width: 9, height: 9, borderRadius: "50%", background: "#fff" }} />
        Start capture
      </Button>
    </>
  );

  return (
    <Dialog title="New capture" tag="NOT RECORDING YET" onClose={onClose} footer={footer}>
      {error && <Alert><strong>Can’t use this microphone.</strong> {error}</Alert>}
      <Field label="Meeting name">
        <input value={name} placeholder={`Meeting ${new Date().toLocaleDateString()}`} onChange={(e) => setName(e.target.value)} />
      </Field>
      <fieldset style={{ border: 0, margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 10 }}>
        <legend className="ui-muted" style={{ fontSize: 12.5, marginBottom: 8 }}>Audio</legend>
        <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 12, alignItems: "end" }}>
          <Field label="Your microphone">
            <select value={mic ?? ""} onChange={(e) => setMic(e.target.value)}>
              <option value="">System default</option>
              {devices?.mics.map((d) => <option key={d.id} value={d.id}>{d.name}</option>)}
            </select>
          </Field>
          <span style={{ paddingBottom: 10 }}><LevelMeter label="Microphone level" level={level} /></span>
        </div>
        <Checkbox checked={systemAudio} disabled={!devices?.system_audio} onChange={setSystemAudio}>
          Also record meeting audio (other participants){!devices?.system_audio && <span className="ui-muted"> — Windows only for now</span>}
        </Checkbox>
      </fieldset>
      <Checkbox boxed checked={consent} onChange={setConsent}>
        I allow this app to record audio for this meeting, and everyone in the meeting knows it’s being recorded.
      </Checkbox>
    </Dialog>
  );
}
