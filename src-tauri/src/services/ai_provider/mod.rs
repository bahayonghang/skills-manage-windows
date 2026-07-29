//! AI provider service: orchestrates the explanation flows shared by the
//! `explain_skill`, `get_skill_explanation`, `explain_skill_stream`, and
//! `refresh_skill_explanation` IPC commands.
//!
//! The internal split:
//! - `error` — `reqwest::Error` classification + actionable hints
//! - `prompt` — protocol detection + prompt / request-body builders
//! - `cache` — `skill_explanations` table I/O
//! - `stream` — SSE streaming + provider fallback + Tauri event emission
//! - `claude` — non-streaming one-shot path

mod cache;
mod claude;
mod config;
mod error;
mod prompt;
mod secret;
mod stream;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

pub use claude::AiConnectionTestResult;
pub(crate) use config::resolve_ai_provider_config;
#[cfg(test)]
pub(crate) use error::AI_CONNECT;
pub(crate) use error::{
    coded_error, coded_error_with_details, AI_CLIENT_BUILD_FAILED, AI_EMPTY_RESPONSE,
    AI_INVALID_API_KEY, AI_MISSING_API_KEY, AI_RATE_LIMIT, AI_REQUEST_FAILED, AI_RESPONSE_ERROR,
    AI_RESPONSE_PARSE_FAILED, AI_RESPONSE_READ_FAILED,
};
pub use error::{AiProviderError, ExplanationErrorInfo, ExplanationErrorKind};
pub use prompt::ExplanationApiProtocol;
pub use secret::{
    clear_ai_api_key_impl, get_ai_api_key_state_impl, migrate_ai_api_key_on_startup,
    set_ai_api_key_impl, AiApiKeyState,
};
pub use stream::{ExplanationChunkPayload, ExplanationCompletePayload};

use tauri::{AppHandle, Emitter};

/// Helper: read a non-sensitive AI setting from the DB, filtering out empty values.
pub(crate) async fn get_ai_setting(pool: &crate::db::DbPool, key: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
}

pub(crate) async fn get_ai_api_key_for_provider(
    pool: &crate::db::DbPool,
    secrets: &dyn crate::secrets::SecretStore,
    provider: &str,
) -> Result<Option<String>, AiProviderError> {
    secret::ai_api_key_from_secret_store(pool, secrets, Some(provider)).await
}

/// Issue a single non-streaming explanation request and return the parsed text.
pub async fn explain_skill_impl(
    pool: &crate::db::DbPool,
    secrets: &dyn crate::secrets::SecretStore,
    content: String,
) -> Result<String, AiProviderError> {
    claude::explain_skill(pool, secrets, content).await
}

/// Issue a minimal request to validate the current provider configuration.
pub async fn test_ai_connection_impl(
    pool: &crate::db::DbPool,
    secrets: &dyn crate::secrets::SecretStore,
) -> Result<claude::AiConnectionTestResult, AiProviderError> {
    claude::test_ai_connection(pool, secrets).await
}

/// Read a cached explanation if one exists; never triggers the AI provider.
pub async fn get_skill_explanation_impl(
    pool: &crate::db::DbPool,
    skill_id: String,
    lang: String,
) -> Result<Option<String>, AiProviderError> {
    cache::load_cached_skill_explanation(pool, &skill_id, &lang).await
}

/// Read cached explanations for many skills. Never triggers the AI provider.
pub async fn get_skill_explanation_summaries_impl(
    pool: &crate::db::DbPool,
    skill_ids: Vec<String>,
    lang: String,
) -> Result<HashMap<String, String>, AiProviderError> {
    cache::load_cached_skill_explanation_summaries(pool, &skill_ids, &lang).await
}

/// Stream an AI-generated explanation. Cache hits are emitted as a single chunk
/// + complete pair so the frontend can react with the same listener wiring.
pub async fn explain_skill_stream_impl(
    pool: &crate::db::DbPool,
    secrets: &dyn crate::secrets::SecretStore,
    app: &AppHandle,
    skill_id: String,
    content: String,
    lang: String,
) -> Result<(), AiProviderError> {
    if let Some(explanation) = cache::load_cached_skill_explanation(pool, &skill_id, &lang).await? {
        let _ = app.emit(
            "skill:explanation:chunk",
            ExplanationChunkPayload {
                skill_id: skill_id.clone(),
                text: explanation.clone(),
            },
        );
        let _ = app.emit(
            "skill:explanation:complete",
            ExplanationCompletePayload {
                skill_id: skill_id.clone(),
                explanation: Some(explanation),
            },
        );
        return Ok(());
    }

    stream::do_explain_skill_stream(pool, secrets, app, &skill_id, &content, &lang).await
}

/// Discard the cached explanation and stream a fresh one.
pub async fn refresh_skill_explanation_impl(
    pool: &crate::db::DbPool,
    secrets: &dyn crate::secrets::SecretStore,
    app: &AppHandle,
    skill_id: String,
    content: String,
    lang: String,
) -> Result<(), AiProviderError> {
    cache::delete_cached_skill_explanation(pool, &skill_id, &lang).await?;
    stream::do_explain_skill_stream(pool, secrets, app, &skill_id, &content, &lang).await
}
