//! Tauri IPC shells for GitHub repository import.
//!
//! Business logic lives in `crate::services::github_import`. This module keeps
//! the existing command names and public type paths stable while translating
//! `State<AppState>` into service inputs.

use tauri::{AppHandle, State};

use crate::services::github_import;
use crate::targets::ActiveTarget;
use crate::AppState;

pub use crate::services::github_import::{
    DuplicateResolution, GitHubImportProgressPayload, GitHubImportProgressPhase, GitHubPatState,
    GitHubPatTestResult, GitHubRepoImportResult, GitHubRepoPreview, GitHubRepoRef,
    GitHubSkillConflict, GitHubSkillImportSelection, GitHubSkillPreview,
    ImportedGitHubSkillSummary,
};

#[tauri::command]
pub async fn preview_github_repo_import(
    state: State<'_, AppState>,
    repo_url: String,
) -> crate::ipc_error::IpcResult<GitHubRepoPreview> {
    crate::ipc_boundary!(
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
                ActiveTarget::Local => github_import::preview_github_repo_import_with_auth(
                    &pool,
                    &repo_url,
                    auth.as_deref(),
                )
                .await
                .map_err(|e| e.to_ipc_error()),
                ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                    github_import::preview_github_repo_import_remote_with_auth(
                        &pool,
                        &active_target,
                        &repo_url,
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
    selections: Vec<GitHubSkillImportSelection>,
) -> crate::ipc_error::IpcResult<GitHubRepoImportResult> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            github_import::import_github_repo_skills_from_preview(
                &pool,
                &active_target,
                &preview_id,
                &repo_url,
                selections,
                Some(&app),
            )
            .await
            .map_err(|e| e.to_ipc_error())
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
        async move {
            let request_context = state.resolve_target_context().await?;
            github_import::discard_preview_snapshot_for_target(
                request_context.target(),
                &preview_id,
            )
            .await;
            Ok(())
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
        async move {
            let result =
                github_import::set_github_pat_impl(&state.db, state.secrets.as_ref(), value)
                    .await
                    .map_err(|e| e.to_string());
            if result.is_ok() {
                state.central_update_snapshots.clear();
            }
            result
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
        async move {
            let result = github_import::clear_github_pat_impl(&state.db, state.secrets.as_ref())
                .await
                .map_err(|e| e.to_string());
            if result.is_ok() {
                state.central_update_snapshots.clear();
            }
            result
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
        async move {
            github_import::test_github_pat_impl(&state.db, state.secrets.as_ref())
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}
