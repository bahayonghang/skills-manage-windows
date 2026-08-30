use tauri::{AppHandle, State};

use crate::observability::{
    CommandLogPolicy, OperationContext, OperationDefinition, OperationTarget, ReviewedDiagnostic,
    ReviewedFailure, SafeOperationResult,
};
use crate::services::startup::{
    backup_database_set, StartupCoordinator, StartupError, StartupIssue, StartupStatus,
};
use crate::AppState;

fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("startup command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => unreachable!("startup command must have an operation policy"),
    }
}

fn reviewed_failure(definition: OperationDefinition) -> ReviewedFailure {
    ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
}

fn startup_result(status: &StartupStatus) -> SafeOperationResult {
    if *status == StartupStatus::Ready {
        SafeOperationResult::succeeded("Startup completed.")
    } else {
        SafeOperationResult::partial("Startup requires attention.")
    }
}

#[tauri::command]
pub fn get_startup_status(coordinator: State<'_, StartupCoordinator>) -> StartupStatus {
    coordinator.status()
}

#[tauri::command]
pub async fn retry_startup(
    app: AppHandle,
    state: State<'_, AppState>,
    coordinator: State<'_, StartupCoordinator>,
) -> crate::ipc_error::IpcResult<StartupStatus> {
    crate::ipc_boundary!(
        "retry_startup",
        async move {
            let definition = operation_definition("retry_startup");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(OperationTarget::local()),
                startup_result,
                || async {
                    let _operation = coordinator.lock_operation().await;
                    if coordinator.status() == StartupStatus::Ready {
                        return Ok::<_, ReviewedFailure>(StartupStatus::Ready);
                    }
                    Ok(crate::run_startup_attempt(&app, coordinator.inner(), false).await)
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn rebuild_startup_database(
    app: AppHandle,
    state: State<'_, AppState>,
    coordinator: State<'_, StartupCoordinator>,
) -> crate::ipc_error::IpcResult<StartupStatus> {
    crate::ipc_boundary!(
        "rebuild_startup_database",
        async move {
            let definition = operation_definition("rebuild_startup_database");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(OperationTarget::local()),
                startup_result,
                || async {
                    let _operation = coordinator.lock_operation().await;
                    let previous = coordinator.status();
                    let diagnostic = match previous {
                        StartupStatus::RecoveryRequired {
                            diagnostic,
                            can_rebuild: true,
                            ..
                        } => diagnostic,
                        _ => return Err(reviewed_failure(definition)),
                    };

                    coordinator.set_status(StartupStatus::Checking);
                    match backup_database_set(coordinator.db_path()).await {
                        Ok(_) => {
                            Ok(crate::run_startup_attempt(&app, coordinator.inner(), true).await)
                        }
                        Err(error) => {
                            tracing::error!(
                                code = StartupIssue::DatabaseRecoveryFailed.code(),
                                "Startup database recovery backup failed"
                            );
                            let status = if matches!(error, StartupError::RecoveryRollback { .. }) {
                                StartupStatus::Fatal {
                                    issue: StartupIssue::DatabaseRecoveryFailed,
                                }
                            } else {
                                StartupStatus::RecoveryRequired {
                                    issue: StartupIssue::DatabaseRecoveryFailed,
                                    diagnostic,
                                    can_rebuild: coordinator.db_path().is_file(),
                                    backup_created: false,
                                }
                            };
                            coordinator.set_status(status.clone());
                            Ok(status)
                        }
                    }
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn exit_startup(
    app: AppHandle,
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        "exit_startup",
        async move {
            let definition = operation_definition("exit_startup");
            crate::observability::record_terminal(
                &state.db,
                definition,
                OperationContext::new(OperationTarget::local()),
                SafeOperationResult::succeeded("Startup exit requested."),
            )
            .await;
            app.exit(0);
            Ok::<(), String>(())
        }
        .await
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::startup::StartupDiagnostic;

    #[test]
    fn startup_status_serialization_never_contains_paths_or_internal_errors() {
        let status = StartupStatus::RecoveryRequired {
            issue: StartupIssue::DatabaseOpenFailed,
            diagnostic: StartupDiagnostic::Corrupt,
            can_rebuild: true,
            backup_created: false,
        };

        let value = serde_json::to_value(status).unwrap();

        assert_eq!(value["phase"], "recovery_required");
        assert_eq!(value["issue"], "database_open_failed");
        assert_eq!(value["diagnostic"], "corrupt");
        assert_eq!(value["canRebuild"], true);
        assert_eq!(value["backupCreated"], false);
        let serialized = value.to_string();
        assert!(!serialized.contains("db.sqlite"));
        assert!(!serialized.contains("sqlx"));
    }
}
