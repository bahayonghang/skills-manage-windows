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
mod error;
mod prompt;
mod stream;

#[cfg(test)]
mod tests;

pub use error::{ExplanationErrorInfo, ExplanationErrorKind};
pub use stream::{ExplanationChunkPayload, ExplanationCompletePayload};

use tauri::{AppHandle, Emitter};

/// Helper: read a setting from the DB, filtering out empty values. Used by
/// every flow that needs `ai_api_key` / `ai_api_url` / `ai_model` / `ai_provider`.
pub(crate) async fn get_ai_setting(pool: &crate::db::DbPool, key: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
}

/// Issue a single non-streaming explanation request and return the parsed text.
pub async fn explain_skill_impl(
    pool: &crate::db::DbPool,
    content: String,
) -> Result<String, String> {
    claude::explain_skill(pool, content).await
}

/// Read a cached explanation if one exists; never triggers the AI provider.
pub async fn get_skill_explanation_impl(
    pool: &crate::db::DbPool,
    skill_id: String,
    lang: String,
) -> Result<Option<String>, String> {
    cache::load_cached_skill_explanation(pool, &skill_id, &lang).await
}

/// Stream an AI-generated explanation. Cache hits are emitted as a single chunk
/// + complete pair so the frontend can react with the same listener wiring.
pub async fn explain_skill_stream_impl(
    pool: &crate::db::DbPool,
    app: &AppHandle,
    skill_id: String,
    content: String,
    lang: String,
) -> Result<(), String> {
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

    stream::do_explain_skill_stream(pool, app, &skill_id, &content, &lang).await
}

/// Discard the cached explanation and stream a fresh one.
pub async fn refresh_skill_explanation_impl(
    pool: &crate::db::DbPool,
    app: &AppHandle,
    skill_id: String,
    content: String,
    lang: String,
) -> Result<(), String> {
    cache::delete_cached_skill_explanation(pool, &skill_id, &lang).await?;
    stream::do_explain_skill_stream(pool, app, &skill_id, &content, &lang).await
}
