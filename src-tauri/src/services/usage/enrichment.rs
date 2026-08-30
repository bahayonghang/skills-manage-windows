use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::db::{NewSkillUsageMetadata, Skill};
use crate::services::resource_budget::ResourceBudget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageSkillMatchStatus {
    Matched,
    Ambiguous,
    Unmatched,
}

impl UsageSkillMatchStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Ambiguous => "ambiguous",
            Self::Unmatched => "unmatched",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "matched" => Self::Matched,
            "ambiguous" => Self::Ambiguous,
            _ => Self::Unmatched,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedUsageSkill {
    pub skill: String,
    pub match_status: UsageSkillMatchStatus,
    pub resolved_skill_id: Option<String>,
    pub file_path: Option<String>,
}

pub fn resolve_usage_skills(
    skill_names: &[String],
    candidates: &[Skill],
) -> Vec<ResolvedUsageSkill> {
    skill_names
        .iter()
        .map(|skill| {
            let normalized = normalize_identity(skill);
            let id_matches = candidates
                .iter()
                .filter(|candidate| {
                    candidate.is_central && normalize_identity(&candidate.id) == normalized
                })
                .collect::<Vec<_>>();
            let matches = if id_matches.is_empty() {
                candidates
                    .iter()
                    .filter(|candidate| {
                        candidate.is_central && normalize_identity(&candidate.name) == normalized
                    })
                    .collect::<Vec<_>>()
            } else {
                id_matches
            };

            if matches.len() == 1 {
                let matched = matches[0];
                ResolvedUsageSkill {
                    skill: skill.clone(),
                    match_status: UsageSkillMatchStatus::Matched,
                    resolved_skill_id: Some(matched.id.clone()),
                    file_path: Some(matched.file_path.clone()),
                }
            } else {
                ResolvedUsageSkill {
                    skill: skill.clone(),
                    match_status: if matches.is_empty() {
                        UsageSkillMatchStatus::Unmatched
                    } else {
                        UsageSkillMatchStatus::Ambiguous
                    },
                    resolved_skill_id: None,
                    file_path: None,
                }
            }
        })
        .collect()
}

pub(super) fn normalize_identity(value: &str) -> String {
    value.trim().to_lowercase()
}

pub fn estimate_static_tokens(content: &str) -> i64 {
    let mut cjk = 0i64;
    let mut other = 0i64;
    for ch in content.chars().filter(|ch| !ch.is_whitespace()) {
        if is_cjk(ch) {
            cjk += 1;
        } else {
            other += 1;
        }
    }

    cjk + (other * 10 + 37) / 38
}

pub fn build_usage_metadata(
    resolved: &[ResolvedUsageSkill],
    content_by_path: &HashMap<String, String>,
    budget: ResourceBudget,
) -> Vec<NewSkillUsageMetadata> {
    resolved
        .iter()
        .map(|item| {
            let static_metrics = item
                .file_path
                .as_ref()
                .and_then(|path| content_by_path.get(path).map(|content| (path, content)))
                .filter(|(path, content)| {
                    if budget
                        .reject_file_read_size(path, content.len() as u64)
                        .is_err()
                    {
                        tracing::warn!(
                            file_size = content.len(),
                            "usage Skill.md exceeds resource budget"
                        );
                        false
                    } else {
                        true
                    }
                })
                .map(|(_, content)| (estimate_static_tokens(content), content.len() as i64));

            NewSkillUsageMetadata {
                skill: item.skill.clone(),
                match_status: item.match_status.as_str().to_string(),
                resolved_skill_id: item.resolved_skill_id.clone(),
                static_token_estimate: static_metrics.map(|(tokens, _)| tokens),
                static_byte_count: static_metrics.map(|(_, bytes)| bytes),
            }
        })
        .collect()
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2EBEF
            | 0x3000..=0x303F
            | 0x3040..=0x30FF
            | 0xAC00..=0xD7AF
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::central_skill_row;
    use std::path::Path;

    fn candidate(id: &str, name: &str) -> Skill {
        Skill {
            name: name.to_string(),
            ..central_skill_row(id, Path::new("/tmp/central"))
        }
    }

    #[test]
    fn resolver_prefers_exact_normalized_id_then_unique_name() {
        let candidates = vec![
            candidate("review", "Code Review"),
            candidate("commit", "Git Commit"),
        ];
        let names = vec!["  REVIEW  ".to_string(), "git commit".to_string()];

        let resolved = resolve_usage_skills(&names, &candidates);

        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].match_status, UsageSkillMatchStatus::Matched);
        assert_eq!(resolved[0].resolved_skill_id.as_deref(), Some("review"));
        assert_eq!(resolved[1].match_status, UsageSkillMatchStatus::Matched);
        assert_eq!(resolved[1].resolved_skill_id.as_deref(), Some("commit"));
    }

    #[test]
    fn resolver_keeps_duplicate_names_ambiguous_and_unknown_names_unmatched() {
        let candidates = vec![
            candidate("review-a", "Review"),
            candidate("review-b", " review "),
        ];
        let names = vec!["review".to_string(), "missing".to_string()];

        let resolved = resolve_usage_skills(&names, &candidates);

        assert_eq!(resolved[0].match_status, UsageSkillMatchStatus::Ambiguous);
        assert_eq!(resolved[0].resolved_skill_id, None);
        assert_eq!(resolved[1].match_status, UsageSkillMatchStatus::Unmatched);
        assert_eq!(resolved[1].resolved_skill_id, None);
    }

    #[test]
    fn static_token_estimate_handles_ascii_cjk_mixed_and_empty_content() {
        assert_eq!(estimate_static_tokens(""), 0);
        assert_eq!(estimate_static_tokens(" \n\t"), 0);
        assert_eq!(estimate_static_tokens("abcd"), 2);
        assert_eq!(estimate_static_tokens("技能"), 2);
        assert_eq!(estimate_static_tokens("abc 技能"), 3);
    }

    #[test]
    fn metadata_keeps_match_when_content_is_missing_or_over_budget() {
        let resolved = vec![ResolvedUsageSkill {
            skill: "review".to_string(),
            match_status: UsageSkillMatchStatus::Matched,
            resolved_skill_id: Some("review".to_string()),
            file_path: Some("/skills/review/SKILL.md".to_string()),
        }];

        let missing =
            build_usage_metadata(&resolved, &HashMap::new(), ResourceBudget::default_skill());
        assert_eq!(missing[0].resolved_skill_id.as_deref(), Some("review"));
        assert_eq!(missing[0].static_token_estimate, None);
        assert_eq!(missing[0].static_byte_count, None);

        let content = HashMap::from([(
            "/skills/review/SKILL.md".to_string(),
            "oversized".to_string(),
        )]);
        let over_budget = build_usage_metadata(
            &resolved,
            &content,
            ResourceBudget {
                file_bytes: 3,
                ..ResourceBudget::default_skill()
            },
        );
        assert_eq!(over_budget[0].resolved_skill_id.as_deref(), Some("review"));
        assert_eq!(over_budget[0].static_token_estimate, None);
        assert_eq!(over_budget[0].static_byte_count, None);
    }
}
