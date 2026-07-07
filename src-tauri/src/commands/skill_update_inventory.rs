//! Tauri IPC shells for the Skill Update Inventory (Update Center panel).
//!
//! Business logic lives in `crate::services::central_updates::inventory`.
//! This module keeps the existing command names and payload shapes stable
//! while translating `State<AppState>` into service inputs.

use tauri::{AppHandle, State};

use crate::services::central_updates::inventory::{
    apply_skill_update_decisions_impl, clear_skill_update_inventory_impl,
    force_mirror_central_repositories_impl, force_update_central_skills_impl,
    get_skill_update_inventory_impl_scoped, refresh_skill_update_inventory_impl,
    scan_deleted_platform_copies_with_pool, scan_platform_duplicate_skills_with_pool,
    DeletedPlatformCopyGroup, ForceRepositoryMirrorRequest, ForceRepositoryMirrorResult,
    ForceSkillUpdateRequest, ForceSkillUpdateResult, PlatformDuplicateGroup, SkillRefreshScope,
    SkillUpdateApplyResult, SkillUpdateDecisions, SkillUpdateInventory,
};
use crate::services::central_updates::{CentralFs, SnapshotCachePolicy};
use crate::services::github_import;
use crate::AppState;

#[tauri::command]
pub async fn refresh_skill_update_inventory(
    state: State<'_, AppState>,
    scope: SkillRefreshScope,
) -> Result<SkillUpdateInventory, String> {
    let pool = state.active_db().await?;
    let fs = CentralFs::from_active_target(state.active_target().await?)
        .await
        .map_err(|e| e.to_string())?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    let client = github_import::github_client().map_err(|e| e.to_string())?;
    refresh_skill_update_inventory_impl(
        &pool,
        &fs,
        auth.as_deref(),
        &client,
        &state.central_update_snapshots,
        scope,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_skill_update_inventory(
    state: State<'_, AppState>,
    scope: Option<SkillRefreshScope>,
) -> Result<SkillUpdateInventory, String> {
    let pool = state.active_db().await?;
    get_skill_update_inventory_impl_scoped(&pool, scope)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_skill_update_inventory(
    state: State<'_, AppState>,
    scope: Option<SkillRefreshScope>,
) -> Result<(), String> {
    let pool = state.active_db().await?;
    clear_skill_update_inventory_impl(&pool, scope)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apply_skill_update_decisions(
    app: AppHandle,
    state: State<'_, AppState>,
    decisions: SkillUpdateDecisions,
) -> Result<SkillUpdateApplyResult, String> {
    let pool = state.active_db().await?;
    let active_target = state.active_target().await?;
    let fs = CentralFs::from_active_target(active_target.clone())
        .await
        .map_err(|e| e.to_string())?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    let client = github_import::github_client().map_err(|e| e.to_string())?;
    apply_skill_update_decisions_impl(
        Some(&app),
        &pool,
        &active_target,
        &fs,
        &state.central_update_cancel,
        auth.as_deref(),
        &client,
        &state.central_update_snapshots,
        decisions,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn force_update_central_skills(
    state: State<'_, AppState>,
    request: ForceSkillUpdateRequest,
) -> Result<ForceSkillUpdateResult, String> {
    let pool = state.active_db().await?;
    let fs = CentralFs::from_active_target(state.active_target().await?)
        .await
        .map_err(|e| e.to_string())?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    let client = github_import::github_client().map_err(|e| e.to_string())?;
    force_update_central_skills_impl(
        &pool,
        &fs,
        auth.as_deref(),
        &client,
        &state.central_update_snapshots,
        SnapshotCachePolicy::Bypass,
        request,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn force_mirror_central_repositories(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ForceRepositoryMirrorRequest,
) -> Result<ForceRepositoryMirrorResult, String> {
    let pool = state.active_db().await?;
    let active_target = state.active_target().await?;
    let fs = CentralFs::from_active_target(active_target.clone())
        .await
        .map_err(|e| e.to_string())?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    let client = github_import::github_client().map_err(|e| e.to_string())?;
    force_mirror_central_repositories_impl(
        Some(&app),
        &pool,
        &active_target,
        &fs,
        auth.as_deref(),
        &client,
        &state.central_update_snapshots,
        SnapshotCachePolicy::Bypass,
        request,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_platform_duplicate_skills(
    state: State<'_, AppState>,
    agent_ids: Option<Vec<String>>,
) -> Result<Vec<PlatformDuplicateGroup>, String> {
    let pool = state.active_db().await?;
    scan_platform_duplicate_skills_with_pool(&pool, agent_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_deleted_platform_copies(
    state: State<'_, AppState>,
    agent_ids: Option<Vec<String>>,
) -> Result<Vec<DeletedPlatformCopyGroup>, String> {
    let pool = state.active_db().await?;
    scan_deleted_platform_copies_with_pool(&pool, agent_ids)
        .await
        .map_err(|e| e.to_string())
}
