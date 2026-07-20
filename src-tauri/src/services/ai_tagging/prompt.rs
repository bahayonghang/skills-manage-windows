use reqwest::Client;
use serde_json::Value;

use crate::{
    db::{derive_skill_tag_id, Skill, SkillTag, UNCATEGORIZED_TAG_ID},
    services::ai_provider,
};

use super::error::AiTaggingError;
use super::types::{
    AiTaggingContext, RawAiTagSuggestion, RawAiTagSuggestionEnvelope, ResolvedAiSuggestions,
    SkillTagProposal, SkillTagSuggestion,
};

pub(crate) async fn suggest_skill_tags_for_skill(
    context: &AiTaggingContext,
    skill: &Skill,
) -> Result<ResolvedAiSuggestions, AiTaggingError> {
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
    resolve_ai_suggestions(&skill.id, &context.tags, parsed)
}

pub(crate) fn build_tagging_prompt(
    name: &str,
    description: Option<&str>,
    content: &str,
    tags: &[SkillTag],
) -> String {
    let candidates = tags
        .iter()
        .filter(|tag| tag.id != UNCATEGORIZED_TAG_ID)
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
        "你是 SkillPort 的本地分类器。优先从候选标签中选择 0 到 3 个最具体的已有标签。\n\
         tags 只能输出候选列表中的 tag id；custom 标签优先于宽泛 built-in 标签。\n\
         仅当技能明确属于候选中不存在的类别时，才可额外提议 1 个 new_tag。\n\
         已有标签能覆盖时禁止提议；不确定时不要提议；不得输出系统 fallback 标签。\n\
         new_tag.name 不超过 12 个中文字符或等长英文短语，description 使用一句英文。\n\
         只有强匹配时 confidence 才能 >= 0.7；无匹配且无提议时返回 {{\"tags\":[]}}。\n\
         输出必须是 JSON，不要解释额外文本。\n\
         JSON 格式：{{\"tags\":[{{\"tag\":\"候选标签ID\",\"confidence\":0.0,\"reason\":\"不超过20字\"}}],\"new_tag\":{{\"name\":\"新标签\",\"description\":\"English sentence.\",\"confidence\":0.0,\"reason\":\"不超过20字\"}}}}\n\n\
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
) -> Result<RawAiTagSuggestionEnvelope, AiTaggingError> {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(envelope) = serde_json::from_str::<RawAiTagSuggestionEnvelope>(cleaned) {
        return Ok(envelope);
    }
    if let Ok(list) = serde_json::from_str::<Vec<RawAiTagSuggestion>>(cleaned) {
        return Ok(RawAiTagSuggestionEnvelope {
            tags: list,
            new_tag: None,
        });
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
        .or_else(|_| {
            serde_json::from_str::<Vec<RawAiTagSuggestion>>(json_slice).map(|tags| {
                RawAiTagSuggestionEnvelope {
                    tags,
                    new_tag: None,
                }
            })
        })
        .map_err(|e| AiTaggingError::Parse(format!("Failed to parse AI tagging JSON: {}", e)))
}

pub(crate) fn resolve_ai_suggestions(
    skill_id: &str,
    tags: &[SkillTag],
    raw: RawAiTagSuggestionEnvelope,
) -> Result<ResolvedAiSuggestions, AiTaggingError> {
    let mut suggestions = map_known_ai_suggestions(skill_id, tags, raw.tags);
    let mut proposals = Vec::new();

    if let Some(proposal) = raw.new_tag {
        let name = proposal.name.trim();
        let weighted_len = name
            .chars()
            .map(|ch| if ch.is_ascii() { 1 } else { 2 })
            .sum::<usize>();
        if !name.is_empty() && weighted_len <= 24 {
            let tag_id = derive_skill_tag_id(name);
            let collision = tags
                .iter()
                .find(|tag| tag.id == tag_id || tag.name.trim().eq_ignore_ascii_case(name));
            let confidence = proposal.confidence.unwrap_or(0.6).clamp(0.0, 1.0);
            let reason = proposal
                .reason
                .unwrap_or_else(|| "AI 建议新分类".to_string());
            if let Some(tag) = collision {
                if tag.id != UNCATEGORIZED_TAG_ID
                    && !suggestions.iter().any(|item| item.tag.id == tag.id)
                {
                    suggestions.push(SkillTagSuggestion {
                        skill_id: skill_id.to_string(),
                        tag: tag.clone(),
                        confidence,
                        reason,
                    });
                }
            } else {
                proposals.push(SkillTagProposal {
                    skill_id: skill_id.to_string(),
                    tag_id,
                    proposed_name: name.to_string(),
                    proposed_description: proposal
                        .description
                        .map(|description| description.trim().to_string())
                        .filter(|description| !description.is_empty()),
                    confidence,
                    reason,
                });
            }
        }
    }

    if suggestions.is_empty() && proposals.is_empty() {
        suggestions = fallback_ai_suggestion(skill_id, tags)?;
    }

    Ok(ResolvedAiSuggestions {
        suggestions,
        proposals,
    })
}

#[cfg(test)]
pub(crate) fn map_ai_suggestions(
    skill_id: &str,
    tags: &[SkillTag],
    raw: Vec<RawAiTagSuggestion>,
) -> Result<Vec<SkillTagSuggestion>, AiTaggingError> {
    let suggestions = map_known_ai_suggestions(skill_id, tags, raw);

    if suggestions.is_empty() {
        return fallback_ai_suggestion(skill_id, tags);
    }

    Ok(suggestions)
}

fn map_known_ai_suggestions(
    skill_id: &str,
    tags: &[SkillTag],
    raw: Vec<RawAiTagSuggestion>,
) -> Vec<SkillTagSuggestion> {
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

    suggestions
}

fn fallback_ai_suggestion(
    skill_id: &str,
    tags: &[SkillTag],
) -> Result<Vec<SkillTagSuggestion>, AiTaggingError> {
    let fallback = tags
        .iter()
        .find(|tag| tag.id == UNCATEGORIZED_TAG_ID)
        .cloned()
        .ok_or(AiTaggingError::NoUsableCandidateTags)?;
    Ok(vec![SkillTagSuggestion {
        skill_id: skill_id.to_string(),
        tag: fallback,
        confidence: 0.2,
        reason: "未命中候选大类".to_string(),
    }])
}
