//! Skill Usage 子系统的 Tauri IPC 入口。
//!
//! 对应前端 src/stores/usageStore.ts 的 invoke 调用。所有命令都按
//! P1 阶段固定走 `Scope::Local`；P2 接 active target 后由
//! `services::usage::Scope::Remote` 替换。
//!
//! 命令一览：
//! - `usage_refresh(force)` —— 触发扫描，5 分钟缓存内 force=false 直接命中缓存
//! - `usage_get_overview(top_skills_limit)` —— KPI + Top skills + 16w 热力图
//! - `usage_get_recent(limit)` —— 最近 N 条调用，给 RecentCallsFeed
//! - `usage_get_providers()` —— Provider 健康表（含 stub）
//! - `usage_get_skill_counts(skills, days)` —— 给 PlatformView/CentralSkillsView
//!   注入「近 N 天 K 次」徽章用，批量 name → count
//! - `usage_resolve_skill_id(name)` —— 名称匹配中央库 skill_id，给柱图点击跳详情
//! - `usage_get_skill_detail(skill)` —— 单技能详情（按项目分布 + 16w 稀疏图）

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::services::usage::{
    self,
    aggregate::{DayCount, SkillCount, UsageOverview},
    ProviderHealth, RefreshSummary, Scope, SkillCall,
};
use crate::targets::{connect_remote_target, ActiveTarget};
use crate::AppState;

#[derive(Debug, Clone)]
struct ActiveUsageTarget {
    active: ActiveTarget,
    target_id: String,
    label: String,
    is_remote: bool,
}

async fn active_usage_target(state: &State<'_, AppState>) -> Result<ActiveUsageTarget, String> {
    let active = state.targets.active_target(&state.db).await?;
    Ok(ActiveUsageTarget {
        target_id: active.id().to_string(),
        label: active.label().to_string(),
        is_remote: active.is_remote_like(),
        active,
    })
}

fn scope_info_for_target(
    target: &ActiveUsageTarget,
    remote_reachable: Option<bool>,
) -> UsageScopeInfo {
    UsageScopeInfo {
        target_id: target.target_id.clone(),
        label: target.label.clone(),
        is_remote: target.is_remote,
        remote_reachable: if target.is_remote {
            remote_reachable.unwrap_or(true)
        } else {
            false
        },
    }
}

fn cached_fallback_summary(last_scan_ms: Option<i64>) -> RefreshSummary {
    RefreshSummary {
        cached: last_scan_ms.is_some(),
        calls_written: 0,
        providers_available: 0,
        scanned_at_ms: last_scan_ms.unwrap_or(0),
    }
}

fn empty_skill_detail(skill: String) -> SkillUsageDetail {
    SkillUsageDetail {
        skill,
        count: 0,
        sessions: 0,
        first_used_ms: 0,
        last_used_ms: 0,
        by_project: vec![],
        weekly: vec![],
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRefreshResult {
    pub summary: RefreshSummary,
    pub overview: UsageOverview,
    pub recent: Vec<SkillCall>,
    pub providers: Vec<ProviderHealth>,
    pub scope: UsageScopeInfo,
    pub used_cached_data: bool,
    pub refresh_error: Option<String>,
}

async fn build_refresh_page(
    state: &State<'_, AppState>,
    target_id: &str,
    summary: RefreshSummary,
    scope: UsageScopeInfo,
    used_cached_data: bool,
    refresh_error: Option<String>,
) -> Result<UsageRefreshResult, String> {
    let overview = usage::build_overview(&state.db, target_id, 50).await?;
    let recent =
        usage::rows_to_skill_calls(crate::db::list_recent_calls(&state.db, target_id, 20).await?);
    let providers = usage::list_provider_health(&state.db, target_id).await?;

    Ok(UsageRefreshResult {
        summary,
        overview,
        recent,
        providers,
        scope,
        used_cached_data,
        refresh_error,
    })
}

#[tauri::command]
pub async fn usage_refresh(
    state: State<'_, AppState>,
    force: bool,
) -> Result<UsageRefreshResult, String> {
    let target = active_usage_target(&state).await?;

    if target.is_remote && !force {
        if let Some(last_scan_ms) =
            crate::db::get_last_scan_ms(&state.db, &target.target_id).await?
        {
            let now_ms = Utc::now().timestamp_millis();
            if now_ms - last_scan_ms < usage::CACHE_TTL_MS {
                return build_refresh_page(
                    &state,
                    &target.target_id,
                    RefreshSummary {
                        cached: true,
                        calls_written: 0,
                        providers_available: 0,
                        scanned_at_ms: last_scan_ms,
                    },
                    scope_info_for_target(&target, None),
                    true,
                    None,
                )
                .await;
            }
        }
    }

    match &target.active {
        ActiveTarget::Local => {
            let summary = usage::refresh(&state.db, &Scope::Local, force).await?;
            build_refresh_page(
                &state,
                &target.target_id,
                summary.clone(),
                scope_info_for_target(&target, Some(false)),
                summary.cached,
                None,
            )
            .await
        }
        remote @ (ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_)) => {
            let remote_home = remote.remote_home().unwrap_or("/").to_string();
            match connect_remote_target(remote).await {
                Ok(connection) => {
                    let scope = Scope::Remote {
                        target_id: target.target_id.clone(),
                        remote_home,
                        connection: Arc::new(connection),
                    };
                    let summary = usage::refresh(&state.db, &scope, force).await?;
                    build_refresh_page(
                        &state,
                        &target.target_id,
                        summary.clone(),
                        scope_info_for_target(&target, Some(true)),
                        summary.cached,
                        None,
                    )
                    .await
                }
                Err(error) => {
                    tracing::warn!(
                        target_id = %target.target_id,
                        error = %error,
                        "Skill Usage: remote refresh failed; returning cached local usage data"
                    );
                    let last_scan_ms =
                        crate::db::get_last_scan_ms(&state.db, &target.target_id).await?;
                    build_refresh_page(
                        &state,
                        &target.target_id,
                        cached_fallback_summary(last_scan_ms),
                        scope_info_for_target(&target, Some(false)),
                        last_scan_ms.is_some(),
                        Some(error.to_string()),
                    )
                    .await
                }
            }
        }
    }
}

#[tauri::command]
pub async fn usage_get_overview(
    state: State<'_, AppState>,
    top_skills_limit: Option<usize>,
) -> Result<UsageOverview, String> {
    let target = active_usage_target(&state).await?;
    let limit = top_skills_limit.unwrap_or(50);
    usage::build_overview(&state.db, &target.target_id, limit).await
}

#[tauri::command]
pub async fn usage_get_recent(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<SkillCall>, String> {
    let n = limit.unwrap_or(20).max(1);
    let target = active_usage_target(&state).await?;
    let rows = crate::db::list_recent_calls(&state.db, &target.target_id, n).await?;
    Ok(usage::rows_to_skill_calls(rows))
}

#[tauri::command]
pub async fn usage_get_providers(
    state: State<'_, AppState>,
) -> Result<Vec<ProviderHealth>, String> {
    let target = active_usage_target(&state).await?;
    usage::list_provider_health(&state.db, &target.target_id).await
}

/// 单 skill 详情 —— 当 SkillBarChart 没有匹配到中央库 skill_id 时的内嵌备选视图。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUsageDetail {
    pub skill: String,
    pub count: i64,
    pub sessions: i64,
    pub first_used_ms: i64,
    pub last_used_ms: i64,
    pub by_project: Vec<SkillCount>,
    pub weekly: Vec<DayCount>,
}

#[tauri::command]
pub async fn usage_get_skill_detail(
    state: State<'_, AppState>,
    skill: String,
) -> Result<SkillUsageDetail, String> {
    let target = active_usage_target(&state).await?;
    let summary = crate::db::get_skill_detail_summary(&state.db, &target.target_id, &skill)
        .await?
        .unwrap_or_default();

    if summary.count == 0 {
        return Ok(empty_skill_detail(skill));
    }

    let by_project = crate::db::list_skill_project_counts(&state.db, &target.target_id, &skill)
        .await?
        .into_iter()
        .map(|row| SkillCount {
            skill: row.skill,
            count: row.count,
            projects: row.projects,
            sessions: row.sessions,
            last_used_ms: row.last_used_ms,
        })
        .collect();
    let cutoff_ms = Utc::now().timestamp_millis() - (16 * 7 * 86_400_000);
    let weekly = usage::aggregate::heatmap_grid_16w_from_daily_counts(
        &crate::db::list_skill_daily_counts_since(&state.db, &target.target_id, &skill, cutoff_ms)
            .await?
            .into_iter()
            .map(|row| DayCount {
                date: row.date,
                count: row.count,
            })
            .collect::<Vec<_>>(),
        Utc::now().timestamp_millis(),
    );

    Ok(SkillUsageDetail {
        skill,
        count: summary.count,
        sessions: summary.sessions,
        first_used_ms: summary.first_used_ms,
        last_used_ms: summary.last_used_ms,
        by_project,
        weekly,
    })
}

/// 给 PlatformView / CentralSkillsView 的 skill 卡片注入 "近 N 天 K 次" 徽章。
/// 返回 `{ skill_name → count }` 的 map。空请求返回空 map。
#[tauri::command]
pub async fn usage_get_skill_counts(
    state: State<'_, AppState>,
    skills: Vec<String>,
    days: u32,
) -> Result<HashMap<String, i64>, String> {
    if skills.is_empty() {
        return Ok(HashMap::new());
    }
    let target = active_usage_target(&state).await?;
    let cutoff = (Utc::now() - chrono::Duration::days(days as i64)).timestamp_millis();
    let mut out: HashMap<String, i64> = HashMap::new();
    // 提前用 0 占位，让前端拿到完整 keyset 不用做 fallback
    for s in &skills {
        out.insert(s.clone(), 0);
    }
    for (skill, count) in
        crate::db::list_skill_counts_since(&state.db, &target.target_id, &skills, cutoff).await?
    {
        out.insert(skill, count);
    }
    Ok(out)
}

/// 名字匹配中央库 skill_id —— 给 SkillBarChart 点击跳 `/skill/:id`。
/// 大小写不敏感，agent 无关；优先返回 `is_central=1` 的记录。匹配不到返回 None。
#[tauri::command]
pub async fn usage_resolve_skill_id(
    state: State<'_, AppState>,
    skill_name: String,
) -> Result<Option<String>, String> {
    let trimmed = skill_name.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    // 直接 SQL：先 is_central desc，再 name 大小写不敏感等值
    let id: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM skills
         WHERE LOWER(name) = LOWER(?)
         ORDER BY is_central DESC, scanned_at DESC
         LIMIT 1",
    )
    .bind(trimmed)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(id.map(|(s,)| s))
}

/// 给 UI 显示「上次扫描时间」用的格式化辅助。time-ago 由前端 i18n 处理，
/// 这里只把 ms 透传出去，避免后端引入 locale 状态。
#[allow(dead_code)]
fn ms_to_iso(ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(ms)
        .unwrap_or_default()
        .to_rfc3339()
}

/// Skill Usage 当前作用域信息 —— 给 UsageShell 顶部 ScopeBadge 显示。
///
/// `targetId` = "local" 或远程 target id；`isRemote` 让前端拍板是否
/// 显示降级提示；`label` 是 UI 上显示给人看的（例如 "alice@host"）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageScopeInfo {
    pub target_id: String,
    pub label: String,
    pub is_remote: bool,
    /// 远程 reachability。显式 refresh 会返回权威值；只读 getter 为避免额外
    /// SSH/WSL 建连，会对远程 target 乐观返回 true。
    pub remote_reachable: bool,
}

#[tauri::command]
pub async fn usage_get_scope_info(state: State<'_, AppState>) -> Result<UsageScopeInfo, String> {
    let target = active_usage_target(&state).await?;
    // 只读 getter 不做 SSH/WSL 建连；显式 refresh 返回权威 reachability。
    Ok(scope_info_for_target(&target, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_info_for_remote_read_paths_is_optimistic() {
        let target = ActiveUsageTarget {
            active: ActiveTarget::Local,
            target_id: "ssh-prod".to_string(),
            label: "alice@prod".to_string(),
            is_remote: true,
        };

        let optimistic = scope_info_for_target(&target, None);
        assert!(optimistic.remote_reachable);

        let unreachable = scope_info_for_target(&target, Some(false));
        assert!(!unreachable.remote_reachable);
    }

    #[test]
    fn cached_fallback_summary_preserves_last_successful_scan_time() {
        let summary = cached_fallback_summary(Some(1_700_000_000_000));
        assert!(summary.cached);
        assert_eq!(summary.scanned_at_ms, 1_700_000_000_000);

        let empty_summary = cached_fallback_summary(None);
        assert!(!empty_summary.cached);
        assert_eq!(empty_summary.scanned_at_ms, 0);
    }

    #[test]
    fn empty_skill_detail_returns_zero_shape() {
        let detail = empty_skill_detail("review".to_string());
        assert_eq!(detail.skill, "review");
        assert_eq!(detail.count, 0);
        assert!(detail.by_project.is_empty());
        assert!(detail.weekly.is_empty());
    }
}
