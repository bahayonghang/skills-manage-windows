//! Prompt construction and protocol detection for AI explanation requests.
//!
//! `ExplanationApiProtocol` distinguishes Anthropic-style (`/v1/messages`)
//! and OpenAI-style (`/v1/chat/completions`) endpoints so the streaming and
//! non-streaming paths can pick the right header/body shape.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplanationApiProtocol {
    AnthropicCompatible,
    OpenAiCompatible,
    Unknown,
}

impl ExplanationApiProtocol {
    pub fn is_anthropic_compatible(self) -> bool {
        matches!(self, Self::AnthropicCompatible | Self::Unknown)
    }
}

pub(crate) fn detect_explanation_api_protocol(api_url: &str) -> ExplanationApiProtocol {
    let path = reqwest::Url::parse(api_url)
        .ok()
        .map(|url| url.path().trim_end_matches('/').to_ascii_lowercase())
        .unwrap_or_else(|| api_url.trim_end_matches('/').to_ascii_lowercase());

    if path.ends_with("/v1/messages") || path.contains("/anthropic/v1/messages") {
        return ExplanationApiProtocol::AnthropicCompatible;
    }

    if path.ends_with("/v1/chat/completions") {
        return ExplanationApiProtocol::OpenAiCompatible;
    }

    ExplanationApiProtocol::Unknown
}

pub(crate) fn resolve_api_protocol(
    api_url: &str,
    explicit_protocol: Option<&str>,
) -> ExplanationApiProtocol {
    match explicit_protocol.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("anthropic") => ExplanationApiProtocol::AnthropicCompatible,
        Some("openai") => ExplanationApiProtocol::OpenAiCompatible,
        _ => detect_explanation_api_protocol(api_url),
    }
}

pub(crate) fn resolve_custom_url(raw_url: &str, protocol: ExplanationApiProtocol) -> String {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let normalized = trimmed.trim_end_matches('/');
    let lower = normalized.to_ascii_lowercase();
    if lower.ends_with("/v1/messages")
        || lower.contains("/anthropic/v1/messages")
        || lower.ends_with("/v1/chat/completions")
    {
        return normalized.to_string();
    }

    match protocol {
        ExplanationApiProtocol::OpenAiCompatible => {
            if lower.ends_with("/v1") {
                format!("{normalized}/chat/completions")
            } else {
                format!("{normalized}/v1/chat/completions")
            }
        }
        ExplanationApiProtocol::AnthropicCompatible | ExplanationApiProtocol::Unknown => {
            if lower.ends_with("/v1") {
                format!("{normalized}/messages")
            } else {
                format!("{normalized}/v1/messages")
            }
        }
    }
}

/// Truncate skill content to 8000 chars to keep prompts within typical context limits.
pub(crate) fn truncate_content(content: &str) -> String {
    if content.len() > 8000 {
        format!("{}...\n\n(内容已截断)", &content[..8000])
    } else {
        content.to_string()
    }
}

/// Build the explanation prompt based on language. Returns ZH by default.
pub(crate) fn build_explanation_prompt(truncated: &str, lang: &str) -> String {
    match lang {
        "en" => format!(
            "Please explain in English concisely the purpose, use cases, and key features \
            of the following AI Agent Skill (SKILL.md). \
            Divide into three parts: 1) One-sentence summary 2) Applicable scenarios 3) Key features. \
            Keep it under 200 words.\n\n---\n\n{}",
            truncated
        ),
        _ => format!(
            "请用中文简洁地解释以下 AI Agent Skill（SKILL.md）的用途、使用场景和关键功能。\
            分为三部分：1) 一句话总结 2) 适用场景 3) 关键功能点。\
            控制在 200 字以内。\n\n---\n\n{}",
            truncated
        ),
    }
}

/// Build the streaming request body as serde_json::Value.
/// Both Anthropic and OpenAI use the same messages format with `stream: true`.
pub(crate) fn build_stream_request_body(model: &str, prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "max_tokens": 1024,
        "stream": true,
        "messages": [{
            "role": "user",
            "content": prompt
        }]
    })
}
