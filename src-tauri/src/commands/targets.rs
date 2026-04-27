use tauri::State;

use crate::targets::{
    create_ssh_target_impl, delete_target_impl, get_active_target_impl, set_active_target_impl,
    test_ssh_target_impl, update_ssh_target_password_impl, CreateSshTargetRequest,
    SshTargetTestResult, TargetSummary, TestSshTargetRequest,
};
use crate::AppState;

#[tauri::command]
pub async fn list_targets(state: State<'_, AppState>) -> Result<Vec<TargetSummary>, String> {
    state.targets.list_targets(&state.db).await
}

#[tauri::command]
pub async fn create_ssh_target(
    state: State<'_, AppState>,
    request: CreateSshTargetRequest,
) -> Result<TargetSummary, String> {
    create_ssh_target_impl(&state.targets, &state.db, request).await
}

#[tauri::command]
pub async fn test_ssh_target(
    state: State<'_, AppState>,
    request: TestSshTargetRequest,
) -> Result<SshTargetTestResult, String> {
    test_ssh_target_impl(&state.targets, &state.db, request).await
}

#[tauri::command]
pub async fn update_ssh_target_password(
    state: State<'_, AppState>,
    target_id: String,
    password: String,
) -> Result<SshTargetTestResult, String> {
    update_ssh_target_password_impl(&state.targets, &state.db, &target_id, &password).await
}

#[tauri::command]
pub async fn delete_target(state: State<'_, AppState>, target_id: String) -> Result<(), String> {
    delete_target_impl(&state.targets, &state.db, &target_id).await
}

#[tauri::command]
pub async fn set_active_target(
    state: State<'_, AppState>,
    target_id: String,
) -> Result<TargetSummary, String> {
    set_active_target_impl(&state.targets, &state.db, &target_id).await
}

#[tauri::command]
pub async fn get_active_target(state: State<'_, AppState>) -> Result<TargetSummary, String> {
    get_active_target_impl(&state.targets, &state.db).await
}
