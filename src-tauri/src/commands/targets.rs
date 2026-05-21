use tauri::State;

use crate::operation_log::{
    record_operation_log_best_effort, target_context_from_target_summary, OperationLogEvent,
};
use crate::targets::{
    create_ssh_target_impl, create_wsl_target_impl, delete_target_impl, get_active_target_impl,
    list_wsl_distributions_impl, set_active_target_impl, test_ssh_target_impl,
    test_wsl_target_impl, update_ssh_target_impl, update_ssh_target_password_impl,
    update_wsl_target_impl, CreateSshTargetRequest, CreateWslTargetRequest, SshTargetTestResult,
    TargetKind, TargetSummary, TestSshTargetRequest, TestWslTargetRequest, UpdateSshTargetRequest,
    UpdateWslTargetRequest, WslDistributionSummary, WslTargetTestResult,
};
use crate::AppState;

fn target_kind_string(kind: TargetKind) -> &'static str {
    match kind {
        TargetKind::Local => "local",
        TargetKind::Ssh => "ssh",
        TargetKind::Wsl => "wsl",
    }
}

#[tauri::command]
pub async fn list_targets(state: State<'_, AppState>) -> Result<Vec<TargetSummary>, String> {
    state.targets.list_targets(&state.db).await
}

#[tauri::command]
pub async fn list_wsl_distributions() -> Result<Vec<WslDistributionSummary>, String> {
    list_wsl_distributions_impl().await
}

#[tauri::command]
pub async fn create_ssh_target(
    state: State<'_, AppState>,
    request: CreateSshTargetRequest,
) -> Result<TargetSummary, String> {
    let log_request = request.clone();
    let result = create_ssh_target_impl(&state.targets, &state.db, request).await;
    match &result {
        Ok(target) => {
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(
                    &target.id,
                    target_kind_string(target.kind),
                    &target.label,
                ),
                OperationLogEvent::new(
                    "target",
                    "ssh.create",
                    "succeeded",
                    format!("Created SSH target {}", target.label),
                )
                .subject("target", &target.id, &target.label),
            )
            .await;
        }
        Err(error) => {
            let label = if log_request.label.trim().is_empty() {
                "SSH target".to_string()
            } else {
                log_request.label
            };
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary("ssh:new", "ssh", &label),
                OperationLogEvent::new(
                    "target",
                    "ssh.create",
                    "failed",
                    format!("Failed to create SSH target {}", label),
                )
                .subject("target", "ssh:new", &label)
                .error(error),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn update_ssh_target(
    state: State<'_, AppState>,
    request: UpdateSshTargetRequest,
) -> Result<TargetSummary, String> {
    let log_request = request.clone();
    let result = update_ssh_target_impl(&state.targets, &state.db, request).await;
    match &result {
        Ok(target) => {
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(
                    &target.id,
                    target_kind_string(target.kind),
                    &target.label,
                ),
                OperationLogEvent::new(
                    "target",
                    "ssh.update",
                    "succeeded",
                    format!("Updated SSH target {}", target.label),
                )
                .subject("target", &target.id, &target.label),
            )
            .await;
        }
        Err(error) => {
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(&log_request.id, "ssh", &log_request.label),
                OperationLogEvent::new(
                    "target",
                    "ssh.update",
                    "failed",
                    format!("Failed to update SSH target {}", log_request.label),
                )
                .subject("target", &log_request.id, &log_request.label)
                .error(error),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn test_ssh_target(
    state: State<'_, AppState>,
    request: TestSshTargetRequest,
) -> Result<SshTargetTestResult, String> {
    let log_request = request.clone();
    let target_id = log_request.id.unwrap_or_else(|| "ssh:new".to_string());
    let target_label = log_request
        .label
        .filter(|label| !label.trim().is_empty())
        .unwrap_or_else(|| target_id.clone());
    let result = test_ssh_target_impl(&state.targets, &state.db, request).await;
    match &result {
        Ok(test_result) => {
            let status = if test_result.ok {
                "succeeded"
            } else {
                "failed"
            };
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(&target_id, "ssh", &target_label),
                OperationLogEvent::new("target", "ssh.test", status, test_result.message.clone())
                    .subject("target", &target_id, &target_label),
            )
            .await;
        }
        Err(error) => {
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(&target_id, "ssh", &target_label),
                OperationLogEvent::new(
                    "target",
                    "ssh.test",
                    "failed",
                    format!("Failed to test SSH target {}", target_label),
                )
                .subject("target", &target_id, &target_label)
                .error(error),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn update_ssh_target_password(
    state: State<'_, AppState>,
    target_id: String,
    password: String,
) -> Result<SshTargetTestResult, String> {
    let result =
        update_ssh_target_password_impl(&state.targets, &state.db, &target_id, &password).await;
    match &result {
        Ok(test_result) => {
            let status = if test_result.ok {
                "succeeded"
            } else {
                "failed"
            };
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(&target_id, "ssh", &target_id),
                OperationLogEvent::new(
                    "target",
                    "ssh.password.update",
                    status,
                    test_result.message.clone(),
                )
                .subject("target", &target_id, &target_id),
            )
            .await;
        }
        Err(error) => {
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(&target_id, "ssh", &target_id),
                OperationLogEvent::new(
                    "target",
                    "ssh.password.update",
                    "failed",
                    format!("Failed to update SSH password for {}", target_id),
                )
                .subject("target", &target_id, &target_id)
                .error(error),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn create_wsl_target(
    state: State<'_, AppState>,
    request: CreateWslTargetRequest,
) -> Result<TargetSummary, String> {
    let log_request = request.clone();
    let result = create_wsl_target_impl(&state.targets, &state.db, request).await;
    match &result {
        Ok(target) => {
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(&target.id, "wsl", &target.label),
                OperationLogEvent::new(
                    "target",
                    "wsl.create",
                    "succeeded",
                    format!("Created WSL target {}", target.label),
                )
                .subject("target", &target.id, &target.label),
            )
            .await;
        }
        Err(error) => {
            let label = if log_request.label.trim().is_empty() {
                "WSL target".to_string()
            } else {
                log_request.label
            };
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary("wsl:new", "wsl", &label),
                OperationLogEvent::new(
                    "target",
                    "wsl.create",
                    "failed",
                    format!("Failed to create WSL target {}", label),
                )
                .subject("target", "wsl:new", &label)
                .error(error),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn update_wsl_target(
    state: State<'_, AppState>,
    request: UpdateWslTargetRequest,
) -> Result<TargetSummary, String> {
    let log_request = request.clone();
    let result = update_wsl_target_impl(&state.targets, &state.db, request).await;
    match &result {
        Ok(target) => {
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(&target.id, "wsl", &target.label),
                OperationLogEvent::new(
                    "target",
                    "wsl.update",
                    "succeeded",
                    format!("Updated WSL target {}", target.label),
                )
                .subject("target", &target.id, &target.label),
            )
            .await;
        }
        Err(error) => {
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(&log_request.id, "wsl", &log_request.label),
                OperationLogEvent::new(
                    "target",
                    "wsl.update",
                    "failed",
                    format!("Failed to update WSL target {}", log_request.label),
                )
                .subject("target", &log_request.id, &log_request.label)
                .error(error),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn test_wsl_target(
    state: State<'_, AppState>,
    request: TestWslTargetRequest,
) -> Result<WslTargetTestResult, String> {
    let log_request = request.clone();
    let target_id = log_request.id.unwrap_or_else(|| "wsl:new".to_string());
    let target_label = log_request
        .label
        .filter(|label| !label.trim().is_empty())
        .unwrap_or_else(|| target_id.clone());
    let result = test_wsl_target_impl(&state.db, request).await;
    match &result {
        Ok(test_result) => {
            let status = if test_result.ok {
                "succeeded"
            } else {
                "failed"
            };
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(&target_id, "wsl", &target_label),
                OperationLogEvent::new("target", "wsl.test", status, test_result.message.clone())
                    .subject("target", &target_id, &target_label),
            )
            .await;
        }
        Err(error) => {
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(&target_id, "wsl", &target_label),
                OperationLogEvent::new(
                    "target",
                    "wsl.test",
                    "failed",
                    format!("Failed to test WSL target {}", target_label),
                )
                .subject("target", &target_id, &target_label)
                .error(error),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn delete_target(state: State<'_, AppState>, target_id: String) -> Result<(), String> {
    let result = delete_target_impl(&state.targets, &state.db, &target_id).await;
    let status = if result.is_ok() {
        "succeeded"
    } else {
        "failed"
    };
    let mut event = OperationLogEvent::new(
        "target",
        "target.delete",
        status,
        if result.is_ok() {
            format!("Deleted target {}", target_id)
        } else {
            format!("Failed to delete target {}", target_id)
        },
    )
    .subject("target", &target_id, &target_id);
    if let Err(error) = &result {
        event = event.error(error);
    }
    record_operation_log_best_effort(
        &state.db,
        target_context_from_target_summary(&target_id, "ssh", &target_id),
        event,
    )
    .await;
    result
}

#[tauri::command]
pub async fn set_active_target(
    state: State<'_, AppState>,
    target_id: String,
) -> Result<TargetSummary, String> {
    let result = set_active_target_impl(&state.targets, &state.db, &target_id).await;
    match &result {
        Ok(target) => {
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(
                    &target.id,
                    target_kind_string(target.kind),
                    &target.label,
                ),
                OperationLogEvent::new(
                    "target",
                    "target.switch",
                    "succeeded",
                    format!("Switched active target to {}", target.label),
                )
                .subject("target", &target.id, &target.label),
            )
            .await;
        }
        Err(error) => {
            record_operation_log_best_effort(
                &state.db,
                target_context_from_target_summary(&target_id, "local", &target_id),
                OperationLogEvent::new(
                    "target",
                    "target.switch",
                    "failed",
                    format!("Failed to switch active target to {}", target_id),
                )
                .subject("target", &target_id, &target_id)
                .error(error),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn get_active_target(state: State<'_, AppState>) -> Result<TargetSummary, String> {
    get_active_target_impl(&state.targets, &state.db).await
}
