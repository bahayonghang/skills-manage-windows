//! `skill_repository_pending_additions` 表 CRUD —— Phase P2 (Update Mechanism Overhaul)。
//!
//! refresh 阶段 upsert 新发现的远端 skill 候选；apply 阶段按 import/skip/unskip
//! 分支清除对应行；clear 命令用于关闭"更新中心"或用户主动重置。

use crate::db::types::{DbPool, SkillRepositoryPendingAddition};

pub async fn list_pending_additions(
    pool: &DbPool,
) -> Result<Vec<SkillRepositoryPendingAddition>, String> {
    sqlx::query_as::<_, SkillRepositoryPendingAddition>(
        "SELECT *
         FROM skill_repository_pending_additions
         ORDER BY repository_id, discovered_at DESC, source_path",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn list_pending_additions_for_repos(
    pool: &DbPool,
    repository_ids: &[String],
) -> Result<Vec<SkillRepositoryPendingAddition>, String> {
    if repository_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = repository_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT *
         FROM skill_repository_pending_additions
         WHERE repository_id IN ({})
         ORDER BY repository_id, discovered_at DESC, source_path",
        placeholders
    );
    let mut query = sqlx::query_as::<_, SkillRepositoryPendingAddition>(&sql);
    for repository_id in repository_ids {
        query = query.bind(repository_id);
    }

    query.fetch_all(pool).await.map_err(|e| e.to_string())
}

pub async fn upsert_pending_addition(
    pool: &DbPool,
    addition: &SkillRepositoryPendingAddition,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO skill_repository_pending_additions
         (repository_id, source_path, skill_id, skill_name, conflict_existing_skill_id, discovered_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(repository_id, source_path) DO UPDATE SET
           skill_id                   = excluded.skill_id,
           skill_name                 = excluded.skill_name,
           conflict_existing_skill_id = excluded.conflict_existing_skill_id,
           discovered_at              = excluded.discovered_at",
    )
    .bind(&addition.repository_id)
    .bind(&addition.source_path)
    .bind(&addition.skill_id)
    .bind(&addition.skill_name)
    .bind(&addition.conflict_existing_skill_id)
    .bind(&addition.discovered_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub async fn delete_pending_addition(
    pool: &DbPool,
    repository_id: &str,
    source_path: &str,
) -> Result<(), String> {
    sqlx::query(
        "DELETE FROM skill_repository_pending_additions
         WHERE repository_id = ? AND source_path = ?",
    )
    .bind(repository_id)
    .bind(source_path)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub async fn clear_pending_additions(pool: &DbPool) -> Result<(), String> {
    sqlx::query("DELETE FROM skill_repository_pending_additions")
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub async fn prune_orphaned_pending_additions(pool: &DbPool) -> Result<u64, String> {
    sqlx::query(
        "DELETE FROM skill_repository_pending_additions
         WHERE NOT EXISTS (
           SELECT 1 FROM skill_repositories
           WHERE skill_repositories.id = skill_repository_pending_additions.repository_id
         )",
    )
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
    .map_err(|e| e.to_string())
}

pub async fn clear_pending_additions_for_repos(
    pool: &DbPool,
    repository_ids: &[String],
) -> Result<(), String> {
    if repository_ids.is_empty() {
        return Ok(());
    }

    let placeholders = repository_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "DELETE FROM skill_repository_pending_additions WHERE repository_id IN ({})",
        placeholders
    );
    let mut query = sqlx::query(&sql);
    for repository_id in repository_ids {
        query = query.bind(repository_id);
    }
    query
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub async fn clear_pending_additions_for_skill_ids(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<(), String> {
    if skill_ids.is_empty() {
        return Ok(());
    }

    let placeholders = skill_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "DELETE FROM skill_repository_pending_additions WHERE skill_id IN ({})",
        placeholders
    );
    let mut query = sqlx::query(&sql);
    for skill_id in skill_ids {
        query = query.bind(skill_id);
    }
    query
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}
