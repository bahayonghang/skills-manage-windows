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

/// 当前 Usage 作用域。远程连接失败时保留 active target id，不回退到 Local，
/// 让读命令返回空的远程形状并由 UI 显示 unreachable banner。
#[derive(Debug)]
enum CurrentUsageScope {
    Reachable(Scope),
    RemoteUnavailable { target_id: String },
}

impl CurrentUsageScope {
    fn target_id(&self) -> String {
        match self {
            CurrentUsageScope::Reachable(scope) => scope.target_id(),
            CurrentUsageScope::RemoteUnavailable { target_id } => target_id.clone(),
        }
    }

    fn remote_unavailable(&self) -> bool {
        matches!(self, CurrentUsageScope::RemoteUnavailable { .. })
    }
}

async fn current_scope(state: &State<'_, AppState>) -> Result<CurrentUsageScope, String> {
    let active = state.targets.active_target(&state.db).await?;
    match active {
        ActiveTarget::Local => Ok(CurrentUsageScope::Reachable(Scope::Local)),
        remote @ (ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_)) => {
            let target_id = remote.id().to_string();
            let remote_home = remote.remote_home().unwrap_or("/").to_string();
            match connect_remote_target(&remote).await {
                Ok(connection) => Ok(CurrentUsageScope::Reachable(Scope::Remote {
                    target_id,
                    remote_home,
                    connection: Arc::new(connection),
                })),
                Err(error) => {
                    tracing::warn!(
                        target_id = %target_id,
                        error = %error,
                        "Skill Usage: failed to connect remote target; returning empty remote usage scope"
                    );
                    Ok(CurrentUsageScope::RemoteUnavailable { target_id })
                }
            }
        }
    }
}

fn empty_overview() -> UsageOverview {
    UsageOverview {
        kpis: usage::aggregate::UsageKpis::default(),
        top_skills: vec![],
        heatmap: usage::aggregate::heatmap_grid_16w(&[], Utc::now().timestamp_millis()),
        last_scan_ms: None,
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

#[tauri::command]
pub async fn usage_refresh(
    state: State<'_, AppState>,
    force: bool,
) -> Result<RefreshSummary, String> {
    let scope = current_scope(&state).await?;
    match scope {
        CurrentUsageScope::Reachable(scope) => usage::refresh(&state.db, &scope, force).await,
        CurrentUsageScope::RemoteUnavailable { .. } => Ok(RefreshSummary {
            cached: false,
            calls_written: 0,
            providers_available: 0,
            scanned_at_ms: Utc::now().timestamp_millis(),
        }),
    }
}

#[tauri::command]
pub async fn usage_get_overview(
    state: State<'_, AppState>,
    top_skills_limit: Option<usize>,
) -> Result<UsageOverview, String> {
    let scope = current_scope(&state).await?;
    if scope.remote_unavailable() {
        return Ok(empty_overview());
    }
    let target_id = scope.target_id();
    let limit = top_skills_limit.unwrap_or(50);
    usage::build_overview(&state.db, &target_id, limit).await
}

#[tauri::command]
pub async fn usage_get_recent(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<SkillCall>, String> {
    let scope = current_scope(&state).await?;
    if scope.remote_unavailable() {
        return Ok(vec![]);
    }
    let target_id = scope.target_id();
    let n = limit.unwrap_or(20).max(1);
    let rows = crate::db::list_recent_calls(&state.db, &target_id, n).await?;
    Ok(usage::rows_to_skill_calls(rows))
}

#[tauri::command]
pub async fn usage_get_providers(
    state: State<'_, AppState>,
) -> Result<Vec<ProviderHealth>, String> {
    let scope = current_scope(&state).await?;
    if scope.remote_unavailable() {
        return Ok(vec![]);
    }
    let target_id = scope.target_id();
    usage::list_provider_health(&state.db, &target_id).await
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
    let scope = current_scope(&state).await?;
    if scope.remote_unavailable() {
        return Ok(empty_skill_detail(skill));
    }
    let target_id = scope.target_id();
    let rows = crate::db::list_calls_for_target(&state.db, &target_id).await?;
    let filtered: Vec<_> = rows.into_iter().filter(|r| r.skill == skill).collect();

    if filtered.is_empty() {
        return Ok(SkillUsageDetail {
            skill,
            count: 0,
            sessions: 0,
            first_used_ms: 0,
            last_used_ms: 0,
            by_project: vec![],
            weekly: vec![],
        });
    }

    let count = filtered.len() as i64;
    let mut sessions = std::collections::HashSet::new();
    let mut first_used = i64::MAX;
    let mut last_used = 0i64;
    for r in &filtered {
        sessions.insert(r.session_id.clone());
        first_used = first_used.min(r.timestamp_ms);
        last_used = last_used.max(r.timestamp_ms);
    }

    // 按项目计数（复用 top_skills_from_rows 的 bucket 形态：把 project 当 skill 算）
    let by_project = {
        let mut m: HashMap<String, (i64, i64)> = HashMap::new();
        for r in &filtered {
            let e = m.entry(r.project.clone()).or_insert((0, 0));
            e.0 += 1;
            if r.timestamp_ms > e.1 {
                e.1 = r.timestamp_ms;
            }
        }
        let mut v: Vec<_> = m
            .into_iter()
            .map(|(name, (c, last))| SkillCount {
                skill: name,
                count: c,
                projects: 1,
                sessions: 0,
                last_used_ms: last,
            })
            .collect();
        v.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then(b.last_used_ms.cmp(&a.last_used_ms))
        });
        v
    };

    // 16 周稀疏图：每周一格 count 总和
    let weekly = usage::aggregate::heatmap_grid_16w(&filtered, Utc::now().timestamp_millis());

    Ok(SkillUsageDetail {
        skill,
        count,
        sessions: sessions.len() as i64,
        first_used_ms: first_used,
        last_used_ms: last_used,
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
    let scope = current_scope(&state).await?;
    if scope.remote_unavailable() {
        return Ok(skills.into_iter().map(|skill| (skill, 0)).collect());
    }
    let target_id = scope.target_id();
    let rows = crate::db::list_calls_for_target(&state.db, &target_id).await?;

    let cutoff = (Utc::now() - chrono::Duration::days(days as i64)).timestamp_millis();
    let want: std::collections::HashSet<&str> = skills.iter().map(|s| s.as_str()).collect();

    let mut out: HashMap<String, i64> = HashMap::new();
    // 提前用 0 占位，让前端拿到完整 keyset 不用做 fallback
    for s in &skills {
        out.insert(s.clone(), 0);
    }
    for r in &rows {
        if r.timestamp_ms < cutoff {
            continue;
        }
        if want.contains(r.skill.as_str()) {
            *out.entry(r.skill.clone()).or_insert(0) += 1;
        }
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
    /// 远程时是否能成功连接。connect_remote_target 失败时为 false 且
    /// is_remote=true，让前端区分「连接失败 fallback Local」和「真正在 Local」。
    pub remote_reachable: bool,
}

#[tauri::command]
pub async fn usage_get_scope_info(state: State<'_, AppState>) -> Result<UsageScopeInfo, String> {
    let active = state.targets.active_target(&state.db).await?;
    let target_id = active.id().to_string();
    let label = active.label().to_string();
    let is_remote = active.is_remote_like();
    let remote_reachable = if is_remote {
        connect_remote_target(&active).await.is_ok()
    } else {
        false
    };
    Ok(UsageScopeInfo {
        target_id,
        label,
        is_remote,
        remote_reachable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_unavailable_scope_keeps_remote_target_id() {
        let scope = CurrentUsageScope::RemoteUnavailable {
            target_id: "ssh-prod".to_string(),
        };

        assert_eq!(scope.target_id(), "ssh-prod");
        assert!(scope.remote_unavailable());
    }

    #[test]
    fn empty_remote_shapes_do_not_include_stale_calls() {
        let overview = empty_overview();
        assert_eq!(overview.kpis.total_calls, 0);
        assert!(overview.top_skills.is_empty());
        assert_eq!(overview.heatmap.len(), 16 * 7);
        assert!(overview.last_scan_ms.is_none());

        let detail = empty_skill_detail("review".to_string());
        assert_eq!(detail.skill, "review");
        assert_eq!(detail.count, 0);
        assert!(detail.by_project.is_empty());
        assert!(detail.weekly.is_empty());
    }
}
