//! IPC surface for Operation Log entries and runtime diagnostic logs.
//!
//! The policy for building, sanitizing and persisting log records lives in
//! `crate::operation_log` and `crate::logging`. This file is intentionally
//! thin: it only exposes the read/clear/export commands consumed by the
//! front-end.

use tauri::State;

use crate::db::{
    self, DailyOperationCount, OperationLogEntry, OperationLogFilter, OperationLogPage,
};
use crate::logging::{
    self, FrontendRuntimeLogPayload, RuntimeLogClearRequest, RuntimeLogFile, RuntimeLogReadRequest,
    RuntimeLogReadResult,
};
use crate::observability::{
    CommandLogPolicy, OperationContext, OperationDefinition, OperationSubjectKind, OperationTarget,
    OperationTargetKind, ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeIdentifier,
    SafeOperationResult,
};
use crate::services::central_operation::{
    PendingOperationSummary, PreparedDeleteReconciliationPreview,
};
use crate::AppState;

fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("log command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => unreachable!("log command must have an operation policy"),
    }
}

fn reviewed_failure(definition: OperationDefinition) -> ReviewedFailure {
    ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
}

fn audit_target(target: &crate::targets::ActiveTarget) -> OperationTarget {
    match target {
        crate::targets::ActiveTarget::Local => OperationTarget::local(),
        crate::targets::ActiveTarget::Ssh(target) => {
            OperationTarget::new(OperationTargetKind::Ssh, &target.id)
        }
        crate::targets::ActiveTarget::Wsl(target) => {
            OperationTarget::new(OperationTargetKind::Wsl, &target.id)
        }
    }
}

#[tauri::command]
pub async fn list_operation_logs(
    state: State<'_, AppState>,
    filter: OperationLogFilter,
) -> crate::ipc_error::IpcResult<OperationLogPage> {
    crate::ipc_boundary!(
        "list_operation_logs",
        async move {
            db::list_operation_logs(&state.db, filter)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn get_operation_log(
    state: State<'_, AppState>,
    log_id: String,
) -> crate::ipc_error::IpcResult<Option<OperationLogEntry>> {
    crate::ipc_boundary!(
        "get_operation_log",
        async move {
            db::get_operation_log(&state.db, &log_id)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn clear_operation_logs(
    state: State<'_, AppState>,
    filter: OperationLogFilter,
) -> crate::ipc_error::IpcResult<u64> {
    crate::ipc_boundary!(
        "clear_operation_logs",
        async move {
            let definition = operation_definition("clear_operation_logs");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(OperationTarget::local()),
                |count| {
                    SafeOperationResult::succeeded("Operation logs cleared.")
                        .count(SafeDetailKey::AffectedCount, *count)
                },
                || async {
                    db::clear_operation_logs(&state.db, filter)
                        .await
                        .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn export_operation_logs(
    state: State<'_, AppState>,
    filter: OperationLogFilter,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        "export_operation_logs",
        async move {
            let definition = operation_definition("export_operation_logs");
            match db::export_operation_logs_json(&state.db, filter).await {
                Ok(payload) => {
                    crate::observability::record_terminal(
                        &state.db,
                        definition,
                        OperationContext::new(OperationTarget::local()),
                        SafeOperationResult::succeeded("Operation logs exported."),
                    )
                    .await;
                    Ok(payload)
                }
                Err(_) => {
                    crate::observability::run_operation(
                        &state,
                        definition,
                        OperationContext::new(OperationTarget::local()),
                        |_| SafeOperationResult::succeeded("Operation logs exported."),
                        || async { Err::<String, _>(reviewed_failure(definition)) },
                    )
                    .await
                }
            }
        }
        .await
    )
}

#[tauri::command]
pub async fn list_pending_fs_db_operations(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<PendingOperationSummary>> {
    crate::ipc_boundary!(
        "list_pending_fs_db_operations",
        async move {
            let context = state.resolve_target_context().await?;
            crate::services::central_operation::list_pending_operations(
                context.db(),
                context.target(),
            )
            .await
            .map_err(|error| error.redacted_message())
        }
        .await
    )
}

#[tauri::command]
pub async fn retry_fs_db_operation(
    state: State<'_, AppState>,
    operation_id: String,
) -> crate::ipc_error::IpcResult<Vec<PendingOperationSummary>> {
    crate::ipc_boundary!(
        "retry_fs_db_operation",
        async move {
            let operation_id = parse_operation_id(&operation_id)?;
            let context = state.resolve_target_context().await?;
            let target = context.target().clone();
            let definition = operation_definition("retry_fs_db_operation");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&target)).subject(
                    OperationSubjectKind::Operation,
                    SafeIdentifier::new(&operation_id),
                ),
                |pending: &Vec<PendingOperationSummary>| {
                    SafeOperationResult::succeeded("Pending Central operation retried.")
                        .count(SafeDetailKey::AffectedCount, pending.len() as u64)
                },
                || async {
                    crate::services::central_operation::retry_operation(
                        context.db(),
                        &target,
                        &operation_id,
                    )
                    .await
                    .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn preview_fs_db_operation_reconciliation(
    state: State<'_, AppState>,
    operation_id: String,
) -> crate::ipc_error::IpcResult<PreparedDeleteReconciliationPreview> {
    crate::ipc_boundary!(
        "preview_fs_db_operation_reconciliation",
        async move {
            let operation_id = parse_operation_id(&operation_id)?;
            let context = state.resolve_target_context().await?;
            crate::services::central_operation::preview_prepared_delete_reconciliation(
                context.db(),
                context.target(),
                &operation_id,
            )
            .await
            .map_err(|error| error.redacted_message())
        }
        .await
    )
}

#[tauri::command]
pub async fn reconcile_fs_db_operation(
    state: State<'_, AppState>,
    operation_id: String,
) -> crate::ipc_error::IpcResult<Vec<PendingOperationSummary>> {
    crate::ipc_boundary!(
        "reconcile_fs_db_operation",
        async move {
            let operation_id = parse_operation_id(&operation_id)?;
            let context = state.resolve_target_context().await?;
            let target = context.target().clone();
            let definition = operation_definition("reconcile_fs_db_operation");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&target)).subject(
                    OperationSubjectKind::Operation,
                    SafeIdentifier::new(&operation_id),
                ),
                |pending: &Vec<PendingOperationSummary>| {
                    SafeOperationResult::succeeded("Prepared Central operation reconciled.")
                        .count(SafeDetailKey::AffectedCount, pending.len() as u64)
                },
                || async {
                    crate::services::central_operation::reconcile_prepared_delete(
                        context.db(),
                        &target,
                        &operation_id,
                    )
                    .await
                    .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await
    )
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
) -> crate::ipc_error::IpcResult<Vec<DailyOperationCount>> {
    crate::ipc_boundary!(
        "get_daily_operation_counts",
        async move {
            let today = chrono::Local::now().date_naive();
            db::list_daily_operation_counts(&state.db, today, days)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub fn list_runtime_log_files() -> crate::ipc_error::IpcResult<Vec<RuntimeLogFile>> {
    crate::ipc_boundary!(
        "list_runtime_log_files",
        logging::list_runtime_log_files().map_err(|e| e.to_string())
    )
}

#[tauri::command]
pub fn read_runtime_log_file(
    request: RuntimeLogReadRequest,
) -> crate::ipc_error::IpcResult<RuntimeLogReadResult> {
    crate::ipc_boundary!(
        "read_runtime_log_file",
        logging::read_runtime_log_file(request).map_err(|e| e.to_string())
    )
}

#[tauri::command]
pub async fn export_runtime_log_file(
    state: State<'_, AppState>,
    file_name: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        "export_runtime_log_file",
        async move {
            let definition = operation_definition("export_runtime_log_file");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(OperationTarget::local()),
                |_| SafeOperationResult::succeeded("Runtime log exported."),
                || async {
                    logging::export_runtime_log_file(file_name)
                        .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn clear_runtime_logs(
    state: State<'_, AppState>,
    request: RuntimeLogClearRequest,
) -> crate::ipc_error::IpcResult<u64> {
    crate::ipc_boundary!(
        "clear_runtime_logs",
        async move {
            let definition = operation_definition("clear_runtime_logs");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(OperationTarget::local()),
                |count| {
                    SafeOperationResult::succeeded("Runtime logs cleared.")
                        .count(SafeDetailKey::AffectedCount, *count)
                },
                || async {
                    logging::clear_runtime_logs(request).map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await
    )
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
