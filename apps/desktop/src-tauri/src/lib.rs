use jobs::{Handlers, JobUpdate};
use mc_core::AppError;
use serde::Serialize;
use sqlx::SqlitePool;
use tauri::{Manager, State};
use tauri_specta::{Event, collect_commands, collect_events};

/// Pushed to the UI whenever a job changes status.
#[derive(Clone, Serialize, specta::Type, Event)]
struct JobUpdated(JobUpdate);

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![settings_get, settings_set])
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
            // Each phase registers its job kinds here; none exist yet.
            tauri::async_runtime::spawn(jobs::run_worker(pool, Handlers::new(), move |update| {
                let _ = JobUpdated(update).emit(&handle);
            }));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
