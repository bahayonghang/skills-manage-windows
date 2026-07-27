use tauri::{AppHandle, State};

use crate::services::startup::{
    backup_database_set, StartupCoordinator, StartupError, StartupIssue, StartupStatus,
};

#[tauri::command]
pub fn get_startup_status(coordinator: State<'_, StartupCoordinator>) -> StartupStatus {
    coordinator.status()
}

#[tauri::command]
pub async fn retry_startup(
    app: AppHandle,
    coordinator: State<'_, StartupCoordinator>,
) -> Result<StartupStatus, String> {
    let _operation = coordinator.lock_operation().await;
    if coordinator.status() == StartupStatus::Ready {
        return Ok(StartupStatus::Ready);
    }
    Ok(crate::run_startup_attempt(&app, coordinator.inner(), false).await)
}

#[tauri::command]
pub async fn rebuild_startup_database(
    app: AppHandle,
    coordinator: State<'_, StartupCoordinator>,
) -> Result<StartupStatus, String> {
    let _operation = coordinator.lock_operation().await;
    let previous = coordinator.status();
    let diagnostic = match previous {
        StartupStatus::RecoveryRequired {
            diagnostic,
            can_rebuild: true,
            ..
        } => diagnostic,
        _ => {
            return Err(
                "startup.rebuild_unavailable: Database rebuild is not available.".to_string(),
            )
        }
    };

    coordinator.set_status(StartupStatus::Checking);
    match backup_database_set(coordinator.db_path()).await {
        Ok(_) => Ok(crate::run_startup_attempt(&app, coordinator.inner(), true).await),
        Err(error) => {
            tracing::error!(
                code = StartupIssue::DatabaseRecoveryFailed.code(),
                error = %error,
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
}

#[tauri::command]
pub fn exit_startup(app: AppHandle) {
    app.exit(0);
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
