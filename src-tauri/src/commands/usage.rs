//! Skill Usage 子系统的 Tauri IPC 入口。
//!
//! 对应前端 src/stores/usageStore.ts 的 invoke 调用。所有命令都按
//! P1 阶段固定走 `Scope::Local`；P2 接 active target 后由
//! `services::usage::Scope::Remote` 替换。
//!
//! 命令一览：
//! - `usage_refresh(force)` —— 触发扫描；5 分钟缓存内 force=false 直接命中缓存；
//!   本地 target 缓存过期时立即返回缓存页（`scanning=true`）并后台重扫，
//!   完成后 emit `usage://scan-completed`（payload = target id）
//! - `usage_get_overview(top_skills_limit)` —— KPI + Top skills + 16w 热力图
//! - `usage_get_recent(limit)` —— 最近 N 条调用，给 RecentCallsFeed
//! - `usage_get_providers()` —— Provider 健康表（含 stub）
//! - `usage_get_skill_counts(skills, days)` —— 给 PlatformView/CentralSkillsView
//!   注入「近 N 天 K 次」徽章用，批量 name → count
//! - `usage_get_skill_usage_stats(skills, days)` —— 全历史（或近 N 天）
//!   name → `{ count, lastUsedMs }`；`days = None` 为全部已记录历史
//! - `usage_resolve_skill_id(name)` —— 名称匹配中央库 skill_id，给柱图点击跳详情
//! - `usage_get_skill_detail(skill)` —— 单技能详情（按项目分布 + 16w 稀疏图）
//! - `usage_get_unused_skills(source, threshold_days)` —— 从未使用/长期未用
//!   报表（Central 库 + 平台安装双维度，只读派生）

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ipc_error::{public_message_for_code, IpcError, REVIEWED_IPC_ERROR_CODES};
use crate::observability::{
    CommandLogPolicy, OperationContext, OperationDefinition, OperationTarget, OperationTargetKind,
    ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeOperationResult,
};
use crate::services::usage::{
    self,
    aggregate::{RecentSkillCall, SkillUsageDetail, UnusedSkillsReport, UsageOverview},
    ProviderHealth, RefreshSummary, Scope,
};
use crate::targets::{connect_remote_target, ActiveTarget};
use crate::AppState;

/// 本地后台重扫在途标记（按 target id 去重，避免页面反复进入时叠加扫描）。
static USAGE_SCAN_IN_FLIGHT: std::sync::LazyLock<std::sync::Mutex<HashSet<String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashSet::new()));

#[derive(Debug, Clone)]
struct ActiveUsageTarget {
    active: ActiveTarget,
    target_id: String,
    label: String,
    is_remote: bool,
}

fn usage_refresh_definition() -> OperationDefinition {
    match crate::ipc_registry::command_policy("usage_refresh")
        .expect("usage_refresh must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => unreachable!("usage_refresh must have an operation policy"),
    }
}

fn usage_operation_target(target: &ActiveTarget) -> OperationTarget {
    match target {
        ActiveTarget::Local => OperationTarget::local(),
        ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
        ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
    }
}

fn usage_refresh_failure(
    definition: OperationDefinition,
    error: impl Into<IpcError>,
) -> ReviewedFailure {
    let error = error.into();
    let code = REVIEWED_IPC_ERROR_CODES
        .iter()
        .copied()
        .find(|code| *code == error.safe_code())
        .unwrap_or("internal.unexpected");
    let message = public_message_for_code(code)
        .unwrap_or("The operation failed. See runtime logs for details.");
    ReviewedFailure::new(ReviewedDiagnostic::new(
        code,
        definition.category().as_str(),
        definition.default_phase(),
        message,
        error.retryable,
    ))
}

fn usage_refresh_operation_result(result: &UsageRefreshResult, force: bool) -> SafeOperationResult {
    let (mut safe, mode) = if result.refresh_error.is_some() {
        (
            SafeOperationResult::partial("Usage refresh fell back to cached data."),
            "fallback",
        )
    } else if result.scanning {
        (
            SafeOperationResult::partial("Usage refresh continues in the background."),
            "background",
        )
    } else if result.used_cached_data {
        (
            SafeOperationResult::succeeded("Usage refresh used cached data."),
            "cached",
        )
    } else {
        (
            SafeOperationResult::succeeded("Usage refresh completed."),
            "refreshed",
        )
    };
    let scope = if result.scope.is_remote {
        "remote"
    } else {
        "local"
    };
    safe = safe
        .count(
            SafeDetailKey::AffectedCount,
            result.summary.calls_written.max(0) as u64,
        )
        .count(
            SafeDetailKey::SucceededCount,
            result.summary.providers_available.max(0) as u64,
        )
        .flag(SafeDetailKey::Changed, force)
        .stable(SafeDetailKey::Mode, mode)
        .stable(SafeDetailKey::Scope, scope);
    safe
}

async fn active_usage_target(state: &State<'_, AppState>) -> Result<ActiveUsageTarget, String> {
    let active = state
        .targets
        .active_target(&state.db)
        .await
        .map_err(|e| e.to_string())?;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRefreshResult {
    pub summary: RefreshSummary,
    pub overview: UsageOverview,
    pub recent: Vec<RecentSkillCall>,
    pub providers: Vec<ProviderHealth>,
    pub scope: UsageScopeInfo,
    pub used_cached_data: bool,
    /// 本地 target 存在过期缓存时为 true：本次返回的是缓存页，后台重扫
    /// 进行中，完成后经 `usage://scan-completed` 事件（payload = target id）
    /// 通知前端静默重取。增量字段，旧前端忽略即可。
    pub scanning: bool,
    pub refresh_error: Option<String>,
}

async fn build_refresh_page(
    state: &State<'_, AppState>,
    target_id: &str,
    summary: RefreshSummary,
    scope: UsageScopeInfo,
    used_cached_data: bool,
    scanning: bool,
    refresh_error: Option<String>,
) -> Result<UsageRefreshResult, String> {
    let overview = usage::build_overview(&state.db, target_id, None, 50)
        .await
        .map_err(|e| e.to_string())?;
    let recent = usage::list_recent_usage(&state.db, target_id, None, 20)
        .await
        .map_err(|e| e.to_string())?;
    let providers = usage::list_provider_health(&state.db, target_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(UsageRefreshResult {
        summary,
        overview,
        recent,
        providers,
        scope,
        used_cached_data,
        scanning,
        refresh_error,
    })
}

/// 本地分支是否走「缓存优先 + 后台重扫」：有历史扫描记录（缓存页可展示）、
/// 已过 5 分钟 TTL、且不是 force 强刷。首次扫描（无记录）维持阻塞。
fn should_background_rescan(force: bool, last_scan_ms: Option<i64>, now_ms: i64) -> bool {
    !force && matches!(last_scan_ms, Some(last) if now_ms - last >= usage::CACHE_TTL_MS)
}

/// 启动本地后台重扫：同一 target 同时只允许一个在途任务；完成后（无论
/// 成败）emit `usage://scan-completed`（payload = target id）。
fn spawn_background_usage_scan(app: &tauri::AppHandle, pool: crate::db::DbPool, target_id: String) {
    {
        let mut in_flight = USAGE_SCAN_IN_FLIGHT.lock().unwrap();
        if !in_flight.insert(target_id.clone()) {
            return;
        }
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = usage::refresh(&pool, &Scope::Local, true).await;
        if result.is_err() {
            tracing::warn!("background usage rescan failed");
        }
        USAGE_SCAN_IN_FLIGHT.lock().unwrap().remove(&target_id);
        use tauri::Emitter;
        let _ = app.emit("usage://scan-completed", &target_id);
    });
}

fn usage_error_to_ipc(error: usage::UsageError) -> IpcError {
    if error.is_target_fatal() {
        IpcError::new(
            error.stable_code(),
            error.public_message(),
            error.retryable(),
        )
    } else {
        IpcError::from_display(error)
    }
}

async fn usage_refresh_impl(
    app: &tauri::AppHandle,
    state: &State<'_, AppState>,
    force: bool,
    target: ActiveUsageTarget,
) -> Result<UsageRefreshResult, IpcError> {
    if target.is_remote && !force {
        if let Some(last_scan_ms) = crate::db::get_last_scan_ms(&state.db, &target.target_id)
            .await
            .map_err(|e| IpcError::from(e.to_string()))?
        {
            let now_ms = Utc::now().timestamp_millis();
            if now_ms - last_scan_ms < usage::CACHE_TTL_MS {
                return build_refresh_page(
                    state,
                    &target.target_id,
                    RefreshSummary {
                        cached: true,
                        calls_written: 0,
                        providers_available: 0,
                        scanned_at_ms: last_scan_ms,
                    },
                    scope_info_for_target(&target, None),
                    true,
                    false,
                    None,
                )
                .await
                .map_err(IpcError::from);
            }
        }
    }

    match &target.active {
        ActiveTarget::Local => {
            if !force {
                if let Some(last_scan_ms) =
                    crate::db::get_last_scan_ms(&state.db, &target.target_id)
                        .await
                        .map_err(|e| IpcError::from(e.to_string()))?
                {
                    let now_ms = Utc::now().timestamp_millis();
                    if should_background_rescan(force, Some(last_scan_ms), now_ms) {
                        // 过期缓存立即返回 + 后台重扫；完成后前端经
                        // `usage://scan-completed` 静默重取。
                        spawn_background_usage_scan(
                            app,
                            state.db.clone(),
                            target.target_id.clone(),
                        );
                        return build_refresh_page(
                            state,
                            &target.target_id,
                            RefreshSummary {
                                cached: true,
                                calls_written: 0,
                                providers_available: 0,
                                scanned_at_ms: last_scan_ms,
                            },
                            scope_info_for_target(&target, Some(false)),
                            true,
                            true,
                            None,
                        )
                        .await
                        .map_err(IpcError::from);
                    }
                }
            }
            let summary = usage::refresh(&state.db, &Scope::Local, force)
                .await
                .map_err(usage_error_to_ipc)?;
            build_refresh_page(
                state,
                &target.target_id,
                summary.clone(),
                scope_info_for_target(&target, Some(false)),
                summary.cached,
                false,
                None,
            )
            .await
            .map_err(IpcError::from)
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
                    let summary = usage::refresh(&state.db, &scope, force)
                        .await
                        .map_err(usage_error_to_ipc)?;
                    build_refresh_page(
                        state,
                        &target.target_id,
                        summary.clone(),
                        scope_info_for_target(&target, Some(true)),
                        summary.cached,
                        false,
                        None,
                    )
                    .await
                    .map_err(IpcError::from)
                }
                Err(_error) => {
                    tracing::warn!(
                        "Skill Usage: remote refresh failed; returning cached local usage data"
                    );
                    let last_scan_ms = crate::db::get_last_scan_ms(&state.db, &target.target_id)
                        .await
                        .map_err(|e| IpcError::from(e.to_string()))?;
                    build_refresh_page(
                        state,
                        &target.target_id,
                        cached_fallback_summary(last_scan_ms),
                        scope_info_for_target(&target, Some(false)),
                        last_scan_ms.is_some(),
                        false,
                        Some("Remote usage refresh failed.".to_string()),
                    )
                    .await
                    .map_err(IpcError::from)
                }
            }
        }
    }
}

/// `usage_refresh(force)` —— 触发扫描。
///
/// 本地 target 的感知延迟优化：存在过期缓存时立即返回缓存页
/// （`scanning=true`）并 spawn 后台重扫，完成后 emit
/// `usage://scan-completed`；首次扫描（无任何缓存）与 force 强刷维持阻塞。
/// Remote target 的乐观缓存路径不变。
#[tauri::command]
pub async fn usage_refresh(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    force: bool,
) -> crate::ipc_error::IpcResult<UsageRefreshResult> {
    crate::ipc_boundary!(
        "usage_refresh",
        async move {
            let target = active_usage_target(&state).await?;
            let definition = usage_refresh_definition();
            let context = OperationContext::new(usage_operation_target(&target.active));
            crate::observability::run_operation(
                &state,
                definition,
                context,
                move |result| usage_refresh_operation_result(result, force),
                || async {
                    usage_refresh_impl(&app, &state, force, target)
                        .await
                        .map_err(|error| usage_refresh_failure(definition, error))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn usage_get_overview(
    state: State<'_, AppState>,
    top_skills_limit: Option<usize>,
    source: Option<String>,
) -> crate::ipc_error::IpcResult<UsageOverview> {
    crate::ipc_boundary!(
        "usage_get_overview",
        async move {
            let target = active_usage_target(&state).await?;
            let limit = top_skills_limit.unwrap_or(0);
            usage::build_overview(&state.db, &target.target_id, source.as_deref(), limit)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn usage_get_recent(
    state: State<'_, AppState>,
    limit: Option<i64>,
    source: Option<String>,
) -> crate::ipc_error::IpcResult<Vec<RecentSkillCall>> {
    crate::ipc_boundary!(
        "usage_get_recent",
        async move {
            let n = limit.unwrap_or(20).max(1);
            let target = active_usage_target(&state).await?;
            usage::list_recent_usage(&state.db, &target.target_id, source.as_deref(), n)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn usage_get_providers(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<ProviderHealth>> {
    crate::ipc_boundary!(
        "usage_get_providers",
        async move {
            let target = active_usage_target(&state).await?;
            usage::list_provider_health(&state.db, &target.target_id)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn usage_get_skill_detail(
    state: State<'_, AppState>,
    skill: String,
    source: Option<String>,
) -> crate::ipc_error::IpcResult<SkillUsageDetail> {
    crate::ipc_boundary!(
        "usage_get_skill_detail",
        async move {
            let target = active_usage_target(&state).await?;
            usage::build_skill_detail(&state.db, &target.target_id, &skill, source.as_deref())
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

/// 给 PlatformView / CentralSkillsView 的 skill 卡片注入 "近 N 天 K 次" 徽章。
/// 返回 `{ skill_name → count }` 的 map。空请求返回空 map。
#[tauri::command]
pub async fn usage_get_skill_counts(
    state: State<'_, AppState>,
    skills: Vec<String>,
    days: u32,
) -> crate::ipc_error::IpcResult<HashMap<String, i64>> {
    crate::ipc_boundary!(
        "usage_get_skill_counts",
        async move {
            if skills.is_empty() {
                return Ok::<_, String>(HashMap::new());
            }
            let target = active_usage_target(&state).await?;
            let cutoff = (Utc::now() - chrono::Duration::days(days as i64)).timestamp_millis();
            let mut out: HashMap<String, i64> = HashMap::new();
            // 提前用 0 占位，让前端拿到完整 keyset 不用做 fallback
            for s in &skills {
                out.insert(s.clone(), 0);
            }
            for (skill, count) in
                crate::db::list_skill_counts_since(&state.db, &target.target_id, &skills, cutoff)
                    .await
                    .map_err(|e| e.to_string())?
            {
                out.insert(skill, count);
            }
            Ok(out)
        }
        .await
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillUsageStat {
    pub count: i64,
    pub last_used_ms: Option<i64>,
}

fn overlay_skill_usage_stats(
    skills: &[String],
    rows: Vec<crate::db::SkillUsageStatRow>,
) -> HashMap<String, SkillUsageStat> {
    let mut out: HashMap<String, SkillUsageStat> = HashMap::new();
    for skill in skills {
        out.insert(
            skill.clone(),
            SkillUsageStat {
                count: 0,
                last_used_ms: None,
            },
        );
    }
    for row in rows {
        out.insert(
            row.skill,
            SkillUsageStat {
                count: row.count,
                last_used_ms: row.last_used_ms,
            },
        );
    }
    out
}

/// 给 PlatformView 排序 / 名次注入全历史（或近 N 天）次数与最近使用时间。
/// 返回 `{ skill_name → { count, lastUsedMs } }`。空请求返回空 map。
/// `days = None` 表示全部已记录历史；禁止调用方把 `0` 当成全历史。
#[tauri::command]
pub async fn usage_get_skill_usage_stats(
    state: State<'_, AppState>,
    skills: Vec<String>,
    days: Option<u32>,
) -> crate::ipc_error::IpcResult<HashMap<String, SkillUsageStat>> {
    crate::ipc_boundary!(
        "usage_get_skill_usage_stats",
        async move {
            if skills.is_empty() {
                return Ok::<_, String>(HashMap::new());
            }
            let target = active_usage_target(&state).await?;
            let cutoff_ms =
                days.map(|n| (Utc::now() - chrono::Duration::days(n as i64)).timestamp_millis());
            let rows =
                crate::db::list_skill_usage_stats(&state.db, &target.target_id, &skills, cutoff_ms)
                    .await
                    .map_err(|e| e.to_string())?;
            Ok(overlay_skill_usage_stats(&skills, rows))
        }
        .await
    )
}

/// 名字匹配中央库 skill_id —— 给 SkillBarChart 点击跳 `/skill/:id`。
/// 大小写不敏感，agent 无关；优先返回 `is_central=1` 的记录。匹配不到返回 None。
#[tauri::command]
pub async fn usage_resolve_skill_id(
    state: State<'_, AppState>,
    skill_name: String,
) -> crate::ipc_error::IpcResult<Option<String>> {
    crate::ipc_boundary!(
        "usage_resolve_skill_id",
        async move {
            let target = active_usage_target(&state).await?;
            usage::resolve_skill_id(&state.db, &target.target_id, &skill_name)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

/// 从未使用 / 长期未用报表：Central 库（`skills.is_central=1`）+ 平台安装
/// （`agent_skill_observations`）两个维度。只读派生，不改 `skill_calls` 语义。
///
/// usage 事实沿用本模块既有口径——always-local 池按 `target_id` 隔离；技能库
/// 侧用 `TargetRegistry::db_for_target` 从同一个已解析 target 派生缓存池
/// （local 时与 `state.db` 相同），不引入 AppState 迁移期 ambient helper。
#[tauri::command]
pub async fn usage_get_unused_skills(
    state: State<'_, AppState>,
    source: Option<String>,
    threshold_days: Option<u32>,
) -> crate::ipc_error::IpcResult<UnusedSkillsReport> {
    crate::ipc_boundary!(
        "usage_get_unused_skills",
        async move {
            let target = active_usage_target(&state).await?;
            let skills_db = state
                .targets
                .db_for_target(&state.db, &target.active)
                .await
                .map_err(|e| e.to_string())?;
            usage::build_unused_report(
                &state.db,
                &skills_db,
                &target.target_id,
                source.as_deref(),
                threshold_days.unwrap_or(usage::DEFAULT_UNUSED_THRESHOLD_DAYS),
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
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
pub async fn usage_get_scope_info(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<UsageScopeInfo> {
    crate::ipc_boundary!(
        "usage_get_scope_info",
        async move {
            let target = active_usage_target(&state).await?;
            // 只读 getter 不做 SSH/WSL 建连；显式 refresh 返回权威 reachability。
            Ok::<_, String>(scope_info_for_target(&target, None))
        }
        .await
    )
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;
