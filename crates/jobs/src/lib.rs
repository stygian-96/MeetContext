//! Durable background jobs: rows in the `jobs` table, run one at a time by a tokio task.

use mc_core::{AppError, now_ms};
use serde::Serialize;
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub const MAX_ATTEMPTS: i32 = 5;

pub type HandlerFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;
/// Called with the job's payload and its attempt number (1 on the first run).
pub type Handler = Arc<dyn Fn(Value, i32) -> HandlerFuture + Send + Sync>;
pub type Handlers = HashMap<&'static str, Handler>;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct JobUpdate {
    pub id: i32,
    pub kind: String,
    pub status: String,
    pub attempts: i32,
    pub last_error: Option<String>,
}

pub async fn enqueue(pool: &SqlitePool, kind: &str, payload: &Value) -> Result<i32, AppError> {
    let now = now_ms();
    Ok(sqlx::query_scalar(
        "INSERT INTO jobs (kind, payload_json, run_after, created_at) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(kind)
    .bind(payload.to_string())
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await?)
}

/// Jobs left `running` by a crash or quit go back in the queue. Call once at startup.
pub async fn recover(pool: &SqlitePool) -> Result<u64, AppError> {
    Ok(sqlx::query("UPDATE jobs SET status = 'queued' WHERE status = 'running'")
        .execute(pool)
        .await?
        .rows_affected())
}

/// Claims and runs the next due job. Returns false when nothing was due.
pub async fn run_next(
    pool: &SqlitePool,
    handlers: &Handlers,
    now: i64,
    on_update: &(impl Fn(JobUpdate) + Sync),
) -> Result<bool, AppError> {
    let claimed: Option<(i32, String, String, i32)> = sqlx::query_as(
        "UPDATE jobs SET status = 'running', attempts = attempts + 1
         WHERE id = (SELECT id FROM jobs WHERE status = 'queued' AND run_after <= ? ORDER BY run_after, id LIMIT 1)
         RETURNING id, kind, payload_json, attempts",
    )
    .bind(now)
    .fetch_optional(pool)
    .await?;
    let Some((id, kind, payload, attempts)) = claimed else { return Ok(false) };

    let update = |status: &str, last_error: Option<String>| JobUpdate {
        id,
        kind: kind.clone(),
        status: status.into(),
        attempts,
        last_error,
    };
    on_update(update("running", None));

    let result = match (handlers.get(kind.as_str()), serde_json::from_str(&payload)) {
        (Some(handler), Ok(payload)) => handler(payload, attempts).await,
        (None, _) => Err(format!("no handler for job kind '{kind}'")),
        (_, Err(e)) => Err(format!("bad payload: {e}")),
    };

    let (status, run_after, error) = match result {
        Ok(()) => ("done", now, None),
        Err(e) if attempts >= MAX_ATTEMPTS => ("failed", now, Some(e)),
        Err(e) => ("queued", now + 1000 * (1i64 << attempts), Some(e)),
    };
    sqlx::query("UPDATE jobs SET status = ?, run_after = ?, last_error = ? WHERE id = ?")
        .bind(status)
        .bind(run_after)
        .bind(&error)
        .bind(id)
        .execute(pool)
        .await?;
    on_update(update(status, error));
    Ok(true)
}

/// Runs forever: one job at a time, checking for due jobs every second when idle.
pub async fn run_worker(pool: SqlitePool, handlers: Handlers, on_update: impl Fn(JobUpdate) + Sync) {
    loop {
        match run_next(&pool, &handlers, now_ms(), &on_update).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(e) => eprintln!("job worker: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        mc_core::migrate(&pool).await.unwrap();
        pool
    }

    fn fails_first(times: i32) -> Handlers {
        let handler: Handler = Arc::new(move |_, attempt| {
            Box::pin(async move { if attempt <= times { Err(format!("fail {attempt}")) } else { Ok(()) } })
        });
        HashMap::from([("test", handler)])
    }

    async fn status(pool: &SqlitePool, id: i32) -> (String, i32) {
        sqlx::query_as("SELECT status, attempts FROM jobs WHERE id = ?").bind(id).fetch_one(pool).await.unwrap()
    }

    // Far enough ahead that any backoff has passed.
    fn later(step: i64) -> i64 {
        now_ms() + step * 3_600_000
    }

    #[tokio::test]
    async fn retries_then_succeeds() {
        let pool = pool().await;
        let id = enqueue(&pool, "test", &json!({})).await.unwrap();
        let handlers = fails_first(2);
        for step in 1..=3 {
            assert!(run_next(&pool, &handlers, later(step), &|_| {}).await.unwrap());
        }
        assert_eq!(status(&pool, id).await, ("done".into(), 3));
        assert!(!run_next(&pool, &handlers, later(4), &|_| {}).await.unwrap());
    }

    #[tokio::test]
    async fn backoff_delays_retry() {
        let pool = pool().await;
        enqueue(&pool, "test", &json!({})).await.unwrap();
        let now = now_ms();
        assert!(run_next(&pool, &fails_first(1), now, &|_| {}).await.unwrap());
        assert!(!run_next(&pool, &fails_first(1), now, &|_| {}).await.unwrap());
    }

    #[tokio::test]
    async fn fails_after_max_attempts() {
        let pool = pool().await;
        let id = enqueue(&pool, "test", &json!({})).await.unwrap();
        let handlers = fails_first(i32::MAX);
        for step in 1..=i64::from(MAX_ATTEMPTS) + 1 {
            run_next(&pool, &handlers, later(step), &|_| {}).await.unwrap();
        }
        assert_eq!(status(&pool, id).await, ("failed".into(), MAX_ATTEMPTS));
    }

    #[tokio::test]
    async fn running_job_survives_restart() {
        let pool = pool().await;
        let id = enqueue(&pool, "test", &json!({})).await.unwrap();
        // Simulate a crash mid-run: the job is left 'running'.
        sqlx::query("UPDATE jobs SET status = 'running', attempts = 1 WHERE id = ?").bind(id).execute(&pool).await.unwrap();
        assert_eq!(recover(&pool).await.unwrap(), 1);
        assert!(run_next(&pool, &fails_first(0), later(1), &|_| {}).await.unwrap());
        assert_eq!(status(&pool, id).await, ("done".into(), 2));
    }
}
