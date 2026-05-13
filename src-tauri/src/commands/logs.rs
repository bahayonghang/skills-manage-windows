//! IPC surface for Operation Log entries.
//!
//! The policy for building, sanitizing and persisting log records lives in
//! `crate::operation_log`. This file is intentionally thin: it only exposes
//! the four read/clear/export commands consumed by the front-end.

use tauri::State;

use crate::db::{self, OperationLogEntry, OperationLogFilter, OperationLogPage};
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
