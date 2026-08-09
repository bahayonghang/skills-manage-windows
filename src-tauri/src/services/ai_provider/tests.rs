#![cfg(test)]
//! AI provider tests: protocol detection, error classification, fallback
//! routing, and explanation cache behavior.

use super::cache::{
    cache_skill_explanation, load_cached_skill_explanation, load_cached_skill_explanation_summaries,
};
use super::error::{classify_reqwest_error, format_reqwest_error, ExplanationErrorKind};
use super::prompt::{
    detect_explanation_api_protocol, resolve_api_protocol, resolve_custom_url, truncate_content,
    ExplanationApiProtocol,
};
use super::stream::get_fallback_endpoint;
use super::{read_ai_response_body, read_ai_response_body_with_timeout, AiProviderError};
use tempfile::TempDir;

async fn setup_test_db() -> (crate::db::DbPool, TempDir) {
    crate::test_support::file_pool().await
}

async fn chunked_response_that_waits_for_reader(
    status: &'static str,
    body: &'static [u8],
) -> (reqwest::Response, tokio::sync::oneshot::Sender<()>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut request = [0_u8; 1024];
        let _ = socket.read(&mut request).await.expect("request");
        let headers = format!(
            "HTTP/1.1 {status}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:X}\r\n",
            body.len()
        );
        socket.write_all(headers.as_bytes()).await.expect("headers");
        socket.write_all(body).await.expect("body");
        socket.write_all(b"\r\n").await.expect("chunk end");
        let _ = release_rx.await;
    });

    let response = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .get(format!("http://{address}/response"))
        .send()
        .await
        .expect("response headers");
    (response, release_tx)
}

#[tokio::test]
async fn chunked_ai_success_body_stops_at_limit_before_eof() {
    let (response, release) = chunked_response_that_waits_for_reader("200 OK", b"abcdef").await;
    let error = read_ai_response_body(response, 5, "success response body")
        .await
        .expect_err("cap+1 body must fail before EOF");
    assert!(matches!(
        error,
        AiProviderError::ResponseTooLarge {
            phase: "success response body",
            limit: 5
        }
    ));
    let _ = release.send(());
}

#[tokio::test]
async fn chunked_ai_error_body_stops_at_limit_before_eof() {
    let (response, release) =
        chunked_response_that_waits_for_reader("500 Internal Server Error", b"secret").await;
    let error = read_ai_response_body(response, 5, "error response body")
        .await
        .expect_err("cap+1 diagnostic must fail before EOF");
    assert!(matches!(error, AiProviderError::ResponseTooLarge { .. }));
    assert!(!error.to_string().contains("secret"));
    let _ = release.send(());
}

#[tokio::test(start_paused = true)]
async fn finite_ai_body_has_an_explicit_typed_deadline() {
    let (response, release) = chunked_response_that_waits_for_reader("200 OK", b"ok").await;
    let error = read_ai_response_body_with_timeout(
        response,
        5,
        "success response body",
        std::time::Duration::from_secs(5),
    )
    .await
    .expect_err("unterminated response body must time out");
    assert!(matches!(
        error,
        AiProviderError::ResponseTimeout {
            phase: "success response body",
            timeout_ms: 5_000
        }
    ));
    let _ = release.send(());
}

#[test]
fn status_classification_preserves_auth_and_rate_limit_variants() {
    assert!(matches!(
        AiProviderError::from_status(401, "auth".to_string()),
        AiProviderError::AccessDenied(_)
    ));
    assert!(matches!(
        AiProviderError::from_status(403, "auth".to_string()),
        AiProviderError::AccessDenied(_)
    ));
    assert!(matches!(
        AiProviderError::from_status(429, "rate".to_string()),
        AiProviderError::RateLimited(_)
    ));
}

#[test]
fn truncate_content_counts_unicode_scalars_without_panicking() {
    for content in [
        String::new(),
        "a".repeat(8_001),
        "中".repeat(8_001),
        "🙂".repeat(8_001),
        "e\u{301}".repeat(4_001),
    ] {
        let truncated = truncate_content(&content);
        assert!(truncated.is_char_boundary(truncated.len()));
        if content.chars().count() > 8_000 {
            assert!(truncated.ends_with("...\n\n(内容已截断)"));
            assert_eq!(
                truncated
                    .trim_end_matches("...\n\n(内容已截断)")
                    .chars()
                    .count(),
                8_000
            );
        } else {
            assert_eq!(truncated, content);
        }
    }
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
    assert!(!msg.contains("127.0.0.1"));
    assert!(!msg.contains("http://"));
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
    assert!(!info.details.contains("127.0.0.1"));
    assert!(!info.details.contains("http://"));
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
    let (pool, dir) = setup_test_db().await;
    crate::test_support::seed_central_skill(
        &pool,
        &dir.path().join("defuddle"),
        "defuddle",
        "Test skill",
    )
    .await;

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
    assert!(err.to_string().contains("no content"));

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
    let (pool, dir) = setup_test_db().await;

    for (skill_id, explanation, lang) in [
        ("defuddle", "中文解释", "zh"),
        ("task-planner", "  有空白也应修剪  ", "zh"),
        ("empty-row", "   ", "zh"),
        ("english-only", "English summary", "en"),
    ] {
        crate::test_support::seed_central_skill(
            &pool,
            &dir.path().join(skill_id),
            skill_id,
            "Test skill",
        )
        .await;
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
    assert_eq!(
        summaries.get("defuddle").map(String::as_str),
        Some("中文解释")
    );
    assert_eq!(
        summaries.get("task-planner").map(String::as_str),
        Some("有空白也应修剪")
    );
    assert!(!summaries.contains_key("empty-row"));
    assert!(!summaries.contains_key("english-only"));
    assert!(!summaries.contains_key("unknown"));
}
