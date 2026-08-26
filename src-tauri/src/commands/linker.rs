//! Tauri IPC shells for skill install / uninstall operations.
//!
//! Business logic lives in `crate::services::installation::*` (the
//! `install_skill` / `uninstall_skill` orchestration over the
//! `InstallTransport` seam, batch dispatch, project-scoped install). This
//! module is just a thin IPC layer that:
//!
//! 1. Translates `State<AppState>` + arguments into service calls.
//! 2. Runs every mutating command through the registered Operation boundary.
//!
//! Down-stream callers (commands/collections.rs, commands/central_updates.rs)
//! still see the same
//! types and helpers under `commands::linker::*` because of the `pub use`
//! bridge near the top of this file.

use tauri::State;

use crate::observability::{
    CommandLogPolicy, OperationContext, OperationDefinition, OperationTarget, OperationTargetKind,
    ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeOperationResult,
};
use crate::services::installation::{self, InstallOutcome, InstallTransport};
use crate::targets::ActiveTarget;
use crate::AppState;

// Re-export the public surface so existing call-sites under `super::linker::*`
// or `crate::commands::linker::*` (collections / central_updates)
// keep compiling without changes.
pub use crate::services::installation::{
    batch_install_central_skills_impl, batch_uninstall_skills_from_agent_impl, copy_dir_all,
    create_symlink, install_skill, make_relative_path, symlink_target_path, uninstall_skill,
    BatchInstallResult, BatchUninstallSkillFailure, BatchUninstallSkillRequest,
    BatchUninstallSkillResult, BatchUninstallSkillSuccess, CentralBatchInstallFailure,
    CentralBatchInstallResult, CentralBatchInstallSkipped, CentralBatchInstallSuccess,
    FailedInstall, InstallResult, SkippedInstall,
};

fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("linker command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => unreachable!("linker command must have an operation policy"),
    }
}

fn reviewed_failure(definition: OperationDefinition) -> ReviewedFailure {
    ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
}

fn audit_target(target: &ActiveTarget) -> OperationTarget {
    match target {
        ActiveTarget::Local => OperationTarget::local(),
        ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
        ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
    }
}

fn audit_method(method: &str) -> &'static str {
    match method {
        "copy" => "copy",
        "symlink" => "symlink",
        _ => "auto",
    }
}

fn bounded_count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn batch_result(
    requested: usize,
    succeeded: usize,
    skipped: usize,
    failed: usize,
    mode: &'static str,
    success_summary: &'static str,
    partial_summary: &'static str,
) -> SafeOperationResult {
    let result = match installation::batch_operation_status(succeeded, skipped, failed) {
        "succeeded" => SafeOperationResult::succeeded(success_summary),
        _ => SafeOperationResult::partial(partial_summary),
    };
    result
        .count(SafeDetailKey::RequestedCount, bounded_count(requested))
        .count(SafeDetailKey::SucceededCount, bounded_count(succeeded))
        .count(SafeDetailKey::SkippedCount, bounded_count(skipped))
        .count(SafeDetailKey::FailedCount, bounded_count(failed))
        .stable(SafeDetailKey::Mode, mode)
}

fn batch_uninstall_result(
    requested: usize,
    result: &BatchUninstallSkillResult,
) -> SafeOperationResult {
    batch_result(
        requested,
        result.succeeded.len(),
        0,
        result.failed.len(),
        "batch",
        "Skills uninstalled.",
        "Some skills could not be uninstalled.",
    )
}

fn batch_install_result(
    requested: usize,
    result: &BatchInstallResult,
    mode: &'static str,
) -> SafeOperationResult {
    batch_result(
        requested,
        result.succeeded.len(),
        result.skipped.len(),
        result.failed.len(),
        mode,
        "Skill batch install completed.",
        "Skill batch install partially completed.",
    )
}

fn central_batch_install_result(
    requested: usize,
    result: &CentralBatchInstallResult,
    mode: &'static str,
) -> SafeOperationResult {
    batch_result(
        requested,
        result.succeeded.len(),
        result.skipped.len(),
        result.failed.len(),
        mode,
        "Central skill batch install completed.",
        "Central skill batch install partially completed.",
    )
}

/// Tauri command: install a skill to a single agent via relative symlink.
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn install_skill_to_agent(
    state: State<'_, AppState>,
    skill_id: String,
    agent_id: String,
    method: Option<String>,
) -> crate::ipc_error::IpcResult<InstallResult> {
    crate::ipc_boundary!(
        "install_skill_to_agent",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let method = method.as_deref().unwrap_or("auto");
            let definition = operation_definition("install_skill_to_agent");
            let audit_method = audit_method(method);
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)),
                |_| {
                    SafeOperationResult::succeeded("Skill installed.")
                        .count(SafeDetailKey::AffectedCount, 1)
                        .stable(SafeDetailKey::Mode, audit_method)
                },
                || async {
                    let transport = InstallTransport::for_target(&active_target)
                        .await
                        .map_err(|_| reviewed_failure(definition))?;
                    installation::install_skill(&pool, &transport, &skill_id, &agent_id, method)
                        .await
                        .map(InstallOutcome::into_install_result)
                        .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await
    )
}

/// Tauri command: remove a skill's symlink from an agent.
#[tauri::command]
pub async fn uninstall_skill_from_agent(
    state: State<'_, AppState>,
    skill_id: String,
    agent_id: String,
    row_id: Option<String>,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        "uninstall_skill_from_agent",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("uninstall_skill_from_agent");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)),
                |_| {
                    SafeOperationResult::succeeded("Skill uninstalled.")
                        .count(SafeDetailKey::AffectedCount, 1)
                },
                || async {
                    let transport = InstallTransport::for_target(&active_target)
                        .await
                        .map_err(|_| reviewed_failure(definition))?;
                    installation::uninstall_skill(
                        &pool,
                        &transport,
                        &skill_id,
                        &agent_id,
                        row_id.as_deref(),
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

/// Tauri command: remove multiple skills from one agent.
#[tauri::command]
pub async fn batch_uninstall_skills_from_agent(
    state: State<'_, AppState>,
    agent_id: String,
    requests: Vec<BatchUninstallSkillRequest>,
) -> crate::ipc_error::IpcResult<BatchUninstallSkillResult> {
    crate::ipc_boundary!(
        "batch_uninstall_skills_from_agent",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("batch_uninstall_skills_from_agent");
            let requested = requests.len();
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)),
                |result: &BatchUninstallSkillResult| batch_uninstall_result(requested, result),
                || async {
                    let result = match InstallTransport::for_target(&active_target).await {
                        Ok(transport) => {
                            installation::batch_uninstall_skills_from_agent_impl(
                                &pool, &transport, &agent_id, requests,
                            )
                            .await
                        }
                        Err(error) => {
                            let error = error.to_string();
                            BatchUninstallSkillResult {
                                succeeded: Vec::new(),
                                failed: requests_to_failures(requests, &error),
                            }
                        }
                    };
                    Ok::<_, ReviewedFailure>(result)
                },
            )
            .await
        }
        .await
    )
}

fn requests_to_failures(
    requests: Vec<BatchUninstallSkillRequest>,
    error: &str,
) -> Vec<BatchUninstallSkillFailure> {
    requests
        .into_iter()
        .map(|request| BatchUninstallSkillFailure {
            skill_id: request.skill_id,
            row_id: request.row_id,
            error: error.to_string(),
        })
        .collect()
}

/// Tauri command: install a skill to multiple agents in one call.
///
/// `method` must be either `"symlink"` (default, creates a relative symlink) or
/// `"copy"` (copies the skill directory). Each agent install is attempted
/// independently; failures are collected in the `failed` list rather than
/// short-circuiting the entire batch.
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn batch_install_to_agents(
    state: State<'_, AppState>,
    skill_id: String,
    agent_ids: Vec<String>,
    method: Option<String>,
) -> crate::ipc_error::IpcResult<BatchInstallResult> {
    crate::ipc_boundary!(
        "batch_install_to_agents",
        async move {
            let method = method.as_deref().unwrap_or("auto");
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("batch_install_to_agents");
            let requested = agent_ids.len();
            let audit_method = audit_method(method);
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)),
                |result: &BatchInstallResult| batch_install_result(requested, result, audit_method),
                || async {
                    let mut succeeded = Vec::new();
                    let mut skipped = Vec::new();
                    let mut failed = Vec::new();
                    if agent_ids.is_empty() {
                        return Ok::<_, ReviewedFailure>(BatchInstallResult {
                            succeeded,
                            skipped,
                            failed,
                        });
                    }

                    let transport = match InstallTransport::for_target(&active_target).await {
                        Ok(transport) => transport,
                        Err(error) => {
                            let error = error.to_string();
                            failed.extend(agent_ids.iter().map(|agent_id| FailedInstall {
                                agent_id: agent_id.clone(),
                                error: error.clone(),
                            }));
                            return Ok(BatchInstallResult {
                                succeeded,
                                skipped,
                                failed,
                            });
                        }
                    };

                    for agent_id in &agent_ids {
                        match installation::install_skill(
                            &pool, &transport, &skill_id, agent_id, method,
                        )
                        .await
                        {
                            Ok(InstallOutcome::Installed(_)) => succeeded.push(agent_id.clone()),
                            Ok(InstallOutcome::Skipped(item)) => skipped.push(item),
                            Err(error) => failed.push(FailedInstall {
                                agent_id: agent_id.clone(),
                                error: error.to_string(),
                            }),
                        }
                    }

                    Ok(BatchInstallResult {
                        succeeded,
                        skipped,
                        failed,
                    })
                },
            )
            .await
        }
        .await
    )
}

/// Tauri command: install multiple Central skills to multiple platform or project targets.
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn batch_install_central_skills(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
    agent_ids: Vec<String>,
    method: Option<String>,
    project_path: Option<String>,
) -> crate::ipc_error::IpcResult<CentralBatchInstallResult> {
    crate::ipc_boundary!(
        "batch_install_central_skills",
        async move {
            let method = method.as_deref().unwrap_or("auto");
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let project_path = project_path
                .map(|path| path.trim().to_string())
                .filter(|path| !path.is_empty());
            let pool = request_context.db().clone();

            let skill_ids = installation::dedupe_ordered(skill_ids);
            let agent_ids = installation::dedupe_ordered(agent_ids);
            let definition = operation_definition("batch_install_central_skills");
            let requested = skill_ids.len().saturating_mul(agent_ids.len());
            let audit_method = audit_method(method);
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)),
                |result: &CentralBatchInstallResult| {
                    central_batch_install_result(requested, result, audit_method)
                },
                || async {
                    if skill_ids.is_empty() || agent_ids.is_empty() {
                        return Ok::<_, ReviewedFailure>(CentralBatchInstallResult {
                            succeeded: Vec::new(),
                            skipped: Vec::new(),
                            failed: Vec::new(),
                        });
                    }

                    let transport = match InstallTransport::for_target(&active_target).await {
                        Ok(transport) => transport,
                        Err(error) => {
                            let error = error.to_string();
                            let mut failed = Vec::new();
                            for skill_id in &skill_ids {
                                for agent_id in &agent_ids {
                                    failed.push(CentralBatchInstallFailure {
                                        skill_id: skill_id.clone(),
                                        agent_id: agent_id.clone(),
                                        error: error.clone(),
                                    });
                                }
                            }
                            return Ok(CentralBatchInstallResult {
                                succeeded: Vec::new(),
                                skipped: Vec::new(),
                                failed,
                            });
                        }
                    };

                    Ok(installation::batch_install_central_skills_impl(
                        &pool,
                        &transport,
                        skill_ids,
                        agent_ids,
                        method,
                        project_path.as_deref(),
                    )
                    .await)
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
    use crate::db::{self, OperationLogFilter};
    use crate::observability::OperationLifecycle;
    use sqlx::SqlitePool;
    use std::sync::Arc;

    fn test_app_state(pool: SqlitePool) -> AppState {
        AppState {
            db: pool,
            ai_tag_jobs: crate::AiTagJobRegistry::default(),
            central_update_jobs: crate::services::exclusive_job::ExclusiveJobRegistry::new(
                "job.central_update_busy",
                "A Central update job is already running.",
            ),
            central_update_snapshots: crate::CentralUpdateSnapshotCache::default(),
            portable_state_jobs: crate::services::exclusive_job::ExclusiveJobRegistry::new(
                "job.portability_busy",
                "A portability job is already running.",
            ),
            skills_cli_jobs: crate::services::exclusive_job::ExclusiveJobRegistry::new(
                "job.skills_cli_busy",
                "A Skills CLI job is already running.",
            ),
            secrets: Arc::new(crate::secrets::MockSecretStore::default()),
            targets: crate::targets::TargetRegistry::default(),
        }
    }

    #[test]
    fn linker_commands_are_named_started_then_terminal_operations() {
        let source = include_str!("linker.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        for command in [
            "install_skill_to_agent",
            "uninstall_skill_from_agent",
            "batch_uninstall_skills_from_agent",
            "batch_install_to_agents",
            "batch_install_central_skills",
        ] {
            let entry = crate::ipc_registry::command_policy(command).unwrap();
            let CommandLogPolicy::Operation(definition) = entry.policy else {
                panic!("{command} must have an Operation policy");
            };
            assert_eq!(
                definition.lifecycle(),
                OperationLifecycle::StartedThenTerminal
            );
            assert!(production.contains(&format!("\"{command}\"")));
        }
        assert!(!production.contains("OperationLogEvent"));
        assert!(!production.contains("record_operation_log_best_effort"));
        assert!(!production.contains("\"targetPath\""));
        assert!(!production.contains("\"projectPath\""));
    }

    #[tokio::test]
    async fn linker_batch_updates_one_row_without_business_paths_errors_or_users() {
        let pool = crate::test_support::mem_pool().await;
        let state = test_app_state(pool.clone());
        let definition = operation_definition("batch_install_to_agents");
        let planted_user = "agent-private-user";
        let planted_path = r"C:\Users\private\skills\secret";
        let planted_error = "ssh transport stderr ghp_private_token";

        let returned = crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |result: &BatchInstallResult| batch_install_result(3, result, "copy"),
            || async {
                Ok::<_, ReviewedFailure>(BatchInstallResult {
                    succeeded: vec![planted_user.to_string()],
                    skipped: vec![SkippedInstall {
                        agent_id: planted_user.to_string(),
                        target_path: planted_path.to_string(),
                        reason: "already exists".to_string(),
                    }],
                    failed: vec![FailedInstall {
                        agent_id: planted_user.to_string(),
                        error: planted_error.to_string(),
                    }],
                })
            },
        )
        .await
        .unwrap();
        assert_eq!(returned.failed.len(), 1);

        let page = db::list_operation_logs(&pool, OperationLogFilter::default())
            .await
            .unwrap();
        assert_eq!(page.entries.len(), 1, "started must update in place");
        let entry = &page.entries[0];
        assert_eq!(entry.status, "partial");
        let details: serde_json::Value =
            serde_json::from_str(entry.details_json.as_deref().unwrap()).unwrap();
        assert_eq!(details["operationId"], entry.id);
        assert_eq!(details["requestedCount"], 3);
        assert_eq!(details["succeededCount"], 1);
        assert_eq!(details["skippedCount"], 1);
        assert_eq!(details["failedCount"], 1);
        let serialized = serde_json::to_string(entry).unwrap();
        for planted in [planted_user, planted_path, planted_error] {
            assert!(!serialized.contains(planted));
        }
    }

    #[tokio::test]
    async fn linker_failure_correlation_matches_the_single_terminal_row() {
        let pool = crate::test_support::mem_pool().await;
        let state = test_app_state(pool.clone());
        let definition = operation_definition("install_skill_to_agent");
        let planted = r"C:\Users\private ssh stderr ghp_private_token";
        let result = crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |_| SafeOperationResult::succeeded("Skill installed."),
            || async {
                let _raw_transport_error_stays_outside_audit = planted;
                Err::<(), _>(reviewed_failure(definition))
            },
        )
        .await;
        let error = result.unwrap_err();
        let correlation_id = error.correlation_id.unwrap();

        let page = db::list_operation_logs(&pool, OperationLogFilter::default())
            .await
            .unwrap();
        assert_eq!(page.entries.len(), 1, "started must update in place");
        assert_eq!(page.entries[0].id, correlation_id);
        assert_eq!(page.entries[0].status, "failed");
        assert!(!serde_json::to_string(&page.entries[0])
            .unwrap()
            .contains(planted));
    }
}
