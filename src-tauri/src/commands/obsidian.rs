//! Tauri IPC shells for Obsidian vault scanning + source-import. Replaces the
//! corresponding entry points that used to live in `commands::discover`.

use std::path::{Path, PathBuf};

use tauri::State;

use crate::services::obsidian::{self, ObsidianImportResult, ObsidianSkill, ObsidianVault};
use crate::AppState;

#[tauri::command]
pub async fn get_obsidian_vaults(state: State<'_, AppState>) -> Result<Vec<ObsidianVault>, String> {
    if state.active_target().await?.is_remote_like() {
        return Ok(Vec::new());
    }
    obsidian::get_obsidian_vaults_impl(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_obsidian_vault_skills(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<ObsidianSkill>, String> {
    if state.active_target().await?.is_remote_like() {
        return Ok(Vec::new());
    }
    obsidian::get_obsidian_vault_skills_impl(&state.db, &vault_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_obsidian_path(state: State<'_, AppState>, path: String) -> Result<(), String> {
    if state.active_target().await?.is_remote_like() {
        return Err(
            "Remote Obsidian paths cannot be opened in the local file manager.".to_string(),
        );
    }

    let candidate = canonicalize_existing_path(&path)?;
    let vaults = obsidian::get_obsidian_vaults_impl(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    let allowed = vaults
        .iter()
        .filter_map(|vault| PathBuf::from(&vault.path).canonicalize().ok())
        .any(|vault_root| candidate == vault_root || candidate.starts_with(&vault_root));

    if !allowed {
        return Err(format!(
            "Refusing to open '{}': path is not under a detected Obsidian vault.",
            candidate.display()
        ));
    }

    open_local_path_in_file_manager(&candidate)
}

#[tauri::command]
pub async fn import_obsidian_skill_to_central(
    state: State<'_, AppState>,
    dir_path: String,
) -> Result<ObsidianImportResult, String> {
    if state.active_target().await?.is_remote_like() {
        return Err("Remote Obsidian import is not supported in this version.".to_string());
    }
    obsidian::import_obsidian_skill_to_central_impl(&state.db, &dir_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_obsidian_skill_to_platform(
    state: State<'_, AppState>,
    dir_path: String,
    agent_id: String,
    method: Option<String>,
) -> Result<ObsidianImportResult, String> {
    if state.active_target().await?.is_remote_like() {
        return Err("Remote Obsidian import is not supported in this version.".to_string());
    }
    obsidian::import_obsidian_skill_to_platform_impl(
        &state.db,
        &dir_path,
        &agent_id,
        method.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

fn canonicalize_existing_path(path: &str) -> Result<PathBuf, String> {
    Path::new(path)
        .canonicalize()
        .map_err(|e| format!("Failed to resolve '{}': {}", path, e))
}

fn open_local_path_in_file_manager(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }

    Ok(())
}
