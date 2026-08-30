use sqlx::{Sqlite, Transaction};

pub(super) const SOURCE: &str = include_str!("v7.rs");

/// Skills CLI update-center cache, per-skill baseline/observed/pending state,
/// and a recoverable apply journal. Append-only: does not rewrite v1–v6.
pub(crate) async fn apply(transaction: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE skills_cli_update_repositories (
            repository_key TEXT PRIMARY KEY NOT NULL,
            normalized_source TEXT NOT NULL,
            branch TEXT NOT NULL,
            observed_revision_sha TEXT,
            repository_snapshot_digest TEXT,
            etag TEXT,
            status TEXT NOT NULL CHECK (status IN (
                'not_checked', 'current', 'rate_limited', 'failed'
            )),
            last_checked_at TEXT,
            last_attempted_at TEXT,
            last_error_code TEXT,
            rate_limit_remaining INTEGER,
            rate_limit_reset_at TEXT,
            updated_at TEXT NOT NULL
        )",
    )
    .execute(&mut **transaction)
    .await?;

    sqlx::query(
        "CREATE TABLE skills_cli_update_states (
            skill_name TEXT PRIMARY KEY NOT NULL,
            repository_key TEXT,
            normalized_source TEXT,
            skill_path TEXT,
            installed_revision_sha TEXT,
            installed_upstream_digest TEXT,
            installed_local_digest TEXT,
            installed_at TEXT,
            observed_revision_sha TEXT,
            observed_upstream_digest TEXT,
            observed_at TEXT,
            pending_revision_sha TEXT,
            pending_upstream_digest TEXT,
            pending_detected_at TEXT,
            status TEXT NOT NULL CHECK (status IN (
                'not_checked', 'current', 'update_available', 'local_modified',
                'baseline_required', 'unsupported', 'rate_limited', 'failed'
            )),
            last_error_code TEXT,
            is_stale INTEGER NOT NULL DEFAULT 0 CHECK (is_stale IN (0, 1)),
            updated_at TEXT NOT NULL,
            FOREIGN KEY (repository_key)
                REFERENCES skills_cli_update_repositories(repository_key)
        )",
    )
    .execute(&mut **transaction)
    .await?;

    sqlx::query(
        "CREATE INDEX idx_skills_cli_update_states_repo_status
         ON skills_cli_update_states(repository_key, status)",
    )
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "CREATE INDEX idx_skills_cli_update_states_pending
         ON skills_cli_update_states(pending_revision_sha)
         WHERE pending_revision_sha IS NOT NULL",
    )
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "CREATE INDEX idx_skills_cli_update_states_updated
         ON skills_cli_update_states(updated_at)",
    )
    .execute(&mut **transaction)
    .await?;

    sqlx::query(
        "CREATE TABLE skills_cli_update_operations (
            id TEXT PRIMARY KEY NOT NULL,
            singleton INTEGER NOT NULL DEFAULT 1 CHECK (singleton = 1),
            phase TEXT NOT NULL CHECK (phase IN (
                'prepared', 'backups_staged', 'cli_started', 'cli_succeeded',
                'db_committed', 'cleanup_pending', 'completed', 'rolled_back',
                'recovery_required'
            )),
            manifest_version INTEGER NOT NULL CHECK (manifest_version > 0),
            manifest_json TEXT NOT NULL,
            last_error_code TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT
        )",
    )
    .execute(&mut **transaction)
    .await?;

    sqlx::query(
        "CREATE UNIQUE INDEX idx_skills_cli_update_operations_one_active
         ON skills_cli_update_operations(singleton)
         WHERE phase NOT IN ('completed', 'rolled_back')",
    )
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "CREATE INDEX idx_skills_cli_update_operations_phase_updated
         ON skills_cli_update_operations(phase, updated_at)",
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
