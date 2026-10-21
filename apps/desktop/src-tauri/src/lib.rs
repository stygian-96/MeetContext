use capture::{Capture, Devices, MicMonitor, Recorder};
use jobs::{Handlers, JobUpdate};
use mc_core::AppError;
use prd::{Prd, Project};
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_specta::{Event, collect_commands, collect_events};

/// Pushed to the UI whenever a job changes status.
#[derive(Clone, Serialize, specta::Type, Event)]
struct JobUpdated(JobUpdate);

/// Pushed when a capture starts, stops or fails.
#[derive(Clone, Serialize, specta::Type, Event)]
struct CaptureState {
    capture_id: i32,
    state: String,
    error: Option<String>,
}

/// Pushed every 500 ms while recording, for the overlay.
#[derive(Clone, Serialize, specta::Type, Event)]
struct CaptureTick {
    elapsed_ms: u32,
    mic_level: f32,
    system_level: f32,
}

struct Active {
    id: i32,
    recorder: Recorder,
}

/// The one capture that may be recording.
type ActiveCapture = Arc<Mutex<Option<Active>>>;

/// The New capture dialog's mic meter, while the dialog is open.
type MicMonitorState = Mutex<Option<MicMonitor>>;

const OVERLAY_WIDTH: f64 = 340.0;

fn err(code: &'static str) -> impl Fn(String) -> AppError {
    move |e| AppError::new(code, e)
}

fn captures_dir(app: &AppHandle, id: i32) -> Result<PathBuf, AppError> {
    let dir = app.path().app_data_dir().map_err(|e| AppError::new("path", e))?;
    Ok(dir.join("captures").join(id.to_string()))
}

#[tauri::command]
#[specta::specta]
async fn settings_get(pool: State<'_, SqlitePool>, key: String) -> Result<Option<String>, AppError> {
    mc_core::setting_get(&pool, &key).await
}

#[tauri::command]
#[specta::specta]
async fn settings_set(pool: State<'_, SqlitePool>, key: String, value: String) -> Result<(), AppError> {
    mc_core::setting_set(&pool, &key, &value).await
}

#[tauri::command]
#[specta::specta]
async fn project_list(pool: State<'_, SqlitePool>) -> Result<Vec<Project>, AppError> {
    prd::list(&pool).await
}

#[tauri::command]
#[specta::specta]
async fn project_create(pool: State<'_, SqlitePool>, name: String, description: String) -> Result<i32, AppError> {
    prd::create(&pool, &name, &description).await
}

#[tauri::command]
#[specta::specta]
async fn project_update(pool: State<'_, SqlitePool>, id: i32, name: String, description: String) -> Result<(), AppError> {
    prd::update(&pool, id, &name, &description).await
}

#[tauri::command]
#[specta::specta]
async fn prd_get(pool: State<'_, SqlitePool>, project_id: i32) -> Result<Prd, AppError> {
    prd::get(&pool, project_id).await
}

#[tauri::command]
#[specta::specta]
async fn prd_save(pool: State<'_, SqlitePool>, project_id: i32, markdown: String, base_version: i32) -> Result<Prd, AppError> {
    prd::save(&pool, project_id, &markdown, base_version, "edit").await
}

#[tauri::command]
#[specta::specta]
fn capture_devices() -> Devices {
    capture::devices()
}

/// Starts the dialog's mic meter, or switches it to another mic.
#[tauri::command]
#[specta::specta]
async fn mic_monitor_start(monitor: State<'_, MicMonitorState>, device_id: Option<String>) -> Result<(), AppError> {
    let mut m = monitor.lock().unwrap();
    *m = None; // release the previous device before opening the next
    *m = Some(MicMonitor::start(device_id.as_deref()).map_err(err("device"))?);
    Ok(())
}

#[tauri::command]
#[specta::specta]
fn mic_monitor_level(monitor: State<'_, MicMonitorState>) -> f32 {
    monitor.lock().unwrap().as_ref().map_or(0.0, |m| capture::level(&m.level))
}

#[tauri::command]
#[specta::specta]
fn mic_monitor_stop(monitor: State<'_, MicMonitorState>) {
    *monitor.lock().unwrap() = None;
}

#[tauri::command]
#[specta::specta]
async fn capture_list(pool: State<'_, SqlitePool>) -> Result<Vec<Capture>, AppError> {
    capture::list(&pool).await
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)] // Tauri hands app state in as arguments
async fn capture_start(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    active: State<'_, ActiveCapture>,
    monitor: State<'_, MicMonitorState>,
    name: String,
    mic_device_id: Option<String>,
    system_audio: bool,
    consent: bool,
) -> Result<i32, AppError> {
    if !consent {
        return Err(AppError::new("consent", "recording needs your permission"));
    }
    if active.lock().unwrap().is_some() {
        return Err(AppError::new("busy", "a capture is already recording"));
    }
    *monitor.lock().unwrap() = None;
    let id = capture::create(&pool, &name).await?;
    let pool_for_chunks = pool.inner().clone();
    let on_chunk: capture::OnChunk = Arc::new(move |chunk| {
        let pool = pool_for_chunks.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = capture::insert_chunk(&pool, id, &chunk).await {
                eprintln!("saving chunk failed: {e}");
            }
        });
    });
    let recorder = match Recorder::start(&captures_dir(&app, id)?, mic_device_id.as_deref(), system_audio, on_chunk) {
        Ok(r) => r,
        Err(e) => {
            capture::finish(&pool, id, "failed", Some(&e)).await?;
            return Err(AppError::new("device", e));
        }
    };
    *active.lock().unwrap() = Some(Active { id, recorder });
    let _ = CaptureState { capture_id: id, state: "recording".into(), error: None }.emit(&app);

    if let Some(main) = app.get_webview_window("main") {
        let _ = main.minimize();
    }
    // Top centre of the main screen, like the design's capture bar.
    let x = app
        .primary_monitor()
        .ok()
        .flatten()
        .map_or(24.0, |m| (m.size().to_logical::<f64>(m.scale_factor()).width - OVERLAY_WIDTH) / 2.0);
    let overlay = WebviewWindowBuilder::new(&app, "overlay", WebviewUrl::App("index.html?view=overlay".into()))
        .title("MeetContext capture")
        .inner_size(OVERLAY_WIDTH, 60.0)
        .position(x, 12.0)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .content_protected(true);
    // shortcut: transparent windows on macOS need Tauri's macos-private-api; square bar there until then.
    #[cfg(not(target_os = "macos"))]
    let overlay = overlay.transparent(true);
    overlay.build().map_err(|e| AppError::new("window", e))?;

    let (app, active) = (app.clone(), active.inner().clone());
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let tick = match active.lock().unwrap().as_ref() {
                Some(a) if a.id == id => CaptureTick {
                    elapsed_ms: a.recorder.started.elapsed().as_millis() as u32,
                    mic_level: capture::level(&a.recorder.mic_level),
                    system_level: capture::level(&a.recorder.system_level),
                },
                _ => break,
            };
            let _ = tick.emit(&app);
        }
    });
    Ok(id)
}

#[tauri::command]
#[specta::specta]
async fn capture_stop(app: AppHandle, pool: State<'_, SqlitePool>, active: State<'_, ActiveCapture>) -> Result<(), AppError> {
    let Some(Active { id, recorder }) = active.lock().unwrap().take() else {
        return Err(AppError::new("idle", "nothing is recording"));
    };
    let result = tauri::async_runtime::spawn_blocking(move || recorder.stop()).await.map_err(|e| AppError::new("stop", e))?;
    let (state, error) = match &result {
        Ok(()) => ("ready", None),
        Err(e) => ("failed", Some(e.as_str())),
    };
    capture::finish(&pool, id, state, error).await?;
    let _ = CaptureState { capture_id: id, state: state.into(), error: error.map(String::from) }.emit(&app);
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.close();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
    result.map_err(err("device"))
}

#[tauri::command]
#[specta::specta]
async fn capture_delete(app: AppHandle, pool: State<'_, SqlitePool>, id: i32) -> Result<(), AppError> {
    capture::delete(&pool, id, &captures_dir(&app, id)?).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            settings_get,
            settings_set,
            project_list,
            project_create,
            project_update,
            prd_get,
            prd_save,
            capture_devices,
            mic_monitor_start,
            mic_monitor_level,
            mic_monitor_stop,
            capture_list,
            capture_start,
            capture_stop,
            capture_delete
        ])
        .events(collect_events![JobUpdated, CaptureState, CaptureTick]);

    #[cfg(debug_assertions)]
    builder
        .export(specta_typescript::Typescript::default(), "../src/bindings.ts")
        .expect("failed to export TypeScript bindings");

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let pool = tauri::async_runtime::block_on(async {
                let pool = mc_core::open_db(&dir.join("meetcontext.db")).await?;
                jobs::recover(&pool).await?;
                capture::recover(&pool).await?;
                Ok::<_, AppError>(pool)
            })?;
            app.manage(pool.clone());
            app.manage(ActiveCapture::default());
            app.manage(MicMonitorState::default());
            let handle = app.handle().clone();
            // Each phase registers its job kinds here; none exist yet.
            tauri::async_runtime::spawn(jobs::run_worker(pool, Handlers::new(), move |update| {
                let _ = JobUpdated(update).emit(&handle);
            }));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
