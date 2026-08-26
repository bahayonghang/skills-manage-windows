//! Skills CLI update-center v7 tables.
//!
//! Repositories return `sqlx::Error` only. Callers map domain failures.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Sqlite, Transaction};

use crate::db::types::DbPool;

const TERMINAL_OPERATION_PHASES: [&str; 2] = ["completed", "rolled_back"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct SkillsCliUpdateRepositoryRow {
    pub repository_key: String,
    pub normalized_source: String,
    pub branch: String,
    pub observed_revision_sha: Option<String>,
    pub repository_snapshot_digest: Option<String>,
    pub etag: Option<String>,
    pub status: String,
    pub last_checked_at: Option<String>,
    pub last_attempted_at: Option<String>,
    pub last_error_code: Option<String>,
    pub rate_limit_remaining: Option<i64>,
    pub rate_limit_reset_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct SkillsCliUpdateStateRow {
    pub skill_name: String,
    pub repository_key: Option<String>,
    pub normalized_source: Option<String>,
    pub skill_path: Option<String>,
    pub installed_revision_sha: Option<String>,
    pub installed_upstream_digest: Option<String>,
    pub installed_local_digest: Option<String>,
    pub installed_at: Option<String>,
    pub observed_revision_sha: Option<String>,
    pub observed_upstream_digest: Option<String>,
    pub observed_at: Option<String>,
    pub pending_revision_sha: Option<String>,
    pub pending_upstream_digest: Option<String>,
    pub pending_detected_at: Option<String>,
    pub status: String,
    pub last_error_code: Option<String>,
    pub is_stale: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct SkillsCliUpdateOperationRow {
    pub id: String,
    pub singleton: i64,
    pub phase: String,
    pub manifest_version: i64,
    pub manifest_json: String,
    pub last_error_code: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewSkillsCliUpdateOperation<'a> {
    pub id: &'a str,
    pub phase: &'a str,
    pub manifest_version: i64,
    pub manifest_json: &'a str,
}

pub async fn get_update_repository(
    pool: &DbPool,
    repository_key: &str,
) -> Result<Option<SkillsCliUpdateRepositoryRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillsCliUpdateRepositoryRow>(
        "SELECT * FROM skills_cli_update_repositories WHERE repository_key = ?",
    )
    .bind(repository_key)
    .fetch_optional(pool)
    .await
}

pub async fn list_update_repositories(
    pool: &DbPool,
) -> Result<Vec<SkillsCliUpdateRepositoryRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillsCliUpdateRepositoryRow>(
        "SELECT * FROM skills_cli_update_repositories ORDER BY updated_at DESC, repository_key",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_update_state(
    pool: &DbPool,
    skill_name: &str,
) -> Result<Option<SkillsCliUpdateStateRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillsCliUpdateStateRow>(
        "SELECT * FROM skills_cli_update_states WHERE skill_name = ?",
    )
    .bind(skill_name)
    .fetch_optional(pool)
    .await
}

pub async fn list_update_states(
    pool: &DbPool,
) -> Result<Vec<SkillsCliUpdateStateRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillsCliUpdateStateRow>(
        "SELECT * FROM skills_cli_update_states ORDER BY skill_name",
    )
    .fetch_all(pool)
    .await
}

pub async fn upsert_update_repository_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    row: &SkillsCliUpdateRepositoryRow,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO skills_cli_update_repositories (
            repository_key, normalized_source, branch, observed_revision_sha,
            repository_snapshot_digest, etag, status, last_checked_at,
            last_attempted_at, last_error_code, rate_limit_remaining,
            rate_limit_reset_at, updated_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(repository_key) DO UPDATE SET
            normalized_source = excluded.normalized_source,
            branch = excluded.branch,
            observed_revision_sha = COALESCE(excluded.observed_revision_sha, skills_cli_update_repositories.observed_revision_sha),
            repository_snapshot_digest = COALESCE(excluded.repository_snapshot_digest, skills_cli_update_repositories.repository_snapshot_digest),
            etag = COALESCE(excluded.etag, skills_cli_update_repositories.etag),
            status = excluded.status,
            last_checked_at = COALESCE(excluded.last_checked_at, skills_cli_update_repositories.last_checked_at),
            last_attempted_at = excluded.last_attempted_at,
            last_error_code = excluded.last_error_code,
            rate_limit_remaining = excluded.rate_limit_remaining,
            rate_limit_reset_at = excluded.rate_limit_reset_at,
            updated_at = excluded.updated_at",
    )
    .bind(&row.repository_key)
    .bind(&row.normalized_source)
    .bind(&row.branch)
    .bind(&row.observed_revision_sha)
    .bind(&row.repository_snapshot_digest)
    .bind(&row.etag)
    .bind(&row.status)
    .bind(&row.last_checked_at)
    .bind(&row.last_attempted_at)
    .bind(&row.last_error_code)
    .bind(row.rate_limit_remaining)
    .bind(&row.rate_limit_reset_at)
    .bind(&row.updated_at)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

/// Upsert a skill row. `NULL` installed/pending fields preserve the previous
/// baseline; callers that must clear pending pass `clear_pending = true`.
pub async fn upsert_update_state_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    row: &SkillsCliUpdateStateRow,
    overwrite_installed: bool,
    clear_pending: bool,
) -> Result<(), sqlx::Error> {
    let existing = sqlx::query_as::<_, SkillsCliUpdateStateRow>(
        "SELECT * FROM skills_cli_update_states WHERE skill_name = ?",
    )
    .bind(&row.skill_name)
    .fetch_optional(&mut **transaction)
    .await?;

    let merged = match existing {
        Some(previous) => merge_skill_state(previous, row, overwrite_installed, clear_pending),
        None => row.clone(),
    };

    sqlx::query(
        "INSERT INTO skills_cli_update_states (
            skill_name, repository_key, normalized_source, skill_path,
            installed_revision_sha, installed_upstream_digest, installed_local_digest,
            installed_at, observed_revision_sha, observed_upstream_digest, observed_at,
            pending_revision_sha, pending_upstream_digest, pending_detected_at,
            status, last_error_code, is_stale, updated_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(skill_name) DO UPDATE SET
            repository_key = excluded.repository_key,
            normalized_source = excluded.normalized_source,
            skill_path = excluded.skill_path,
            installed_revision_sha = excluded.installed_revision_sha,
            installed_upstream_digest = excluded.installed_upstream_digest,
            installed_local_digest = excluded.installed_local_digest,
            installed_at = excluded.installed_at,
            observed_revision_sha = excluded.observed_revision_sha,
            observed_upstream_digest = excluded.observed_upstream_digest,
            observed_at = excluded.observed_at,
            pending_revision_sha = excluded.pending_revision_sha,
            pending_upstream_digest = excluded.pending_upstream_digest,
            pending_detected_at = excluded.pending_detected_at,
            status = excluded.status,
            last_error_code = excluded.last_error_code,
            is_stale = excluded.is_stale,
            updated_at = excluded.updated_at",
    )
    .bind(&merged.skill_name)
    .bind(&merged.repository_key)
    .bind(&merged.normalized_source)
    .bind(&merged.skill_path)
    .bind(&merged.installed_revision_sha)
    .bind(&merged.installed_upstream_digest)
    .bind(&merged.installed_local_digest)
    .bind(&merged.installed_at)
    .bind(&merged.observed_revision_sha)
    .bind(&merged.observed_upstream_digest)
    .bind(&merged.observed_at)
    .bind(&merged.pending_revision_sha)
    .bind(&merged.pending_upstream_digest)
    .bind(&merged.pending_detected_at)
    .bind(&merged.status)
    .bind(&merged.last_error_code)
    .bind(merged.is_stale)
    .bind(&merged.updated_at)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn merge_skill_state(
    previous: SkillsCliUpdateStateRow,
    incoming: &SkillsCliUpdateStateRow,
    overwrite_installed: bool,
    clear_pending: bool,
) -> SkillsCliUpdateStateRow {
    let mut merged = incoming.clone();
    if !overwrite_installed {
        merged.installed_revision_sha = previous.installed_revision_sha;
        merged.installed_upstream_digest = previous.installed_upstream_digest;
        merged.installed_local_digest = previous.installed_local_digest;
        merged.installed_at = previous.installed_at;
    }
    if clear_pending {
        merged.pending_revision_sha = None;
        merged.pending_upstream_digest = None;
        merged.pending_detected_at = None;
    } else if incoming.pending_revision_sha.is_none() {
        merged.pending_revision_sha = previous.pending_revision_sha;
        merged.pending_upstream_digest = previous.pending_upstream_digest;
        merged.pending_detected_at = previous.pending_detected_at;
    }
    if incoming.observed_revision_sha.is_none() {
        merged.observed_revision_sha = previous.observed_revision_sha;
        merged.observed_upstream_digest = previous.observed_upstream_digest;
        merged.observed_at = previous.observed_at;
    }
    merged
}

pub async fn insert_update_operation(
    pool: &DbPool,
    operation: NewSkillsCliUpdateOperation<'_>,
) -> Result<SkillsCliUpdateOperationRow, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO skills_cli_update_operations (
            id, singleton, phase, manifest_version, manifest_json,
            created_at, updated_at
         ) VALUES (?, 1, ?, ?, ?, ?, ?)",
    )
    .bind(operation.id)
    .bind(operation.phase)
    .bind(operation.manifest_version)
    .bind(operation.manifest_json)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    get_update_operation(pool, operation.id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

pub async fn get_update_operation(
    pool: &DbPool,
    operation_id: &str,
) -> Result<Option<SkillsCliUpdateOperationRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillsCliUpdateOperationRow>(
        "SELECT * FROM skills_cli_update_operations WHERE id = ?",
    )
    .bind(operation_id)
    .fetch_optional(pool)
    .await
}

pub async fn list_pending_update_operations(
    pool: &DbPool,
) -> Result<Vec<SkillsCliUpdateOperationRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillsCliUpdateOperationRow>(
        "SELECT * FROM skills_cli_update_operations
         WHERE phase NOT IN ('completed', 'rolled_back')
         ORDER BY created_at, id",
    )
    .fetch_all(pool)
    .await
}

pub async fn transition_update_operation(
    pool: &DbPool,
    operation_id: &str,
    expected_phase: &str,
    next_phase: &str,
    last_error_code: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    transition_update_operation_in_transaction(
        &mut transaction,
        operation_id,
        expected_phase,
        next_phase,
        last_error_code,
    )
    .await?;
    transaction.commit().await
}

pub async fn transition_update_operation_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    operation_id: &str,
    expected_phase: &str,
    next_phase: &str,
    last_error_code: Option<&str>,
) -> Result<(), sqlx::Error> {
    if expected_phase == next_phase {
        return Ok(());
    }
    let now = Utc::now().to_rfc3339();
    let completed_at = if TERMINAL_OPERATION_PHASES.contains(&next_phase) {
        Some(now.clone())
    } else {
        None
    };
    let result = sqlx::query(
        "UPDATE skills_cli_update_operations
         SET phase = ?, last_error_code = ?, updated_at = ?, completed_at = COALESCE(?, completed_at)
         WHERE id = ? AND phase = ?",
    )
    .bind(next_phase)
    .bind(last_error_code)
    .bind(&now)
    .bind(completed_at)
    .bind(operation_id)
    .bind(expected_phase)
    .execute(&mut **transaction)
    .await?;
    if result.rows_affected() != 1 {
        return Err(sqlx::Error::InvalidArgument(format!(
            "skills_cli update operation {operation_id} was not in phase {expected_phase}"
        )));
    }
    Ok(())
}
