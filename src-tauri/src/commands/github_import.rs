//! Tauri IPC shells for GitHub repository import.
//!
//! Business logic lives in `crate::services::github_import`. This module keeps
//! the existing command names and public type paths stable while translating
//! `State<AppState>` into service inputs.

use tauri::{AppHandle, State};

use crate::services::github_import;
use crate::targets::ActiveTarget;
use crate::AppState;
use crate::{
    ipc_error::{public_message_for_code, IpcError, REVIEWED_IPC_ERROR_CODES},
    observability::{
        OperationContext, OperationDefinition, OperationTarget, OperationTargetKind,
        ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeOperationResult,
    },
};

pub use crate::services::github_import::{
    DuplicateResolution, GitHubImportProgressPayload, GitHubImportProgressPhase, GitHubPatState,
    GitHubPatTestResult, GitHubRepoImportResult, GitHubRepoPreview, GitHubRepoRef,
    GitHubSkillConflict, GitHubSkillImportSelection, GitHubSkillPreview,
    ImportedGitHubSkillSummary,
};

fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("GitHub import command must be registered")
        .policy
    {
        crate::observability::CommandLogPolicy::Operation(definition) => definition,
        _ => panic!("GitHub import mutation must use Operation policy"),
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

#[tauri::command]
pub async fn preview_github_repo_import(
    state: State<'_, AppState>,
    repo_url: String,
    branch: Option<String>,
) -> crate::ipc_error::IpcResult<GitHubRepoPreview> {
    crate::ipc_boundary!(
        "preview_github_repo_import",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let auth = github_import::github_direct_auth_from_secret_store(
                &state.db,
                state.secrets.as_ref(),
            )
            .await
            .map_err(|e| e.to_ipc_error())?;
            match &active_target {
                ActiveTarget::Local => {
                    github_import::preview_github_repo_import_with_branch_and_auth(
                        &pool,
                        &repo_url,
                        branch.as_deref(),
                        auth.as_deref(),
                    )
                    .await
                    .map_err(|e| e.to_ipc_error())
                }
                ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                    github_import::preview_github_repo_import_remote_with_auth(
                        &pool,
                        &active_target,
                        &repo_url,
                        branch.as_deref(),
                        auth.as_deref(),
                    )
                    .await
                    .map_err(|e| e.to_ipc_error())
                }
            }
        }
        .await
    )
}

/// Import the skills confirmed in a registered preview snapshot.
///
/// `previewId` is required for every target. The command never falls back to
/// re-resolving the repository URL, so a branch that moved after preview cannot
/// change what is imported.
#[tauri::command]
pub async fn import_github_repo_skills(
    app: AppHandle,
    state: State<'_, AppState>,
    preview_id: String,
    repo_url: String,
    branch: Option<String>,
    selections: Vec<GitHubSkillImportSelection>,
) -> crate::ipc_error::IpcResult<GitHubRepoImportResult> {
    crate::ipc_boundary!(
        "import_github_repo_skills",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("import_github_repo_skills");
            let context = OperationContext::new(operation_target(&active_target));
            let requested = selections.len() as u64;
            crate::observability::run_operation(
                &state,
                definition,
                context,
                |result: &GitHubRepoImportResult| {
                    let imported = result.imported_skills.len() as u64;
                    let skipped = result.skipped_skills.len() as u64;
                    SafeOperationResult::succeeded("Imported skills from a GitHub repository.")
                        .count(SafeDetailKey::RequestedCount, requested)
                        .count(SafeDetailKey::SucceededCount, imported)
                        .count(SafeDetailKey::SkippedCount, skipped)
                },
                || async move {
                    github_import::import_github_repo_skills_from_preview_with_branch(
                        &pool,
                        &active_target,
                        &preview_id,
                        &repo_url,
                        branch.as_deref(),
                        selections,
                        Some(&app),
                    )
                    .await
                    .map_err(|error| {
                        reviewed_failure(definition, IpcError::from(error.to_ipc_error()))
                    })
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn fetch_github_skill_markdown(
    state: State<'_, AppState>,
    preview_id: String,
    repo: GitHubRepoRef,
    source_path: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        "fetch_github_skill_markdown",
        async move {
            let request_context = state.resolve_target_context().await?;
            github_import::fetch_github_skill_markdown_from_snapshot(
                request_context.target(),
                &preview_id,
                &repo,
                &source_path,
            )
            .await
            .map_err(|e| e.to_ipc_error())
        }
        .await
    )
}

#[tauri::command]
pub async fn discard_github_repo_preview_snapshot(
    state: State<'_, AppState>,
    preview_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        "discard_github_repo_preview_snapshot",
        async move {
            let request_context = state.resolve_target_context().await?;
            github_import::discard_preview_snapshot_for_target(
                request_context.target(),
                &preview_id,
            )
            .await;
            Ok::<(), IpcError>(())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn get_github_pat(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<GitHubPatState> {
    crate::ipc_boundary!(
        "get_github_pat",
        async move {
            github_import::get_github_pat_state_impl(&state.db, state.secrets.as_ref())
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn set_github_pat(
    state: State<'_, AppState>,
    value: String,
) -> crate::ipc_error::IpcResult<GitHubPatState> {
    crate::ipc_boundary!(
        "set_github_pat",
        async move {
            let definition = operation_definition("set_github_pat");
            crate::observability::run_operation(
                &state,
                definition,
                OperationTarget::local(),
                |result: &GitHubPatState| {
                    SafeOperationResult::succeeded("Saved the GitHub credential.")
                        .flag(SafeDetailKey::Changed, result.configured)
                },
                || async {
                    let result = github_import::set_github_pat_impl(
                        &state.db,
                        state.secrets.as_ref(),
                        value,
                    )
                    .await
                    .map_err(|error| reviewed_failure(definition, IpcError::from_display(error)));
                    if result.is_ok() {
                        state.central_update_snapshots.clear();
                    }
                    result
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn clear_github_pat(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<GitHubPatState> {
    crate::ipc_boundary!(
        "clear_github_pat",
        async move {
            let definition = operation_definition("clear_github_pat");
            crate::observability::run_operation(
                &state,
                definition,
                OperationTarget::local(),
                |_| SafeOperationResult::succeeded("Cleared the GitHub credential."),
                || async {
                    let result =
                        github_import::clear_github_pat_impl(&state.db, state.secrets.as_ref())
                            .await
                            .map_err(|error| {
                                reviewed_failure(definition, IpcError::from_display(error))
                            });
                    if result.is_ok() {
                        state.central_update_snapshots.clear();
                    }
                    result
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn test_github_pat(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<GitHubPatTestResult> {
    crate::ipc_boundary!(
        "test_github_pat",
        async move {
            let definition = operation_definition("test_github_pat");
            crate::observability::run_operation(
                &state,
                definition,
                OperationTarget::local(),
                |result: &GitHubPatTestResult| {
                    if result.ok {
                        SafeOperationResult::succeeded("Tested the GitHub credential successfully.")
                    } else {
                        SafeOperationResult::partial(
                            "The GitHub credential test completed with a failed result.",
                        )
                    }
                    .flag(SafeDetailKey::Changed, result.configured)
                },
                || async {
                    github_import::test_github_pat_impl(&state.db, state.secrets.as_ref())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_import_dynamic_inputs_do_not_enter_the_ipc_envelope() {
        let secret = r"https://user:ghp_secret@example.invalid/private?ref=deadbeef";
        let error = github_import::GithubImportError::InvalidUrl(secret.to_string());
        let ipc = IpcError::from(error.to_ipc_error());
        let serialized = serde_json::to_string(&ipc).unwrap();
        assert_eq!(ipc.code, "github_import.invalid_url");
        assert!(!serialized.contains(secret));
        assert!(!serialized.contains("ghp_secret"));
        assert!(!serialized.contains("deadbeef"));
    }

    #[test]
    fn github_import_mutations_are_registered_as_operations() {
        for command in [
            "set_github_pat",
            "clear_github_pat",
            "test_github_pat",
            "import_github_repo_skills",
        ] {
            assert!(matches!(
                crate::ipc_registry::command_policy(command).unwrap().policy,
                crate::observability::CommandLogPolicy::Operation(_)
            ));
        }
    }
}
