//! One-shot (non-streaming) explanation flow. Issues a single
//! Anthropic-format / OpenAI-format request and parses the response into
//! plain text. Used by the `explain_skill` IPC command before the streaming
//! variant existed; kept for callers that want a blocking result.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::error::format_reqwest_error;
use super::prompt::{detect_explanation_api_protocol, ExplanationApiProtocol};

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    #[serde(rename = "type", default)]
    block_type: String,
    #[serde(default)]
    text: String,
}

pub(crate) async fn explain_skill(
    pool: &crate::db::DbPool,
    content: String,
) -> Result<String, String> {
    let api_key = super::get_ai_setting(pool, "ai_api_key")
        .await
        .ok_or_else(|| "请先在设置中配置 AI API Key".to_string())?;

    let api_url = super::get_ai_setting(pool, "ai_api_url")
        .await
        .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());

    let model = super::get_ai_setting(pool, "ai_model")
        .await
        .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

    let client = reqwest::Client::builder()
        .user_agent(crate::commands::APP_USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    // Truncate content if too long
    let truncated = if content.len() > 8000 {
        format!("{}...\n\n(内容已截断)", &content[..8000])
    } else {
        content
    };

    let request = ClaudeRequest {
        model,
        max_tokens: 1024,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: format!(
                "请用中文简洁地解释以下 AI Agent Skill（SKILL.md）的用途、使用场景和关键功能。\
                分为三部分：1) 一句话总结 2) 适用场景 3) 关键功能点。\
                控制在 200 字以内。\n\n---\n\n{}",
                truncated
            ),
        }],
    };

    let protocol = detect_explanation_api_protocol(&api_url);
    let mut req_builder = client
        .post(&api_url)
        .header("content-type", "application/json");

    match protocol {
        ExplanationApiProtocol::AnthropicCompatible | ExplanationApiProtocol::Unknown => {
            req_builder = req_builder
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01");
        }
        ExplanationApiProtocol::OpenAiCompatible => {
            req_builder = req_builder.header("authorization", format!("Bearer {}", api_key));
        }
    }

    let resp = req_builder
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("API 请求失败: {}", format_reqwest_error(&e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("API 返回错误 {}: {}", status, body));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    // Try parsing as Anthropic format: { "content": [{ "type": "text", "text": "..." }] }
    if let Ok(claude_resp) = serde_json::from_str::<ClaudeResponse>(&body) {
        // Filter for "text" type blocks, skip "thinking" blocks
        if let Some(block) = claude_resp
            .content
            .iter()
            .find(|b| b.block_type.is_empty() || b.block_type == "text")
        {
            if !block.text.is_empty() {
                return Ok(block.text.clone());
            }
        }
    }

    // Fallback: try extracting text from any JSON with a "text" or "content" field
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
        // Some providers return { "choices": [{ "message": { "content": "..." } }] }
        if let Some(text) = val
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
        {
            return Ok(text.to_string());
        }
    }

    Err(format!("无法解析响应: {}", &body[..body.len().min(500)]))
}
