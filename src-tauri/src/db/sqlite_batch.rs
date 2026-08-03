//! Shared SQLite bind-budget and text-ID validation helpers.

use std::collections::HashSet;

use sqlx::{QueryBuilder, Sqlite, SqliteConnection};

/// Keep generated statements below SQLite's historical 999-variable floor.
pub(crate) const SQLITE_SAFE_BIND_BUDGET: usize = 900;

#[derive(Clone, Copy)]
pub(crate) enum TextIdTable {
    Skills,
    SkillTags,
}

impl TextIdTable {
    fn sql_name(self) -> &'static str {
        match self {
            Self::Skills => "skills",
            Self::SkillTags => "skill_tags",
        }
    }
}

pub(crate) fn sqlite_rows_per_batch(bindings_per_row: usize) -> Result<usize, sqlx::Error> {
    if bindings_per_row == 0 {
        return Err(sqlx::Error::InvalidArgument(
            "SQLite bindings per row must be greater than zero".to_string(),
        ));
    }

    SQLITE_SAFE_BIND_BUDGET
        .checked_div(bindings_per_row)
        .filter(|rows| *rows > 0)
        .ok_or_else(|| {
            sqlx::Error::InvalidArgument(format!(
                "SQLite row requires {bindings_per_row} bindings, exceeding the safe budget"
            ))
        })
}

pub(crate) async fn validate_text_ids_exist(
    connection: &mut SqliteConnection,
    table: TextIdTable,
    entity: &'static str,
    ids: &[String],
) -> Result<(), sqlx::Error> {
    if ids.is_empty() {
        return Ok(());
    }

    let rows_per_batch = sqlite_rows_per_batch(1)?;
    let mut existing = HashSet::with_capacity(ids.len());
    for chunk in ids.chunks(rows_per_batch) {
        let mut builder = QueryBuilder::<Sqlite>::new(format!(
            "SELECT id FROM {} WHERE id IN (",
            table.sql_name()
        ));
        let mut separated = builder.separated(", ");
        for id in chunk {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");
        existing.extend(
            builder
                .build_query_scalar::<String>()
                .fetch_all(&mut *connection)
                .await?,
        );
    }

    if let Some(missing) = ids.iter().find(|id| !existing.contains(id.as_str())) {
        return Err(sqlx::Error::InvalidArgument(format!(
            "{entity} '{}' not found",
            missing
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_batch_size_is_bounded_and_rejects_invalid_rows() {
        assert_eq!(sqlite_rows_per_batch(6).unwrap(), 150);
        assert!(sqlite_rows_per_batch(0).is_err());
        assert!(sqlite_rows_per_batch(SQLITE_SAFE_BIND_BUDGET + 1).is_err());
    }
}
