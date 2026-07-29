use sqlx::{Row, Sqlite, Transaction};

pub(super) const SOURCE: &str = concat!(
    include_str!("v2.rs"),
    include_str!("../../repos/skill_relations_spec.rs"),
);

pub(crate) async fn apply(transaction: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
    for relation in crate::db::repos::skill_relations_spec::owned_skill_relations() {
        let old_count_sql = format!("SELECT COUNT(*) AS row_count FROM {}", relation.table);
        let old_count = sqlx::query(&old_count_sql)
            .fetch_one(&mut **transaction)
            .await?
            .try_get::<i64, _>("row_count")?;

        sqlx::query(relation.rebuild_sql)
            .execute(&mut **transaction)
            .await?;
        let copy_sql = format!(
            "INSERT INTO __skill_relation_rebuild ({columns}) \
             SELECT {columns} FROM {table}",
            columns = relation.copy_columns,
            table = relation.table,
        );
        sqlx::query(&copy_sql).execute(&mut **transaction).await?;

        let new_count = sqlx::query("SELECT COUNT(*) AS row_count FROM __skill_relation_rebuild")
            .fetch_one(&mut **transaction)
            .await?
            .try_get::<i64, _>("row_count")?;
        if old_count != new_count {
            return Err(sqlx::Error::InvalidArgument(format!(
                "Row count changed while rebuilding {}: {old_count} -> {new_count}",
                relation.table
            )));
        }

        let drop_sql = format!("DROP TABLE {}", relation.table);
        sqlx::query(&drop_sql).execute(&mut **transaction).await?;
        let rename_sql = format!(
            "ALTER TABLE __skill_relation_rebuild RENAME TO {}",
            relation.table
        );
        sqlx::query(&rename_sql).execute(&mut **transaction).await?;
        for index_sql in relation.index_sql {
            sqlx::query(index_sql).execute(&mut **transaction).await?;
        }
    }

    let violation = sqlx::query("PRAGMA foreign_key_check")
        .fetch_optional(&mut **transaction)
        .await?;
    if violation.is_some() {
        return Err(sqlx::Error::InvalidArgument(
            "Foreign key validation failed after owned relation migration".to_string(),
        ));
    }

    Ok(())
}
