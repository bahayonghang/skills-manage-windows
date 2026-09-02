//! Tauri IPC shells for Central metadata and AI tagging commands.
//!
//! Repository/tag CRUD stays in this command module. AI tagging orchestration
//! lives in `crate::services::ai_tagging` and is re-exported here to keep IPC
//! payload type paths stable.

use std::sync::{atomic::Ordering, Arc};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::{
    db::{
        self, CentralTopTag, SkillAiTagReview, SkillRepository, SkillRepositoryWithStats, SkillTag,
    },
    observability::{
        CommandLogPolicy, OperationContext, OperationSubjectKind, OperationTarget,
        OperationTargetKind, ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeIdentifier,
        SafeOperationResult,
    },
    services::ai_tagging,
    targets::ActiveTarget,
    AppState,
};

use crate::services::ai_tagging::AI_TAG_PROGRESS_EVENT;
pub use crate::services::ai_tagging::{
    AiTagProgressPayload, AiTagProgressStatus, SkillTagSuggestion, SkillTagSuggestionResult,
};

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn get_skill_repositories(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<SkillRepositoryWithStats>> {
    crate::ipc_boundary!(
        "get_skill_repositories",
        async move {
            let pool = state.active_db().await?;
            db::get_skill_repositories_with_stats(&pool)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
#[allow(clippy::too_many_arguments)]
pub async fn create_or_update_skill_repository(
    state: State<'_, AppState>,
    id: Option<String>,
    name: String,
    source_type: Option<String>,
    owner: Option<String>,
    repo: Option<String>,
    branch: Option<String>,
    url: Option<String>,
    is_unknown: Option<bool>,
) -> crate::ipc_error::IpcResult<SkillRepository> {
    crate::ipc_boundary_async!("create_or_update_skill_repository", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("create_or_update_skill_repository")
            .expect("create_or_update_skill_repository must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("create_or_update_skill_repository must be auditable")
        };
        let context = match id.as_deref() {
            Some(id) => OperationContext::new(audit_target)
                .subject(OperationSubjectKind::Repository, SafeIdentifier::new(id)),
            None => OperationContext::new(audit_target),
        };
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |repository: &SkillRepository| {
                SafeOperationResult::succeeded("Skill repository saved.").identifier(
                    SafeDetailKey::Identifier,
                    SafeIdentifier::new(&repository.id),
                )
            },
            || async move {
                db::create_or_update_skill_repository(
                    &pool,
                    id.as_deref(),
                    &name,
                    source_type.as_deref().unwrap_or("manual"),
                    owner.as_deref(),
                    repo.as_deref(),
                    branch.as_deref(),
                    url.as_deref(),
                    is_unknown.unwrap_or(false),
                )
                .await
                .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn assign_skills_to_repository(
    state: State<'_, AppState>,
    repository_id: String,
    skill_ids: Vec<String>,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("assign_skills_to_repository", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let context = OperationContext::new(audit_target).subject(
            OperationSubjectKind::Repository,
            SafeIdentifier::new(&repository_id),
        );
        let pool = request_context.db().clone();
        let requested_count = skill_ids.len() as u64;
        let entry = crate::ipc_registry::command_policy("assign_skills_to_repository")
            .expect("assign_skills_to_repository must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("assign_skills_to_repository must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            context,
            move |_| {
                SafeOperationResult::succeeded("Repository assignments updated.")
                    .count(SafeDetailKey::RequestedCount, requested_count)
            },
            || async move {
                db::assign_skills_to_repository(&pool, &repository_id, &skill_ids, None)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn set_skill_repository_pinned(
    state: State<'_, AppState>,
    repository_id: String,
    pinned: bool,
) -> crate::ipc_error::IpcResult<SkillRepository> {
    crate::ipc_boundary_async!("set_skill_repository_pinned", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let context = OperationContext::new(audit_target).subject(
            OperationSubjectKind::Repository,
            SafeIdentifier::new(&repository_id),
        );
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("set_skill_repository_pinned")
            .expect("set_skill_repository_pinned must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("set_skill_repository_pinned must be auditable")
        };
        let mode = if pinned { "pinned" } else { "unpinned" };
        crate::observability::run_operation(
            &state,
            definition,
            context,
            move |_| {
                SafeOperationResult::succeeded("Repository pin updated.")
                    .stable(SafeDetailKey::Mode, mode)
            },
            || async move {
                db::set_skill_repository_pinned(&pool, &repository_id, pinned)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn get_skill_tags(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<SkillTag>> {
    crate::ipc_boundary!(
        "get_skill_tags",
        async move {
            let pool = state.active_db().await?;
            db::get_skill_tags(&pool).await.map_err(|e| e.to_string())
        }
        .await
    )
}

/// 仪表盘中央库热门标签 Top-N：`limit` 由 repo 层 clamp 到 1..=50。
#[tauri::command]
pub async fn get_central_top_tags(
    state: State<'_, AppState>,
    limit: u32,
) -> crate::ipc_error::IpcResult<Vec<CentralTopTag>> {
    crate::ipc_boundary!(
        "get_central_top_tags",
        async move {
            let pool = state.active_db().await?;
            db::list_central_top_tags(&pool, limit)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn create_skill_tag(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
    color: Option<String>,
) -> crate::ipc_error::IpcResult<SkillTag> {
    crate::ipc_boundary_async!("create_skill_tag", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("create_skill_tag")
            .expect("create_skill_tag must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("create_skill_tag must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(audit_target),
            |tag: &SkillTag| {
                SafeOperationResult::succeeded("Skill tag created.")
                    .identifier(SafeDetailKey::Identifier, SafeIdentifier::new(&tag.id))
            },
            || async move {
                db::create_skill_tag(&pool, &name, description.as_deref(), color.as_deref())
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn assign_skill_tags(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
    tag_ids: Vec<String>,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("assign_skill_tags", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let context = OperationContext::new(audit_target);
        let pool = request_context.db().clone();
        let affected_count = skill_ids.len().saturating_mul(tag_ids.len()) as u64;
        let entry = crate::ipc_registry::command_policy("assign_skill_tags")
            .expect("assign_skill_tags must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("assign_skill_tags must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            context,
            move |_| {
                SafeOperationResult::succeeded("Skill tags assigned.")
                    .count(SafeDetailKey::AffectedCount, affected_count)
            },
            || async move {
                db::assign_skill_tags(&pool, &skill_ids, &tag_ids, "manual", None, None)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn unassign_skill_tags(
    state: State<'_, AppState>,
    skill_id: String,
    tag_ids: Vec<String>,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("unassign_skill_tags", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let context = OperationContext::new(audit_target)
            .subject(OperationSubjectKind::Skill, SafeIdentifier::new(&skill_id));
        let pool = request_context.db().clone();
        let affected_count = tag_ids.len() as u64;
        let entry = crate::ipc_registry::command_policy("unassign_skill_tags")
            .expect("unassign_skill_tags must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("unassign_skill_tags must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            context,
            move |_| {
                SafeOperationResult::succeeded("Skill tags unassigned.")
                    .count(SafeDetailKey::AffectedCount, affected_count)
            },
            || async move {
                db::unassign_skill_tags(&pool, &skill_id, &tag_ids)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn get_pending_ai_tag_reviews(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<SkillAiTagReview>> {
    crate::ipc_boundary!(
        "get_pending_ai_tag_reviews",
        async move {
            let pool = state.active_db().await?;
            db::get_pending_ai_tag_reviews(&pool)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn accept_ai_tag_review(
    state: State<'_, AppState>,
    skill_id: String,
    tag_ids: Vec<String>,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("accept_ai_tag_review", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let context = OperationContext::new(audit_target)
            .subject(OperationSubjectKind::Skill, SafeIdentifier::new(&skill_id));
        let pool = request_context.db().clone();
        let affected_count = tag_ids.len() as u64;
        let entry = crate::ipc_registry::command_policy("accept_ai_tag_review")
            .expect("accept_ai_tag_review must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("accept_ai_tag_review must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            context,
            move |_| {
                SafeOperationResult::succeeded("AI tag review accepted.")
                    .count(SafeDetailKey::AffectedCount, affected_count)
            },
            || async move {
                db::accept_ai_tag_reviews(&pool, &skill_id, &tag_ids)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skip_ai_tag_review(
    state: State<'_, AppState>,
    skill_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("skip_ai_tag_review", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let context = OperationContext::new(audit_target)
            .subject(OperationSubjectKind::Skill, SafeIdentifier::new(&skill_id));
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("skip_ai_tag_review")
            .expect("skip_ai_tag_review must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("skip_ai_tag_review must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| SafeOperationResult::succeeded("AI tag review skipped."),
            || async move {
                db::skip_ai_tag_reviews(&pool, &skill_id)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
pub async fn suggest_skill_tags(
    state: State<'_, AppState>,
    skill_id: String,
) -> crate::ipc_error::IpcResult<Vec<SkillTagSuggestion>> {
    crate::ipc_boundary_async!("suggest_skill_tags", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let context = OperationContext::new(audit_target)
            .subject(OperationSubjectKind::Skill, SafeIdentifier::new(&skill_id));
        let pool = request_context.db().clone();
        let secrets = Arc::clone(&state.secrets);
        let entry = crate::ipc_registry::command_policy("suggest_skill_tags")
            .expect("suggest_skill_tags must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("suggest_skill_tags must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |suggestions: &Vec<SkillTagSuggestion>| {
                SafeOperationResult::succeeded("AI tag suggestions generated.")
                    .count(SafeDetailKey::AffectedCount, suggestions.len() as u64)
            },
            || async move {
                ai_tagging::suggest_skill_tags_for_skill_id(&pool, secrets.as_ref(), skill_id)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn bulk_suggest_skill_tags(
    app: AppHandle,
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
) -> crate::ipc_error::IpcResult<Vec<SkillTagSuggestionResult>> {
    crate::ipc_boundary_async!("bulk_suggest_skill_tags", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let context = OperationContext::new(audit_target);
        let pool = request_context.db().clone();
        let requested_count = skill_ids.len() as u64;
        let app = Arc::new(app);
        let job_id = Uuid::new_v4().to_string();
        let operation_job_id = job_id.clone();
        let cancel_flag = state.ai_tag_jobs.register(&job_id);
        let audit_cancel_flag = Arc::clone(&cancel_flag);
        let secrets = Arc::clone(&state.secrets);
        let entry = crate::ipc_registry::command_policy("bulk_suggest_skill_tags")
            .expect("bulk_suggest_skill_tags must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("bulk_suggest_skill_tags must be auditable")
        };
        let result = crate::observability::run_operation(
            &state,
            definition,
            context,
            move |results: &Vec<SkillTagSuggestionResult>| {
                let succeeded = results.iter().filter(|result| result.succeeded).count() as u64;
                let failed = results.len() as u64 - succeeded;
                let summary = if audit_cancel_flag.load(Ordering::SeqCst) {
                    SafeOperationResult::cancelled("AI tag suggestion cancelled.")
                } else if failed > 0 {
                    SafeOperationResult::partial("AI tag suggestion partially completed.")
                } else {
                    SafeOperationResult::succeeded("AI tag suggestion completed.")
                };
                summary
                    .count(SafeDetailKey::RequestedCount, requested_count)
                    .count(SafeDetailKey::SucceededCount, succeeded)
                    .count(SafeDetailKey::FailedCount, failed)
            },
            || async move {
                ai_tagging::bulk_suggest_skill_tags_impl(
                    &pool,
                    secrets.as_ref(),
                    skill_ids,
                    operation_job_id,
                    cancel_flag,
                    move |payload| {
                        let _ = app.emit(AI_TAG_PROGRESS_EVENT, payload);
                    },
                )
                .await
                .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await;
        state.ai_tag_jobs.finish(&job_id);
        result
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn cancel_ai_tag_job(
    state: State<'_, AppState>,
    job_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("cancel_ai_tag_job", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let context = OperationContext::new(audit_target);
        let entry = crate::ipc_registry::command_policy("cancel_ai_tag_job")
            .expect("cancel_ai_tag_job must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("cancel_ai_tag_job must be auditable")
        };
        let app_state = state.inner();
        crate::observability::run_operation(
            app_state,
            definition,
            context,
            |_| SafeOperationResult::cancelled("AI tag cancellation requested."),
            || async move {
                if app_state.ai_tag_jobs.cancel(&job_id) {
                    Ok(())
                } else {
                    Err(ReviewedFailure::new(ReviewedDiagnostic::unexpected(
                        definition,
                    )))
                }
            },
        )
        .await
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn central_metadata_commands_have_named_boundaries_and_typed_audit_owners() {
        let source = include_str!("central_metadata.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        for command in [
            "get_skill_repositories",
            "create_or_update_skill_repository",
            "assign_skills_to_repository",
            "set_skill_repository_pinned",
            "get_skill_tags",
            "get_central_top_tags",
            "create_skill_tag",
            "assign_skill_tags",
            "unassign_skill_tags",
            "get_pending_ai_tag_reviews",
            "accept_ai_tag_review",
            "skip_ai_tag_review",
            "suggest_skill_tags",
            "bulk_suggest_skill_tags",
            "cancel_ai_tag_job",
        ] {
            assert!(production.contains(&format!("\"{command}\"")), "{command}");
        }
        for command in [
            "create_or_update_skill_repository",
            "assign_skills_to_repository",
            "set_skill_repository_pinned",
            "create_skill_tag",
            "assign_skill_tags",
            "unassign_skill_tags",
            "accept_ai_tag_review",
            "skip_ai_tag_review",
            "suggest_skill_tags",
            "bulk_suggest_skill_tags",
            "cancel_ai_tag_job",
        ] {
            assert!(
                production.contains(&format!("command_policy(\"{command}\")")),
                "{command}"
            );
        }
        for banned in [
            "SafeIdentifier::new(&name)",
            "SafeIdentifier::new(&url)",
            "SafeIdentifier::new(&repo)",
            "error = %",
            "OperationLogEvent",
        ] {
            assert!(!production.contains(banned), "banned audit input: {banned}");
        }
    }
}
