//! Tauri IPC shells for Obsidian vault scanning + source-import. Replaces the
//! corresponding entry points that used to live in `commands::discover`.

use tauri::State;

use crate::services::obsidian::{
    self, ObsidianImportResult, ObsidianSkill, ObsidianVault,
};
use crate::targets::ActiveTarget;
use crate::AppState;

#[tauri::command]
pub async fn get_obsidian_vaults(state: State<'_, AppState>) -> Result<Vec<ObsidianVault>, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Ok(Vec::new());
    }
    obsidian::get_obsidian_vaults_impl(&state.db).await
}

#[tauri::command]
pub async fn get_obsidian_vault_skills(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<ObsidianSkill>, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Ok(Vec::new());
    }
    obsidian::get_obsidian_vault_skills_impl(&state.db, &vault_id).await
}

#[tauri::command]
pub async fn import_obsidian_skill_to_central(
    state: State<'_, AppState>,
    dir_path: String,
) -> Result<ObsidianImportResult, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Err("Remote Obsidian import is not supported in this version.".to_string());
    }
    obsidian::import_obsidian_skill_to_central_impl(&state.db, &dir_path).await
}

#[tauri::command]
pub async fn import_obsidian_skill_to_platform(
    state: State<'_, AppState>,
    dir_path: String,
    agent_id: String,
    method: Option<String>,
) -> Result<ObsidianImportResult, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Err("Remote Obsidian import is not supported in this version.".to_string());
    }
    obsidian::import_obsidian_skill_to_platform_impl(
        &state.db,
        &dir_path,
        &agent_id,
        method.as_deref(),
    )
    .await
}
