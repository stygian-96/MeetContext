use serde::Serialize;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// The one error type every command returns; the UI receives `{ code, message }`.
#[derive(Debug, Serialize, specta::Type)]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
}

impl AppError {
    pub fn new(code: &'static str, message: impl ToString) -> Self {
        Self { code, message: message.to_string() }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::new("db", e)
    }
}

impl From<sqlx::migrate::MigrateError> for AppError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        Self::new("migrate", e)
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}

/// Opens (or creates) the SQLite file and applies pending migrations.
pub async fn open_db(path: &Path) -> Result<SqlitePool, AppError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    migrate(&pool).await?;
    Ok(pool)
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
}

pub async fn setting_get(pool: &SqlitePool, key: &str) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?)
}

pub async fn setting_set(pool: &SqlitePool, key: &str, value: &str) -> Result<(), AppError> {
    sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}
