//! Tauri command shells for the Marketplace + AI explanation flows.
//!
//! Business logic lives in `crate::services::marketplace` (registry CRUD,
//! GitHub sync, install) and `crate::services::ai_provider` (Anthropic /
//! OpenAI-compatible explanation, cache, streaming). This file translates
//! IPC arguments + state into service calls and re-exports the public
//! types so frontend bindings remain stable.

use tauri::{AppHandle, State};

use std::collections::HashMap;

use crate::services::ai_provider;
use crate::services::marketplace;
use crate::AppState;

// Re-export the types frontend code already references via this module path.
pub use crate::services::ai_provider::{
    AiConnectionTestResult, ExplanationChunkPayload, ExplanationCompletePayload,
    ExplanationErrorInfo, ExplanationErrorKind,
};
pub use crate::services::marketplace::{
    MarketplaceSkill, RegistryCacheMetadata, RegistrySyncStatus, SkillRegistry, SkillsShFileEntry,
    SkillsShSkill, SyncRegistryOptions,
};

// ─── Registry CRUD ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_registries(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<SkillRegistry>> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            marketplace::list_registries_impl(&pool)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn add_registry(
    state: State<'_, AppState>,
    name: String,
    source_type: String,
    url: String,
) -> crate::ipc_error::IpcResult<SkillRegistry> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            marketplace::add_registry_impl(&pool, name, source_type, url, None)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn remove_registry(
    state: State<'_, AppState>,
    registry_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            marketplace::remove_registry_impl(&pool, registry_id)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn sync_registry(
    state: State<'_, AppState>,
    registry_id: String,
) -> crate::ipc_error::IpcResult<Vec<MarketplaceSkill>> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            marketplace::sync_registry_impl(
                &pool,
                &state.db,
                state.secrets.as_ref(),
                registry_id,
                SyncRegistryOptions::default(),
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn sync_registry_with_options(
    state: State<'_, AppState>,
    registry_id: String,
    options: Option<SyncRegistryOptions>,
) -> crate::ipc_error::IpcResult<Vec<MarketplaceSkill>> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            marketplace::sync_registry_impl(
                &pool,
                &state.db,
                state.secrets.as_ref(),
                registry_id,
                options.unwrap_or_default(),
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn search_marketplace_skills(
    state: State<'_, AppState>,
    registry_id: Option<String>,
    query: Option<String>,
) -> crate::ipc_error::IpcResult<Vec<MarketplaceSkill>> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            marketplace::search_marketplace_skills_impl(&pool, registry_id, query)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn install_marketplace_skill(
    state: State<'_, AppState>,
    skill_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            marketplace::install_marketplace_skill_impl(&pool, active_target, skill_id)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn search_skills_sh(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> crate::ipc_error::IpcResult<Vec<SkillsShSkill>> {
    crate::ipc_boundary!(
        async move {
            marketplace::search_skills_sh_impl(&state.db, state.secrets.as_ref(), query, limit)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn resolve_skills_sh_url(
    state: State<'_, AppState>,
    source: String,
    skill_id: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        async move {
            marketplace::resolve_skills_sh_url_impl(
                &state.db,
                state.secrets.as_ref(),
                source,
                skill_id,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn browse_skills_sh_directory(
    state: State<'_, AppState>,
    source: String,
    skill_id: String,
) -> crate::ipc_error::IpcResult<Vec<SkillsShFileEntry>> {
    crate::ipc_boundary!(
        async move {
            marketplace::browse_skills_sh_directory_impl(
                &state.db,
                state.secrets.as_ref(),
                source,
                skill_id,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn read_skills_sh_file(
    state: State<'_, AppState>,
    source: String,
    file_path: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        async move {
            marketplace::read_skills_sh_file_impl(
                &state.db,
                state.secrets.as_ref(),
                source,
                file_path,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn install_from_skills_sh(
    state: State<'_, AppState>,
    source: String,
    skill_id: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            marketplace::install_from_skills_sh_impl(
                &pool,
                state.secrets.as_ref(),
                active_target,
                source,
                skill_id,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

// ─── AI Explanation ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn explain_skill(
    state: State<'_, AppState>,
    content: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        async move {
            ai_provider::explain_skill_impl(&state.db, state.secrets.as_ref(), content)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn test_ai_connection(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<AiConnectionTestResult> {
    crate::ipc_boundary!(
        async move {
            ai_provider::test_ai_connection_impl(&state.db, state.secrets.as_ref())
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn get_skill_explanation(
    state: State<'_, AppState>,
    skill_id: String,
    lang: String,
) -> crate::ipc_error::IpcResult<Option<String>> {
    crate::ipc_boundary!(
        async move {
            ai_provider::get_skill_explanation_impl(&state.db, skill_id, lang)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn get_skill_explanation_summaries(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
    lang: String,
) -> crate::ipc_error::IpcResult<HashMap<String, String>> {
    crate::ipc_boundary!(
        async move {
            ai_provider::get_skill_explanation_summaries_impl(&state.db, skill_ids, lang)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn explain_skill_stream(
    state: State<'_, AppState>,
    app: AppHandle,
    skill_id: String,
    content: String,
    lang: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        async move {
            ai_provider::explain_skill_stream_impl(
                &state.db,
                state.secrets.as_ref(),
                &app,
                skill_id,
                content,
                lang,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn refresh_skill_explanation(
    state: State<'_, AppState>,
    app: AppHandle,
    skill_id: String,
    content: String,
    lang: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        async move {
            ai_provider::refresh_skill_explanation_impl(
                &state.db,
                state.secrets.as_ref(),
                &app,
                skill_id,
                content,
                lang,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}
