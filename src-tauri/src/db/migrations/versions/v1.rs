use sqlx::{Row, Sqlite, SqliteConnection, Transaction};

pub(super) const SOURCE: &str = concat!(
    include_str!("v1.rs"),
    include_str!("../../schema/mod.rs"),
    include_str!("../../schema/core.rs"),
    include_str!("../../schema/collections.rs"),
    include_str!("../../schema/metadata.rs"),
    include_str!("../../schema/discovery.rs"),
    include_str!("../../schema/settings.rs"),
    include_str!("../../schema/marketplace.rs"),
    include_str!("../../schema/saved_views.rs"),
    include_str!("../../schema/projects.rs"),
    include_str!("../../schema/usage.rs"),
);

pub(crate) async fn ensure_column(
    connection: &mut SqliteConnection,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<(), sqlx::Error> {
    let pragma = format!("PRAGMA table_info({table})");
    let rows = sqlx::query(&pragma).fetch_all(&mut *connection).await?;
    let has_column = rows
        .iter()
        .any(|row| row.get::<String, _>("name") == column);
    if !has_column {
        sqlx::query(alter_sql).execute(&mut *connection).await?;
    }
    Ok(())
}

pub(crate) async fn apply(transaction: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE schema_migrations (
            version    INTEGER PRIMARY KEY,
            checksum   TEXT NOT NULL,
            applied_at TEXT NOT NULL
        )",
    )
    .execute(&mut **transaction)
    .await?;

    crate::db::schema::init(transaction).await
}
