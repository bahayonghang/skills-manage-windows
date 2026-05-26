//! `skill_calls` / `skill_call_providers` / `skill_call_scan_state` CRUD —
//! 给 `services::usage` 编排器与命令层用。
//!
//! 关键约束：每次 `replace_calls_for_target` 都在事务内 DELETE + INSERT，
//! 让前端读端永远看到完整一批数据，不会卡在「半批」状态。这个原子性等价
//! 于 skilled 用临时文件 + rename 替换 db 文件的做法，但因为 SkillPort
//! 复用主库不能整库替换，所以用事务。

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::db::types::DbPool;

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
    scan_completed_at_ms: i64,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM skill_calls WHERE target_id = ?")
        .bind(target_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    for call in calls {
        sqlx::query(
            "INSERT INTO skill_calls (target_id, skill, timestamp_ms, project, session_id, source)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(target_id)
        .bind(&call.skill)
        .bind(call.timestamp_ms)
        .bind(&call.project)
        .bind(&call.session_id)
        .bind(&call.source)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
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
        .await
        .map_err(|e| e.to_string())?;
    }

    sqlx::query(
        "INSERT OR REPLACE INTO skill_call_scan_state (target_id, last_full_scan_ms)
         VALUES (?, ?)",
    )
    .bind(target_id)
    .bind(scan_completed_at_ms)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())
}

/// 读取指定 target 的最近一次扫描时间戳，给 5 分钟缓存判定用。
/// 没扫过返回 None。
pub async fn get_last_scan_ms(pool: &DbPool, target_id: &str) -> Result<Option<i64>, String> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT last_full_scan_ms FROM skill_call_scan_state WHERE target_id = ?")
            .bind(target_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
    Ok(row.map(|(ms,)| ms))
}

/// 列出指定 target 的所有 provider 健康状态行。stub provider 也会出现在结果里。
pub async fn list_provider_rows(
    pool: &DbPool,
    target_id: &str,
) -> Result<Vec<SkillCallProviderRow>, String> {
    sqlx::query_as::<_, SkillCallProviderRow>(
        "SELECT target_id, provider_id, display_name, available, call_count, scanned_at
         FROM skill_call_providers
         WHERE target_id = ?
         ORDER BY display_name ASC",
    )
    .bind(target_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

/// 列出指定 target 的所有调用记录，按时间升序。聚合层接到内存里再排序。
pub async fn list_calls_for_target(
    pool: &DbPool,
    target_id: &str,
) -> Result<Vec<SkillCallRow>, String> {
    sqlx::query_as::<_, SkillCallRow>(
        "SELECT id, target_id, skill, timestamp_ms, project, session_id, source
         FROM skill_calls
         WHERE target_id = ?
         ORDER BY timestamp_ms ASC",
    )
    .bind(target_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

/// 列出最近 N 条调用，按时间倒序。给 RecentCallsFeed 直接用。
pub async fn list_recent_calls(
    pool: &DbPool,
    target_id: &str,
    limit: i64,
) -> Result<Vec<SkillCallRow>, String> {
    sqlx::query_as::<_, SkillCallRow>(
        "SELECT id, target_id, skill, timestamp_ms, project, session_id, source
         FROM skill_calls
         WHERE target_id = ?
         ORDER BY timestamp_ms DESC
         LIMIT ?",
    )
    .bind(target_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}
