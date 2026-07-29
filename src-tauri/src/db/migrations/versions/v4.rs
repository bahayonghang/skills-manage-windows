use sqlx::{Sqlite, Transaction};

pub(super) const SOURCE: &str = include_str!("v4.rs");

/// Per-skill GitHub import provenance.
///
/// `skill_repositories` rows are shared by every skill imported from the same
/// repository, so a repository-level commit column would overwrite provenance
/// whenever two skills came from different preview snapshots. Provenance
/// therefore lives on the per-skill membership row.
///
/// Both columns are nullable and existing rows stay NULL, which is read as
/// "provenance unknown". This migration is append-only: it must not rewrite the
/// frozen table definition installed by migration 2.
pub(crate) async fn apply(transaction: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query("ALTER TABLE skill_repository_members ADD COLUMN resolved_commit_sha TEXT")
        .execute(&mut **transaction)
        .await?;
    sqlx::query("ALTER TABLE skill_repository_members ADD COLUMN content_digest TEXT")
        .execute(&mut **transaction)
        .await?;
    Ok(())
}
