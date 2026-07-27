//! IPC surface for Operation Log entries and runtime diagnostic logs.
//!
//! The policy for building, sanitizing and persisting log records lives in
//! `crate::operation_log` and `crate::logging`. This file is intentionally
//! thin: it only exposes the read/clear/export commands consumed by the
//! front-end.

use std::time::Instant;

use serde_json::json;
use tauri::State;

use crate::db::{
    self, DailyOperationCount, OperationLogEntry, OperationLogFilter, OperationLogPage,
};
use crate::logging::{
    self, FrontendRuntimeLogPayload, RuntimeLogClearRequest, RuntimeLogFile, RuntimeLogReadRequest,
    RuntimeLogReadResult,
};
use crate::operation_log::{
    record_operation_log_best_effort, target_context_from_active_target, OperationLogEvent,
};
use crate::services::central_operation::PendingOperationSummary;
use crate::AppState;

#[tauri::command]
pub async fn list_operation_logs(
    state: State<'_, AppState>,
    filter: OperationLogFilter,
) -> Result<OperationLogPage, String> {
    db::list_operation_logs(&state.db, filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_operation_log(
    state: State<'_, AppState>,
    log_id: String,
) -> Result<Option<OperationLogEntry>, String> {
    db::get_operation_log(&state.db, &log_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_operation_logs(
    state: State<'_, AppState>,
    filter: OperationLogFilter,
) -> Result<u64, String> {
    db::clear_operation_logs(&state.db, filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_operation_logs(
    state: State<'_, AppState>,
    filter: OperationLogFilter,
) -> Result<String, String> {
    db::export_operation_logs_json(&state.db, filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_pending_fs_db_operations(
    state: State<'_, AppState>,
) -> Result<Vec<PendingOperationSummary>, String> {
    let context = state.resolve_target_context().await?;
    crate::services::central_operation::list_pending_operations(context.db(), context.target())
        .await
        .map_err(|error| error.redacted_message())
}

#[tauri::command]
pub async fn retry_fs_db_operation(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<Vec<PendingOperationSummary>, String> {
    let operation_id = parse_operation_id(&operation_id)?;
    let context = state.resolve_target_context().await?;
    let target = context.target().clone();
    let started = Instant::now();
    let result =
        crate::services::central_operation::retry_operation(context.db(), &target, &operation_id)
            .await
            .map_err(|error| error.redacted_message());
    let status = if result.is_ok() {
        "succeeded"
    } else {
        "failed"
    };
    let mut event = OperationLogEvent::new(
        "recovery",
        "central.operation_recovery",
        status,
        if result.is_ok() {
            "Recovered a pending Central operation"
        } else {
            "Failed to recover a pending Central operation"
        },
    )
    .subject("operation", &operation_id, "Central operation recovery")
    .details(json!({
        "operationId": operation_id,
        "pendingCount": result.as_ref().ok().map(Vec::len),
    }))
    .duration_ms(started.elapsed().as_millis() as i64);
    if let Err(error) = &result {
        event = event.error(error);
    }
    record_operation_log_best_effort(&state.db, target_context_from_active_target(&target), event)
        .await;
    result
}

fn parse_operation_id(value: &str) -> Result<String, String> {
    uuid::Uuid::parse_str(value)
        .map(|value| value.to_string())
        .map_err(|_| "Central operation identity is invalid".to_string())
}

/// 仪表盘每日操作数聚合：窗口为本机今天起向前 `days - 1` 天，按本地日历日
/// 分桶并零值填充。`days` 由 repo 层 clamp 到 1..=60。
#[tauri::command]
pub async fn get_daily_operation_counts(
    state: State<'_, AppState>,
    days: u32,
) -> Result<Vec<DailyOperationCount>, String> {
    let today = chrono::Local::now().date_naive();
    db::list_daily_operation_counts(&state.db, today, days)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_runtime_log_files() -> Result<Vec<RuntimeLogFile>, String> {
    logging::list_runtime_log_files().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_runtime_log_file(
    request: RuntimeLogReadRequest,
) -> Result<RuntimeLogReadResult, String> {
    logging::read_runtime_log_file(request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_runtime_log_file(file_name: String) -> Result<String, String> {
    logging::export_runtime_log_file(file_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_runtime_logs(request: RuntimeLogClearRequest) -> Result<u64, String> {
    logging::clear_runtime_logs(request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn record_frontend_runtime_log(payload: FrontendRuntimeLogPayload) {
    logging::record_frontend_runtime_log(payload);
}

#[cfg(test)]
mod tests {
    use super::parse_operation_id;

    #[test]
    fn retry_operation_id_rejects_untrusted_diagnostic_text() {
        assert_eq!(
            parse_operation_id("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        let error = parse_operation_id("C:/Users/private/manifest.json?token=secret").unwrap_err();
        assert_eq!(error, "Central operation identity is invalid");
        assert!(!error.contains("private"));
        assert!(!error.contains("secret"));
    }
}
