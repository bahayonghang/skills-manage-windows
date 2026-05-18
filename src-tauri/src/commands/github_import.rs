//! Tauri IPC shells for GitHub repository import.
//!
//! Business logic lives in `crate::services::github_import`. This module keeps
//! the existing command names and public type paths stable while translating
//! `State<AppState>` into service inputs.

use tauri::{AppHandle, State};

use crate::services::github_import;
use crate::targets::ActiveTarget;
use crate::AppState;

pub(crate) use crate::services::github_import::{
    build_preview_skills, build_repo_skill_candidates_from_snapshot_at_path,
    download_repo_snapshot, github_client, github_direct_auth_from_secret_store,
    import_github_repo_skills_ssh_with_auth, import_github_repo_skills_with_auth,
    inspect_repo_skill_candidates_from_snapshot_at_path, resolve_repo_source, GitHubRepoSnapshot,
    RemoteSkillCandidate,
};
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
    let active_target = state.active_target().await?;
    let pool = state.active_db().await?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await?;
    match active_target {
        ActiveTarget::Local => {
            github_import::preview_github_repo_import_with_auth(&pool, &repo_url, auth.as_deref())
                .await
        }
        ActiveTarget::Ssh(target) => {
            github_import::preview_github_repo_import_ssh_with_auth(
                &pool,
                &target,
                &repo_url,
                auth.as_deref(),
            )
            .await
        }
    }
}

#[tauri::command]
pub async fn import_github_repo_skills(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_url: String,
    selections: Vec<GitHubSkillImportSelection>,
    preview_workspace_id: Option<String>,
) -> Result<GitHubRepoImportResult, String> {
    let active_target = state.active_target().await?;
    let pool = state.active_db().await?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await?;
    match active_target {
        ActiveTarget::Local => {
            github_import::import_github_repo_skills_with_auth(
                &pool,
                &repo_url,
                selections,
                Some(&app),
                auth.as_deref(),
            )
            .await
        }
        ActiveTarget::Ssh(target) => {
            github_import::import_github_repo_skills_ssh_with_auth(
                &pool,
                &target,
                &repo_url,
                selections,
                preview_workspace_id.as_deref(),
                Some(&app),
                auth.as_deref(),
            )
            .await
        }
    }
}

#[tauri::command]
pub async fn fetch_github_skill_markdown(
    state: State<'_, AppState>,
    download_url: String,
    source_path: Option<String>,
    preview_workspace_id: Option<String>,
) -> Result<String, String> {
    if let Some(workspace_id) = preview_workspace_id.as_deref() {
        return github_import::fetch_github_skill_markdown_from_remote_workspace(
            &state,
            workspace_id,
            source_path.as_deref(),
        )
        .await;
    }

    let client = github_import::github_client()?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await?;
    github_import::fetch_raw_text(&client, &download_url, auth.as_deref()).await
}

#[tauri::command]
pub async fn discard_github_repo_preview_workspace(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<(), String> {
    github_import::discard_preview_workspace_for_active_target(&state, &workspace_id).await;
    Ok(())
}

#[tauri::command]
pub async fn get_github_pat(state: State<'_, AppState>) -> Result<GitHubPatState, String> {
    github_import::get_github_pat_state_impl(&state.db, state.secrets.as_ref()).await
}

#[tauri::command]
pub async fn set_github_pat(
    state: State<'_, AppState>,
    value: String,
) -> Result<GitHubPatState, String> {
    let result = github_import::set_github_pat_impl(&state.db, state.secrets.as_ref(), value).await;
    if result.is_ok() {
        state.central_update_snapshots.clear();
    }
    result
}

#[tauri::command]
pub async fn clear_github_pat(state: State<'_, AppState>) -> Result<GitHubPatState, String> {
    let result = github_import::clear_github_pat_impl(&state.db, state.secrets.as_ref()).await;
    if result.is_ok() {
        state.central_update_snapshots.clear();
    }
    result
}

#[tauri::command]
pub async fn test_github_pat(state: State<'_, AppState>) -> Result<GitHubPatTestResult, String> {
    github_import::test_github_pat_impl(&state.db, state.secrets.as_ref()).await
}
