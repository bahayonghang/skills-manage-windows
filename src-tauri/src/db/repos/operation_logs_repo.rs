//! `operation_logs` table CRUD + filter builder — Phase 2c.

use chrono::Utc;
use serde::Serialize;
use sqlx::{QueryBuilder, Row, Sqlite};
use uuid::Uuid;

use crate::db::types::{
    DbPool, NewOperationLogEntry, OperationLogEntry, OperationLogFilter, OperationLogPage,
};

pub(crate) const DEFAULT_OPERATION_LOG_LIMIT: i64 = 100;
pub(crate) const MAX_OPERATION_LOG_LIMIT: i64 = 500;

fn normalize_optional_filter(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_required_log_value(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn operation_log_limit(limit: Option<i64>) -> i64 {
    limit
        .unwrap_or(DEFAULT_OPERATION_LOG_LIMIT)
        .clamp(1, MAX_OPERATION_LOG_LIMIT)
}

fn operation_log_offset(offset: Option<i64>) -> i64 {
    offset.unwrap_or(0).max(0)
}

#[derive(Debug, Clone)]
struct NormalizedOperationLogFilter {
    query: Option<String>,
    target_kind: Option<String>,
    target_id: Option<String>,
    level: Option<String>,
    status: Option<String>,
    category: Option<String>,
    action: Option<String>,
    created_after: Option<String>,
    created_before: Option<String>,
}

impl From<&OperationLogFilter> for NormalizedOperationLogFilter {
    fn from(filter: &OperationLogFilter) -> Self {
        Self {
            query: normalize_optional_filter(&filter.query).map(|value| format!("%{}%", value)),
            target_kind: normalize_optional_filter(&filter.target_kind),
            target_id: normalize_optional_filter(&filter.target_id),
            level: normalize_optional_filter(&filter.level),
            status: normalize_optional_filter(&filter.status),
            category: normalize_optional_filter(&filter.category),
            action: normalize_optional_filter(&filter.action),
            created_after: normalize_optional_filter(&filter.created_after),
            created_before: normalize_optional_filter(&filter.created_before),
        }
    }
}

fn push_operation_log_filters(
    builder: &mut QueryBuilder<'_, Sqlite>,
    filter: &NormalizedOperationLogFilter,
) {
    builder.push(" WHERE 1 = 1");

    if let Some(value) = &filter.target_kind {
        builder.push(" AND target_kind = ").push_bind(value.clone());
    }
    if let Some(value) = &filter.target_id {
        builder.push(" AND target_id = ").push_bind(value.clone());
    }
    if let Some(value) = &filter.level {
        builder.push(" AND level = ").push_bind(value.clone());
    }
    if let Some(value) = &filter.status {
        builder.push(" AND status = ").push_bind(value.clone());
    }
    if let Some(value) = &filter.category {
        builder.push(" AND category = ").push_bind(value.clone());
    }
    if let Some(value) = &filter.action {
        builder.push(" AND action = ").push_bind(value.clone());
    }
    if let Some(value) = &filter.created_after {
        builder.push(" AND created_at >= ").push_bind(value.clone());
    }
    if let Some(value) = &filter.created_before {
        builder.push(" AND created_at <= ").push_bind(value.clone());
    }
    if let Some(value) = &filter.query {
        builder.push(
            " AND (
                summary LIKE ",
        );
        builder.push_bind(value.clone());
        builder
            .push(" OR error_summary LIKE ")
            .push_bind(value.clone());
        builder
            .push(" OR subject_id LIKE ")
            .push_bind(value.clone());
        builder
            .push(" OR subject_label LIKE ")
            .push_bind(value.clone());
        builder.push(" OR action LIKE ").push_bind(value.clone());
        builder.push(")");
    }
}

pub async fn insert_operation_log(
    pool: &DbPool,
    entry: NewOperationLogEntry,
) -> Result<OperationLogEntry, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let level = normalize_required_log_value(&entry.level, "info");
    let target_kind = normalize_required_log_value(&entry.target_kind, "local");
    let target_id = normalize_required_log_value(&entry.target_id, "local");
    let category = normalize_required_log_value(&entry.category, "general");
    let action = normalize_required_log_value(&entry.action, "operation");
    let status = normalize_required_log_value(&entry.status, "succeeded");
    let summary = normalize_required_log_value(&entry.summary, "Operation completed.");

    sqlx::query(
        "INSERT INTO operation_logs (
            id, created_at, level, target_kind, target_id, target_label,
            category, action, status, subject_type, subject_id, subject_label,
            summary, error_summary, details_json, duration_ms, batch_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&created_at)
    .bind(level)
    .bind(target_kind)
    .bind(target_id)
    .bind(entry.target_label)
    .bind(category)
    .bind(action)
    .bind(status)
    .bind(entry.subject_type)
    .bind(entry.subject_id)
    .bind(entry.subject_label)
    .bind(summary)
    .bind(entry.error_summary)
    .bind(entry.details_json)
    .bind(entry.duration_ms)
    .bind(entry.batch_id)
    .execute(pool)
    .await?;

    get_operation_log(pool, &id).await?.ok_or_else(|| {
        sqlx::Error::InvalidArgument("Inserted operation log was not found.".to_string())
    })
}

pub async fn get_operation_log(
    pool: &DbPool,
    log_id: &str,
) -> Result<Option<OperationLogEntry>, sqlx::Error> {
    sqlx::query_as::<_, OperationLogEntry>(
        "SELECT
            id, created_at, level, target_kind, target_id, target_label,
            category, action, status, subject_type, subject_id, subject_label,
            summary, error_summary, details_json, duration_ms, batch_id
         FROM operation_logs
         WHERE id = ?",
    )
    .bind(log_id)
    .fetch_optional(pool)
    .await
}

pub async fn list_operation_logs(
    pool: &DbPool,
    filter: OperationLogFilter,
) -> Result<OperationLogPage, sqlx::Error> {
    let normalized = NormalizedOperationLogFilter::from(&filter);
    let limit = operation_log_limit(filter.limit);
    let offset = operation_log_offset(filter.offset);

    let mut count_builder =
        QueryBuilder::<Sqlite>::new("SELECT COUNT(*) AS cnt FROM operation_logs");
    push_operation_log_filters(&mut count_builder, &normalized);
    let count_row = count_builder.build().fetch_one(pool).await?;
    let total = count_row.try_get::<i64, _>("cnt")?;

    let mut entries_builder = QueryBuilder::<Sqlite>::new(
        "SELECT
            id, created_at, level, target_kind, target_id, target_label,
            category, action, status, subject_type, subject_id, subject_label,
            summary, error_summary, details_json, duration_ms, batch_id
         FROM operation_logs",
    );
    push_operation_log_filters(&mut entries_builder, &normalized);
    entries_builder
        .push(" ORDER BY created_at DESC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);

    let entries = entries_builder
        .build_query_as::<OperationLogEntry>()
        .fetch_all(pool)
        .await?;

    Ok(OperationLogPage {
        entries,
        total,
        limit,
        offset,
    })
}

pub async fn clear_operation_logs(
    pool: &DbPool,
    filter: OperationLogFilter,
) -> Result<u64, sqlx::Error> {
    let normalized = NormalizedOperationLogFilter::from(&filter);
    let mut builder = QueryBuilder::<Sqlite>::new("DELETE FROM operation_logs");
    push_operation_log_filters(&mut builder, &normalized);

    builder
        .build()
        .execute(pool)
        .await
        .map(|result| result.rows_affected())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationLogsExport {
    exported_at: String,
    filter: OperationLogFilter,
    total: i64,
    entries: Vec<OperationLogEntry>,
}

pub async fn export_operation_logs_json(
    pool: &DbPool,
    filter: OperationLogFilter,
) -> Result<String, sqlx::Error> {
    let page = list_operation_logs(
        pool,
        OperationLogFilter {
            limit: Some(MAX_OPERATION_LOG_LIMIT),
            offset: Some(0),
            ..filter.clone()
        },
    )
    .await?;

    serde_json::to_string_pretty(&OperationLogsExport {
        exported_at: Utc::now().to_rfc3339(),
        filter,
        total: page.total,
        entries: page.entries,
    })
    .map_err(|e| sqlx::Error::InvalidArgument(e.to_string()))
}
