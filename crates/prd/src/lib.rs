//! Projects and their Markdown PRDs: versions on every save, stable section IDs.

pub mod sections;

use mc_core::{AppError, now_ms};
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Serialize, specta::Type, sqlx::FromRow)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub description: String,
    /// 0 until the PRD is first saved.
    pub version: i32,
    #[specta(type = i32)] // ms timestamp, exact in a JS number
    pub updated_at: i64,
}

#[derive(Serialize, specta::Type, sqlx::FromRow)]
pub struct SectionInfo {
    pub id: i32,
    pub heading: String,
    pub level: i32,
}

#[derive(Serialize, specta::Type)]
pub struct Prd {
    pub project: Project,
    pub markdown: String,
    pub sections: Vec<SectionInfo>,
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Project>, AppError> {
    Ok(sqlx::query_as(
        "SELECT p.id, p.name, p.description, p.updated_at,
            COALESCE((SELECT MAX(version) FROM prd_versions WHERE project_id = p.id), 0) AS version
         FROM projects p ORDER BY p.name COLLATE NOCASE",
    ).fetch_all(pool).await?)
}

pub async fn create(pool: &SqlitePool, name: &str, description: &str) -> Result<i32, AppError> {
    let now = now_ms();
    Ok(sqlx::query_scalar(
        "INSERT INTO projects (name, description, created_at, updated_at) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(name)
    .bind(description)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await?)
}

pub async fn update(pool: &SqlitePool, id: i32, name: &str, description: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE projects SET name = ?, description = ?, updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(description)
        .bind(now_ms())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get(pool: &SqlitePool, id: i32) -> Result<Prd, AppError> {
    let project: Project = sqlx::query_as(
        "SELECT p.id, p.name, p.description, p.updated_at,
            COALESCE((SELECT MAX(version) FROM prd_versions WHERE project_id = p.id), 0) AS version
         FROM projects p WHERE p.id = ?",
    )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::new("not_found", "no such project"))?;
    let markdown = sqlx::query_scalar("SELECT markdown FROM prd_versions WHERE project_id = ? AND version = ?")
        .bind(id)
        .bind(project.version)
        .fetch_optional(pool)
        .await?
        .unwrap_or_default();
    let sections = sqlx::query_as("SELECT id, heading, level FROM prd_sections WHERE project_id = ? ORDER BY position")
        .bind(id)
        .fetch_all(pool)
        .await?;
    Ok(Prd { project, markdown, sections })
}

/// Saves a new version. Refused with `version_conflict` if someone saved since `base_version`.
pub async fn save(pool: &SqlitePool, id: i32, markdown: &str, base_version: i32, source: &str) -> Result<Prd, AppError> {
    let mut tx = pool.begin().await?;
    let (current, old_md): (i32, String) = sqlx::query_as(
        "SELECT COALESCE(MAX(version), 0), COALESCE((SELECT markdown FROM prd_versions WHERE project_id = ?1 ORDER BY version DESC LIMIT 1), '')
         FROM prd_versions WHERE project_id = ?1",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    if current != base_version {
        return Err(AppError::new("version_conflict", "the PRD changed since you opened it; reload to see the latest"));
    }
    let version = current + 1;
    let now = now_ms();
    sqlx::query("INSERT INTO prd_versions (project_id, version, markdown, source, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(id)
        .bind(version)
        .bind(markdown)
        .bind(source)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE projects SET updated_at = ? WHERE id = ?").bind(now).bind(id).execute(&mut *tx).await?;

    // Old rows are stored in the same order `split` returns the old text's sections.
    let old_rows: Vec<(i32, String, i32)> =
        sqlx::query_as("SELECT id, content_hash, updated_version FROM prd_sections WHERE project_id = ? ORDER BY position")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;
    let (old, new) = (sections::split(&old_md), sections::split(markdown));
    let matches = sections::match_sections(&old, &new);
    let kept: Vec<i32> = matches.iter().flatten().filter_map(|&j| old_rows.get(j).map(|r| r.0)).collect();
    for (row_id, ..) in &old_rows {
        if !kept.contains(row_id) {
            sqlx::query("DELETE FROM prd_sections WHERE id = ?").bind(row_id).execute(&mut *tx).await?;
        }
    }
    for (pos, (s, m)) in new.iter().zip(&matches).enumerate() {
        let hash = sections::hash(&s.text);
        match m.and_then(|j| old_rows.get(j)) {
            Some((row_id, old_hash, old_version)) => {
                let changed_in = if *old_hash == hash { *old_version } else { version };
                sqlx::query("UPDATE prd_sections SET heading = ?, level = ?, position = ?, content_hash = ?, updated_version = ? WHERE id = ?")
                    .bind(&s.heading)
                    .bind(s.level)
                    .bind(pos as i32)
                    .bind(&hash)
                    .bind(changed_in)
                    .bind(row_id)
                    .execute(&mut *tx)
                    .await?;
            }
            None => {
                sqlx::query("INSERT INTO prd_sections (project_id, heading, level, position, content_hash, updated_version) VALUES (?, ?, ?, ?, ?, ?)")
                    .bind(id)
                    .bind(&s.heading)
                    .bind(s.level)
                    .bind(pos as i32)
                    .bind(&hash)
                    .bind(version)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }
    tx.commit().await?;
    get(pool, id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        mc_core::migrate(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn section_ids_persist_across_saves() {
        let pool = pool().await;
        let id = create(&pool, "Checkout", "One-page checkout").await.unwrap();
        let v1 = save(&pool, id, "## Overview\nOne page.\n\n## Scope\nCards.\n", 0, "edit").await.unwrap();
        let ids1: Vec<i32> = v1.sections.iter().map(|s| s.id).collect();

        let v2 = save(&pool, id, "## Scope\nCards and wallets.\n\n## Overview\nOne page.\n\n## Risks\nFraud.\n", 1, "edit").await.unwrap();
        assert_eq!(v2.project.version, 2);
        let ids2: Vec<i32> = v2.sections.iter().map(|s| s.id).collect();
        assert_eq!((ids2[0], ids2[1]), (ids1[1], ids1[0]), "reordered sections keep their IDs");
        assert!(!ids1.contains(&ids2[2]), "a new section gets a new ID");

        let changed: Vec<(String, i32)> = sqlx::query_as("SELECT heading, updated_version FROM prd_sections ORDER BY position")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(changed, [("Scope".into(), 2), ("Overview".into(), 1), ("Risks".into(), 2)]);
    }

    #[tokio::test]
    async fn stale_save_is_refused() {
        let pool = pool().await;
        let id = create(&pool, "Checkout", "").await.unwrap();
        save(&pool, id, "## A\n", 0, "edit").await.unwrap();
        let err = save(&pool, id, "## B\n", 0, "edit").await.err().unwrap();
        assert_eq!(err.code, "version_conflict");
        assert_eq!(get(&pool, id).await.unwrap().markdown, "## A\n");
    }
}
