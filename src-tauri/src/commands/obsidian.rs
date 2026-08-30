//! Tauri IPC shells for Obsidian vault scanning + source-import. Replaces the
//! corresponding entry points that used to live in `commands::discover`.

use std::path::{Path, PathBuf};

use tauri::State;

use crate::observability::{
    CommandLogPolicy, OperationContext, OperationSubjectKind, OperationTarget, ReviewedDiagnostic,
    ReviewedFailure, SafeDetailKey, SafeIdentifier, SafeOperationResult,
};
use crate::services::obsidian::{self, ObsidianImportResult, ObsidianSkill, ObsidianVault};
use crate::AppState;

#[tauri::command]
pub async fn get_obsidian_vaults(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<ObsidianVault>> {
    crate::ipc_boundary!(
        "get_obsidian_vaults",
        async move {
            if state.active_target().await?.is_remote_like() {
                return Ok(Vec::new());
            }
            obsidian::get_obsidian_vaults_impl(&state.db)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn get_obsidian_vault_skills(
    state: State<'_, AppState>,
    vault_id: String,
) -> crate::ipc_error::IpcResult<Vec<ObsidianSkill>> {
    crate::ipc_boundary!(
        "get_obsidian_vault_skills",
        async move {
            if state.active_target().await?.is_remote_like() {
                return Ok(Vec::new());
            }
            obsidian::get_obsidian_vault_skills_impl(&state.db, &vault_id)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn open_obsidian_path(
    state: State<'_, AppState>,
    path: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("open_obsidian_path", {
        let entry = crate::ipc_registry::command_policy("open_obsidian_path")
            .expect("open_obsidian_path must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("open_obsidian_path must be auditable")
        };
        let app_state = state.inner();
        crate::observability::run_operation(
            app_state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |_| SafeOperationResult::succeeded("Obsidian path opened."),
            || async move {
                if app_state
                    .active_target()
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))?
                    .is_remote_like()
                {
                    return Err(ReviewedFailure::new(ReviewedDiagnostic::unexpected(
                        definition,
                    )));
                }
                let candidate = canonicalize_existing_path(&path).map_err(|_| {
                    ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
                })?;
                let vaults = obsidian::get_obsidian_vaults_impl(&app_state.db)
                    .await
                    .map_err(|_| {
                        ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
                    })?;
                let allowed = vaults
                    .iter()
                    .filter_map(|vault| PathBuf::from(&vault.path).canonicalize().ok())
                    .any(|vault_root| {
                        candidate == vault_root || candidate.starts_with(&vault_root)
                    });
                if !allowed {
                    return Err(ReviewedFailure::new(ReviewedDiagnostic::unexpected(
                        definition,
                    )));
                }
                open_local_path_in_file_manager(&candidate)
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn import_obsidian_skill_to_central(
    state: State<'_, AppState>,
    dir_path: String,
) -> crate::ipc_error::IpcResult<ObsidianImportResult> {
    crate::ipc_boundary_async!("import_obsidian_skill_to_central", {
        let entry = crate::ipc_registry::command_policy("import_obsidian_skill_to_central")
            .expect("import_obsidian_skill_to_central must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("import_obsidian_skill_to_central must be auditable")
        };
        let app_state = state.inner();
        crate::observability::run_operation(
            app_state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |result: &ObsidianImportResult| {
                SafeOperationResult::succeeded("Obsidian skill imported to Central.").identifier(
                    SafeDetailKey::Identifier,
                    SafeIdentifier::new(&result.skill_id),
                )
            },
            || async move {
                if app_state
                    .active_target()
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))?
                    .is_remote_like()
                {
                    return Err(ReviewedFailure::new(ReviewedDiagnostic::unexpected(
                        definition,
                    )));
                }
                obsidian::import_obsidian_skill_to_central_impl(&app_state.db, &dir_path)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn import_obsidian_skill_to_platform(
    state: State<'_, AppState>,
    dir_path: String,
    agent_id: String,
    method: Option<String>,
) -> crate::ipc_error::IpcResult<ObsidianImportResult> {
    crate::ipc_boundary_async!("import_obsidian_skill_to_platform", {
        let entry = crate::ipc_registry::command_policy("import_obsidian_skill_to_platform")
            .expect("import_obsidian_skill_to_platform must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("import_obsidian_skill_to_platform must be auditable")
        };
        let app_state = state.inner();
        let context = OperationContext::new(OperationTarget::local())
            .subject(OperationSubjectKind::Agent, SafeIdentifier::new(&agent_id));
        crate::observability::run_operation(
            app_state,
            definition,
            context,
            |result: &ObsidianImportResult| {
                SafeOperationResult::succeeded("Obsidian skill imported to agent.").identifier(
                    SafeDetailKey::Identifier,
                    SafeIdentifier::new(&result.skill_id),
                )
            },
            || async move {
                if app_state
                    .active_target()
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))?
                    .is_remote_like()
                {
                    return Err(ReviewedFailure::new(ReviewedDiagnostic::unexpected(
                        definition,
                    )));
                }
                obsidian::import_obsidian_skill_to_platform_impl(
                    &app_state.db,
                    &dir_path,
                    &agent_id,
                    method.as_deref(),
                )
                .await
                .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
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

#[cfg(test)]
mod tests {
    #[test]
    fn obsidian_commands_have_named_boundaries_without_path_diagnostics() {
        let source = include_str!("obsidian.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        for command in [
            "get_obsidian_vaults",
            "get_obsidian_vault_skills",
            "open_obsidian_path",
            "import_obsidian_skill_to_central",
            "import_obsidian_skill_to_platform",
        ] {
            assert!(production.contains(&format!("\"{command}\"")), "{command}");
        }
        for banned in [
            "SafeIdentifier::new(&path)",
            "SafeIdentifier::new(&dir_path)",
            "candidate.display()",
            "error = %",
            "OperationLogEvent",
        ] {
            assert!(!production.contains(banned), "banned audit input: {banned}");
        }
    }
}
