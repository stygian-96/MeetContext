use jobs::{Handler, Handlers, JobUpdate};
use mc_core::AppError;
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::{Manager, State};
use tauri_specta::{Event, collect_commands, collect_events};

#[derive(Serialize, specta::Type)]
struct AppInfo {
    version: String,
    os: String,
    data_dir: String,
}

/// Pushed to the UI whenever a job changes status.
#[derive(Clone, Serialize, specta::Type, Event)]
struct JobUpdated(JobUpdate);

#[tauri::command]
#[specta::specta]
fn app_info(app: tauri::AppHandle) -> Result<AppInfo, AppError> {
    let data_dir = app.path().app_data_dir().map_err(|e| AppError::new("path", e))?;
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        os: std::env::consts::OS.into(),
        data_dir: data_dir.display().to_string(),
    })
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

/// Dev only: queues a job that sleeps `sleep_ms`, then fails its first `fail_times` attempts.
#[tauri::command]
#[specta::specta]
async fn dev_enqueue_test_job(pool: State<'_, SqlitePool>, fail_times: u32, sleep_ms: u32) -> Result<i32, AppError> {
    if !cfg!(debug_assertions) {
        return Err(AppError::new("dev_only", "test jobs exist only in dev builds"));
    }
    jobs::enqueue(&pool, "test", &serde_json::json!({ "fail_times": fail_times, "sleep_ms": sleep_ms })).await
}

fn handlers() -> Handlers {
    let test: Handler = Arc::new(|payload, attempt| {
        Box::pin(async move {
            let sleep_ms = payload["sleep_ms"].as_u64().unwrap_or(0);
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
            if i64::from(attempt) <= payload["fail_times"].as_i64().unwrap_or(0) {
                return Err(format!("test failure on attempt {attempt}"));
            }
            Ok(())
        })
    });
    HashMap::from([("test", test)])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![app_info, settings_get, settings_set, dev_enqueue_test_job])
        .events(collect_events![JobUpdated]);

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
                Ok::<_, AppError>(pool)
            })?;
            app.manage(pool.clone());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(jobs::run_worker(pool, handlers(), move |update| {
                let _ = JobUpdated(update).emit(&handle);
            }));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
