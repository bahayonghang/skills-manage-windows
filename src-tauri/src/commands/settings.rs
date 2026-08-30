#[cfg(test)]
use serde_json::json;
use std::collections::HashMap;
use tauri::State;

use super::settings_policy::{
    category_for_key, setting_audit_details, validate_setting, SettingCategory,
};
use crate::db::{self, DbPool, ScanDirectory};
use crate::observability::{
    CommandLogPolicy, OperationContext, OperationDefinition, OperationTarget, OperationTargetKind,
    ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeOperationResult,
};
use crate::paths::{expand_home_path, expand_remote_home_path, path_to_string};
use crate::secrets::{AI_API_KEY_SECRET_KEY, GITHUB_PAT_SECRET_KEY};
use crate::AppState;

fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("settings command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => unreachable!("settings command must have an operation policy"),
    }
}

fn reviewed_failure(definition: OperationDefinition) -> ReviewedFailure {
    ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
}

fn audit_target(target: &crate::targets::ActiveTarget) -> (OperationTargetKind, OperationTarget) {
    match target {
        crate::targets::ActiveTarget::Local => {
            (OperationTargetKind::Local, OperationTarget::local())
        }
        crate::targets::ActiveTarget::Ssh(target) => (
            OperationTargetKind::Ssh,
            OperationTarget::new(OperationTargetKind::Ssh, &target.id),
        ),
        crate::targets::ActiveTarget::Wsl(target) => (
            OperationTargetKind::Wsl,
            OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        ),
    }
}

// ─── Core Implementations (testable without Tauri State) ──────────────────────

const PROTECTED_SETTINGS_KEYS: &[&str] = &[GITHUB_PAT_SECRET_KEY, AI_API_KEY_SECRET_KEY];

fn is_protected_settings_key(key: &str) -> bool {
    let trimmed = key.trim();
    PROTECTED_SETTINGS_KEYS
        .iter()
        .any(|protected_key| trimmed.eq_ignore_ascii_case(protected_key))
        || trimmed
            .to_ascii_lowercase()
            .starts_with(&format!("{}__", AI_API_KEY_SECRET_KEY))
}

fn protected_settings_error(key: &str) -> String {
    format!(
        "Setting '{}' is managed by secure storage; use the dedicated command instead.",
        key.trim()
    )
}

/// Return all scan directories, built-in first then custom ordered by added_at.
pub async fn get_scan_directories_impl(pool: &DbPool) -> Result<Vec<ScanDirectory>, String> {
    db::get_scan_directories(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Add a new custom (non-builtin) scan directory.
/// Returns the newly created record.
pub async fn add_scan_directory_impl(
    pool: &DbPool,
    path: &str,
    label: Option<&str>,
) -> Result<ScanDirectory, String> {
    add_scan_directory_impl_for_home(pool, path, label, None).await
}

async fn add_scan_directory_impl_for_home(
    pool: &DbPool,
    path: &str,
    label: Option<&str>,
    remote_home: Option<&str>,
) -> Result<ScanDirectory, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Scan directory path cannot be empty".to_string());
    }
    let expanded_path = expand_scan_directory_path(path, remote_home);
    db::add_scan_directory(pool, &expanded_path, label)
        .await
        .map_err(|e| e.to_string())
}

fn expand_scan_directory_path(path: &str, remote_home: Option<&str>) -> String {
    match remote_home {
        Some(home) => expand_remote_home_path(path, home),
        None => path_to_string(&expand_home_path(path)),
    }
}

/// Remove a custom (non-builtin) scan directory by path.
/// Returns an error if the directory is built-in or not found.
pub async fn remove_scan_directory_impl(pool: &DbPool, path: &str) -> Result<(), String> {
    db::remove_scan_directory(pool, path)
        .await
        .map_err(|e| e.to_string())
}

/// Toggle the `is_active` flag on a scan directory by path.
pub async fn set_scan_directory_active_impl(
    pool: &DbPool,
    path: &str,
    is_active: bool,
) -> Result<(), String> {
    db::toggle_scan_directory(pool, path, is_active)
        .await
        .map_err(|e| e.to_string())
}

/// Get a settings value by key. Returns `None` if the key is not set.
pub async fn get_setting_impl(pool: &DbPool, key: &str) -> Result<Option<String>, String> {
    if is_protected_settings_key(key) {
        return Err(protected_settings_error(key));
    }
    db::get_setting(pool, key).await.map_err(|e| e.to_string())
}

/// Get multiple settings values by key. Missing keys map to `None`.
pub async fn get_settings_impl(
    pool: &DbPool,
    keys: &[String],
) -> Result<HashMap<String, Option<String>>, String> {
    if let Some(key) = keys.iter().find(|key| is_protected_settings_key(key)) {
        return Err(protected_settings_error(key));
    }
    db::get_settings(pool, keys)
        .await
        .map_err(|e| e.to_string())
}

/// Set (upsert) a settings value.
pub async fn set_setting_impl(pool: &DbPool, key: &str, value: &str) -> Result<(), String> {
    validate_setting(key, value)?;
    db::set_setting(pool, key, value)
        .await
        .map_err(|e| e.to_string())
}

/// Set (upsert) multiple settings values in one batch.
pub async fn set_settings_impl(
    pool: &DbPool,
    values: &HashMap<String, String>,
) -> Result<(), String> {
    for (key, value) in values {
        validate_setting(key, value)?;
    }
    db::set_settings(pool, values)
        .await
        .map_err(|e| e.to_string())
}

// ─── Tauri Commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_scan_directories(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<ScanDirectory>> {
    crate::ipc_boundary_async!("get_scan_directories", {
        let pool = state.active_db().await?;
        get_scan_directories_impl(&pool).await
    })
}

#[tauri::command]
pub async fn add_scan_directory(
    state: State<'_, AppState>,
    path: String,
    label: Option<String>,
) -> crate::ipc_error::IpcResult<ScanDirectory> {
    crate::ipc_boundary_async!("add_scan_directory", {
        let request_context = state.resolve_target_context().await?;
        let active_target = request_context.target().clone();
        let (_, audit_target) = audit_target(&active_target);
        let pool = request_context.db().clone();
        let remote_home = active_target.remote_home();
        let definition = operation_definition("add_scan_directory");
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(audit_target),
            |_| SafeOperationResult::succeeded("Scan directory added."),
            || async {
                add_scan_directory_impl_for_home(&pool, &path, label.as_deref(), remote_home)
                    .await
                    .map_err(|_| reviewed_failure(definition))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn remove_scan_directory(
    state: State<'_, AppState>,
    path: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("remove_scan_directory", {
        let request_context = state.resolve_target_context().await?;
        let active_target = request_context.target().clone();
        let (_, audit_target) = audit_target(&active_target);
        let pool = request_context.db().clone();
        let definition = operation_definition("remove_scan_directory");
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(audit_target),
            |_| SafeOperationResult::succeeded("Scan directory removed."),
            || async {
                remove_scan_directory_impl(&pool, &path)
                    .await
                    .map_err(|_| reviewed_failure(definition))
            },
        )
        .await
    })
}

#[tauri::command]
pub async fn set_scan_directory_active(
    state: State<'_, AppState>,
    path: String,
    is_active: bool,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("set_scan_directory_active", {
        let request_context = state.resolve_target_context().await?;
        let active_target = request_context.target().clone();
        let (_, audit_target) = audit_target(&active_target);
        let pool = request_context.db().clone();
        let definition = operation_definition("set_scan_directory_active");
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(audit_target),
            |_| {
                SafeOperationResult::succeeded("Scan directory state updated.")
                    .flag(SafeDetailKey::Changed, true)
            },
            || async {
                set_scan_directory_active_impl(&pool, &path, is_active)
                    .await
                    .map_err(|_| reviewed_failure(definition))
            },
        )
        .await
    })
}

#[tauri::command]
pub async fn get_setting(
    state: State<'_, AppState>,
    key: String,
) -> crate::ipc_error::IpcResult<Option<String>> {
    crate::ipc_boundary_async!("get_setting", { get_setting_impl(&state.db, &key).await })
}

#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> crate::ipc_error::IpcResult<HashMap<String, Option<String>>> {
    crate::ipc_boundary_async!("get_settings", {
        get_settings_impl(&state.db, &keys).await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn get_ai_api_key_state(
    state: State<'_, AppState>,
    provider: Option<String>,
) -> crate::ipc_error::IpcResult<crate::services::ai_provider::AiApiKeyState> {
    crate::ipc_boundary_async!("get_ai_api_key_state", {
        crate::services::ai_provider::get_ai_api_key_state_impl(
            &state.db,
            state.secrets.as_ref(),
            provider.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn set_ai_api_key(
    state: State<'_, AppState>,
    value: String,
    provider: Option<String>,
) -> crate::ipc_error::IpcResult<crate::services::ai_provider::AiApiKeyState> {
    crate::ipc_boundary_async!("set_ai_api_key", {
        let definition = operation_definition("set_ai_api_key");
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |_| {
                SafeOperationResult::succeeded("AI API credential stored.")
                    .flag(SafeDetailKey::Changed, true)
            },
            || async {
                crate::services::ai_provider::set_ai_api_key_impl(
                    &state.db,
                    state.secrets.as_ref(),
                    value,
                    provider.as_deref(),
                )
                .await
                .map_err(|_| reviewed_failure(definition))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn clear_ai_api_key(
    state: State<'_, AppState>,
    provider: Option<String>,
) -> crate::ipc_error::IpcResult<crate::services::ai_provider::AiApiKeyState> {
    crate::ipc_boundary_async!("clear_ai_api_key", {
        let definition = operation_definition("clear_ai_api_key");
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |_| {
                SafeOperationResult::succeeded("AI API credential cleared.")
                    .flag(SafeDetailKey::Changed, true)
            },
            || async {
                crate::services::ai_provider::clear_ai_api_key_impl(
                    &state.db,
                    state.secrets.as_ref(),
                    provider.as_deref(),
                )
                .await
                .map_err(|_| reviewed_failure(definition))
            },
        )
        .await
    })
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("set_setting", {
        let definition = operation_definition("set_setting");
        let category = category_for_key(&key)
            .map(SettingCategory::as_str)
            .unwrap_or("forbidden");
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |_| {
                SafeOperationResult::succeeded("Setting updated.")
                    .count(SafeDetailKey::AffectedCount, 1)
                    .flag(SafeDetailKey::Changed, true)
                    .stable(SafeDetailKey::Scope, category)
            },
            || async {
                set_setting_impl(&state.db, &key, &value)
                    .await
                    .map_err(|_| reviewed_failure(definition))
            },
        )
        .await
    })
}

#[tauri::command]
pub async fn set_settings(
    state: State<'_, AppState>,
    values: HashMap<String, String>,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("set_settings", {
        let definition = operation_definition("set_settings");
        let count = values.len() as u64;
        let category_count = setting_audit_details(values.keys().map(String::as_str), true)
            ["categories"]
            .as_array()
            .map_or(0, |categories| categories.len() as u64);
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            move |_| {
                SafeOperationResult::succeeded("Settings updated.")
                    .count(SafeDetailKey::AffectedCount, count)
                    .count(SafeDetailKey::RequestedCount, category_count)
                    .flag(SafeDetailKey::Changed, true)
            },
            || async {
                set_settings_impl(&state.db, &values)
                    .await
                    .map_err(|_| reviewed_failure(definition))
            },
        )
        .await
    })
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
