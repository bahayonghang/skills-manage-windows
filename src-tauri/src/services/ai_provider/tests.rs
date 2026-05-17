#![cfg(test)]
//! AI provider tests: protocol detection, error classification, fallback
//! routing, and explanation cache behavior.

use super::cache::{
    cache_skill_explanation, load_cached_skill_explanation,
    load_cached_skill_explanation_summaries,
};
use super::error::{classify_reqwest_error, format_reqwest_error, ExplanationErrorKind};
use super::prompt::{
    detect_explanation_api_protocol, resolve_api_protocol, resolve_custom_url,
    ExplanationApiProtocol,
};
use super::stream::get_fallback_endpoint;
use crate::db;
use tempfile::{tempdir, TempDir};

async fn setup_test_db() -> (crate::db::DbPool, TempDir) {
    let dir = tempdir().expect("create tempdir");
    let db_path = dir.path().join("ai-provider-cache.sqlite");
    let db_path = db_path.to_string_lossy().into_owned();
    let pool = db::create_pool(&db_path).await.expect("create pool");
    db::init_database(&pool).await.expect("init db");
    (pool, dir)
}

#[test]
fn explicit_protocol_overrides_url_detection() {
    assert_eq!(
        resolve_api_protocol("https://example.com/v1/messages", Some("openai")),
        ExplanationApiProtocol::OpenAiCompatible
    );
    assert_eq!(
        resolve_api_protocol("https://example.com/v1/chat/completions", Some("anthropic")),
        ExplanationApiProtocol::AnthropicCompatible
    );
}

#[test]
fn custom_v1_url_expands_for_selected_protocol() {
    assert_eq!(
        resolve_custom_url(
            "https://proxy.example.com/v1",
            ExplanationApiProtocol::OpenAiCompatible
        ),
        "https://proxy.example.com/v1/chat/completions"
    );
    assert_eq!(
        resolve_custom_url(
            "https://proxy.example.com/v1",
            ExplanationApiProtocol::AnthropicCompatible
        ),
        "https://proxy.example.com/v1/messages"
    );
}

#[test]
fn detects_anthropic_compatible_message_endpoints() {
    assert_eq!(
        detect_explanation_api_protocol("https://api.minimaxi.com/anthropic/v1/messages"),
        ExplanationApiProtocol::AnthropicCompatible
    );
    assert_eq!(
        detect_explanation_api_protocol("https://open.bigmodel.cn/api/anthropic/v1/messages"),
        ExplanationApiProtocol::AnthropicCompatible
    );
    assert_eq!(
        detect_explanation_api_protocol("https://api.anthropic.com/v1/messages"),
        ExplanationApiProtocol::AnthropicCompatible
    );
}

#[test]
fn detects_openai_chat_completions_endpoints() {
    assert_eq!(
        detect_explanation_api_protocol("https://api.openai.com/v1/chat/completions"),
        ExplanationApiProtocol::OpenAiCompatible
    );
}

#[test]
fn leaves_unknown_endpoints_unclassified() {
    assert_eq!(
        detect_explanation_api_protocol("https://example.com/custom/generate"),
        ExplanationApiProtocol::Unknown
    );
}

/// A live reqwest error (connect-refused on localhost:1) must be
/// classified with an actionable hint, not just the opaque
/// top-level "error sending request for url (...)".
/// `.no_proxy()` ensures the test is deterministic even when the
/// developer has `HTTP(S)_PROXY` set in their environment.
#[tokio::test]
async fn format_reqwest_error_surfaces_actionable_hint() {
    let client = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(std::time::Duration::from_millis(500))
        .build()
        .expect("build client");
    let err = client
        .post("http://127.0.0.1:1/")
        .send()
        .await
        .expect_err("expected connect failure");
    let msg = format_reqwest_error(&err);
    assert!(
        msg.contains("region endpoint") || msg.contains("Unable to connect"),
        "expected actionable English hint in formatted error, got: {msg}"
    );
}

#[tokio::test]
async fn classify_connect_error_as_connect_kind() {
    let client = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(std::time::Duration::from_millis(500))
        .build()
        .expect("build client");
    let err = client
        .post("http://127.0.0.1:1/")
        .send()
        .await
        .expect_err("expected connect failure");
    let info = classify_reqwest_error(&err, false);
    assert_eq!(info.kind, ExplanationErrorKind::Connect);
    assert_eq!(info.code.as_deref(), Some(super::AI_CONNECT));
    assert!(info.retryable);
    assert!(!info.message.is_empty());
    assert!(!info.details.is_empty());
}

// ── Fallback endpoint tests ──────────────────────────────────────────

#[test]
fn minimax_cn_falls_back_to_intl() {
    let fb = get_fallback_endpoint("minimax", "https://api.minimaxi.com/anthropic/v1/messages");
    assert_eq!(
        fb.as_deref(),
        Some("https://api.minimax.io/anthropic/v1/messages")
    );
}

#[test]
fn minimax_intl_falls_back_to_cn() {
    let fb = get_fallback_endpoint("minimax", "https://api.minimax.io/anthropic/v1/messages");
    assert_eq!(
        fb.as_deref(),
        Some("https://api.minimaxi.com/anthropic/v1/messages")
    );
}

#[test]
fn glm_cn_falls_back_to_intl() {
    let fb = get_fallback_endpoint("glm", "https://open.bigmodel.cn/api/anthropic/v1/messages");
    assert_eq!(
        fb.as_deref(),
        Some("https://api.z.ai/api/anthropic/v1/messages")
    );
}

#[test]
fn glm_intl_falls_back_to_cn() {
    let fb = get_fallback_endpoint("glm", "https://api.z.ai/api/anthropic/v1/messages");
    assert_eq!(
        fb.as_deref(),
        Some("https://open.bigmodel.cn/api/anthropic/v1/messages")
    );
}

#[test]
fn claude_has_no_fallback() {
    let fb = get_fallback_endpoint("claude", "https://api.anthropic.com/v1/messages");
    assert!(fb.is_none());
}

#[test]
fn custom_provider_has_no_fallback() {
    let fb = get_fallback_endpoint("custom", "https://my-proxy.example.com/v1/messages");
    assert!(fb.is_none());
}

#[tokio::test]
async fn load_cached_skill_explanation_drops_empty_rows() {
    let (pool, _dir) = setup_test_db().await;

    sqlx::query(
        "INSERT INTO skill_explanations (skill_id, explanation, lang, model, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("defuddle")
    .bind("")
    .bind("zh")
    .bind("MiniMax-M2.7")
    .bind("2026-04-19T00:00:00Z")
    .bind("2026-04-19T00:00:00Z")
    .execute(&pool)
    .await
    .expect("insert empty explanation");

    let explanation = load_cached_skill_explanation(&pool, "defuddle", "zh")
        .await
        .expect("load cached explanation");
    assert!(explanation.is_none());

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM skill_explanations WHERE skill_id = ? AND lang = ?",
    )
    .bind("defuddle")
    .bind("zh")
    .fetch_one(&pool)
    .await
    .expect("count explanations");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn cache_skill_explanation_rejects_blank_text() {
    let (pool, _dir) = setup_test_db().await;

    let err = cache_skill_explanation(&pool, "defuddle", "zh", "MiniMax-M2.7", "   ")
        .await
        .expect_err("blank explanations should be rejected");
    assert!(err.contains("no content"));

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM skill_explanations WHERE skill_id = ? AND lang = ?",
    )
    .bind("defuddle")
    .bind("zh")
    .fetch_one(&pool)
    .await
    .expect("count explanations");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn load_cached_skill_explanation_summaries_returns_nonblank_lang_matches() {
    let (pool, _dir) = setup_test_db().await;

    for (skill_id, explanation, lang) in [
        ("defuddle", "中文解释", "zh"),
        ("task-planner", "  有空白也应修剪  ", "zh"),
        ("empty-row", "   ", "zh"),
        ("english-only", "English summary", "en"),
    ] {
        sqlx::query(
            "INSERT INTO skill_explanations (skill_id, explanation, lang, model, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(skill_id)
        .bind(explanation)
        .bind(lang)
        .bind("MiniMax-M2.7")
        .bind("2026-04-19T00:00:00Z")
        .bind("2026-04-19T00:00:00Z")
        .execute(&pool)
        .await
        .expect("insert explanation");
    }

    let summaries = load_cached_skill_explanation_summaries(
        &pool,
        &[
            "defuddle".to_string(),
            "task-planner".to_string(),
            "empty-row".to_string(),
            "english-only".to_string(),
            "unknown".to_string(),
            "defuddle".to_string(),
        ],
        "zh",
    )
    .await
    .expect("load explanation summaries");

    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries.get("defuddle").map(String::as_str), Some("中文解释"));
    assert_eq!(
        summaries.get("task-planner").map(String::as_str),
        Some("有空白也应修剪")
    );
    assert!(!summaries.contains_key("empty-row"));
    assert!(!summaries.contains_key("english-only"));
    assert!(!summaries.contains_key("unknown"));
}
