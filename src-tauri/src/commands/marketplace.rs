//! Tauri command shells for the Marketplace + AI explanation flows.
//!
//! Business logic lives in `crate::services::marketplace` (registry CRUD,
//! GitHub sync, install) and `crate::services::ai_provider` (Anthropic /
//! OpenAI-compatible explanation, cache, streaming). This file translates
//! IPC arguments + state into service calls and re-exports the public
//! types so frontend bindings remain stable.

use tauri::{AppHandle, State};

use std::collections::HashMap;

use crate::services::ai_provider;
use crate::services::marketplace;
use crate::targets::ActiveTarget;
use crate::AppState;
use crate::{
    ipc_error::{public_message_for_code, IpcError, REVIEWED_IPC_ERROR_CODES},
    observability::{
        CommandLogPolicy, OperationContext, OperationDefinition, OperationTarget,
        OperationTargetKind, ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeIdentifier,
        SafeOperationResult,
    },
};

// Re-export the types frontend code already references via this module path.
pub use crate::services::ai_provider::{
    AiConnectionTestResult, ExplanationChunkPayload, ExplanationCompletePayload,
    ExplanationErrorInfo, ExplanationErrorKind,
};
pub use crate::services::marketplace::{
    MarketplaceSkill, RegistryCacheMetadata, RegistrySyncStatus, SkillRegistry, SkillsShFileEntry,
    SkillsShSkill, SyncRegistryOptions,
};

fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("Marketplace command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => panic!("Marketplace mutation must use Operation policy"),
    }
}

fn operation_target(target: &ActiveTarget) -> OperationTarget {
    match target {
        ActiveTarget::Local => OperationTarget::local(),
        ActiveTarget::Ssh(_) => OperationTarget::new(OperationTargetKind::Ssh, target.id()),
        ActiveTarget::Wsl(_) => OperationTarget::new(OperationTargetKind::Wsl, target.id()),
    }
}

fn reviewed_failure(definition: OperationDefinition, error: IpcError) -> ReviewedFailure {
    let code = REVIEWED_IPC_ERROR_CODES
        .iter()
        .copied()
        .find(|code| *code == error.safe_code())
        .unwrap_or("internal.unexpected");
    let message = public_message_for_code(code)
        .unwrap_or("The operation failed. See runtime logs for details.");
    ReviewedFailure::new(ReviewedDiagnostic::new(
        code,
        definition.category().as_str(),
        definition.default_phase(),
        message,
        error.retryable,
    ))
}

// ─── Registry CRUD ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_registries(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<SkillRegistry>> {
    crate::ipc_boundary!(
        "list_registries",
        async move {
            let pool = state.active_db().await?;
            marketplace::list_registries_impl(&pool)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn add_registry(
    state: State<'_, AppState>,
    name: String,
    source_type: String,
    url: String,
) -> crate::ipc_error::IpcResult<SkillRegistry> {
    crate::ipc_boundary!(
        "add_registry",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("add_registry");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(operation_target(&active_target)),
                |registry: &SkillRegistry| {
                    SafeOperationResult::succeeded("Added a Marketplace registry.")
                        .identifier(SafeDetailKey::Identifier, SafeIdentifier::new(&registry.id))
                },
                || async move {
                    marketplace::add_registry_impl(&pool, name, source_type, url, None)
                        .await
                        .map_err(|error| {
                            reviewed_failure(definition, IpcError::from_display(error))
                        })
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn remove_registry(
    state: State<'_, AppState>,
    registry_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        "remove_registry",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("remove_registry");
            let context = OperationContext::new(operation_target(&active_target)).subject(
                crate::observability::OperationSubjectKind::Registry,
                SafeIdentifier::new(&registry_id),
            );
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |_| SafeOperationResult::succeeded("Removed a Marketplace registry."),
                || async move {
                    marketplace::remove_registry_impl(&pool, registry_id)
                        .await
                        .map_err(|error| {
                            reviewed_failure(definition, IpcError::from_display(error))
                        })
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn sync_registry(
    state: State<'_, AppState>,
    registry_id: String,
) -> crate::ipc_error::IpcResult<Vec<MarketplaceSkill>> {
    crate::ipc_boundary!(
        "sync_registry",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("sync_registry");
            let context = OperationContext::new(operation_target(&active_target)).subject(
                crate::observability::OperationSubjectKind::Registry,
                SafeIdentifier::new(&registry_id),
            );
            let local_db = state.db.clone();
            let secrets = std::sync::Arc::clone(&state.secrets);
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |skills: &Vec<MarketplaceSkill>| {
                    SafeOperationResult::succeeded("Synchronized a Marketplace registry.")
                        .count(SafeDetailKey::AffectedCount, skills.len() as u64)
                },
                || async move {
                    marketplace::sync_registry_impl(
                        &pool,
                        &local_db,
                        secrets.as_ref(),
                        registry_id,
                        SyncRegistryOptions::default(),
                    )
                    .await
                    .map_err(|error| reviewed_failure(definition, IpcError::from_display(error)))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn sync_registry_with_options(
    state: State<'_, AppState>,
    registry_id: String,
    options: Option<SyncRegistryOptions>,
) -> crate::ipc_error::IpcResult<Vec<MarketplaceSkill>> {
    crate::ipc_boundary!(
        "sync_registry_with_options",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("sync_registry_with_options");
            let context = OperationContext::new(operation_target(&active_target)).subject(
                crate::observability::OperationSubjectKind::Registry,
                SafeIdentifier::new(&registry_id),
            );
            let local_db = state.db.clone();
            let secrets = std::sync::Arc::clone(&state.secrets);
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |skills: &Vec<MarketplaceSkill>| {
                    SafeOperationResult::succeeded("Synchronized a Marketplace registry.")
                        .count(SafeDetailKey::AffectedCount, skills.len() as u64)
                },
                || async move {
                    marketplace::sync_registry_impl(
                        &pool,
                        &local_db,
                        secrets.as_ref(),
                        registry_id,
                        options.unwrap_or_default(),
                    )
                    .await
                    .map_err(|error| reviewed_failure(definition, IpcError::from_display(error)))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn search_marketplace_skills(
    state: State<'_, AppState>,
    registry_id: Option<String>,
    query: Option<String>,
) -> crate::ipc_error::IpcResult<Vec<MarketplaceSkill>> {
    crate::ipc_boundary!(
        "search_marketplace_skills",
        async move {
            let pool = state.active_db().await?;
            marketplace::search_marketplace_skills_impl(&pool, registry_id, query)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn install_marketplace_skill(
    state: State<'_, AppState>,
    skill_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        "install_marketplace_skill",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("install_marketplace_skill");
            let context = OperationContext::new(operation_target(&active_target)).subject(
                crate::observability::OperationSubjectKind::Skill,
                SafeIdentifier::new(&skill_id),
            );
            let local_db = state.db.clone();
            let secrets = std::sync::Arc::clone(&state.secrets);
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |_| SafeOperationResult::succeeded("Installed a Marketplace skill."),
                || async move {
                    marketplace::install_marketplace_skill_impl(
                        &pool,
                        &local_db,
                        secrets.as_ref(),
                        active_target,
                        skill_id,
                    )
                    .await
                    .map_err(|error| {
                        reviewed_failure(definition, marketplace_install_ipc_error(error))
                    })
                },
            )
            .await
        }
        .await
    )
}

fn marketplace_install_ipc_error(
    error: marketplace::MarketplaceError,
) -> crate::ipc_error::IpcError {
    use marketplace::MarketplaceError;

    match error {
        MarketplaceError::SkillNotFound => crate::ipc_error::IpcError::new(
            "resource.not_found",
            "The requested resource was not found.",
            false,
        ),
        MarketplaceError::RegistryDisabled => crate::ipc_error::IpcError::new(
            "marketplace.registry_disabled",
            "The Marketplace registry is disabled.",
            false,
        ),
        MarketplaceError::CandidateStale => crate::ipc_error::IpcError::new(
            "marketplace.registry_stale",
            "The Marketplace cache is stale. Sync the registry and try again.",
            false,
        ),
        MarketplaceError::CandidateAmbiguous => crate::ipc_error::IpcError::new(
            "marketplace.identity_ambiguous",
            "The Marketplace registry contains an ambiguous skill identity.",
            false,
        ),
        MarketplaceError::UnsupportedSourceType(_) => crate::ipc_error::IpcError::new(
            "marketplace.source_unsupported",
            "The Marketplace registry source is not supported.",
            false,
        ),
        MarketplaceError::CentralAgentMissing => crate::ipc_error::IpcError::new(
            "marketplace.install_unavailable",
            "Marketplace installation is unavailable for the selected target.",
            false,
        ),
        MarketplaceError::GithubImport(error) => crate::ipc_error::IpcError::from_display(error),
        _ => crate::ipc_error::IpcError::new(
            "marketplace.install_failed",
            "The Marketplace skill could not be installed.",
            false,
        ),
    }
}

#[tauri::command]
pub async fn search_skills_sh(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> crate::ipc_error::IpcResult<Vec<SkillsShSkill>> {
    crate::ipc_boundary!(
        "search_skills_sh",
        async move {
            marketplace::search_skills_sh_impl(&state.db, state.secrets.as_ref(), query, limit)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn resolve_skills_sh_url(
    state: State<'_, AppState>,
    source: String,
    skill_id: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        "resolve_skills_sh_url",
        async move {
            marketplace::resolve_skills_sh_url_impl(
                &state.db,
                state.secrets.as_ref(),
                source,
                skill_id,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn browse_skills_sh_directory(
    state: State<'_, AppState>,
    source: String,
    skill_id: String,
) -> crate::ipc_error::IpcResult<Vec<SkillsShFileEntry>> {
    crate::ipc_boundary!(
        "browse_skills_sh_directory",
        async move {
            marketplace::browse_skills_sh_directory_impl(
                &state.db,
                state.secrets.as_ref(),
                source,
                skill_id,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn read_skills_sh_file(
    state: State<'_, AppState>,
    source: String,
    file_path: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        "read_skills_sh_file",
        async move {
            marketplace::read_skills_sh_file_impl(
                &state.db,
                state.secrets.as_ref(),
                source,
                file_path,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn install_from_skills_sh(
    state: State<'_, AppState>,
    source: String,
    skill_id: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        "install_from_skills_sh",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("install_from_skills_sh");
            let context = OperationContext::new(operation_target(&active_target)).subject(
                crate::observability::OperationSubjectKind::Skill,
                SafeIdentifier::new(&skill_id),
            );
            let secrets = std::sync::Arc::clone(&state.secrets);
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |_| SafeOperationResult::succeeded("Installed a skills.sh skill."),
                || async move {
                    marketplace::install_from_skills_sh_impl(
                        &pool,
                        secrets.as_ref(),
                        active_target,
                        source,
                        skill_id,
                    )
                    .await
                    .map_err(|error| reviewed_failure(definition, IpcError::from_display(error)))
                },
            )
            .await
        }
        .await
    )
}

// ─── AI Explanation ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn explain_skill(
    state: State<'_, AppState>,
    content: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        "explain_skill",
        async move {
            let definition = operation_definition("explain_skill");
            let db = state.db.clone();
            let secrets = std::sync::Arc::clone(&state.secrets);
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(OperationTarget::local()),
                |_| SafeOperationResult::succeeded("Generated a skill explanation."),
                || async move {
                    ai_provider::explain_skill_impl(&db, secrets.as_ref(), content)
                        .await
                        .map_err(|error| {
                            reviewed_failure(definition, IpcError::from_display(error))
                        })
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn test_ai_connection(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<AiConnectionTestResult> {
    crate::ipc_boundary!(
        "test_ai_connection",
        async move {
            let definition = operation_definition("test_ai_connection");
            let db = state.db.clone();
            let secrets = std::sync::Arc::clone(&state.secrets);
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(OperationTarget::local()),
                |result: &AiConnectionTestResult| {
                    if result.ok {
                        SafeOperationResult::succeeded("Tested the AI connection successfully.")
                    } else {
                        SafeOperationResult::partial(
                            "The AI connection test completed with a failed result.",
                        )
                    }
                },
                || async move {
                    ai_provider::test_ai_connection_impl(&db, secrets.as_ref())
                        .await
                        .map_err(|error| {
                            reviewed_failure(definition, IpcError::from_display(error))
                        })
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn get_skill_explanation(
    state: State<'_, AppState>,
    skill_id: String,
    lang: String,
) -> crate::ipc_error::IpcResult<Option<String>> {
    crate::ipc_boundary!(
        "get_skill_explanation",
        async move {
            ai_provider::get_skill_explanation_impl(&state.db, skill_id, lang)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn get_skill_explanation_summaries(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
    lang: String,
) -> crate::ipc_error::IpcResult<HashMap<String, String>> {
    crate::ipc_boundary!(
        "get_skill_explanation_summaries",
        async move {
            ai_provider::get_skill_explanation_summaries_impl(&state.db, skill_ids, lang)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn explain_skill_stream(
    state: State<'_, AppState>,
    app: AppHandle,
    skill_id: String,
    content: String,
    lang: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        "explain_skill_stream",
        async move {
            let definition = operation_definition("explain_skill_stream");
            let context = OperationContext::new(OperationTarget::local()).subject(
                crate::observability::OperationSubjectKind::Skill,
                SafeIdentifier::new(&skill_id),
            );
            let db = state.db.clone();
            let secrets = std::sync::Arc::clone(&state.secrets);
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |_| SafeOperationResult::succeeded("Generated a streamed skill explanation."),
                || async move {
                    ai_provider::explain_skill_stream_impl(
                        &db,
                        secrets.as_ref(),
                        &app,
                        skill_id,
                        content,
                        lang,
                    )
                    .await
                    .map_err(|error| reviewed_failure(definition, IpcError::from_display(error)))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn refresh_skill_explanation(
    state: State<'_, AppState>,
    app: AppHandle,
    skill_id: String,
    content: String,
    lang: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        "refresh_skill_explanation",
        async move {
            let definition = operation_definition("refresh_skill_explanation");
            let context = OperationContext::new(OperationTarget::local()).subject(
                crate::observability::OperationSubjectKind::Skill,
                SafeIdentifier::new(&skill_id),
            );
            let db = state.db.clone();
            let secrets = std::sync::Arc::clone(&state.secrets);
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |_| SafeOperationResult::succeeded("Refreshed a skill explanation."),
                || async move {
                    ai_provider::refresh_skill_explanation_impl(
                        &db,
                        secrets.as_ref(),
                        &app,
                        skill_id,
                        content,
                        lang,
                    )
                    .await
                    .map_err(|error| reviewed_failure(definition, IpcError::from_display(error)))
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

    #[test]
    fn marketplace_install_semantic_errors_have_stable_public_codes() {
        for (error, expected_code) in [
            (
                marketplace::MarketplaceError::RegistryDisabled,
                "marketplace.registry_disabled",
            ),
            (
                marketplace::MarketplaceError::CandidateStale,
                "marketplace.registry_stale",
            ),
            (
                marketplace::MarketplaceError::CandidateAmbiguous,
                "marketplace.identity_ambiguous",
            ),
            (
                marketplace::MarketplaceError::UnsupportedSourceType("private source".to_string()),
                "marketplace.source_unsupported",
            ),
        ] {
            let ipc = marketplace_install_ipc_error(error);
            assert_eq!(ipc.code, expected_code);
            assert!(!ipc.message.contains("private"));
        }
    }

    #[test]
    fn marketplace_install_internal_errors_do_not_expose_dynamic_diagnostics() {
        let secret = r"C:\Users\alice\private\skill?token=ghp_secret";
        for error in [
            marketplace::MarketplaceError::CentralUpdates(
                crate::services::central_updates::CentralUpdatesError::Batch(secret.to_string()),
            ),
            marketplace::MarketplaceError::GithubImport(
                crate::services::github_import::GithubImportError::InvalidUrl(secret.to_string()),
            ),
        ] {
            let ipc = marketplace_install_ipc_error(error);
            let serialized = serde_json::to_string(&ipc).expect("serialize IPC error");
            assert!(matches!(
                ipc.code.as_str(),
                "marketplace.install_failed" | "internal.unexpected"
            ));
            assert!(!serialized.contains(secret));
            assert!(!serialized.contains("ghp_secret"));
        }
    }
}
