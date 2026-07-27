use sqlx::{Sqlite, Transaction};

pub(super) const SOURCE: &str = include_str!("v3.rs");

pub(crate) async fn apply(transaction: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE fs_db_operations (
            id TEXT PRIMARY KEY NOT NULL,
            batch_id TEXT,
            target_id TEXT NOT NULL,
            target_kind TEXT NOT NULL CHECK (target_kind IN ('local', 'ssh', 'wsl')),
            operation_kind TEXT NOT NULL CHECK (operation_kind IN ('central_delete', 'central_update')),
            skill_id TEXT NOT NULL,
            phase TEXT NOT NULL CHECK (phase IN (
                'prepared', 'fs_staged', 'fs_swapped', 'db_committed',
                'copies_pending', 'completed', 'rolled_back'
            )),
            manifest_version INTEGER NOT NULL CHECK (manifest_version > 0),
            manifest_json TEXT NOT NULL,
            old_fingerprint TEXT,
            new_fingerprint TEXT,
            last_error_code TEXT,
            last_error_message TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT
        )",
    )
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "CREATE INDEX idx_fs_db_operations_target_phase_updated
         ON fs_db_operations(target_id, phase, updated_at)",
    )
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "CREATE INDEX idx_fs_db_operations_batch_skill
         ON fs_db_operations(batch_id, skill_id)",
    )
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "CREATE UNIQUE INDEX idx_fs_db_operations_one_active_skill
         ON fs_db_operations(target_id, skill_id)
         WHERE phase NOT IN ('completed', 'rolled_back')",
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
