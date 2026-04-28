use serde_json::Value;
use tauri::State;

use crate::db::{
    self, DbPool, NewOperationLogEntry, OperationLogEntry, OperationLogFilter, OperationLogPage,
};
use crate::targets::{ActiveTarget, LOCAL_TARGET_ID};
use crate::AppState;

#[derive(Debug, Clone)]
pub struct OperationLogTargetContext {
    pub kind: String,
    pub id: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OperationLogEvent {
    pub category: String,
    pub action: String,
    pub status: String,
    pub subject_type: Option<String>,
    pub subject_id: Option<String>,
    pub subject_label: Option<String>,
    pub summary: String,
    pub error_summary: Option<String>,
    pub details_json: Option<String>,
    pub duration_ms: Option<i64>,
    pub batch_id: Option<String>,
}

impl OperationLogEvent {
    pub fn new(category: &str, action: &str, status: &str, summary: impl Into<String>) -> Self {
        Self {
            category: category.to_string(),
            action: action.to_string(),
            status: status.to_string(),
            subject_type: None,
            subject_id: None,
            subject_label: None,
            summary: summary.into(),
            error_summary: None,
            details_json: None,
            duration_ms: None,
            batch_id: None,
        }
    }

    pub fn subject(mut self, subject_type: &str, subject_id: &str, subject_label: &str) -> Self {
        self.subject_type = Some(subject_type.to_string());
        self.subject_id = Some(subject_id.to_string());
        self.subject_label = Some(subject_label.to_string());
        self
    }

    pub fn error(mut self, error: impl AsRef<str>) -> Self {
        self.error_summary = Some(summarize_error(error.as_ref()));
        self
    }

    pub fn details(mut self, details: Value) -> Self {
        self.details_json = Some(sanitize_details_value(details).to_string());
        self
    }

    pub fn duration_ms(mut self, duration_ms: i64) -> Self {
        self.duration_ms = Some(duration_ms.max(0));
        self
    }

    pub fn batch_id(mut self, batch_id: impl Into<String>) -> Self {
        self.batch_id = Some(batch_id.into());
        self
    }
}

#[tauri::command]
pub async fn list_operation_logs(
    state: State<'_, AppState>,
    filter: OperationLogFilter,
) -> Result<OperationLogPage, String> {
    db::list_operation_logs(&state.db, filter).await
}

#[tauri::command]
pub async fn get_operation_log(
    state: State<'_, AppState>,
    log_id: String,
) -> Result<Option<OperationLogEntry>, String> {
    db::get_operation_log(&state.db, &log_id).await
}

#[tauri::command]
pub async fn clear_operation_logs(
    state: State<'_, AppState>,
    filter: OperationLogFilter,
) -> Result<u64, String> {
    db::clear_operation_logs(&state.db, filter).await
}

#[tauri::command]
pub async fn export_operation_logs(
    state: State<'_, AppState>,
    filter: OperationLogFilter,
) -> Result<String, String> {
    db::export_operation_logs_json(&state.db, filter).await
}

pub fn target_context_from_active_target(
    active_target: &ActiveTarget,
) -> OperationLogTargetContext {
    match active_target {
        ActiveTarget::Local => OperationLogTargetContext {
            kind: "local".to_string(),
            id: LOCAL_TARGET_ID.to_string(),
            label: Some("Local".to_string()),
        },
        ActiveTarget::Ssh(target) => OperationLogTargetContext {
            kind: "ssh".to_string(),
            id: target.id.clone(),
            label: Some(target.label.clone()),
        },
    }
}

pub fn target_context_from_target_summary(
    target_id: &str,
    target_kind: &str,
    target_label: &str,
) -> OperationLogTargetContext {
    OperationLogTargetContext {
        kind: target_kind.to_string(),
        id: target_id.to_string(),
        label: Some(target_label.to_string()),
    }
}

pub fn local_target_context() -> OperationLogTargetContext {
    OperationLogTargetContext {
        kind: "local".to_string(),
        id: LOCAL_TARGET_ID.to_string(),
        label: Some("Local".to_string()),
    }
}

pub async fn record_operation_log_best_effort(
    pool: &DbPool,
    target: OperationLogTargetContext,
    event: OperationLogEvent,
) {
    let level = level_for_status(&event.status);
    let entry = NewOperationLogEntry {
        level,
        target_kind: target.kind,
        target_id: target.id,
        target_label: target.label,
        category: event.category,
        action: event.action,
        status: event.status,
        subject_type: event.subject_type,
        subject_id: event.subject_id,
        subject_label: event.subject_label,
        summary: event.summary,
        error_summary: event.error_summary,
        details_json: event.details_json,
        duration_ms: event.duration_ms,
        batch_id: event.batch_id,
    };

    if let Err(error) = db::insert_operation_log(pool, entry).await {
        eprintln!("Failed to record operation log: {}", error);
    }
}

pub fn summarize_error(error: &str) -> String {
    const MAX_ERROR_SUMMARY_CHARS: usize = 500;

    let normalized = error.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= MAX_ERROR_SUMMARY_CHARS {
        return normalized;
    }

    let mut truncated = normalized
        .chars()
        .take(MAX_ERROR_SUMMARY_CHARS.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

pub fn sanitize_details_value(value: Value) -> Value {
    match value {
        Value::Array(items) => {
            Value::Array(items.into_iter().map(sanitize_details_value).collect())
        }
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if is_sensitive_detail_key(&key) {
                        (key, Value::String("[redacted]".to_string()))
                    } else {
                        (key, sanitize_details_value(value))
                    }
                })
                .collect(),
        ),
        other => other,
    }
}

fn level_for_status(status: &str) -> String {
    match status {
        "failed" => "error".to_string(),
        "partial" | "cancelled" => "warn".to_string(),
        _ => "info".to_string(),
    }
}

fn is_sensitive_detail_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "password",
        "passphrase",
        "token",
        "pat",
        "api_key",
        "apikey",
        "secret",
        "private_key",
        "credential",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[test]
    fn summarize_error_collapses_and_truncates_text() {
        let long_error = format!("failed\n{}", "x".repeat(700));
        let summary = summarize_error(&long_error);

        assert!(summary.len() <= 503);
        assert!(!summary.contains('\n'));
        assert!(summary.ends_with('…'));
    }

    #[test]
    fn sanitize_details_redacts_nested_sensitive_fields() {
        let sanitized = sanitize_details_value(serde_json::json!({
            "password": "secret",
            "nested": {
                "apiKey": "secret",
                "safe": "kept"
            }
        }));

        assert_eq!(sanitized["password"], "[redacted]");
        assert_eq!(sanitized["nested"]["apiKey"], "[redacted]");
        assert_eq!(sanitized["nested"]["safe"], "kept");
    }

    #[tokio::test]
    async fn best_effort_logger_does_not_return_business_errors() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        record_operation_log_best_effort(
            &pool,
            local_target_context(),
            OperationLogEvent::new("test", "test.action", "succeeded", "No schema"),
        )
        .await;
    }
}
