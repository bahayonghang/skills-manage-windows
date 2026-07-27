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
) -> Result<GitHubRepoPreview, String> {
    let request_context = state.resolve_target_context().await?;
    let active_target = request_context.target().clone();
    let pool = request_context.db().clone();
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_ipc_error())?;
    match &active_target {
        ActiveTarget::Local => {
            github_import::preview_github_repo_import_with_auth(&pool, &repo_url, auth.as_deref())
                .await
                .map_err(|e| e.to_ipc_error())
        }
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
) -> Result<GitHubRepoImportResult, String> {
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

#[tauri::command]
pub async fn fetch_github_skill_markdown(
    state: State<'_, AppState>,
    preview_id: String,
    repo: GitHubRepoRef,
    source_path: String,
) -> Result<String, String> {
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

#[tauri::command]
pub async fn discard_github_repo_preview_snapshot(
    state: State<'_, AppState>,
    preview_id: String,
) -> Result<(), String> {
    let request_context = state.resolve_target_context().await?;
    github_import::discard_preview_snapshot_for_target(request_context.target(), &preview_id).await;
    Ok(())
}

#[tauri::command]
pub async fn get_github_pat(state: State<'_, AppState>) -> Result<GitHubPatState, String> {
    github_import::get_github_pat_state_impl(&state.db, state.secrets.as_ref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_github_pat(
    state: State<'_, AppState>,
    value: String,
) -> Result<GitHubPatState, String> {
    let result = github_import::set_github_pat_impl(&state.db, state.secrets.as_ref(), value)
        .await
        .map_err(|e| e.to_string());
    if result.is_ok() {
        state.central_update_snapshots.clear();
    }
    result
}

#[tauri::command]
pub async fn clear_github_pat(state: State<'_, AppState>) -> Result<GitHubPatState, String> {
    let result = github_import::clear_github_pat_impl(&state.db, state.secrets.as_ref())
        .await
        .map_err(|e| e.to_string());
    if result.is_ok() {
        state.central_update_snapshots.clear();
    }
    result
}

#[tauri::command]
pub async fn test_github_pat(state: State<'_, AppState>) -> Result<GitHubPatTestResult, String> {
    github_import::test_github_pat_impl(&state.db, state.secrets.as_ref())
        .await
        .map_err(|e| e.to_string())
}
