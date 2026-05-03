//! Tauri IPC shells for project discovery.
//!
//! Business logic lives in `crate::services::discovery`. This module keeps the
//! existing command names and public type paths stable while translating
//! `State<AppState>` into service inputs and enforcing the current local-only
//! Discover boundary.

use tauri::{AppHandle, State};

use crate::services::discovery;
use crate::targets::ActiveTarget;
use crate::AppState;

// Re-export the public service surface so existing Rust call-sites that import
// `commands::discover::*` keep compiling while implementation lives in services.
pub use crate::services::discovery::*;

#[tauri::command]
pub async fn discover_scan_roots() -> Result<Vec<ScanRoot>, String> {
    Ok(discovery::default_scan_roots())
}

#[tauri::command]
pub async fn get_scan_roots(state: State<'_, AppState>) -> Result<Vec<ScanRoot>, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Ok(Vec::new());
    }
    discovery::get_scan_roots_impl(&state.db).await
}

#[tauri::command]
pub async fn get_obsidian_vaults(
    state: State<'_, AppState>,
) -> Result<Vec<ObsidianVault>, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Ok(Vec::new());
    }
    discovery::get_obsidian_vaults_impl(&state.db).await
}

#[tauri::command]
pub async fn get_obsidian_vault_skills(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<DiscoveredSkill>, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Ok(Vec::new());
    }
    discovery::get_obsidian_vault_skills_impl(&state.db, &vault_id).await
}

#[tauri::command]
pub async fn set_scan_root_enabled(
    state: State<'_, AppState>,
    path: String,
    enabled: bool,
) -> Result<(), String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Err("Remote Discover scanning is not supported in this version.".to_string());
    }
    discovery::set_scan_root_enabled_impl(&state.db, path, enabled).await
}

#[tauri::command]
pub async fn start_project_scan(
    state: State<'_, AppState>,
    app: AppHandle,
    roots: Vec<ScanRoot>,
) -> Result<DiscoverResult, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Err("Remote Discover scanning is not supported in this version.".to_string());
    }
    discovery::start_project_scan_impl(&state.db, &app, roots).await
}

#[tauri::command]
pub async fn stop_project_scan() -> Result<(), String> {
    discovery::stop_project_scan_impl()
}

#[tauri::command]
pub async fn get_discovered_summary(
    state: State<'_, AppState>,
) -> Result<DiscoveredSummary, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Ok(DiscoveredSummary {
            total_skills_found: 0,
            total_projects_found: 0,
        });
    }
    discovery::get_discovered_summary_impl(&state.db).await
}

#[tauri::command]
pub async fn get_discovered_skills(
    state: State<'_, AppState>,
) -> Result<Vec<DiscoveredProject>, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Ok(Vec::new());
    }
    discovery::get_discovered_skills_impl(&state.db).await
}

#[tauri::command]
pub async fn import_discovered_skill_to_central(
    state: State<'_, AppState>,
    discovered_skill_id: String,
) -> Result<ImportResult, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Err("Remote Discover import is not supported in this version.".to_string());
    }
    discovery::import_discovered_skill_to_central_impl(&state.db, &discovered_skill_id).await
}

#[tauri::command]
pub async fn import_source_skill_to_central(
    state: State<'_, AppState>,
    file_path: String,
    dir_path: String,
) -> Result<ImportResult, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Err("Remote Discover import is not supported in this version.".to_string());
    }
    discovery::import_source_skill_to_central_impl(&state.db, &file_path, &dir_path).await
}

#[tauri::command]
pub async fn import_discovered_skill_to_platform(
    state: State<'_, AppState>,
    discovered_skill_id: String,
    agent_id: String,
    method: Option<String>,
) -> Result<ImportResult, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Err("Remote Discover import is not supported in this version.".to_string());
    }
    discovery::import_discovered_skill_to_platform_with_method_impl(
        &state.db,
        &discovered_skill_id,
        &agent_id,
        method.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn import_source_skill_to_platform(
    state: State<'_, AppState>,
    file_path: String,
    dir_path: String,
    agent_id: String,
    method: Option<String>,
) -> Result<ImportResult, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Err("Remote Discover import is not supported in this version.".to_string());
    }
    discovery::import_source_skill_to_platform_with_method_impl(
        &state.db,
        &file_path,
        &dir_path,
        &agent_id,
        method.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn clear_discovered_skills(state: State<'_, AppState>) -> Result<(), String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Ok(());
    }
    discovery::clear_discovered_skills_impl(&state.db).await
}
