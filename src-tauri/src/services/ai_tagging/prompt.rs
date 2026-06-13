use reqwest::Client;
use serde_json::Value;

use crate::{
    db::{Skill, SkillTag, ACADEMIC_RESEARCH_WRITING_TAG_ID, UNCATEGORIZED_TAG_ID},
    services::ai_provider,
};

use super::error::AiTaggingError;
use super::types::{
    AiTaggingContext, RawAiTagSuggestion, RawAiTagSuggestionEnvelope, SkillTagSuggestion,
};

pub(crate) async fn suggest_skill_tags_for_skill(
    context: &AiTaggingContext,
    skill: &Skill,
) -> Result<Vec<SkillTagSuggestion>, AiTaggingError> {
    let content = skill
        .content
        .clone()
        .or_else(|| std::fs::read_to_string(&skill.file_path).ok())
        .unwrap_or_default();
    let prompt = build_tagging_prompt(
        &skill.name,
        skill.description.as_deref(),
        &content,
        &context.tags,
    );
    let raw = call_ai_for_tagging(
        &context.client,
        &context.api_url,
        &context.api_key,
        &context.model,
        context.protocol,
        &prompt,
    )
    .await?;
    let parsed = parse_ai_tag_suggestions(&raw)?;
    map_ai_suggestions(&skill.id, &context.tags, parsed)
}

pub(crate) fn build_tagging_prompt(
    name: &str,
    description: Option<&str>,
    content: &str,
    tags: &[SkillTag],
) -> String {
    let candidates = tags
        .iter()
        .filter(|tag| !tag.is_builtin || tag.id == ACADEMIC_RESEARCH_WRITING_TAG_ID)
        .map(|tag| {
            let kind = if tag.is_builtin { "built-in" } else { "custom" };
            let description = tag.description.as_deref().unwrap_or("无");
            format!(
                "- id: {} | name: {} | kind: {} | description: {}",
                tag.id, tag.name, kind, description
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let summary = content.chars().take(4_000).collect::<String>();

    format!(
        "你是 SkillPort 的本地分类器。请只从候选标签中选择 0 到 3 个标签。\n\
         只能输出候选列表中的 tag id，不要输出名称、翻译、同义词或新标签。\n\
         优先复用最具体的已有 custom 标签；只有强匹配时 confidence 才能 >= 0.7。\n\
         没有明确匹配时返回 {{\"tags\":[]}}，不要为了凑数选择宽泛默认标签。\n\
         输出必须是 JSON，不要解释额外文本。\n\
         JSON 格式：{{\"tags\":[{{\"tag\":\"候选标签ID\",\"confidence\":0.0,\"reason\":\"不超过20字\"}}]}}\n\n\
         候选标签：\n{candidates}\n\n\
         Skill 名称：{name}\n\
         Description：{}\n\
         SKILL.md 摘要：\n{}",
        description.unwrap_or(""),
        summary
    )
}

async fn call_ai_for_tagging(
    client: &Client,
    api_url: &str,
    api_key: &str,
    model: &str,
    protocol: ai_provider::ExplanationApiProtocol,
    prompt: &str,
) -> Result<String, AiTaggingError> {
    let is_openai = !protocol.is_anthropic_compatible();
    let body = if is_openai {
        serde_json::json!({
            "model": model,
            "temperature": 0.1,
            "messages": [{ "role": "user", "content": prompt }],
        })
    } else {
        serde_json::json!({
            "model": model,
            "max_tokens": 600,
            "messages": [{ "role": "user", "content": prompt }],
        })
    };

    let mut request = client.post(api_url).json(&body);
    request = if is_openai {
        request.header("authorization", format!("Bearer {}", api_key))
    } else {
        request
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
    };

    let response = request.send().await.map_err(|e| {
        AiTaggingError::Http(ai_provider::coded_error_with_details(
            ai_provider::AI_REQUEST_FAILED,
            "AI tagging request failed.",
            e.to_string(),
        ))
    })?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| AiTaggingError::Http(format!("Failed to read AI tagging response: {}", e)))?;
    if !status.is_success() {
        if status.as_u16() == 429 {
            return Err(AiTaggingError::RateLimited(ai_provider::coded_error_with_details(
                ai_provider::AI_RATE_LIMIT,
                "AI tagging was rate limited. Reduce AI Tag concurrency or increase the request interval in Settings.",
                format!("HTTP {status}: {text}"),
            )));
        }
        return Err(AiTaggingError::Http(ai_provider::coded_error_with_details(
            ai_provider::AI_RESPONSE_ERROR,
            format!("AI tagging returned HTTP {status}."),
            text,
        )));
    }

    extract_ai_response_text(&text, is_openai)
}

fn extract_ai_response_text(
    response_text: &str,
    is_openai: bool,
) -> Result<String, AiTaggingError> {
    let value: Value = serde_json::from_str(response_text)
        .map_err(|e| AiTaggingError::Parse(format!("AI tagging response is not JSON: {}", e)))?;
    if is_openai {
        return value["choices"][0]["message"]["content"]
            .as_str()
            .map(ToString::to_string)
            .ok_or_else(|| {
                AiTaggingError::Parse(
                    "AI tagging response did not include message content.".to_string(),
                )
            });
    }

    value["content"]
        .as_array()
        .and_then(|items| {
            items.iter().find_map(|item| {
                item.get("text")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
        })
        .ok_or_else(|| {
            AiTaggingError::Parse("AI tagging response did not include text content.".to_string())
        })
}

pub(crate) fn parse_ai_tag_suggestions(
    raw: &str,
) -> Result<Vec<RawAiTagSuggestion>, AiTaggingError> {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(envelope) = serde_json::from_str::<RawAiTagSuggestionEnvelope>(cleaned) {
        return Ok(envelope.tags);
    }
    if let Ok(list) = serde_json::from_str::<Vec<RawAiTagSuggestion>>(cleaned) {
        return Ok(list);
    }

    let start = cleaned
        .find('{')
        .or_else(|| cleaned.find('['))
        .ok_or_else(|| {
            AiTaggingError::Parse("AI tagging response did not include JSON.".to_string())
        })?;
    let end = cleaned
        .rfind('}')
        .or_else(|| cleaned.rfind(']'))
        .ok_or_else(|| {
            AiTaggingError::Parse("AI tagging response did not include complete JSON.".to_string())
        })?;
    let json_slice = &cleaned[start..=end];
    serde_json::from_str::<RawAiTagSuggestionEnvelope>(json_slice)
        .map(|envelope| envelope.tags)
        .or_else(|_| serde_json::from_str::<Vec<RawAiTagSuggestion>>(json_slice))
        .map_err(|e| AiTaggingError::Parse(format!("Failed to parse AI tagging JSON: {}", e)))
}

pub(crate) fn map_ai_suggestions(
    skill_id: &str,
    tags: &[SkillTag],
    raw: Vec<RawAiTagSuggestion>,
) -> Result<Vec<SkillTagSuggestion>, AiTaggingError> {
    let mut suggestions = Vec::new();
    for item in raw {
        let key = item.tag.trim();
        if key == UNCATEGORIZED_TAG_ID {
            continue;
        }
        let Some(tag) = tags
            .iter()
            .find(|tag| tag.id == key || tag.name == key)
            .cloned()
        else {
            continue;
        };
        let confidence = item.confidence.unwrap_or(0.6).clamp(0.0, 1.0);
        suggestions.push(SkillTagSuggestion {
            skill_id: skill_id.to_string(),
            tag,
            confidence,
            reason: item.reason.unwrap_or_else(|| "AI 自动标注".to_string()),
        });
    }

    if suggestions.is_empty() {
        let fallback = tags
            .iter()
            .find(|tag| tag.id == UNCATEGORIZED_TAG_ID)
            .cloned()
            .ok_or(AiTaggingError::NoUsableCandidateTags)?;
        suggestions.push(SkillTagSuggestion {
            skill_id: skill_id.to_string(),
            tag: fallback,
            confidence: 0.2,
            reason: "未命中候选大类".to_string(),
        });
    }

    Ok(suggestions)
}
