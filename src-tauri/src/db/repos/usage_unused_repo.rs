//! `usage_get_unused_skills` 的未使用技能派生查询 —— 从 `usage_repo.rs` 拆出
//! 以遵守 800 行文件预算。只读查询，不写任何表。

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::db::types::DbPool;

/// `usage_get_unused_skills` 平台维度候选：跨 agent 的安装观察行。
///
/// 权威来源是 `agent_skill_observations`（每次 agent 扫描落盘的事实表，覆盖
/// 插件源与未入 Central 的平台散件，且自带 name/dir_path）；`skill_installations`
/// 只覆盖可管理来源且无 name 列，不适合做平台维度的名称匹配。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlatformSkillObservationRow {
    pub agent_id: String,
    pub name: String,
    pub dir_path: String,
}

pub async fn list_platform_skill_observations(
    pool: &DbPool,
) -> Result<Vec<PlatformSkillObservationRow>, sqlx::Error> {
    sqlx::query_as::<_, PlatformSkillObservationRow>(
        "SELECT agent_id, name, dir_path
         FROM agent_skill_observations
         ORDER BY agent_id ASC, name ASC, dir_path ASC",
    )
    .fetch_all(pool)
    .await
}

/// 按 `skill_usage_metadata.resolved_skill_id` 聚合的调用事实，给未使用报表的
/// Central 维度做归属。`source` 过滤只作用于 calls 聚合，与 overview/recent/
/// detail 的口径一致（`skill-usage-analytics.md`）。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedCallAggregateRow {
    pub resolved_skill_id: String,
    pub call_count: i64,
    pub last_used_ms: Option<i64>,
    pub static_token_estimate: Option<i64>,
    pub static_byte_count: Option<i64>,
}

pub async fn list_resolved_call_aggregates(
    pool: &DbPool,
    target_id: &str,
    source: Option<&str>,
) -> Result<Vec<ResolvedCallAggregateRow>, sqlx::Error> {
    // 只有 matched 行才携带 resolved_skill_id；source 过滤放在 LEFT JOIN 的
    // ON 子句里，让「该 source 下零调用」的 Central skill 仍以 call_count=0
    // 出现在结果中（对该 source 而言即 never_used）。
    sqlx::query_as::<_, ResolvedCallAggregateRow>(
        "SELECT m.resolved_skill_id AS resolved_skill_id,
                COUNT(c.timestamp_ms) AS call_count,
                MAX(c.timestamp_ms) AS last_used_ms,
                MAX(m.static_token_estimate) AS static_token_estimate,
                MAX(m.static_byte_count) AS static_byte_count
         FROM skill_usage_metadata m
         LEFT JOIN skill_calls c
                ON c.target_id = m.target_id
               AND c.skill = m.skill
               AND (? IS NULL OR c.source = ?)
         WHERE m.target_id = ?
           AND m.resolved_skill_id IS NOT NULL
         GROUP BY m.resolved_skill_id",
    )
    .bind(source)
    .bind(source)
    .bind(target_id)
    .fetch_all(pool)
    .await
}

/// 按 normalized skill 名（`LOWER(TRIM(skill))`，与 enrichment 的
/// `normalize_identity` 同一规则）聚合的调用事实，给平台维度直查
/// `skill_calls` 用。平台散件没有 metadata 行可经 resolved_skill_id 归属。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedCallAggregateRow {
    pub normalized_skill: String,
    pub call_count: i64,
    pub last_used_ms: Option<i64>,
}

pub async fn list_normalized_call_aggregates(
    pool: &DbPool,
    target_id: &str,
    source: Option<&str>,
) -> Result<Vec<NormalizedCallAggregateRow>, sqlx::Error> {
    sqlx::query_as::<_, NormalizedCallAggregateRow>(
        "SELECT LOWER(TRIM(skill)) AS normalized_skill,
                COUNT(*) AS call_count,
                MAX(timestamp_ms) AS last_used_ms
         FROM skill_calls
         WHERE target_id = ?
           AND (? IS NULL OR source = ?)
         GROUP BY LOWER(TRIM(skill))",
    )
    .bind(target_id)
    .bind(source)
    .bind(source)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repos::usage_repo::{
        replace_calls_for_target, NewSkillCall, NewSkillUsageMetadata,
    };
    use crate::test_support::mem_pool;

    fn call(skill: &str) -> NewSkillCall {
        NewSkillCall {
            skill: skill.to_string(),
            timestamp_ms: 1_700_000_000_000,
            project: "/project".to_string(),
            session_id: "session".to_string(),
            source: "Codex CLI".to_string(),
        }
    }

    fn metadata(skill: &str, status: &str, resolved: Option<&str>) -> NewSkillUsageMetadata {
        NewSkillUsageMetadata {
            skill: skill.to_string(),
            match_status: status.to_string(),
            resolved_skill_id: resolved.map(str::to_string),
            static_token_estimate: Some(12),
            static_byte_count: Some(42),
        }
    }

    #[tokio::test]
    async fn resolved_call_aggregates_attribute_by_resolved_id_with_source_and_target_scope() {
        let pool = mem_pool().await;
        // `call()` 默认 source 是 "Codex CLI"；显式区分两个 source。
        let mut claude_call = call("review");
        claude_call.source = "Claude Code".to_string();
        let mut codex_call = call("review");
        codex_call.timestamp_ms += 1_000;
        let mut other_target = call("review");
        other_target.timestamp_ms += 2_000;
        replace_calls_for_target(
            &pool,
            "local",
            &[claude_call, codex_call],
            &[],
            &[metadata("review", "matched", Some("review"))],
            10,
        )
        .await
        .unwrap();
        replace_calls_for_target(
            &pool,
            "ssh-prod",
            &[other_target],
            &[],
            &[metadata("review", "matched", Some("review"))],
            20,
        )
        .await
        .unwrap();

        let all = list_resolved_call_aggregates(&pool, "local", None)
            .await
            .unwrap();
        assert_eq!(all.len(), 1, "other target rows must stay invisible");
        assert_eq!(all[0].resolved_skill_id, "review");
        assert_eq!(all[0].call_count, 2);
        assert_eq!(all[0].last_used_ms, Some(1_700_000_001_000));
        assert_eq!(all[0].static_token_estimate, Some(12));
        assert_eq!(all[0].static_byte_count, Some(42));

        let claude = list_resolved_call_aggregates(&pool, "local", Some("Claude Code"))
            .await
            .unwrap();
        assert_eq!(claude[0].call_count, 1);
        assert_eq!(claude[0].last_used_ms, Some(1_700_000_000_000));

        // source 过滤掉全部调用时行仍保留、聚归零 —— 对该 source 即 never_used
        let absent = list_resolved_call_aggregates(&pool, "local", Some("Ghost Provider"))
            .await
            .unwrap();
        assert_eq!(absent[0].call_count, 0);
        assert_eq!(absent[0].last_used_ms, None);
    }

    #[tokio::test]
    async fn normalized_call_aggregates_normalize_names_and_filter_source_and_target() {
        let pool = mem_pool().await;
        // `call()` 默认 source 是 "Codex CLI"；显式区分两个 source。
        let mut first = call("review");
        first.source = "Claude Code".to_string();
        let mut spaced = call("  REVIEW ");
        spaced.timestamp_ms += 1_000;
        spaced.source = "Claude Code".to_string();
        let mut codex_call = call("Review");
        codex_call.timestamp_ms += 2_000;
        replace_calls_for_target(&pool, "local", &[first, spaced, codex_call], &[], &[], 10)
            .await
            .unwrap();
        replace_calls_for_target(&pool, "ssh-prod", &[call("review")], &[], &[], 20)
            .await
            .unwrap();

        let all = list_normalized_call_aggregates(&pool, "local", None)
            .await
            .unwrap();
        assert_eq!(all.len(), 1, "trim/case variants must fold into one bucket");
        assert_eq!(all[0].normalized_skill, "review");
        assert_eq!(all[0].call_count, 3);
        assert_eq!(all[0].last_used_ms, Some(1_700_000_002_000));

        let claude = list_normalized_call_aggregates(&pool, "local", Some("Claude Code"))
            .await
            .unwrap();
        assert_eq!(claude[0].call_count, 2);
        assert_eq!(claude[0].last_used_ms, Some(1_700_000_001_000));

        let remote = list_normalized_call_aggregates(&pool, "ssh-prod", None)
            .await
            .unwrap();
        assert_eq!(remote[0].call_count, 1);
    }
}
