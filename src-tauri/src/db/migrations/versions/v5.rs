use sqlx::{Sqlite, Transaction};

pub(super) const SOURCE: &str = include_str!("v5.rs");

/// Incremental skill-usage scan cache.
///
/// One row per scanned provider log file. The `(mtime_ms, size)` fingerprint
/// lets refresh skip files that did not change on disk; `calls_json` holds the
/// `SkillCall[]` parsed from that file so unchanged files are never re-read.
/// Vanished files get their rows deleted; changed or new files are re-parsed
/// and upserted.
///
/// Contract notes:
/// - This is derived, rebuildable cache. `skill_calls` stays the pure fact
///   table; file paths live exclusively in this table and must not flow into
///   logs, IPC payloads, or state exports (redaction policy).
/// - Append-only migration: it creates the cache table and touches no
///   existing table.
pub(crate) async fn apply(transaction: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE skill_call_file_cache (
            target_id     TEXT NOT NULL,
            provider      TEXT NOT NULL,
            file_path     TEXT NOT NULL,
            mtime_ms      INTEGER NOT NULL,
            size          INTEGER NOT NULL,
            calls_json    TEXT NOT NULL,
            scanned_at_ms INTEGER NOT NULL,
            PRIMARY KEY (target_id, provider, file_path)
        )",
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
