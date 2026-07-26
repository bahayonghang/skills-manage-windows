//! Tauri IPC shells for Central skill update checks, updates, and
//! repository-level sync.
//!
//! Business logic lives in `crate::services::central_updates`. This module
//! keeps the existing command names, payload shapes, and deprecation notes
//! stable while translating `State<AppState>` into service inputs.

use tauri::{AppHandle, State};

use crate::db::SkillUpdateState;
use crate::services::central_updates::{
    apply_central_repository_sync_impl, check_central_repository_sync_impl,
    check_central_skill_updates_impl, get_central_skill_update_states_impl,
    keep_remote_missing_central_skills_impl, update_central_skills_impl, CentralFs,
    CentralRepositorySyncApplyResult, CentralRepositorySyncDecisions, CentralRepositorySyncPreview,
    CentralSkillUpdateResult,
};
use crate::services::github_import;
use crate::AppState;

#[tauri::command]
pub async fn get_central_skill_update_states(
    state: State<'_, AppState>,
) -> Result<Vec<SkillUpdateState>, String> {
    let pool = state.active_db().await?;
    get_central_skill_update_states_impl(&pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[deprecated(
    note = "Use refresh_skill_update_inventory + apply_skill_update_decisions instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn check_central_skill_updates(
    app: AppHandle,
    state: State<'_, AppState>,
    skill_ids: Option<Vec<String>>,
) -> Result<Vec<SkillUpdateState>, String> {
    let request_context = state.resolve_target_context().await?;
    let pool = request_context.db().clone();
    let fs = CentralFs::from_active_target(request_context.target().clone())
        .await
        .map_err(|e| e.to_string())?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    let client = github_import::github_client().map_err(|e| e.to_string())?;
    check_central_skill_updates_impl(
        Some(&app),
        &pool,
        &fs,
        &state.central_update_cancel,
        auth.as_deref(),
        &client,
        &state.central_update_snapshots,
        skill_ids,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
#[deprecated(
    note = "Use refresh_skill_update_inventory with scope=Repositories instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn check_central_repository_sync(
    app: AppHandle,
    state: State<'_, AppState>,
    repository_ids: Vec<String>,
    skill_ids: Option<Vec<String>>,
) -> Result<CentralRepositorySyncPreview, String> {
    let request_context = state.resolve_target_context().await?;
    let pool = request_context.db().clone();
    let fs = CentralFs::from_active_target(request_context.target().clone())
        .await
        .map_err(|e| e.to_string())?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    let client = github_import::github_client().map_err(|e| e.to_string())?;
    check_central_repository_sync_impl(
        Some(&app),
        &pool,
        &fs,
        &state.central_update_cancel,
        auth.as_deref(),
        &client,
        &state.central_update_snapshots,
        repository_ids,
        skill_ids,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
#[deprecated(
    note = "Use apply_skill_update_decisions instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn apply_central_repository_sync(
    app: AppHandle,
    state: State<'_, AppState>,
    decisions: CentralRepositorySyncDecisions,
) -> Result<CentralRepositorySyncApplyResult, String> {
    let request_context = state.resolve_target_context().await?;
    let pool = request_context.db().clone();
    let active_target = request_context.target().clone();
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    apply_central_repository_sync_impl(
        Some(&app),
        &pool,
        &active_target,
        auth.as_deref(),
        decisions,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
#[deprecated(
    note = "Use apply_skill_update_decisions with `updates` field instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn update_central_skills(
    app: AppHandle,
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
) -> Result<CentralSkillUpdateResult, String> {
    let request_context = state.resolve_target_context().await?;
    let pool = request_context.db().clone();
    let fs = CentralFs::from_active_target(request_context.target().clone())
        .await
        .map_err(|e| e.to_string())?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    let client = github_import::github_client().map_err(|e| e.to_string())?;
    update_central_skills_impl(
        Some(&app),
        &pool,
        &fs,
        &state.central_update_cancel,
        auth.as_deref(),
        &client,
        &state.central_update_snapshots,
        skill_ids,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_central_skill_updates(state: State<'_, AppState>) -> Result<(), String> {
    state
        .central_update_cancel
        .store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
#[deprecated(
    note = "Use apply_skill_update_decisions with `keep_missing` field instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn keep_remote_missing_central_skills(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
) -> Result<Vec<String>, String> {
    let pool = state.active_db().await?;
    keep_remote_missing_central_skills_impl(&pool, &skill_ids)
        .await
        .map_err(|e| e.to_string())
}
