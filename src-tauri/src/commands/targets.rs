use tauri::State;

use crate::operation_log::{
    record_operation_log_best_effort, target_context_from_target_summary, OperationLogEvent,
};
use crate::targets::{
    create_ssh_target_impl, create_wsl_target_impl, delete_target_impl, get_active_target_impl,
    get_target_config_quarantine_status_impl, list_wsl_distributions_impl, set_active_target_impl,
    test_ssh_target_impl, test_wsl_target_impl, update_ssh_target_impl,
    update_ssh_target_password_impl, update_wsl_target_impl, CreateSshTargetRequest,
    CreateWslTargetRequest, SshTargetTestResult, TargetConfigQuarantineStatus, TargetKind,
    TargetSummary, TestSshTargetRequest, TestWslTargetRequest, UpdateSshTargetRequest,
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
pub async fn list_targets(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<TargetSummary>> {
    crate::ipc_boundary!(
        async move {
            state
                .targets
                .list_targets(&state.db)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn get_target_config_quarantine_status(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<TargetConfigQuarantineStatus> {
    crate::ipc_boundary!(
        async move {
            get_target_config_quarantine_status_impl(&state.db)
                .await
                .map_err(|error| error.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn list_wsl_distributions() -> crate::ipc_error::IpcResult<Vec<WslDistributionSummary>> {
    crate::ipc_boundary!(
        async move {
            list_wsl_distributions_impl()
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn create_ssh_target(
    state: State<'_, AppState>,
    request: CreateSshTargetRequest,
) -> crate::ipc_error::IpcResult<TargetSummary> {
    crate::ipc_boundary!(
        async move {
            let log_request = request.clone();
            let result = create_ssh_target_impl(&state.targets, &state.db, request)
                .await
                .map_err(|e| e.to_string());
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
        .await
    )
}

#[tauri::command]
pub async fn update_ssh_target(
    state: State<'_, AppState>,
    request: UpdateSshTargetRequest,
) -> crate::ipc_error::IpcResult<TargetSummary> {
    crate::ipc_boundary!(
        async move {
            let log_request = request.clone();
            let result = update_ssh_target_impl(&state.targets, &state.db, request)
                .await
                .map_err(|e| e.to_string());
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
                        target_context_from_target_summary(
                            &log_request.id,
                            "ssh",
                            &log_request.label,
                        ),
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
        .await
    )
}

#[tauri::command]
pub async fn test_ssh_target(
    state: State<'_, AppState>,
    request: TestSshTargetRequest,
) -> crate::ipc_error::IpcResult<SshTargetTestResult> {
    crate::ipc_boundary!(
        async move {
            let log_request = request.clone();
            let target_id = log_request.id.unwrap_or_else(|| "ssh:new".to_string());
            let target_label = log_request
                .label
                .filter(|label| !label.trim().is_empty())
                .unwrap_or_else(|| target_id.clone());
            let result = test_ssh_target_impl(&state.targets, &state.db, request)
                .await
                .map_err(|e| e.to_string());
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
                        OperationLogEvent::new(
                            "target",
                            "ssh.test",
                            status,
                            test_result.message.clone(),
                        )
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
        .await
    )
}

#[tauri::command]
pub async fn update_ssh_target_password(
    state: State<'_, AppState>,
    target_id: String,
    password: String,
) -> crate::ipc_error::IpcResult<SshTargetTestResult> {
    crate::ipc_boundary!(
        async move {
            let result =
                update_ssh_target_password_impl(&state.targets, &state.db, &target_id, &password)
                    .await
                    .map_err(|e| e.to_string());
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
        .await
    )
}

#[tauri::command]
pub async fn create_wsl_target(
    state: State<'_, AppState>,
    request: CreateWslTargetRequest,
) -> crate::ipc_error::IpcResult<TargetSummary> {
    crate::ipc_boundary!(
        async move {
            let log_request = request.clone();
            let result = create_wsl_target_impl(&state.targets, &state.db, request)
                .await
                .map_err(|e| e.to_string());
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
        .await
    )
}

#[tauri::command]
pub async fn update_wsl_target(
    state: State<'_, AppState>,
    request: UpdateWslTargetRequest,
) -> crate::ipc_error::IpcResult<TargetSummary> {
    crate::ipc_boundary!(
        async move {
            let log_request = request.clone();
            let result = update_wsl_target_impl(&state.targets, &state.db, request)
                .await
                .map_err(|e| e.to_string());
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
                        target_context_from_target_summary(
                            &log_request.id,
                            "wsl",
                            &log_request.label,
                        ),
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
        .await
    )
}

#[tauri::command]
pub async fn test_wsl_target(
    state: State<'_, AppState>,
    request: TestWslTargetRequest,
) -> crate::ipc_error::IpcResult<WslTargetTestResult> {
    crate::ipc_boundary!(
        async move {
            let log_request = request.clone();
            let target_id = log_request.id.unwrap_or_else(|| "wsl:new".to_string());
            let target_label = log_request
                .label
                .filter(|label| !label.trim().is_empty())
                .unwrap_or_else(|| target_id.clone());
            let result = test_wsl_target_impl(&state.db, request)
                .await
                .map_err(|e| e.to_string());
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
                        OperationLogEvent::new(
                            "target",
                            "wsl.test",
                            status,
                            test_result.message.clone(),
                        )
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
        .await
    )
}

#[tauri::command]
pub async fn delete_target(
    state: State<'_, AppState>,
    target_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        async move {
            let result = delete_target_impl(&state.targets, &state.db, &target_id)
                .await
                .map_err(|e| e.to_string());
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
        .await
    )
}

#[tauri::command]
pub async fn set_active_target(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    target_id: String,
) -> crate::ipc_error::IpcResult<TargetSummary> {
    crate::ipc_boundary!(
        async move {
            use tauri::Emitter;

            let result = set_active_target_impl(&state.targets, &state.db, &target_id)
                .await
                .map_err(|e| e.to_string());
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
                    // Skill Usage 子系统订阅这个事件做 evict + reload；其他子系统
                    // 也可以监听同一个事件刷新各自的目标维度数据。
                    let _ = app.emit("usage://target-changed", &target.id);
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
        .await
    )
}

#[tauri::command]
pub async fn get_active_target(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<TargetSummary> {
    crate::ipc_boundary!(
        async move {
            get_active_target_impl(&state.targets, &state.db)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}
