//! Tauri command shells for the Marketplace + AI explanation flows.
//!
//! Business logic lives in `crate::services::marketplace` (registry CRUD,
//! GitHub sync, install) and `crate::services::ai_provider` (Anthropic /
//! OpenAI-compatible explanation, cache, streaming). This file translates
//! IPC arguments + state into service calls and re-exports the public
//! types so frontend bindings remain stable.

use tauri::{AppHandle, State};

use crate::services::ai_provider;
use crate::services::marketplace;
use crate::AppState;

// Re-export the types frontend code already references via this module path.
pub use crate::services::ai_provider::{
    ExplanationChunkPayload, ExplanationCompletePayload, ExplanationErrorInfo, ExplanationErrorKind,
};
pub use crate::services::marketplace::{
    MarketplaceSkill, RegistryCacheMetadata, RegistrySyncStatus, SkillRegistry, SyncRegistryOptions,
};

// ─── Registry CRUD ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_registries(state: State<'_, AppState>) -> Result<Vec<SkillRegistry>, String> {
    let pool = state.active_db().await?;
    marketplace::list_registries_impl(&pool).await
}

#[tauri::command]
pub async fn add_registry(
    state: State<'_, AppState>,
    name: String,
    source_type: String,
    url: String,
) -> Result<SkillRegistry, String> {
    let pool = state.active_db().await?;
    marketplace::add_registry_impl(&pool, name, source_type, url, None).await
}

#[tauri::command]
pub async fn remove_registry(
    state: State<'_, AppState>,
    registry_id: String,
) -> Result<(), String> {
    let pool = state.active_db().await?;
    marketplace::remove_registry_impl(&pool, registry_id).await
}

#[tauri::command]
pub async fn sync_registry(
    state: State<'_, AppState>,
    registry_id: String,
) -> Result<Vec<MarketplaceSkill>, String> {
    let pool = state.active_db().await?;
    marketplace::sync_registry_impl(
        &pool,
        &state.db,
        state.secrets.as_ref(),
        registry_id,
        SyncRegistryOptions::default(),
    )
    .await
}

#[tauri::command]
pub async fn sync_registry_with_options(
    state: State<'_, AppState>,
    registry_id: String,
    options: Option<SyncRegistryOptions>,
) -> Result<Vec<MarketplaceSkill>, String> {
    let pool = state.active_db().await?;
    marketplace::sync_registry_impl(
        &pool,
        &state.db,
        state.secrets.as_ref(),
        registry_id,
        options.unwrap_or_default(),
    )
    .await
}

#[tauri::command]
pub async fn search_marketplace_skills(
    state: State<'_, AppState>,
    registry_id: Option<String>,
    query: Option<String>,
) -> Result<Vec<MarketplaceSkill>, String> {
    let pool = state.active_db().await?;
    marketplace::search_marketplace_skills_impl(&pool, registry_id, query).await
}

#[tauri::command]
pub async fn install_marketplace_skill(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<(), String> {
    let active_target = state.active_target().await?;
    let pool = state.active_db().await?;
    marketplace::install_marketplace_skill_impl(&pool, active_target, skill_id).await
}

// ─── AI Explanation ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn explain_skill(state: State<'_, AppState>, content: String) -> Result<String, String> {
    ai_provider::explain_skill_impl(&state.db, state.secrets.as_ref(), content).await
}

#[tauri::command]
pub async fn get_skill_explanation(
    state: State<'_, AppState>,
    skill_id: String,
    lang: String,
) -> Result<Option<String>, String> {
    ai_provider::get_skill_explanation_impl(&state.db, skill_id, lang).await
}

#[tauri::command]
pub async fn explain_skill_stream(
    state: State<'_, AppState>,
    app: AppHandle,
    skill_id: String,
    content: String,
    lang: String,
) -> Result<(), String> {
    ai_provider::explain_skill_stream_impl(
        &state.db,
        state.secrets.as_ref(),
        &app,
        skill_id,
        content,
        lang,
    )
    .await
}

#[tauri::command]
pub async fn refresh_skill_explanation(
    state: State<'_, AppState>,
    app: AppHandle,
    skill_id: String,
    content: String,
    lang: String,
) -> Result<(), String> {
    ai_provider::refresh_skill_explanation_impl(
        &state.db,
        state.secrets.as_ref(),
        &app,
        skill_id,
        content,
        lang,
    )
    .await
}
