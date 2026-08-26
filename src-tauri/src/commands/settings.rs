use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;
use tauri::State;

use super::settings_policy::{
    category_for_key, setting_audit_details, validate_setting, SettingCategory,
};
use crate::db::{self, DbPool, ScanDirectory};
use crate::operation_log::{
    record_operation_log_best_effort, target_context_from_active_target, with_operation_log,
    OperationLogEvent, OperationSpec,
};
use crate::paths::{expand_home_path, expand_remote_home_path, path_to_string};
use crate::secrets::{AI_API_KEY_SECRET_KEY, GITHUB_PAT_SECRET_KEY};
use crate::AppState;

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
    crate::ipc_boundary_async!({
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
    crate::ipc_boundary_async!({
        let request_context = state.resolve_target_context().await?;
        let active_target = request_context.target().clone();
        let target_context = target_context_from_active_target(&active_target);
        let pool = request_context.db().clone();
        let remote_home = active_target.remote_home();
        let started_at = Instant::now();
        let result =
            add_scan_directory_impl_for_home(&pool, &path, label.as_deref(), remote_home).await;
        match &result {
            Ok(directory) => {
                record_operation_log_best_effort(
                    &state.db,
                    target_context,
                    OperationLogEvent::new(
                        "settings",
                        "scan_dir.add",
                        "succeeded",
                        format!("Added scan directory {}", directory.path),
                    )
                    .subject("scan_root", &directory.path, &directory.path)
                    .details(json!({
                        "path": &directory.path,
                        "label": &directory.label,
                    }))
                    .duration_ms(started_at.elapsed().as_millis() as i64),
                )
                .await;
            }
            Err(error) => {
                record_operation_log_best_effort(
                    &state.db,
                    target_context,
                    OperationLogEvent::new(
                        "settings",
                        "scan_dir.add",
                        "failed",
                        format!("Failed to add scan directory {}", path),
                    )
                    .subject("scan_root", &path, &path)
                    .error(error)
                    .duration_ms(started_at.elapsed().as_millis() as i64),
                )
                .await;
            }
        }
        result
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn remove_scan_directory(
    state: State<'_, AppState>,
    path: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!({
        let request_context = state.resolve_target_context().await?;
        let active_target = request_context.target().clone();
        let target_context = target_context_from_active_target(&active_target);
        let pool = request_context.db().clone();
        with_operation_log(
            &state,
            OperationSpec::new(
                target_context,
                |_: &(), duration_ms| {
                    OperationLogEvent::new(
                        "settings",
                        "scan_dir.remove",
                        "succeeded",
                        format!("Removed scan directory {}", path),
                    )
                    .subject("scan_root", &path, &path)
                    .duration_ms(duration_ms)
                },
                |_: &String, duration_ms| {
                    OperationLogEvent::new(
                        "settings",
                        "scan_dir.remove",
                        "failed",
                        format!("Failed to remove scan directory {}", path),
                    )
                    .subject("scan_root", &path, &path)
                    .duration_ms(duration_ms)
                },
            ),
            || remove_scan_directory_impl(&pool, &path),
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
    crate::ipc_boundary_async!({
        let request_context = state.resolve_target_context().await?;
        let active_target = request_context.target().clone();
        let target_context = target_context_from_active_target(&active_target);
        let pool = request_context.db().clone();
        with_operation_log(
            &state,
            OperationSpec::new(
                target_context,
                |_: &(), duration_ms| {
                    OperationLogEvent::new(
                        "settings",
                        "scan_dir.toggle",
                        "succeeded",
                        format!("Updated scan directory {} enabled={}", path, is_active),
                    )
                    .subject("scan_root", &path, &path)
                    .details(json!({
                        "path": &path,
                        "isActive": is_active,
                    }))
                    .duration_ms(duration_ms)
                },
                |_: &String, duration_ms| {
                    OperationLogEvent::new(
                        "settings",
                        "scan_dir.toggle",
                        "failed",
                        format!("Failed to update scan directory {}", path),
                    )
                    .subject("scan_root", &path, &path)
                    .details(json!({
                        "path": &path,
                        "isActive": is_active,
                    }))
                    .duration_ms(duration_ms)
                },
            ),
            || set_scan_directory_active_impl(&pool, &path, is_active),
        )
        .await
    })
}

#[tauri::command]
pub async fn get_setting(
    state: State<'_, AppState>,
    key: String,
) -> crate::ipc_error::IpcResult<Option<String>> {
    crate::ipc_boundary_async!({ get_setting_impl(&state.db, &key).await })
}

#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> crate::ipc_error::IpcResult<HashMap<String, Option<String>>> {
    crate::ipc_boundary_async!({ get_settings_impl(&state.db, &keys).await })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn get_ai_api_key_state(
    state: State<'_, AppState>,
    provider: Option<String>,
) -> crate::ipc_error::IpcResult<crate::services::ai_provider::AiApiKeyState> {
    crate::ipc_boundary_async!({
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
    crate::ipc_boundary_async!({
        crate::services::ai_provider::set_ai_api_key_impl(
            &state.db,
            state.secrets.as_ref(),
            value,
            provider.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn clear_ai_api_key(
    state: State<'_, AppState>,
    provider: Option<String>,
) -> crate::ipc_error::IpcResult<crate::services::ai_provider::AiApiKeyState> {
    crate::ipc_boundary_async!({
        crate::services::ai_provider::clear_ai_api_key_impl(
            &state.db,
            state.secrets.as_ref(),
            provider.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!({
        let target_context = state
            .active_target()
            .await
            .map(|target| target_context_from_active_target(&target))
            .unwrap_or_else(|_| crate::operation_log::local_target_context());
        let started_at = Instant::now();
        let category = category_for_key(&key);
        let result = set_setting_impl(&state.db, &key, &value).await;
        let status = if result.is_ok() {
            "succeeded"
        } else {
            "failed"
        };
        let category_name = category.map(SettingCategory::as_str).unwrap_or("forbidden");
        let mut event = OperationLogEvent::new(
            "settings",
            "settings.set",
            status,
            if result.is_ok() {
                format!("Updated {category_name} setting")
            } else {
                format!("Failed to update {category_name} setting")
            },
        )
        .subject("setting_category", category_name, category_name)
        .details(setting_audit_details(
            std::iter::once(key.as_str()),
            result.is_ok(),
        ))
        .duration_ms(started_at.elapsed().as_millis() as i64);
        if let Err(error) = &result {
            event = event.error(error);
        }
        record_operation_log_best_effort(&state.db, target_context, event).await;
        result
    })
}

#[tauri::command]
pub async fn set_settings(
    state: State<'_, AppState>,
    values: HashMap<String, String>,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!({
        let target_context = state
            .active_target()
            .await
            .map(|target| target_context_from_active_target(&target))
            .unwrap_or_else(|_| crate::operation_log::local_target_context());
        let started_at = Instant::now();
        let result = set_settings_impl(&state.db, &values).await;
        let status = if result.is_ok() {
            "succeeded"
        } else {
            "failed"
        };
        let mut event = OperationLogEvent::new(
            "settings",
            "settings.set_batch",
            status,
            if result.is_ok() {
                format!("Updated {} settings", values.len())
            } else {
                "Failed to update settings batch".to_string()
            },
        )
        .details(setting_audit_details(
            values.keys().map(String::as_str),
            result.is_ok(),
        ))
        .duration_ms(started_at.elapsed().as_millis() as i64);
        if let Err(error) = &result {
            event = event.error(error);
        }
        record_operation_log_best_effort(&state.db, target_context, event).await;
        result
    })
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
