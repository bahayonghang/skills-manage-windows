//! Streaming AI explanation: SSE consumption, provider fallback, and Tauri
//! event emission. Anthropic and OpenAI-compatible endpoints are both handled
//! here — only the SSE delta shape and auth header differ.

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use super::cache::{cache_skill_explanation, explanation_has_content};
use super::error::{classify_reqwest_error, ExplanationErrorInfo, ExplanationErrorKind};
use super::prompt::{
    build_explanation_prompt, build_stream_request_body, detect_explanation_api_protocol,
    truncate_content, ExplanationApiProtocol,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationChunkPayload {
    pub skill_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationCompletePayload {
    pub skill_id: String,
    pub explanation: Option<String>,
}

fn empty_explanation_error_info(lang: &str, saw_thinking_delta: bool) -> ExplanationErrorInfo {
    let message = match lang {
        "en" => "The model returned no displayable explanation text.".to_string(),
        _ => "模型没有返回可显示的解释正文。".to_string(),
    };
    let details = if saw_thinking_delta {
        "Streaming completed without any text_delta content. The provider emitted thinking deltas but no final text block.".to_string()
    } else {
        "Streaming completed without any text_delta content.".to_string()
    };

    ExplanationErrorInfo {
        message,
        details,
        kind: ExplanationErrorKind::Response,
        retryable: true,
        fallback_tried: false,
    }
}

/// Provider fallback endpoint mapping. Returns the alternative endpoint for
/// multi-region providers so the backend can retry once on connect failure.
pub(crate) fn get_fallback_endpoint(provider: &str, current_url: &str) -> Option<String> {
    let alternatives: &[(&str, &str)] = match provider {
        "minimax" => &[
            (
                "minimaxi.com",
                "https://api.minimax.io/anthropic/v1/messages",
            ),
            (
                "minimax.io",
                "https://api.minimaxi.com/anthropic/v1/messages",
            ),
        ],
        "glm" => &[
            ("bigmodel.cn", "https://api.z.ai/api/anthropic/v1/messages"),
            (
                "api.z.ai",
                "https://open.bigmodel.cn/api/anthropic/v1/messages",
            ),
        ],
        _ => return None,
    };
    for (needle, fallback) in alternatives {
        if current_url.contains(needle) {
            return Some(fallback.to_string());
        }
    }
    None
}

/// Send a streaming explanation request to the given URL. Returns the response
/// on success, or a classified `ExplanationErrorInfo` on connect / transport failure.
async fn send_stream_request(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    body: &serde_json::Value,
    is_anthropic: bool,
    fallback_tried: bool,
) -> Result<reqwest::Response, ExplanationErrorInfo> {
    let mut req_builder = client
        .post(api_url)
        .header("content-type", "application/json");

    if is_anthropic {
        req_builder = req_builder
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01");
    } else {
        req_builder = req_builder.header("authorization", format!("Bearer {}", api_key));
    }

    match req_builder.json(body).send().await {
        Ok(resp) => Ok(resp),
        Err(e) => Err(classify_reqwest_error(&e, fallback_tried)),
    }
}

/// Core streaming logic shared by `explain_skill_stream` and `refresh_skill_explanation`.
pub(crate) async fn do_explain_skill_stream(
    pool: &crate::db::DbPool,
    app: &AppHandle,
    skill_id: &str,
    content: &str,
    lang: &str,
) -> Result<(), String> {
    let api_key = super::get_ai_setting(pool, "ai_api_key")
        .await
        .ok_or_else(|| "请先在设置中配置 AI API Key".to_string())?;

    let api_url = super::get_ai_setting(pool, "ai_api_url")
        .await
        .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());

    let model = super::get_ai_setting(pool, "ai_model")
        .await
        .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

    let provider = super::get_ai_setting(pool, "ai_provider")
        .await
        .unwrap_or_default();

    let protocol = detect_explanation_api_protocol(&api_url);
    let is_anthropic = matches!(
        protocol,
        ExplanationApiProtocol::AnthropicCompatible | ExplanationApiProtocol::Unknown
    );

    let truncated = truncate_content(content);
    let prompt = build_explanation_prompt(&truncated, lang);
    let body = build_stream_request_body(&model, &prompt);

    // Streaming: only connect_timeout (total `.timeout()` would kill long streams).
    let client = reqwest::Client::builder()
        .user_agent(crate::commands::APP_USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| e.to_string())?;

    // Try primary endpoint; on connect-layer failure, try fallback once
    let resp =
        match send_stream_request(&client, &api_url, &api_key, &body, is_anthropic, false).await {
            Ok(r) => r,
            Err(err_info) => {
                // Only retry on connect-layer errors that are retryable
                if err_info.retryable {
                    if let Some(fallback_url) = get_fallback_endpoint(&provider, &api_url) {
                        eprintln!(
                            "[explain] primary endpoint failed ({:?}), trying fallback: {}",
                            err_info.kind, fallback_url
                        );
                        let fallback_protocol = detect_explanation_api_protocol(&fallback_url);
                        let fallback_anthropic = matches!(
                            fallback_protocol,
                            ExplanationApiProtocol::AnthropicCompatible
                                | ExplanationApiProtocol::Unknown
                        );
                        match send_stream_request(
                            &client,
                            &fallback_url,
                            &api_key,
                            &body,
                            fallback_anthropic,
                            true,
                        )
                        .await
                        {
                            Ok(r) => r,
                            Err(fallback_err) => {
                                let _ = app.emit(
                                    "skill:explanation:error",
                                    serde_json::json!({
                                        "skill_id": skill_id,
                                        "error": &fallback_err.message,
                                        "error_info": fallback_err,
                                    }),
                                );
                                return Err(fallback_err.message);
                            }
                        }
                    } else {
                        let _ = app.emit(
                            "skill:explanation:error",
                            serde_json::json!({
                                "skill_id": skill_id,
                                "error": &err_info.message,
                                "error_info": err_info,
                            }),
                        );
                        return Err(err_info.message);
                    }
                } else {
                    let _ = app.emit(
                        "skill:explanation:error",
                        serde_json::json!({
                            "skill_id": skill_id,
                            "error": &err_info.message,
                            "error_info": err_info,
                        }),
                    );
                    return Err(err_info.message);
                }
            }
        };

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        let status_code = status.as_u16();
        let err_kind = if status_code == 401 || status_code == 403 {
            ExplanationErrorKind::Auth
        } else {
            ExplanationErrorKind::Response
        };
        let user_msg = if status_code == 401 || status_code == 403 {
            "API Key 无效或权限不足，请检查设置中的 API Key".to_string()
        } else if status_code == 429 {
            "请求过于频繁，请稍后重试".to_string()
        } else {
            format!("API 返回错误 {}", status)
        };
        let err_info = ExplanationErrorInfo {
            message: user_msg,
            details: format!("HTTP {}: {}", status, body_text),
            kind: err_kind,
            retryable: status_code == 429,
            fallback_tried: false,
        };
        let _ = app.emit(
            "skill:explanation:error",
            serde_json::json!({
                "skill_id": skill_id,
                "error": &err_info.message,
                "error_info": err_info,
            }),
        );
        return Err(format!("API 返回错误 {}: {}", status, body_text));
    }

    // Stream SSE response
    let mut stream = resp.bytes_stream();
    let mut full_text = String::new();
    let mut sse_buffer = String::new();
    let mut saw_thinking_delta = false;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("流读取失败: {}", e))?;
        sse_buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete SSE lines
        while let Some(newline_pos) = sse_buffer.find('\n') {
            let line = sse_buffer[..newline_pos].trim().to_string();
            sse_buffer = sse_buffer[newline_pos + 1..].to_string();

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            let data = if let Some(stripped) = line.strip_prefix("data: ") {
                stripped
            } else if let Some(stripped) = line.strip_prefix("data:") {
                stripped.trim()
            } else {
                continue;
            };

            if data == "[DONE]" {
                continue;
            }

            let parsed: serde_json::Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let text_chunk = if is_anthropic {
                // Anthropic SSE: { "type": "content_block_delta", "delta": { "type": "text_delta", "text": "..." } }
                let event_type = parsed.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let delta_type = parsed
                    .get("delta")
                    .and_then(|d| d.get("type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                if event_type == "content_block_delta" && delta_type == "thinking_delta" {
                    saw_thinking_delta = true;
                }
                if event_type == "content_block_delta" {
                    parsed
                        .get("delta")
                        .and_then(|d| d.get("text"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                }
            } else {
                // OpenAI SSE: { "choices": [{ "delta": { "content": "..." } }] }
                parsed
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("delta"))
                    .and_then(|d| d.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string()
            };

            if !text_chunk.is_empty() {
                full_text.push_str(&text_chunk);
                let _ = app.emit(
                    "skill:explanation:chunk",
                    ExplanationChunkPayload {
                        skill_id: skill_id.to_string(),
                        text: text_chunk,
                    },
                );
            }
        }
    }

    if !explanation_has_content(&full_text) {
        let err_info = empty_explanation_error_info(lang, saw_thinking_delta);
        let _ = app.emit(
            "skill:explanation:error",
            serde_json::json!({
                "skill_id": skill_id,
                "error": &err_info.message,
                "error_info": err_info,
            }),
        );
        return Err("AI explanation returned no content.".to_string());
    }

    cache_skill_explanation(pool, skill_id, lang, &model, &full_text).await?;

    let _ = app.emit(
        "skill:explanation:complete",
        ExplanationCompletePayload {
            skill_id: skill_id.to_string(),
            explanation: Some(full_text.clone()),
        },
    );

    Ok(())
}
