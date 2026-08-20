use sqlx::{Sqlite, Transaction};

pub(super) const SOURCE: &str = include_str!("v6.rs");

/// Immutable repository identity for Update Center pending additions.
///
/// Existing rows remain NULL and therefore require a fresh inventory before
/// import. New rows bind the user's selection to the full commit and stable
/// repository digest that produced it without persisting repository bytes or
/// credentials.
pub(crate) async fn apply(transaction: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query(
        "ALTER TABLE skill_repository_pending_additions
         ADD COLUMN resolved_commit_sha TEXT",
    )
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "ALTER TABLE skill_repository_pending_additions
         ADD COLUMN snapshot_digest TEXT",
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
