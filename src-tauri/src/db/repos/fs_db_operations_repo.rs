use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Sqlite, Transaction};

use crate::db::repos::repositories_repo::prune_empty_skill_repositories_in_transaction;
use crate::db::types::DbPool;

const TERMINAL_PHASES: [&str; 2] = ["completed", "rolled_back"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct FsDbOperationRow {
    pub id: String,
    pub batch_id: Option<String>,
    pub target_id: String,
    pub target_kind: String,
    pub operation_kind: String,
    pub skill_id: String,
    pub phase: String,
    pub manifest_version: i64,
    pub manifest_json: String,
    pub old_fingerprint: Option<String>,
    pub new_fingerprint: Option<String>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewFsDbOperation<'a> {
    pub id: &'a str,
    pub batch_id: Option<&'a str>,
    pub target_id: &'a str,
    pub target_kind: &'a str,
    pub operation_kind: &'a str,
    pub skill_id: &'a str,
    pub manifest_version: i64,
    pub manifest_json: &'a str,
    pub old_fingerprint: Option<&'a str>,
    pub new_fingerprint: Option<&'a str>,
}

pub async fn insert_fs_db_operation(
    pool: &DbPool,
    operation: NewFsDbOperation<'_>,
) -> Result<FsDbOperationRow, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO fs_db_operations (
            id, batch_id, target_id, target_kind, operation_kind, skill_id, phase,
            manifest_version, manifest_json, old_fingerprint, new_fingerprint,
            created_at, updated_at
         ) VALUES (?, ?, ?, ?, ?, ?, 'prepared', ?, ?, ?, ?, ?, ?)",
    )
    .bind(operation.id)
    .bind(operation.batch_id)
    .bind(operation.target_id)
    .bind(operation.target_kind)
    .bind(operation.operation_kind)
    .bind(operation.skill_id)
    .bind(operation.manifest_version)
    .bind(operation.manifest_json)
    .bind(operation.old_fingerprint)
    .bind(operation.new_fingerprint)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    get_fs_db_operation(pool, operation.id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

pub async fn get_fs_db_operation(
    pool: &DbPool,
    operation_id: &str,
) -> Result<Option<FsDbOperationRow>, sqlx::Error> {
    sqlx::query_as::<_, FsDbOperationRow>("SELECT * FROM fs_db_operations WHERE id = ?")
        .bind(operation_id)
        .fetch_optional(pool)
        .await
}

pub async fn list_pending_fs_db_operations(
    pool: &DbPool,
    target_id: &str,
) -> Result<Vec<FsDbOperationRow>, sqlx::Error> {
    sqlx::query_as::<_, FsDbOperationRow>(
        "SELECT * FROM fs_db_operations
         WHERE target_id = ? AND phase NOT IN ('completed', 'rolled_back')
         ORDER BY created_at, id",
    )
    .bind(target_id)
    .fetch_all(pool)
    .await
}

pub async fn transition_fs_db_operation(
    pool: &DbPool,
    operation_id: &str,
    expected_phase: &str,
    next_phase: &str,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    transition_fs_db_operation_in_transaction(
        &mut transaction,
        operation_id,
        expected_phase,
        next_phase,
    )
    .await?;
    transaction.commit().await
}

pub async fn commit_delete_fs_db_operation(
    pool: &DbPool,
    operation_id: &str,
    skill_id: &str,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("DELETE FROM skills WHERE id = ?")
        .bind(skill_id)
        .execute(&mut *transaction)
        .await?;
    prune_empty_skill_repositories_in_transaction(&mut transaction).await?;
    transition_fs_db_operation_in_transaction(
        &mut transaction,
        operation_id,
        "fs_staged",
        "db_committed",
    )
    .await?;
    transaction.commit().await
}

pub(crate) async fn transition_fs_db_operation_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    operation_id: &str,
    expected_phase: &str,
    next_phase: &str,
) -> Result<(), sqlx::Error> {
    if !phase_transition_permitted(expected_phase, next_phase) {
        return Err(sqlx::Error::InvalidArgument(format!(
            "Invalid operation phase transition: {expected_phase} -> {next_phase}"
        )));
    }
    let now = Utc::now().to_rfc3339();
    let completed_at = TERMINAL_PHASES
        .contains(&next_phase)
        .then_some(now.as_str());
    let result = sqlx::query(
        "UPDATE fs_db_operations
         SET phase = ?, updated_at = ?, completed_at = ?,
             last_error_code = NULL, last_error_message = NULL
         WHERE id = ? AND phase = ?",
    )
    .bind(next_phase)
    .bind(&now)
    .bind(completed_at)
    .bind(operation_id)
    .bind(expected_phase)
    .execute(&mut **transaction)
    .await?;
    if result.rows_affected() != 1 {
        return Err(sqlx::Error::InvalidArgument(format!(
            "Operation {operation_id} is not in expected phase {expected_phase}"
        )));
    }
    Ok(())
}

fn phase_transition_permitted(current: &str, next: &str) -> bool {
    current == next
        || matches!(
            (current, next),
            ("prepared", "fs_staged" | "rolled_back")
                | ("fs_staged", "fs_swapped" | "db_committed" | "rolled_back")
                | ("fs_swapped", "db_committed" | "rolled_back")
                | ("db_committed", "copies_pending" | "completed")
                | ("copies_pending", "completed")
        )
}

pub async fn record_fs_db_operation_error(
    pool: &DbPool,
    operation_id: &str,
    code: &str,
    message: &str,
) -> Result<(), sqlx::Error> {
    let bounded_message: String = message.chars().take(512).collect();
    let result = sqlx::query(
        "UPDATE fs_db_operations
         SET last_error_code = ?, last_error_message = ?, updated_at = ?
         WHERE id = ? AND phase NOT IN ('completed', 'rolled_back')",
    )
    .bind(code)
    .bind(bounded_message)
    .bind(Utc::now().to_rfc3339())
    .bind(operation_id)
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        return Err(sqlx::Error::InvalidArgument(format!(
            "Operation {operation_id} is not pending"
        )));
    }
    Ok(())
}

pub async fn update_fs_db_operation_manifest(
    pool: &DbPool,
    operation_id: &str,
    expected_phase: &str,
    manifest_json: &str,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        "UPDATE fs_db_operations SET manifest_json = ?, updated_at = ?
         WHERE id = ? AND phase = ?",
    )
    .bind(manifest_json)
    .bind(Utc::now().to_rfc3339())
    .bind(operation_id)
    .bind(expected_phase)
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        return Err(sqlx::Error::InvalidArgument(format!(
            "Operation {operation_id} is not in expected phase {expected_phase}"
        )));
    }
    Ok(())
}

pub async fn delete_terminal_fs_db_operations_before(
    pool: &DbPool,
    cutoff: &str,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "DELETE FROM fs_db_operations
         WHERE phase IN ('completed', 'rolled_back') AND updated_at < ?",
    )
    .bind(cutoff)
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool() -> DbPool {
        let pool = crate::db::create_memory_pool_single_conn().await.unwrap();
        crate::db::migrations::initialize_pool(&pool, Vec::new())
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn pending_rows_transition_strictly_and_terminal_cleanup_is_scoped() {
        let pool = pool().await;
        insert_fs_db_operation(
            &pool,
            NewFsDbOperation {
                id: "op-1",
                batch_id: None,
                target_id: "local",
                target_kind: "local",
                operation_kind: "central_delete",
                skill_id: "skill-a",
                manifest_version: 1,
                manifest_json: "{}",
                old_fingerprint: None,
                new_fingerprint: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            list_pending_fs_db_operations(&pool, "local")
                .await
                .unwrap()
                .len(),
            1
        );
        transition_fs_db_operation(&pool, "op-1", "prepared", "fs_staged")
            .await
            .unwrap();
        assert!(
            transition_fs_db_operation(&pool, "op-1", "fs_staged", "completed")
                .await
                .is_err()
        );
        transition_fs_db_operation(&pool, "op-1", "fs_staged", "rolled_back")
            .await
            .unwrap();
        assert!(list_pending_fs_db_operations(&pool, "local")
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            delete_terminal_fs_db_operations_before(&pool, "9999")
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn active_operation_is_unique_per_target_and_skill() {
        let pool = pool().await;
        let row = NewFsDbOperation {
            id: "op-1",
            batch_id: None,
            target_id: "ssh-a",
            target_kind: "ssh",
            operation_kind: "central_update",
            skill_id: "skill-a",
            manifest_version: 1,
            manifest_json: "{}",
            old_fingerprint: None,
            new_fingerprint: None,
        };
        insert_fs_db_operation(&pool, row.clone()).await.unwrap();
        assert!(
            insert_fs_db_operation(&pool, NewFsDbOperation { id: "op-2", ..row })
                .await
                .is_err()
        );
    }
}
