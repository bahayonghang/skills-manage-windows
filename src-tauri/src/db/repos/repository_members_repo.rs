//! `skill_repository_members` reads used outside repository CRUD.
//!
//! The membership writes and the repository CRUD they belong to live in
//! `repositories_repo`; this module holds the lookups added for update
//! reconciliation.

use crate::db::types::DbPool;

/// Which central skill currently claims `source_path` inside a repository.
///
/// Used before reattaching a moved skill so an occupied path is never taken
/// away from the skill that already tracks it.
pub async fn get_skill_id_for_repository_source_path(
    pool: &DbPool,
    repository_id: &str,
    source_path: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT skill_id FROM skill_repository_members
         WHERE repository_id = ? AND source_path = ?",
    )
    .bind(repository_id)
    .bind(source_path)
    .fetch_optional(pool)
    .await
}
