//! One-shot (non-streaming) explanation flow. Issues a single
//! Anthropic-format / OpenAI-format request and parses the response into
//! plain text. Used by the `explain_skill` IPC command before the streaming
//! variant existed; kept for callers that want a blocking result.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::error::format_reqwest_error;
use super::prompt::{
    build_explanation_prompt, detect_explanation_api_protocol, truncate_content,
    ExplanationApiProtocol,
};

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
    secrets: &dyn crate::secrets::SecretStore,
    content: String,
) -> Result<String, String> {
    let api_key = super::get_ai_api_key(pool, secrets).await?.ok_or_else(|| {
        super::coded_error(
            super::AI_MISSING_API_KEY,
            "Configure an AI API key in Settings before requesting an AI explanation.",
        )
    })?;

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
        .map_err(|e| {
            super::coded_error_with_details(
                super::AI_CLIENT_BUILD_FAILED,
                "Failed to initialize the AI HTTP client.",
                e.to_string(),
            )
        })?;

    let truncated = truncate_content(&content);
    let prompt = build_explanation_prompt(&truncated, "zh");

    let request = ClaudeRequest {
        model,
        max_tokens: 1024,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: prompt,
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

    let resp = req_builder.json(&request).send().await.map_err(|e| {
        super::coded_error_with_details(
            super::AI_REQUEST_FAILED,
            "AI request failed.",
            format_reqwest_error(&e),
        )
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let status_code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        let code = if status_code == 401 || status_code == 403 {
            super::AI_INVALID_API_KEY
        } else if status_code == 429 {
            super::AI_RATE_LIMIT
        } else {
            super::AI_RESPONSE_ERROR
        };
        let message = if status_code == 401 || status_code == 403 {
            "The API key is invalid or does not have permission for this provider.".to_string()
        } else if status_code == 429 {
            "The provider rate limited the request. Try again later or reduce AI Tag concurrency."
                .to_string()
        } else {
            format!("The AI provider returned HTTP {status}.")
        };
        return Err(super::coded_error_with_details(
            code,
            message,
            format!("HTTP {status}: {body}"),
        ));
    }

    let body = resp.text().await.map_err(|e| {
        super::coded_error_with_details(
            super::AI_RESPONSE_READ_FAILED,
            "Failed to read the AI response.",
            e.to_string(),
        )
    })?;

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

    Err(super::coded_error_with_details(
        super::AI_RESPONSE_PARSE_FAILED,
        "Unable to parse the AI response.",
        &body[..body.len().min(500)],
    ))
}
