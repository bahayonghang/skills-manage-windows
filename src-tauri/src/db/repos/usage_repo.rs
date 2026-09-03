//! `skill_calls` / `skill_call_providers` / `skill_call_scan_state` CRUD —
//! 给 `services::usage` 编排器与命令层用。
//!
//! 关键约束：每次 `replace_calls_for_target` 都在事务内 DELETE + INSERT，
//! 让前端读端永远看到完整一批数据，不会卡在「半批」状态。这个原子性等价
//! 于 skilled 用临时文件 + rename 替换 db 文件的做法，但因为 SkillPort
//! 复用主库不能整库替换，所以用事务。

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder, Sqlite};

use crate::db::types::{DbPool, Skill};

/// 一次 skill 调用记录。字段对齐 skilled 的 SkillCall 模型。
///
/// `timestamp_ms` 是 Unix epoch 毫秒；前端拿到后通过 `new Date(timestamp_ms)`
/// 转 JS Date。`source` 是 provider 的 display name（例如 "Claude Code"），
/// 与 `skill_call_providers.display_name` 保持一致以便联表。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillCallRow {
    pub id: i64,
    pub target_id: String,
    pub skill: String,
    pub timestamp_ms: i64,
    pub project: String,
    pub session_id: String,
    pub source: String,
}

/// `skill_call_providers` 行：单个 provider 在一个 target 上的健康状态。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillCallProviderRow {
    pub target_id: String,
    pub provider_id: String,
    pub display_name: String,
    pub available: bool,
    pub call_count: i64,
    pub scanned_at: i64,
}

/// 用于事务批量写入的瘦数据；`id` / `target_id` 在写入时由 repo 填充。
#[derive(Debug, Clone)]
pub struct NewSkillCall {
    pub skill: String,
    pub timestamp_ms: i64,
    pub project: String,
    pub session_id: String,
    pub source: String,
}

/// 一个 provider 在一次扫描中的输出（用于 upsert provider health）。
#[derive(Debug, Clone)]
pub struct ProviderScanOutcome {
    pub provider_id: String,
    pub display_name: String,
    pub available: bool,
    pub call_count: i64,
}

#[derive(Debug, Clone)]
pub struct NewSkillUsageMetadata {
    pub skill: String,
    pub match_status: String,
    pub resolved_skill_id: Option<String>,
    pub static_token_estimate: Option<i64>,
    pub static_byte_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillUsageMetadataRow {
    pub target_id: String,
    pub skill: String,
    pub match_status: String,
    pub resolved_skill_id: Option<String>,
    pub static_token_estimate: Option<i64>,
    pub static_byte_count: Option<i64>,
    pub scanned_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageKpisRow {
    pub total_calls: i64,
    pub unique_skills: i64,
    pub unique_projects: i64,
    pub unique_sources: i64,
    pub unique_sessions: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetailSummaryRow {
    pub count: i64,
    pub sessions: i64,
    pub first_used_ms: i64,
    pub last_used_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillCountRow {
    pub skill: String,
    pub count: i64,
    pub projects: i64,
    pub sessions: i64,
    pub last_used_ms: i64,
}

/// SQLite 绑定变量上限按保守的 999 计：skill_calls 每行 6 个变量、
/// skill_usage_metadata 每行 7 个变量，100 行/块远低于上限。
const INSERT_CHUNK_ROWS: usize = 100;

/// 原子替换指定 target 的 skill_calls：事务内先 DELETE WHERE target_id=?，
/// 再批量 INSERT，最后 upsert provider 健康状态与 scan_state。
///
/// 失败时事务自动回滚（sqlx 的 `Transaction::commit` 不调用即视为 rollback），
/// 老数据保留可用。
pub async fn replace_calls_for_target(
    pool: &DbPool,
    target_id: &str,
    calls: &[NewSkillCall],
    providers: &[ProviderScanOutcome],
    metadata: &[NewSkillUsageMetadata],
    scan_completed_at_ms: i64,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM skill_calls WHERE target_id = ?")
        .bind(target_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM skill_call_providers WHERE target_id = ?")
        .bind(target_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM skill_usage_metadata WHERE target_id = ?")
        .bind(target_id)
        .execute(&mut *tx)
        .await?;

    // 多行批插（QueryBuilder 分块），取代逐行 INSERT 的 per-row 往返。
    for chunk in calls.chunks(INSERT_CHUNK_ROWS) {
        let mut builder = QueryBuilder::<Sqlite>::new(
            "INSERT INTO skill_calls (target_id, skill, timestamp_ms, project, session_id, source) ",
        );
        builder.push_values(chunk, |mut row, call| {
            row.push_bind(target_id)
                .push_bind(&call.skill)
                .push_bind(call.timestamp_ms)
                .push_bind(&call.project)
                .push_bind(&call.session_id)
                .push_bind(&call.source);
        });
        builder.build().execute(&mut *tx).await?;
    }

    // provider 表用 INSERT OR REPLACE，幂等更新一次扫描周期内每个 provider
    // 的健康状态。stub provider 也会进入这张表，UI 渲染时按 available=false
    // 显示「未检测到」。
    for outcome in providers {
        sqlx::query(
            "INSERT OR REPLACE INTO skill_call_providers
             (target_id, provider_id, display_name, available, call_count, scanned_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(target_id)
        .bind(&outcome.provider_id)
        .bind(&outcome.display_name)
        .bind(outcome.available)
        .bind(outcome.call_count)
        .bind(scan_completed_at_ms)
        .execute(&mut *tx)
        .await?;
    }

    for chunk in metadata.chunks(INSERT_CHUNK_ROWS) {
        let mut builder = QueryBuilder::<Sqlite>::new(
            "INSERT INTO skill_usage_metadata
             (target_id, skill, match_status, resolved_skill_id,
              static_token_estimate, static_byte_count, scanned_at_ms) ",
        );
        builder.push_values(chunk, |mut row, item| {
            row.push_bind(target_id)
                .push_bind(&item.skill)
                .push_bind(&item.match_status)
                .push_bind(&item.resolved_skill_id)
                .push_bind(item.static_token_estimate)
                .push_bind(item.static_byte_count)
                .push_bind(scan_completed_at_ms);
        });
        builder.build().execute(&mut *tx).await?;
    }

    sqlx::query(
        "INSERT OR REPLACE INTO skill_call_scan_state (target_id, last_full_scan_ms)
         VALUES (?, ?)",
    )
    .bind(target_id)
    .bind(scan_completed_at_ms)
    .execute(&mut *tx)
    .await?;

    tx.commit().await
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillProjectCountRow {
    pub project: String,
    pub count: i64,
    pub sessions: i64,
    pub last_used_ms: i64,
}

pub async fn list_usage_skill_candidates(
    pool: &DbPool,
    normalized_names: &[String],
) -> Result<Vec<Skill>, sqlx::Error> {
    if normalized_names.is_empty() {
        return Ok(Vec::new());
    }

    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT * FROM skills WHERE is_central = 1 AND (LOWER(TRIM(id)) IN (",
    );
    {
        let mut separated = builder.separated(", ");
        for name in normalized_names {
            separated.push_bind(name);
        }
    }
    builder.push(") OR LOWER(TRIM(name)) IN (");
    {
        let mut separated = builder.separated(", ");
        for name in normalized_names {
            separated.push_bind(name);
        }
    }
    builder.push(")) ORDER BY id ASC");

    builder.build_query_as::<Skill>().fetch_all(pool).await
}

pub async fn list_usage_metadata(
    pool: &DbPool,
    target_id: &str,
) -> Result<Vec<SkillUsageMetadataRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillUsageMetadataRow>(
        "SELECT target_id, skill, match_status, resolved_skill_id,
                static_token_estimate, static_byte_count, scanned_at_ms
         FROM skill_usage_metadata
         WHERE target_id = ?
         ORDER BY skill ASC",
    )
    .bind(target_id)
    .fetch_all(pool)
    .await
}

pub async fn get_usage_metadata_for_skill(
    pool: &DbPool,
    target_id: &str,
    skill: &str,
) -> Result<Option<SkillUsageMetadataRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillUsageMetadataRow>(
        "SELECT target_id, skill, match_status, resolved_skill_id,
                static_token_estimate, static_byte_count, scanned_at_ms
         FROM skill_usage_metadata
         WHERE target_id = ? AND skill = ?",
    )
    .bind(target_id)
    .bind(skill)
    .fetch_optional(pool)
    .await
}

/// 读取指定 target 的最近一次扫描时间戳，给 5 分钟缓存判定用。
/// 没扫过返回 None。
pub async fn get_last_scan_ms(pool: &DbPool, target_id: &str) -> Result<Option<i64>, sqlx::Error> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT last_full_scan_ms FROM skill_call_scan_state WHERE target_id = ?")
            .bind(target_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(ms,)| ms))
}

/// 列出指定 target 的所有 provider 健康状态行。stub provider 也会出现在结果里。
pub async fn list_provider_rows(
    pool: &DbPool,
    target_id: &str,
) -> Result<Vec<SkillCallProviderRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillCallProviderRow>(
        "SELECT target_id, provider_id, display_name, available, call_count, scanned_at
         FROM skill_call_providers
         WHERE target_id = ?
         ORDER BY display_name ASC",
    )
    .bind(target_id)
    .fetch_all(pool)
    .await
}

/// 列出指定 target 的所有调用记录，按时间升序。聚合层接到内存里再排序。
pub async fn list_calls_for_target(
    pool: &DbPool,
    target_id: &str,
) -> Result<Vec<SkillCallRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillCallRow>(
        "SELECT id, target_id, skill, timestamp_ms, project, session_id, source
         FROM skill_calls
         WHERE target_id = ?
         ORDER BY timestamp_ms ASC",
    )
    .bind(target_id)
    .fetch_all(pool)
    .await
}

/// 列出最近 N 条调用，按时间倒序。给 RecentCallsFeed 直接用。
pub async fn list_recent_calls(
    pool: &DbPool,
    target_id: &str,
    source: Option<&str>,
    limit: i64,
) -> Result<Vec<SkillCallRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillCallRow>(
        "SELECT id, target_id, skill, timestamp_ms, project, session_id, source
         FROM skill_calls
         WHERE target_id = ?
           AND (? IS NULL OR source = ?)
         ORDER BY timestamp_ms DESC
         LIMIT ?",
    )
    .bind(target_id)
    .bind(source)
    .bind(source)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get_usage_kpis(
    pool: &DbPool,
    target_id: &str,
    source: Option<&str>,
) -> Result<UsageKpisRow, sqlx::Error> {
    sqlx::query_as::<_, UsageKpisRow>(
        "SELECT
            COUNT(*) AS total_calls,
            COUNT(DISTINCT skill) AS unique_skills,
            COUNT(DISTINCT project) AS unique_projects,
            COUNT(DISTINCT source) AS unique_sources,
            COUNT(DISTINCT session_id) AS unique_sessions
         FROM skill_calls
         WHERE target_id = ?
           AND (? IS NULL OR source = ?)",
    )
    .bind(target_id)
    .bind(source)
    .bind(source)
    .fetch_one(pool)
    .await
}

pub async fn list_top_skills(
    pool: &DbPool,
    target_id: &str,
    source: Option<&str>,
    limit: usize,
) -> Result<Vec<SkillCountRow>, sqlx::Error> {
    let limit = if limit == 0 { i64::MAX } else { limit as i64 };
    sqlx::query_as::<_, SkillCountRow>(
        "SELECT
            skill,
            COUNT(*) AS count,
            COUNT(DISTINCT project) AS projects,
            COUNT(DISTINCT session_id) AS sessions,
            MAX(timestamp_ms) AS last_used_ms
         FROM skill_calls
         WHERE target_id = ?
           AND (? IS NULL OR source = ?)
         GROUP BY skill
         ORDER BY count DESC, last_used_ms DESC, skill ASC
         LIMIT ?",
    )
    .bind(target_id)
    .bind(source)
    .bind(source)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn list_timestamps_since(
    pool: &DbPool,
    target_id: &str,
    source: Option<&str>,
    cutoff_ms: i64,
) -> Result<Vec<i64>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (i64,)>(
        "SELECT timestamp_ms
         FROM skill_calls
         WHERE target_id = ?
           AND (? IS NULL OR source = ?)
           AND timestamp_ms >= ?
         ORDER BY timestamp_ms ASC",
    )
    .bind(target_id)
    .bind(source)
    .bind(source)
    .bind(cutoff_ms)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(timestamp_ms,)| timestamp_ms)
        .collect())
}

pub async fn get_skill_detail_summary(
    pool: &DbPool,
    target_id: &str,
    skill: &str,
    source: Option<&str>,
) -> Result<Option<SkillDetailSummaryRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillDetailSummaryRow>(
        "SELECT
            COUNT(*) AS count,
            COUNT(DISTINCT session_id) AS sessions,
            COALESCE(MIN(timestamp_ms), 0) AS first_used_ms,
            COALESCE(MAX(timestamp_ms), 0) AS last_used_ms
         FROM skill_calls
         WHERE target_id = ?
           AND skill = ?
           AND (? IS NULL OR source = ?)",
    )
    .bind(target_id)
    .bind(skill)
    .bind(source)
    .bind(source)
    .fetch_optional(pool)
    .await
}

pub async fn list_skill_project_counts(
    pool: &DbPool,
    target_id: &str,
    skill: &str,
    source: Option<&str>,
) -> Result<Vec<SkillProjectCountRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillProjectCountRow>(
        "SELECT
            project,
            COUNT(*) AS count,
            COUNT(DISTINCT session_id) AS sessions,
            MAX(timestamp_ms) AS last_used_ms
         FROM skill_calls
         WHERE target_id = ?
           AND skill = ?
           AND (? IS NULL OR source = ?)
         GROUP BY project
         ORDER BY count DESC, last_used_ms DESC, project ASC",
    )
    .bind(target_id)
    .bind(skill)
    .bind(source)
    .bind(source)
    .fetch_all(pool)
    .await
}

pub async fn list_skill_timestamps_since(
    pool: &DbPool,
    target_id: &str,
    skill: &str,
    source: Option<&str>,
    cutoff_ms: i64,
) -> Result<Vec<i64>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (i64,)>(
        "SELECT timestamp_ms
         FROM skill_calls
         WHERE target_id = ?
           AND skill = ?
           AND (? IS NULL OR source = ?)
           AND timestamp_ms >= ?
         ORDER BY timestamp_ms ASC",
    )
    .bind(target_id)
    .bind(skill)
    .bind(source)
    .bind(source)
    .bind(cutoff_ms)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(timestamp_ms,)| timestamp_ms)
        .collect())
}

pub async fn list_skill_counts_since(
    pool: &DbPool,
    target_id: &str,
    skills: &[String],
    cutoff_ms: i64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    if skills.is_empty() {
        return Ok(vec![]);
    }

    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT skill, COUNT(*) AS count
         FROM skill_calls
         WHERE target_id = ",
    );
    builder
        .push_bind(target_id)
        .push(" AND timestamp_ms >= ")
        .push_bind(cutoff_ms)
        .push(" AND skill IN (");

    {
        let mut separated = builder.separated(", ");
        for skill in skills {
            separated.push_bind(skill);
        }
    }

    builder.push(
        ")
         GROUP BY skill",
    );

    let rows = builder
        .build_query_as::<(String, i64)>()
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn metadata_replacement_is_atomic_and_target_scoped() {
        let pool = mem_pool().await;
        replace_calls_for_target(
            &pool,
            "local",
            &[call("review")],
            &[],
            &[metadata("review", "matched", Some("review"))],
            10,
        )
        .await
        .unwrap();
        replace_calls_for_target(
            &pool,
            "ssh-prod",
            &[call("remote-review")],
            &[],
            &[metadata("remote-review", "unmatched", None)],
            20,
        )
        .await
        .unwrap();

        let error = replace_calls_for_target(
            &pool,
            "local",
            &[call("new-call")],
            &[],
            &[metadata("new-call", "invalid", None)],
            30,
        )
        .await;
        assert!(error.is_err(), "invalid metadata must roll back the batch");

        let local_calls = list_calls_for_target(&pool, "local").await.unwrap();
        assert_eq!(local_calls.len(), 1);
        assert_eq!(local_calls[0].skill, "review");
        let local_metadata = list_usage_metadata(&pool, "local").await.unwrap();
        assert_eq!(local_metadata.len(), 1);
        assert_eq!(
            local_metadata[0].resolved_skill_id.as_deref(),
            Some("review")
        );

        let remote_calls = list_calls_for_target(&pool, "ssh-prod").await.unwrap();
        assert_eq!(remote_calls.len(), 1);
        assert_eq!(remote_calls[0].skill, "remote-review");
        let remote_metadata = list_usage_metadata(&pool, "ssh-prod").await.unwrap();
        assert_eq!(remote_metadata.len(), 1);
        assert_eq!(remote_metadata[0].match_status, "unmatched");
    }

    #[tokio::test]
    async fn batched_insert_spans_chunks_and_preserves_all_rows() {
        let pool = mem_pool().await;
        // 250 条 > INSERT_CHUNK_ROWS（100）→ 跨 3 个块
        let calls: Vec<NewSkillCall> = (0..250)
            .map(|i| NewSkillCall {
                skill: format!("skill-{i}"),
                timestamp_ms: 1_700_000_000_000 + i,
                project: format!("/project-{}", i % 5),
                session_id: format!("session-{}", i % 7),
                source: "Codex CLI".to_string(),
            })
            .collect();
        let metadata: Vec<NewSkillUsageMetadata> = (0..250)
            .map(|i| NewSkillUsageMetadata {
                skill: format!("skill-{i}"),
                match_status: "unmatched".to_string(),
                resolved_skill_id: None,
                static_token_estimate: None,
                static_byte_count: None,
            })
            .collect();
        replace_calls_for_target(&pool, "local", &calls, &[], &metadata, 10)
            .await
            .unwrap();

        let stored = list_calls_for_target(&pool, "local").await.unwrap();
        assert_eq!(stored.len(), 250);
        let first = stored.iter().find(|c| c.skill == "skill-0").unwrap();
        assert_eq!(first.project, "/project-0");
        assert_eq!(first.session_id, "session-0");
        let last = stored.iter().find(|c| c.skill == "skill-249").unwrap();
        assert_eq!(last.timestamp_ms, 1_700_000_000_000 + 249);
        assert_eq!(
            list_usage_metadata(&pool, "local").await.unwrap().len(),
            250
        );

        // 替换语义不变：第二批替换第一批
        replace_calls_for_target(&pool, "local", &[call("review")], &[], &[], 20)
            .await
            .unwrap();
        let stored = list_calls_for_target(&pool, "local").await.unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].skill, "review");
    }

    #[tokio::test]
    async fn skill_detail_queries_filter_source_and_count_distinct_project_sessions() {
        let pool = mem_pool().await;
        let mut claude_first = call("review");
        claude_first.source = "Claude Code".to_string();
        claude_first.session_id = "claude-1".to_string();
        let mut claude_repeat = claude_first.clone();
        claude_repeat.timestamp_ms += 1;
        let mut claude_second = claude_first.clone();
        claude_second.timestamp_ms += 2;
        claude_second.session_id = "claude-2".to_string();
        let mut codex = claude_first.clone();
        codex.timestamp_ms += 3;
        codex.source = "Codex CLI".to_string();
        codex.session_id = "codex-1".to_string();

        replace_calls_for_target(
            &pool,
            "local",
            &[claude_first, claude_repeat, claude_second, codex],
            &[],
            &[],
            10,
        )
        .await
        .unwrap();

        let summary = get_skill_detail_summary(&pool, "local", "review", Some("Claude Code"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(summary.count, 3);
        assert_eq!(summary.sessions, 2);

        let projects = list_skill_project_counts(&pool, "local", "review", Some("Claude Code"))
            .await
            .unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].count, 3);
        assert_eq!(projects[0].sessions, 2);

        let timestamps =
            list_skill_timestamps_since(&pool, "local", "review", Some("Claude Code"), 0)
                .await
                .unwrap();
        assert_eq!(timestamps.len(), 3);
    }

    #[tokio::test]
    async fn empty_replacement_clears_one_target_and_leaves_the_other() {
        let pool = mem_pool().await;
        replace_calls_for_target(
            &pool,
            "local",
            &[call("keep-local")],
            &[ProviderScanOutcome {
                provider_id: "codex".to_string(),
                display_name: "Codex CLI".to_string(),
                available: true,
                call_count: 1,
            }],
            &[metadata("keep-local", "unmatched", None)],
            10,
        )
        .await
        .unwrap();
        replace_calls_for_target(
            &pool,
            "ssh-prod",
            &[call("clear-me")],
            &[ProviderScanOutcome {
                provider_id: "codex".to_string(),
                display_name: "Codex CLI".to_string(),
                available: true,
                call_count: 1,
            }],
            &[metadata("clear-me", "unmatched", None)],
            20,
        )
        .await
        .unwrap();

        replace_calls_for_target(&pool, "ssh-prod", &[], &[], &[], 30)
            .await
            .unwrap();

        assert!(list_calls_for_target(&pool, "ssh-prod")
            .await
            .unwrap()
            .is_empty());
        assert!(list_usage_metadata(&pool, "ssh-prod")
            .await
            .unwrap()
            .is_empty());
        assert_eq!(get_last_scan_ms(&pool, "ssh-prod").await.unwrap(), Some(30));

        let local_calls = list_calls_for_target(&pool, "local").await.unwrap();
        assert_eq!(local_calls.len(), 1);
        assert_eq!(local_calls[0].skill, "keep-local");
        let local_metadata = list_usage_metadata(&pool, "local").await.unwrap();
        assert_eq!(local_metadata.len(), 1);
        assert_eq!(get_last_scan_ms(&pool, "local").await.unwrap(), Some(10));
    }
}
