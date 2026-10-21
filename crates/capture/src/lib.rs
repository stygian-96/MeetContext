//! Meeting capture: audio recording plus the `captures` and `audio_chunks` tables.

mod recorder;

pub use recorder::{AudioDevice, Chunk, Devices, MicMonitor, OnChunk, Recorder, Track, devices, level};

use mc_core::{AppError, now_ms};
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::Path;

// Millisecond timestamps and byte counts pass i32 but stay exact in a JS number (up to 2^53),
// so they are typed as plain numbers for TypeScript.
#[derive(Serialize, specta::Type, sqlx::FromRow)]
pub struct Capture {
    pub id: i32,
    pub name: String,
    pub state: String,
    #[specta(type = i32)]
    pub started_at: i64,
    #[specta(type = Option<i32>)]
    pub duration_ms: Option<i64>,
    pub error: Option<String>,
    pub chunks: i32,
    #[specta(type = i32)]
    pub bytes: i64,
}

pub async fn create(pool: &SqlitePool, name: &str) -> Result<i32, AppError> {
    let now = now_ms();
    Ok(sqlx::query_scalar(
        "INSERT INTO captures (name, state, consent_at, started_at) VALUES (?, 'recording', ?, ?) RETURNING id",
    )
    .bind(name)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await?)
}

/// Ends a capture as `ready` or `failed`.
pub async fn finish(pool: &SqlitePool, id: i32, state: &str, error: Option<&str>) -> Result<(), AppError> {
    let now = now_ms();
    sqlx::query("UPDATE captures SET state = ?, ended_at = ?, duration_ms = ? - started_at, error = ? WHERE id = ?")
        .bind(state)
        .bind(now)
        .bind(now)
        .bind(error)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_chunk(pool: &SqlitePool, capture_id: i32, c: &Chunk) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audio_chunks (capture_id, track, seq, start_ms, end_ms, path, bytes) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(capture_id)
    .bind(c.track.as_str())
    .bind(c.seq)
    .bind(c.start_ms)
    .bind(c.end_ms)
    .bind(c.path.to_string_lossy())
    .bind(c.bytes as i64)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Capture>, AppError> {
    Ok(sqlx::query_as(
        "SELECT c.id, c.name, c.state, c.started_at, c.duration_ms, c.error,
                COUNT(a.id) AS chunks, COALESCE(SUM(a.bytes), 0) AS bytes
         FROM captures c LEFT JOIN audio_chunks a ON a.capture_id = c.id
         GROUP BY c.id ORDER BY c.started_at DESC",
    )
    .fetch_all(pool)
    .await?)
}

/// Removes the capture's rows and its folder of audio files.
pub async fn delete(pool: &SqlitePool, id: i32, dir: &Path) -> Result<(), AppError> {
    sqlx::query("DELETE FROM captures WHERE id = ?").bind(id).execute(pool).await?;
    match std::fs::remove_dir_all(dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(AppError::new("io", e)),
        _ => Ok(()),
    }
}

/// Captures still `recording` at startup were cut off by a crash or quit; keep their saved chunks.
pub async fn recover(pool: &SqlitePool) -> Result<u64, AppError> {
    Ok(sqlx::query(
        "UPDATE captures SET state = 'interrupted',
             duration_ms = (SELECT MAX(end_ms) FROM audio_chunks WHERE capture_id = captures.id)
         WHERE state = 'recording'",
    )
    .execute(pool)
    .await?
    .rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn crash_marks_capture_interrupted_and_keeps_chunks() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        mc_core::migrate(&pool).await.unwrap();
        let id = create(&pool, "crashed").await.unwrap();
        let chunk = |seq, end_ms| Chunk { track: Track::Mic, seq, start_ms: 0, end_ms, path: "x.wav".into(), bytes: 10 };
        insert_chunk(&pool, id, &chunk(0, 30_000)).await.unwrap();
        insert_chunk(&pool, id, &chunk(1, 60_000)).await.unwrap();

        // The app restarts without capture_stop ever running.
        assert_eq!(recover(&pool).await.unwrap(), 1);
        let c = &list(&pool).await.unwrap()[0];
        assert_eq!((c.state.as_str(), c.duration_ms, c.chunks), ("interrupted", Some(60_000), 2));
    }
}
