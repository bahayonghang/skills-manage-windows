use tauri::State;

use crate::observability::{
    CommandLogPolicy, OperationContext, OperationDefinition, OperationTarget, OperationTargetKind,
    ReviewedDiagnostic, ReviewedFailure, SafeOperationResult,
};
use crate::targets::{
    create_ssh_target_impl, create_wsl_target_impl, delete_target_impl, get_active_target_impl,
    get_target_config_quarantine_status_impl, list_wsl_distributions_impl, set_active_target_impl,
    test_ssh_target_impl, test_wsl_target_impl, update_ssh_target_impl,
    update_ssh_target_password_impl, update_wsl_target_impl, CreateSshTargetRequest,
    CreateWslTargetRequest, SshTargetTestResult, TargetConfigQuarantineStatus, TargetSummary,
    TestSshTargetRequest, TestWslTargetRequest, UpdateSshTargetRequest, UpdateWslTargetRequest,
    WslDistributionSummary, WslTargetTestResult,
};
use crate::AppState;

fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("owned command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => unreachable!("owned command must have an operation policy"),
    }
}

fn reviewed_failure(definition: OperationDefinition) -> ReviewedFailure {
    ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
}

fn audit_target(kind: OperationTargetKind, id: &str) -> OperationTarget {
    OperationTarget::new(kind, id)
}

fn audit_target_from_id(target_id: &str) -> OperationTarget {
    if target_id.starts_with("ssh-") {
        audit_target(OperationTargetKind::Ssh, target_id)
    } else if target_id.starts_with("wsl-") {
        audit_target(OperationTargetKind::Wsl, target_id)
    } else {
        OperationTarget::local()
    }
}

#[tauri::command]
pub async fn list_targets(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<TargetSummary>> {
    crate::ipc_boundary!(
        "list_targets",
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
        "get_target_config_quarantine_status",
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
        "list_wsl_distributions",
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
        "create_ssh_target",
        target_kind = OperationTargetKind::Ssh,
        async move {
            let definition = operation_definition("create_ssh_target");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(OperationTargetKind::Ssh, "ssh-new")),
                |_| SafeOperationResult::succeeded("SSH target created."),
                || async {
                    create_ssh_target_impl(&state.targets, &state.db, request)
                        .await
                        .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await,
    )
}

#[tauri::command]
pub async fn update_ssh_target(
    state: State<'_, AppState>,
    request: UpdateSshTargetRequest,
) -> crate::ipc_error::IpcResult<TargetSummary> {
    crate::ipc_boundary!(
        "update_ssh_target",
        target_kind = OperationTargetKind::Ssh,
        async move {
            let definition = operation_definition("update_ssh_target");
            let context =
                OperationContext::new(audit_target(OperationTargetKind::Ssh, &request.id));
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |_| SafeOperationResult::succeeded("SSH target updated."),
                || async {
                    update_ssh_target_impl(&state.targets, &state.db, request)
                        .await
                        .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await,
    )
}

#[tauri::command]
pub async fn test_ssh_target(
    state: State<'_, AppState>,
    request: TestSshTargetRequest,
) -> crate::ipc_error::IpcResult<SshTargetTestResult> {
    crate::ipc_boundary!(
        "test_ssh_target",
        target_kind = OperationTargetKind::Ssh,
        async move {
            let definition = operation_definition("test_ssh_target");
            let target_id = request.id.as_deref().unwrap_or("ssh-new");
            let context = OperationContext::new(audit_target(OperationTargetKind::Ssh, target_id));
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |result: &SshTargetTestResult| {
                    if result.ok {
                        SafeOperationResult::succeeded("SSH target connection test succeeded.")
                    } else {
                        SafeOperationResult::partial("SSH target connection test did not succeed.")
                    }
                },
                || async {
                    test_ssh_target_impl(&state.targets, &state.db, request)
                        .await
                        .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await,
    )
}

#[tauri::command]
pub async fn update_ssh_target_password(
    state: State<'_, AppState>,
    target_id: String,
    password: String,
) -> crate::ipc_error::IpcResult<SshTargetTestResult> {
    crate::ipc_boundary!(
        "update_ssh_target_password",
        target_kind = OperationTargetKind::Ssh,
        async move {
            let definition = operation_definition("update_ssh_target_password");
            let context = OperationContext::new(audit_target(OperationTargetKind::Ssh, &target_id));
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |result: &SshTargetTestResult| {
                    if result.ok {
                        SafeOperationResult::succeeded("SSH target credential updated.")
                    } else {
                        SafeOperationResult::partial(
                            "SSH target credential update did not succeed.",
                        )
                    }
                },
                || async {
                    update_ssh_target_password_impl(
                        &state.targets,
                        &state.db,
                        &target_id,
                        &password,
                    )
                    .await
                    .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await,
    )
}

#[tauri::command]
pub async fn create_wsl_target(
    state: State<'_, AppState>,
    request: CreateWslTargetRequest,
) -> crate::ipc_error::IpcResult<TargetSummary> {
    crate::ipc_boundary!(
        "create_wsl_target",
        target_kind = OperationTargetKind::Wsl,
        async move {
            let definition = operation_definition("create_wsl_target");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(OperationTargetKind::Wsl, "wsl-new")),
                |_| SafeOperationResult::succeeded("WSL target created."),
                || async {
                    create_wsl_target_impl(&state.targets, &state.db, request)
                        .await
                        .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await,
    )
}

#[tauri::command]
pub async fn update_wsl_target(
    state: State<'_, AppState>,
    request: UpdateWslTargetRequest,
) -> crate::ipc_error::IpcResult<TargetSummary> {
    crate::ipc_boundary!(
        "update_wsl_target",
        target_kind = OperationTargetKind::Wsl,
        async move {
            let definition = operation_definition("update_wsl_target");
            let context =
                OperationContext::new(audit_target(OperationTargetKind::Wsl, &request.id));
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |_| SafeOperationResult::succeeded("WSL target updated."),
                || async {
                    update_wsl_target_impl(&state.targets, &state.db, request)
                        .await
                        .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await,
    )
}

#[tauri::command]
pub async fn test_wsl_target(
    state: State<'_, AppState>,
    request: TestWslTargetRequest,
) -> crate::ipc_error::IpcResult<WslTargetTestResult> {
    crate::ipc_boundary!(
        "test_wsl_target",
        target_kind = OperationTargetKind::Wsl,
        async move {
            let definition = operation_definition("test_wsl_target");
            let target_id = request.id.as_deref().unwrap_or("wsl-new");
            let context = OperationContext::new(audit_target(OperationTargetKind::Wsl, target_id));
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |result: &WslTargetTestResult| {
                    if result.ok {
                        SafeOperationResult::succeeded("WSL target connection test succeeded.")
                    } else {
                        SafeOperationResult::partial("WSL target connection test did not succeed.")
                    }
                },
                || async {
                    test_wsl_target_impl(&state.db, request)
                        .await
                        .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await,
    )
}

#[tauri::command]
pub async fn delete_target(
    state: State<'_, AppState>,
    target_id: String,
) -> crate::ipc_error::IpcResult<()> {
    let target_kind = if target_id.starts_with("ssh-") {
        OperationTargetKind::Ssh
    } else if target_id.starts_with("wsl-") {
        OperationTargetKind::Wsl
    } else {
        OperationTargetKind::Local
    };
    crate::ipc_boundary!(
        "delete_target",
        target_kind = target_kind,
        async move {
            let definition = operation_definition("delete_target");
            let context = OperationContext::new(audit_target_from_id(&target_id));
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |_| SafeOperationResult::succeeded("Target deleted."),
                || async {
                    delete_target_impl(&state.targets, &state.db, &target_id)
                        .await
                        .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await,
    )
}

#[tauri::command]
pub async fn set_active_target(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    target_id: String,
) -> crate::ipc_error::IpcResult<TargetSummary> {
    use tauri::Emitter;

    let target_kind = if target_id.starts_with("ssh-") {
        OperationTargetKind::Ssh
    } else if target_id.starts_with("wsl-") {
        OperationTargetKind::Wsl
    } else {
        OperationTargetKind::Local
    };
    crate::ipc_boundary!(
        "set_active_target",
        target_kind = target_kind,
        async move {
            let definition = operation_definition("set_active_target");
            let context = OperationContext::new(audit_target_from_id(&target_id));
            let result = crate::observability::run_operation(
                &state,
                definition,
                context,
                |_| SafeOperationResult::succeeded("Active target changed."),
                || async {
                    set_active_target_impl(&state.targets, &state.db, &target_id)
                        .await
                        .map_err(|_| reviewed_failure(definition))
                },
            )
            .await;
            if let Ok(target) = &result {
                let _ = app.emit("usage://target-changed", &target.id);
            }
            result
        }
        .await,
    )
}

#[tauri::command]
pub async fn get_active_target(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<TargetSummary> {
    crate::ipc_boundary!(
        "get_active_target",
        async move {
            get_active_target_impl(&state.targets, &state.db)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}
