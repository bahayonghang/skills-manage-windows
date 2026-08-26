//! Tauri command shell for `scan_all_skills`. Business logic lives in
//! `crate::services::scanner`; this file translates IPC arguments + state into
//! pool/target inputs and records operation logs.

use chrono::Utc;
use std::future::Future;
use std::time::Duration;
use tauri::State;

use crate::db;
use crate::observability::{
    CommandLogPolicy, OperationContext, OperationDefinition, OperationTarget, OperationTargetKind,
    ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeOperationResult,
};
use crate::services::scanner::{scan_all_skills_impl, scan_remote_skills_impl, ScannerError};
use crate::targets::ActiveTarget;
use crate::AppState;

fn operation_definition() -> OperationDefinition {
    match crate::ipc_registry::command_policy("scan_all_skills")
        .expect("scan command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => unreachable!("scan command must have an operation policy"),
    }
}

fn audit_target(target: &ActiveTarget) -> (OperationTargetKind, OperationTarget) {
    match target {
        ActiveTarget::Local => (OperationTargetKind::Local, OperationTarget::local()),
        ActiveTarget::Ssh(target) => (
            OperationTargetKind::Ssh,
            OperationTarget::new(OperationTargetKind::Ssh, &target.id),
        ),
        ActiveTarget::Wsl(target) => (
            OperationTargetKind::Wsl,
            OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        ),
    }
}

// Re-export public types + helpers used by other modules (commands::discover).
// Keeps `super::scanner::parse_skill_md` / `super::scanner::scan_directory`
// call sites in commands/discover.rs working without modification.
pub use crate::services::scanner::{
    detect_link_type, parse_skill_md, parse_skill_md_content, scan_directory, ScanResult,
    ScannedSkill, SkillInfo,
};

async fn run_remote_scan_with_timeout<F>(
    future: F,
    timeout: Duration,
) -> Result<ScanResult, ScannerError>
where
    F: Future<Output = Result<ScanResult, ScannerError>>,
{
    match tokio::time::timeout(timeout, future).await {
        Ok(result) => result,
        Err(_) => Err(ScannerError::Timeout(timeout.as_secs())),
    }
}

/// Tauri command: scan all agent skill directories and persist the results to
/// SQLite. Returns a `ScanResult` with per-agent skill counts.
#[tauri::command]
pub async fn scan_all_skills(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<ScanResult> {
    crate::ipc_boundary!(
        "scan_all_skills",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let (_, audit_target) = audit_target(&active_target);
            let pool = request_context.db().clone();
            let definition = operation_definition();
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target),
                |result: &ScanResult| {
                    SafeOperationResult::succeeded("Skill scan completed.")
                        .count(SafeDetailKey::AffectedCount, result.total_skills as u64)
                        .count(SafeDetailKey::SucceededCount, result.agents_scanned as u64)
                },
                || async {
                    db::set_setting_best_effort(&pool, "scan_state", "refreshing").await;
                    let scan_result = match active_target {
                        ActiveTarget::Local => scan_all_skills_impl(&pool).await,
                        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                            run_remote_scan_with_timeout(
                                scan_remote_skills_impl(&pool, &active_target),
                                Duration::from_secs(90),
                            )
                            .await
                        }
                    };

                    if scan_result.is_ok() {
                        let completed_at = Utc::now().to_rfc3339();
                        db::set_setting_best_effort(&pool, "scan_last_completed_at", &completed_at)
                            .await;
                        db::set_setting_best_effort(&pool, "scan_state", "idle").await;
                    } else {
                        db::set_setting_best_effort(&pool, "scan_state", "error").await;
                    }
                    scan_result.map_err(|_| {
                        ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
                    })
                },
            )
            .await
        }
        .await
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn remote_scan_timeout_returns_timeout_variant() {
        let result = run_remote_scan_with_timeout(
            async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok(ScanResult {
                    total_skills: 0,
                    agents_scanned: 0,
                    skills_by_agent: Default::default(),
                })
            },
            Duration::from_millis(5),
        )
        .await;

        let error = result.unwrap_err();
        assert!(matches!(error, ScannerError::Timeout(_)));
        // User-visible text stays equivalent to the pre-thiserror message.
        assert_eq!(error.to_string(), "Remote skill scan timed out after 0s.");
    }

    #[tokio::test]
    async fn remote_scan_within_timeout_passes_result_through() {
        let result = run_remote_scan_with_timeout(
            async {
                Ok(ScanResult {
                    total_skills: 3,
                    agents_scanned: 1,
                    skills_by_agent: Default::default(),
                })
            },
            Duration::from_secs(5),
        )
        .await;

        assert_eq!(result.unwrap().total_skills, 3);
    }

    #[tokio::test]
    async fn remote_scan_inner_error_is_not_reported_as_timeout() {
        let result = run_remote_scan_with_timeout(
            async { Err(ScannerError::Remote("connection refused".to_string())) },
            Duration::from_secs(5),
        )
        .await;

        let error = result.unwrap_err();
        assert!(!matches!(error, ScannerError::Timeout(_)));
        assert_eq!(error.to_string(), "connection refused");
    }
}
