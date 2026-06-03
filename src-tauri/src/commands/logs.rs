//! IPC surface for Operation Log entries and runtime diagnostic logs.
//!
//! The policy for building, sanitizing and persisting log records lives in
//! `crate::operation_log` and `crate::logging`. This file is intentionally
//! thin: it only exposes the read/clear/export commands consumed by the
//! front-end.

use tauri::State;

use crate::db::{self, OperationLogEntry, OperationLogFilter, OperationLogPage};
use crate::logging::{
    self, FrontendRuntimeLogPayload, RuntimeLogClearRequest, RuntimeLogFile, RuntimeLogReadRequest,
    RuntimeLogReadResult,
};
use crate::AppState;

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

#[tauri::command]
pub fn list_runtime_log_files() -> Result<Vec<RuntimeLogFile>, String> {
    logging::list_runtime_log_files()
}

#[tauri::command]
pub fn read_runtime_log_file(
    request: RuntimeLogReadRequest,
) -> Result<RuntimeLogReadResult, String> {
    logging::read_runtime_log_file(request)
}

#[tauri::command]
pub fn export_runtime_log_file(file_name: String) -> Result<String, String> {
    logging::export_runtime_log_file(file_name)
}

#[tauri::command]
pub fn clear_runtime_logs(request: RuntimeLogClearRequest) -> Result<u64, String> {
    logging::clear_runtime_logs(request)
}

#[tauri::command]
pub fn record_frontend_runtime_log(payload: FrontendRuntimeLogPayload) {
    logging::record_frontend_runtime_log(payload);
}
