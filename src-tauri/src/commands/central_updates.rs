//! Tauri IPC shells for Central skill update checks and updates.
//!
//! Business logic lives in `crate::services::central_updates`. This module
//! keeps the existing command names, payload shapes, and deprecation notes
//! stable while translating `State<AppState>` into service inputs.

use tauri::{AppHandle, State};

use crate::db::SkillUpdateState;
use crate::services::central_updates::{
    check_central_skill_updates_impl, get_central_skill_update_states_impl,
    update_central_skills_impl, CentralFs, CentralSkillUpdateResult,
};
use crate::services::github_import;
use crate::AppState;

pub mod repository_sync;
#[allow(deprecated)]
pub use repository_sync::{
    apply_central_repository_sync, check_central_repository_sync, CentralRemoteAddedSkill,
    CentralRemoteMissingSkill, CentralRepositoryAddedSkillSelection,
    CentralRepositoryAdditionSkipRequest, CentralRepositoryAdditionUnskipRequest,
    CentralRepositorySyncApplyResult, CentralRepositorySyncDecisions, CentralRepositorySyncFailure,
    CentralRepositorySyncPreview, CentralRepositorySyncSummary,
};
// Phase P2: `skill_update_inventory` 模块复用这些 helper 拼装 inventory。
// 内部用 pub(crate) 暴露最小接口；旧 command 行为保持不变。
pub(crate) use repository_sync::{build_remote_missing_skills, collect_remote_added_skills};

// 迁移期桥接：repository_sync 与 skill_update_inventory 的命令模块仍经由本
// 模块路径解析已下沉到 services 的内核；两者归位 services 后随之拆除。
pub(crate) use crate::services::central_updates::{
    emit_update_progress, error_state_from_assignment, keep_remote_missing_central_skills_impl,
    load_remote_skill_content, load_selected_central_skills, prepare_skill_updates,
    prepare_snapshots_for_repo_refs, prepare_snapshots_for_repo_refs_with_policy,
    remote_missing_state_from_assignment, repo_cache_key, state_from_relocated_source,
    state_from_remote, unsupported_state_from_assignment, update_counters_for_state,
    update_one_skill, update_one_skill_with_options, PreparedSkillUpdate, RemoteSkillLoadError,
    SkillUpdateStatus, SnapshotCachePolicy, UpdateCounters,
};

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
    let pool = state.active_db().await?;
    let fs = CentralFs::from_active_target(state.active_target().await?)
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
    note = "Use apply_skill_update_decisions with `updates` field instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn update_central_skills(
    app: AppHandle,
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
) -> Result<CentralSkillUpdateResult, String> {
    let pool = state.active_db().await?;
    let fs = CentralFs::from_active_target(state.active_target().await?)
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
