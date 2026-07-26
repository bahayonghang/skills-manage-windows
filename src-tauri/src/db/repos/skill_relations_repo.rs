//! Shared ownership and repair logic for relations whose parent is `skills`.

use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Row, Sqlite, Transaction};

use crate::db::repos::operation_logs_repo::insert_operation_log_in_transaction;
use crate::db::types::{DbPool, NewOperationLogEntry};

#[derive(Debug, Clone, Copy)]
struct SkillRelationSpec {
    table: &'static str,
    skill_column: &'static str,
}

// Agent observations, project snapshots, usage history, and update inventory
// have separate lifecycles and intentionally stay outside this ownership list.
const OWNED_SKILL_RELATIONS: [SkillRelationSpec; 7] = [
    SkillRelationSpec {
        table: "skill_update_states",
        skill_column: "skill_id",
    },
    SkillRelationSpec {
        table: "skill_repository_members",
        skill_column: "skill_id",
    },
    SkillRelationSpec {
        table: "collection_skills",
        skill_column: "skill_id",
    },
    SkillRelationSpec {
        table: "skill_tag_links",
        skill_column: "skill_id",
    },
    SkillRelationSpec {
        table: "skill_ai_tag_reviews",
        skill_column: "skill_id",
    },
    SkillRelationSpec {
        table: "skill_explanations",
        skill_column: "skill_id",
    },
    SkillRelationSpec {
        table: "skill_installations",
        skill_column: "skill_id",
    },
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrphanRelationReport {
    pub table: String,
    pub skill_ids: Vec<String>,
    pub row_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrphanRepairReport {
    pub relations: Vec<OrphanRelationReport>,
    pub total_rows: u64,
}

pub(crate) async fn delete_owned_skill_relations(
    transaction: &mut Transaction<'_, Sqlite>,
    skill_id: &str,
) -> Result<(), sqlx::Error> {
    for relation in OWNED_SKILL_RELATIONS {
        let sql = format!(
            "DELETE FROM {} WHERE {} = ?",
            relation.table, relation.skill_column
        );
        sqlx::query(&sql)
            .bind(skill_id)
            .execute(&mut **transaction)
            .await?;
    }
    Ok(())
}

pub(crate) async fn delete_owned_skill_relations_not_in(
    transaction: &mut Transaction<'_, Sqlite>,
    keep_skill_ids: &[String],
) -> Result<(), sqlx::Error> {
    for relation in OWNED_SKILL_RELATIONS {
        if keep_skill_ids.is_empty() {
            let sql = format!("DELETE FROM {}", relation.table);
            sqlx::query(&sql).execute(&mut **transaction).await?;
            continue;
        }

        let mut builder = QueryBuilder::<Sqlite>::new(format!(
            "DELETE FROM {} WHERE {} NOT IN (",
            relation.table, relation.skill_column
        ));
        let mut separated = builder.separated(", ");
        for skill_id in keep_skill_ids {
            separated.push_bind(skill_id);
        }
        separated.push_unseparated(")");
        builder.build().execute(&mut **transaction).await?;
    }
    Ok(())
}

pub(crate) async fn delete_owned_skill_relations_missing_from_scan_keep(
    transaction: &mut Transaction<'_, Sqlite>,
) -> Result<(), sqlx::Error> {
    for relation in OWNED_SKILL_RELATIONS {
        let sql = format!(
            "DELETE FROM {table}
             WHERE NOT EXISTS (
               SELECT 1 FROM scan_keep_skills keep
               WHERE keep.skill_id = {table}.{skill_column}
             )",
            table = relation.table,
            skill_column = relation.skill_column,
        );
        sqlx::query(&sql).execute(&mut **transaction).await?;
    }
    Ok(())
}

async fn inventory_orphan_skill_relations(
    transaction: &mut Transaction<'_, Sqlite>,
) -> Result<OrphanRepairReport, sqlx::Error> {
    let mut relations = Vec::new();
    let mut total_rows = 0_u64;

    for relation in OWNED_SKILL_RELATIONS {
        let sql = format!(
            "SELECT relation.{skill_column} AS skill_id, COUNT(*) AS row_count
             FROM {table} relation
             LEFT JOIN skills ON skills.id = relation.{skill_column}
             WHERE skills.id IS NULL
             GROUP BY relation.{skill_column}
             ORDER BY relation.{skill_column}",
            table = relation.table,
            skill_column = relation.skill_column,
        );
        let rows = sqlx::query(&sql).fetch_all(&mut **transaction).await?;
        if rows.is_empty() {
            continue;
        }

        let mut skill_ids = Vec::with_capacity(rows.len());
        let mut relation_rows = 0_u64;
        for row in rows {
            skill_ids.push(row.try_get::<String, _>("skill_id")?);
            let row_count = u64::try_from(row.try_get::<i64, _>("row_count")?).map_err(|_| {
                sqlx::Error::InvalidArgument(format!(
                    "Negative orphan row count for {}",
                    relation.table
                ))
            })?;
            relation_rows = relation_rows.checked_add(row_count).ok_or_else(|| {
                sqlx::Error::InvalidArgument(format!(
                    "Orphan row count overflow for {}",
                    relation.table
                ))
            })?;
        }
        total_rows = total_rows.checked_add(relation_rows).ok_or_else(|| {
            sqlx::Error::InvalidArgument("Orphan repair row count overflow".to_string())
        })?;
        relations.push(OrphanRelationReport {
            table: relation.table.to_string(),
            skill_ids,
            row_count: relation_rows,
        });
    }

    Ok(OrphanRepairReport {
        relations,
        total_rows,
    })
}

async fn delete_orphan_skill_relations(
    transaction: &mut Transaction<'_, Sqlite>,
) -> Result<(), sqlx::Error> {
    for relation in OWNED_SKILL_RELATIONS {
        let sql = format!(
            "DELETE FROM {table}
             WHERE NOT EXISTS (
               SELECT 1 FROM skills
               WHERE skills.id = {table}.{skill_column}
             )",
            table = relation.table,
            skill_column = relation.skill_column,
        );
        sqlx::query(&sql).execute(&mut **transaction).await?;
    }
    Ok(())
}

pub async fn repair_orphan_skill_relations(
    pool: &DbPool,
) -> Result<OrphanRepairReport, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let report = inventory_orphan_skill_relations(&mut transaction).await?;
    if report.total_rows == 0 {
        transaction.commit().await?;
        return Ok(report);
    }

    let details_json =
        serde_json::to_string(&report).map_err(|error| sqlx::Error::Encode(Box::new(error)))?;
    insert_operation_log_in_transaction(
        &mut transaction,
        NewOperationLogEntry {
            level: "info".to_string(),
            target_kind: "local".to_string(),
            target_id: "local".to_string(),
            target_label: None,
            category: "database".to_string(),
            action: "orphan_repair".to_string(),
            status: "succeeded".to_string(),
            subject_type: Some("database".to_string()),
            subject_id: None,
            subject_label: None,
            summary: format!(
                "Removed {} orphaned skill relation rows.",
                report.total_rows
            ),
            error_summary: None,
            details_json: Some(details_json),
            duration_ms: None,
            batch_id: None,
        },
    )
    .await?;
    delete_orphan_skill_relations(&mut transaction).await?;
    transaction.commit().await?;
    Ok(report)
}
