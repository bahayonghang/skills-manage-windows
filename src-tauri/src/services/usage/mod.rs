//! Skill Usage 子系统：聚合 5 个 AI 编码工具的本地会话日志，归一为统一的
//! `SkillCall` 后落到 `skill_calls` 表，再通过 [`aggregate`] 模块派生出
//! KPI / 频次柱图 / 16 周热力图 / 最近调用 feed 所需的形状。
//!
//! 模块职责：
//! - [`mod@providers`]：8 个 provider（5 个真实实现 + 3 个 SkillPort 独有的
//!   stub）；每个 provider 实现 [`UsageProvider`] trait。
//! - [`aggregate`]：纯函数派生层，输入 `Vec<SkillCallRow>` 输出各种视图形状。
//! - [`mod`] 本身：扫描编排器 + 5 分钟缓存判定 + 落库入口（Task 4）。
//!
//! P1 阶段所有 provider 都跑在 [`Scope::Local`] 下；P2 引入
//! [`Scope::Remote`] 时会走 `fs_backend::FsBackend` trait 替换底层 IO。

pub mod aggregate;
mod enrichment;
mod error;
pub mod file_cache;
pub mod fs_backend;
mod incremental;
pub mod providers;
mod unused;

pub use enrichment::UsageSkillMatchStatus;
pub use error::{UsageError, UsageRemoteKind};
pub use file_cache::ProviderFileCache;
pub use unused::build_unused_report;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use futures_util::future::join_all;
use serde::{Deserialize, Serialize};

use crate::db::{
    self, DbPool, NewSkillCall, ProviderScanOutcome, SkillCallProviderRow, SkillCallRow,
};
use crate::services::usage::fs_backend::{FsBackend, LocalFsBackend, RemoteFsBackend};
use crate::targets::ConnectedRemoteTarget;

/// 一次 skill 调用的归一化形状。各 provider 解析完私有日志后产出这种数据。
///
/// 与 skilled `SkillCall` 完全同构。`timestamp_ms` 是 Unix epoch 毫秒，
/// 缺失时置 0。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCall {
    pub skill: String,
    pub timestamp_ms: i64,
    pub project: String,
    pub session_id: String,
    pub source: String,
}

/// 单个 provider 在一次扫描中的报表项。`available=false` 时 calls 必须为空。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderResult {
    pub provider_id: String,
    pub display_name: String,
    pub available: bool,
    pub calls: Vec<SkillCall>,
}

/// 扫描作用域。P1 只走 `Local`；P2 引入 `Remote` 后由
/// `services::usage::fs_backend::FsBackend` 切换底层 IO。
///
/// `target_id` 用于 DB 隔离；`connection` 仅 Remote 时存在，构造
/// [`RemoteFsBackend`] 用。多 provider 共享同一个连接对象（避免每个 provider
/// 都开一条 SSH 通道），故用 `Arc`。
#[derive(Clone)]
pub enum Scope {
    /// 本机扫描，使用 [`crate::paths::resolve_home_dir`] 解析 home。
    Local,
    /// 远程 target，target_id 是 `targets::TargetRegistry` 的主键。
    Remote {
        target_id: String,
        /// 远程主目录（绝对路径），由 connect_remote_target 探测得出。
        /// 替代本地 `resolve_home_dir()` 的角色。
        remote_home: String,
        connection: Arc<ConnectedRemoteTarget>,
    },
}

impl std::fmt::Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::Local => write!(f, "Scope::Local"),
            Scope::Remote {
                target_id,
                remote_home,
                ..
            } => write!(
                f,
                "Scope::Remote {{ target_id: {}, remote_home: {} }}",
                target_id, remote_home
            ),
        }
    }
}

impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Scope::Local, Scope::Local) => true,
            (Scope::Remote { target_id: a, .. }, Scope::Remote { target_id: b, .. }) => a == b,
            _ => false,
        }
    }
}

impl Eq for Scope {}

impl Scope {
    /// `target_id` 用作 `skill_calls.target_id` 列值，区分多 target 的数据。
    pub fn target_id(&self) -> String {
        match self {
            Scope::Local => "local".to_string(),
            Scope::Remote { target_id, .. } => target_id.clone(),
        }
    }

    pub fn is_remote(&self) -> bool {
        matches!(self, Scope::Remote { .. })
    }

    /// 构造合适的 FsBackend：Local 用 std::fs；Remote 共享 connection。
    pub fn fs_backend(&self) -> Arc<dyn FsBackend> {
        match self {
            Scope::Local => Arc::new(LocalFsBackend),
            Scope::Remote { connection, .. } => {
                Arc::new(RemoteFsBackend::new(Arc::clone(connection)))
            }
        }
    }

    /// 解析当前 scope 的「home 目录」。Local 用 `resolve_home_dir`，
    /// Remote 直接用记录的 `remote_home`。Provider 用此作为 path join 起点。
    pub fn home_dir(&self) -> String {
        match self {
            Scope::Local => crate::paths::resolve_home_dir()
                .to_string_lossy()
                .into_owned(),
            Scope::Remote { remote_home, .. } => remote_home.clone(),
        }
    }

    /// Join path segments using the separator semantics of the active scope.
    /// Local paths stay native (important on Windows); remote paths are POSIX.
    pub fn join_path(&self, base: &str, segments: &[&str]) -> String {
        match self {
            Scope::Local => {
                let mut path = PathBuf::from(base);
                for segment in segments {
                    path.push(segment);
                }
                path.to_string_lossy().into_owned()
            }
            Scope::Remote { .. } => join_posix_path(base, segments),
        }
    }

    pub fn join_home(&self, segments: &[&str]) -> String {
        self.join_path(&self.home_dir(), segments)
    }
}

fn join_posix_path(base: &str, segments: &[&str]) -> String {
    let mut out = base.trim_end_matches('/').to_string();
    if out.is_empty() {
        out.push('/');
    }
    for segment in segments {
        let segment = segment.trim_matches('/');
        if segment.is_empty() {
            continue;
        }
        if !out.ends_with('/') {
            out.push('/');
        }
        out.push_str(segment);
    }
    out
}

/// 一个 AI 编码工具的「会话日志解析器」。
///
/// 实现类应当是无状态的（构造一次，多次 `collect`）。`available` 与
/// `collect` 都接收 `&Scope` 是为了 P2 的 Remote 路径预留接口；P1 的实现
/// 收到 `Scope::Remote` 时直接返回空与 false。
#[async_trait]
pub trait UsageProvider: Send + Sync {
    /// 稳定的 provider id（slug 形式，例如 `"claude-code"`）。
    /// 与 `skill_call_providers.provider_id` 列对应。
    fn id(&self) -> &'static str;

    /// 用于 UI 显示的人类可读名称。与 `SkillCall::source` 字段保持一致。
    fn display_name(&self) -> &'static str;

    /// 该 scope 下数据源是否存在。例如 Claude Code 检查
    /// `~/.claude/history.jsonl` 是否存在。Stub provider 永远返回
    /// `Ok(false)`。远程 probe 失败必须上抛，不得伪装成 missing。
    async fn available(&self, scope: &Scope) -> Result<bool, UsageError>;

    /// 解析日志、返回归一化调用列表。本地可容错 parse/source failure 返回
    /// Err 让编排器记 available=false；空目录返回 `Ok(vec![])`。远程
    /// transport/protocol/permission failure 是 target-fatal，编排器在
    /// 落库前中止。
    async fn collect(&self, scope: &Scope) -> Result<Vec<SkillCall>, UsageError>;

    /// 增量收集：`cache` 是从 `skill_call_file_cache` 载入的按文件指纹缓存；
    /// 实现方按「指纹未变 → 缓存 calls，变化/新增 → 读盘解析并 record」合并出
    /// 与全量扫描完全一致的 calls，并返回更新后的缓存句柄供编排器持久化 diff。
    /// 默认实现忽略缓存、退化为全量 [`UsageProvider::collect`]（stub 与
    /// Remote scope 走这条路——后者在编排器侧就不进增量路径）。
    async fn collect_incremental(
        &self,
        scope: &Scope,
        cache: ProviderFileCache,
    ) -> Result<(ProviderFileCache, Vec<SkillCall>), UsageError> {
        let calls = self.collect(scope).await?;
        Ok((cache, calls))
    }
}

// ─── Cache & Refresh ─────────────────────────────────────────────────────────

/// 5 分钟缓存 TTL。低于这个阈值的扫描请求会被跳过，除非 force=true。
pub const CACHE_TTL_MS: i64 = 5 * 60 * 1000;

enum ProviderCollectResult {
    Ready {
        id: &'static str,
        display_name: &'static str,
        available: bool,
        calls: Vec<SkillCall>,
    },
    TargetFatal {
        provider_id: &'static str,
        error: UsageError,
    },
}

/// `usage_refresh` 的返回包，前端用于展示扫描结果摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshSummary {
    /// 是否走了缓存（true 则未实际跑 provider）
    pub cached: bool,
    /// 写入 DB 的调用条数；缓存命中时为 0
    pub calls_written: i64,
    /// 本次 available 的 provider 数；缓存命中时为 0
    pub providers_available: i64,
    /// 完整扫描时间戳（毫秒）
    pub scanned_at_ms: i64,
}

/// 编排扫描：5 分钟缓存判定 → 并发跑全部 provider → 事务原子替换落库。
///
/// `force=true` 跳过缓存。本地可容错 provider failure 不会让整个 refresh
/// 出错，只会在 `skill_call_providers` 里被标 available=false。任一
/// target-fatal 远程错误在 enrichment 与 `replace_calls_for_target` 之前
/// 返回 Err，该 target 的缓存保持刷新前状态。
pub async fn refresh(
    pool: &DbPool,
    scope: &Scope,
    force: bool,
) -> Result<RefreshSummary, UsageError> {
    refresh_with_providers(pool, scope, force, providers::all_providers()).await
}

async fn refresh_with_providers(
    pool: &DbPool,
    scope: &Scope,
    force: bool,
    providers: Vec<Box<dyn UsageProvider>>,
) -> Result<RefreshSummary, UsageError> {
    let target_id = scope.target_id();
    let now_ms = Utc::now().timestamp_millis();

    // 1) 缓存判定
    if !force {
        if let Some(last) = db::get_last_scan_ms(pool, &target_id).await? {
            if now_ms - last < CACHE_TTL_MS {
                return Ok(RefreshSummary {
                    cached: true,
                    calls_written: 0,
                    providers_available: 0,
                    scanned_at_ms: last,
                });
            }
        }
    }

    // 2) 并发跑全部 provider。Local scope 走增量扫描（skill_call_file_cache
    //    指纹 diff，未变文件零磁盘 IO）；Remote scope 保持既有全量路径。
    let local_scan = !scope.is_remote();
    let futures = providers.iter().map(|p| {
        let target_id = target_id.clone();
        async move {
            match p.available(scope).await {
                Ok(false) => {
                    // 数据源整体消失 → 仅本地清空缓存行，避免目录复活时拿陈旧指纹对错号。
                    if local_scan
                        && db::delete_file_cache_for_provider(pool, &target_id, p.id())
                            .await
                            .is_err()
                    {
                        tracing::warn!(
                            target_id = %target_id,
                            provider = p.id(),
                            "usage file cache cleanup failed"
                        );
                    }
                    return ProviderCollectResult::Ready {
                        id: p.id(),
                        display_name: p.display_name(),
                        available: false,
                        calls: Vec::new(),
                    };
                }
                Err(error) if error.is_target_fatal() => {
                    tracing::warn!(
                        target_id = %target_id,
                        provider = p.id(),
                        code = error.stable_code(),
                        retryable = error.retryable(),
                        "usage provider availability failed"
                    );
                    return ProviderCollectResult::TargetFatal {
                        provider_id: p.id(),
                        error,
                    };
                }
                Err(error) => {
                    tracing::warn!(
                        target_id = %target_id,
                        provider = p.id(),
                        code = error.stable_code(),
                        retryable = error.retryable(),
                        "usage provider availability failed"
                    );
                    return ProviderCollectResult::Ready {
                        id: p.id(),
                        display_name: p.display_name(),
                        available: false,
                        calls: Vec::new(),
                    };
                }
                Ok(true) => {}
            }
            let collected = if local_scan {
                incremental::collect_local_incremental(pool, &target_id, p.as_ref(), scope).await
            } else {
                p.collect(scope).await
            };
            match collected {
                Ok(calls) => ProviderCollectResult::Ready {
                    id: p.id(),
                    display_name: p.display_name(),
                    available: true,
                    calls,
                },
                Err(error) if error.is_target_fatal() => {
                    tracing::warn!(
                        target_id = %target_id,
                        provider = p.id(),
                        code = error.stable_code(),
                        retryable = error.retryable(),
                        "usage provider collect failed"
                    );
                    ProviderCollectResult::TargetFatal {
                        provider_id: p.id(),
                        error,
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        target_id = %target_id,
                        provider = p.id(),
                        code = error.stable_code(),
                        retryable = error.retryable(),
                        "usage provider collect failed"
                    );
                    ProviderCollectResult::Ready {
                        id: p.id(),
                        display_name: p.display_name(),
                        available: false,
                        calls: Vec::new(),
                    }
                }
            }
        }
    });
    let outcomes = join_all(futures).await;
    let mut ready = Vec::with_capacity(outcomes.len());
    let mut fatal = None;
    for outcome in outcomes {
        match outcome {
            ProviderCollectResult::TargetFatal { provider_id, error } => {
                if fatal.is_none() {
                    tracing::warn!(
                        target_id = %target_id,
                        provider = provider_id,
                        code = error.stable_code(),
                        retryable = error.retryable(),
                        "usage refresh aborted"
                    );
                    fatal = Some(error);
                }
            }
            ready_outcome @ ProviderCollectResult::Ready { .. } => ready.push(ready_outcome),
        }
    }
    if let Some(error) = fatal {
        return Err(error);
    }

    // 3) 平展为落库形态。此处已确认没有 target-fatal remote error。
    let mut all_calls: Vec<NewSkillCall> = Vec::new();
    let mut provider_outcomes: Vec<ProviderScanOutcome> = Vec::with_capacity(ready.len());
    let mut providers_available = 0i64;
    for outcome in ready {
        let ProviderCollectResult::Ready {
            id,
            display_name,
            available,
            calls,
        } = outcome
        else {
            unreachable!("target-fatal outcomes already returned");
        };
        if available {
            providers_available += 1;
        }
        provider_outcomes.push(ProviderScanOutcome {
            provider_id: id.to_string(),
            display_name: display_name.to_string(),
            available,
            call_count: calls.len() as i64,
        });
        for c in calls {
            all_calls.push(NewSkillCall {
                skill: c.skill,
                timestamp_ms: c.timestamp_ms,
                project: c.project,
                session_id: c.session_id,
                source: c.source,
            });
        }
    }

    let calls_written = all_calls.len() as i64;
    let scan_completed_ms = Utc::now().timestamp_millis();

    let mut skill_names = all_calls
        .iter()
        .map(|call| call.skill.clone())
        .collect::<Vec<_>>();
    skill_names.sort();
    skill_names.dedup();
    let normalized_names = skill_names
        .iter()
        .map(|name| name.trim().to_lowercase())
        .collect::<Vec<_>>();
    let candidates = db::list_usage_skill_candidates(pool, &normalized_names).await?;
    let resolved = enrichment::resolve_usage_skills(&skill_names, &candidates);
    let mut paths = resolved
        .iter()
        .filter_map(|item| item.file_path.clone())
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    let content_by_path = if paths.is_empty() {
        Default::default()
    } else {
        match scope.fs_backend().read_many_to_strings(&paths).await {
            Ok(content) => content,
            Err(_error) => {
                tracing::warn!("usage Skill.md enrichment read failed");
                Default::default()
            }
        }
    };
    let metadata = enrichment::build_usage_metadata(
        &resolved,
        &content_by_path,
        crate::services::resource_budget::ResourceBudget::default_skill(),
    );

    // 4) 事务原子替换
    db::replace_calls_for_target(
        pool,
        &target_id,
        &all_calls,
        &provider_outcomes,
        &metadata,
        scan_completed_ms,
    )
    .await?;

    Ok(RefreshSummary {
        cached: false,
        calls_written,
        providers_available,
        scanned_at_ms: scan_completed_ms,
    })
}

/// 给上层（命令层）一个统一入口构建 [`aggregate::UsageOverview`]：
/// 读 `skill_calls` + `skill_call_providers` + `skill_call_scan_state`。
pub async fn build_overview(
    pool: &DbPool,
    target_id: &str,
    source: Option<&str>,
    top_skills_limit: usize,
) -> Result<aggregate::UsageOverview, UsageError> {
    let kpis_row = db::get_usage_kpis(pool, target_id, source).await?;
    let top_skill_rows = db::list_top_skills(pool, target_id, source, top_skills_limit).await?;
    let metadata = db::list_usage_metadata(pool, target_id)
        .await?
        .into_iter()
        .map(|item| (item.skill.clone(), item))
        .collect::<HashMap<_, _>>();
    let now_ms = Utc::now().timestamp_millis();
    let cutoff_ms = now_ms - (16 * 7 + 1) * 86_400_000;
    let timestamps = db::list_timestamps_since(pool, target_id, source, cutoff_ms).await?;
    let last_scan_ms = db::get_last_scan_ms(pool, target_id).await?;

    Ok(aggregate::UsageOverview {
        kpis: aggregate::UsageKpis {
            total_calls: kpis_row.total_calls,
            unique_skills: kpis_row.unique_skills,
            unique_projects: kpis_row.unique_projects,
            unique_sources: kpis_row.unique_sources,
            unique_sessions: kpis_row.unique_sessions,
        },
        top_skills: top_skill_rows
            .into_iter()
            .map(|row| {
                let identity = metadata.get(&row.skill);
                aggregate::SkillUsageSummary {
                    skill: row.skill,
                    count: row.count,
                    projects: row.projects,
                    sessions: row.sessions,
                    last_used_ms: row.last_used_ms,
                    match_status: identity
                        .map(|item| UsageSkillMatchStatus::from_db(&item.match_status))
                        .unwrap_or(UsageSkillMatchStatus::Unmatched),
                    resolved_skill_id: identity.and_then(|item| item.resolved_skill_id.clone()),
                    static_token_estimate: identity.and_then(|item| item.static_token_estimate),
                    static_byte_count: identity.and_then(|item| item.static_byte_count),
                }
            })
            .collect(),
        heatmap: aggregate::heatmap_grid_16w_from_timestamps(
            &timestamps,
            now_ms,
            &aggregate::SystemLocalDayResolver,
        ),
        last_scan_ms,
    })
}

pub async fn list_recent_usage(
    pool: &DbPool,
    target_id: &str,
    source: Option<&str>,
    limit: i64,
) -> Result<Vec<aggregate::RecentSkillCall>, UsageError> {
    let rows = db::list_recent_calls(pool, target_id, source, limit).await?;
    let metadata = db::list_usage_metadata(pool, target_id)
        .await?
        .into_iter()
        .map(|item| (item.skill.clone(), item))
        .collect::<HashMap<_, _>>();

    Ok(rows
        .into_iter()
        .map(|row| {
            let identity = metadata.get(&row.skill);
            aggregate::RecentSkillCall {
                skill: row.skill,
                timestamp_ms: row.timestamp_ms,
                project: row.project,
                session_id: row.session_id,
                source: row.source,
                match_status: identity
                    .map(|item| UsageSkillMatchStatus::from_db(&item.match_status))
                    .unwrap_or(UsageSkillMatchStatus::Unmatched),
                resolved_skill_id: identity.and_then(|item| item.resolved_skill_id.clone()),
            }
        })
        .collect())
}

pub async fn build_skill_detail(
    pool: &DbPool,
    target_id: &str,
    skill: &str,
    source: Option<&str>,
) -> Result<aggregate::SkillUsageDetail, UsageError> {
    let summary = db::get_skill_detail_summary(pool, target_id, skill, source)
        .await?
        .unwrap_or_default();
    let identity = db::get_usage_metadata_for_skill(pool, target_id, skill).await?;
    let match_status = identity
        .as_ref()
        .map(|item| UsageSkillMatchStatus::from_db(&item.match_status))
        .unwrap_or(UsageSkillMatchStatus::Unmatched);

    if summary.count == 0 {
        return Ok(aggregate::SkillUsageDetail {
            skill: skill.to_string(),
            count: 0,
            sessions: 0,
            first_used_ms: 0,
            last_used_ms: 0,
            by_project: Vec::new(),
            weekly: Vec::new(),
            match_status,
            resolved_skill_id: identity
                .as_ref()
                .and_then(|item| item.resolved_skill_id.clone()),
            static_token_estimate: identity
                .as_ref()
                .and_then(|item| item.static_token_estimate),
            static_byte_count: identity.as_ref().and_then(|item| item.static_byte_count),
        });
    }

    let by_project = db::list_skill_project_counts(pool, target_id, skill, source)
        .await?
        .into_iter()
        .map(|row| aggregate::SkillProjectCount {
            project: row.project,
            count: row.count,
            sessions: row.sessions,
            last_used_ms: row.last_used_ms,
        })
        .collect();
    let now_ms = Utc::now().timestamp_millis();
    let cutoff_ms = now_ms - (16 * 7 + 1) * 86_400_000;
    let timestamps =
        db::list_skill_timestamps_since(pool, target_id, skill, source, cutoff_ms).await?;

    Ok(aggregate::SkillUsageDetail {
        skill: skill.to_string(),
        count: summary.count,
        sessions: summary.sessions,
        first_used_ms: summary.first_used_ms,
        last_used_ms: summary.last_used_ms,
        by_project,
        weekly: aggregate::heatmap_grid_16w_from_timestamps(
            &timestamps,
            now_ms,
            &aggregate::SystemLocalDayResolver,
        ),
        match_status,
        resolved_skill_id: identity
            .as_ref()
            .and_then(|item| item.resolved_skill_id.clone()),
        static_token_estimate: identity
            .as_ref()
            .and_then(|item| item.static_token_estimate),
        static_byte_count: identity.as_ref().and_then(|item| item.static_byte_count),
    })
}

pub async fn resolve_skill_id(
    pool: &DbPool,
    target_id: &str,
    skill_name: &str,
) -> Result<Option<String>, UsageError> {
    let trimmed = skill_name.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if let Some(metadata) = db::get_usage_metadata_for_skill(pool, target_id, skill_name).await? {
        return Ok(metadata.resolved_skill_id);
    }

    let names = vec![trimmed.to_lowercase()];
    let candidates = db::list_usage_skill_candidates(pool, &names).await?;
    let requested = vec![skill_name.to_string()];
    Ok(enrichment::resolve_usage_skills(&requested, &candidates)
        .into_iter()
        .next()
        .and_then(|item| item.resolved_skill_id))
}

/// `usage_get_unused_skills` 的默认「长期未用」阈值（天）。
pub const DEFAULT_UNUSED_THRESHOLD_DAYS: u32 = 90;

/// 把 DB 行投影成可序列化给前端的 [`SkillCall`] 列表。
pub fn rows_to_skill_calls(rows: Vec<SkillCallRow>) -> Vec<SkillCall> {
    rows.into_iter()
        .map(|r| SkillCall {
            skill: r.skill,
            timestamp_ms: r.timestamp_ms,
            project: r.project,
            session_id: r.session_id,
            source: r.source,
        })
        .collect()
}

/// `skill_call_providers` 行 + 是否最近扫描过的派生标记。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderHealth {
    pub provider_id: String,
    pub display_name: String,
    pub available: bool,
    pub call_count: i64,
    pub scanned_at_ms: i64,
}

impl From<SkillCallProviderRow> for ProviderHealth {
    fn from(r: SkillCallProviderRow) -> Self {
        Self {
            provider_id: r.provider_id,
            display_name: r.display_name,
            available: r.available,
            call_count: r.call_count,
            scanned_at_ms: r.scanned_at,
        }
    }
}

pub async fn list_provider_health(
    pool: &DbPool,
    target_id: &str,
) -> Result<Vec<ProviderHealth>, UsageError> {
    let rows = db::list_provider_rows(pool, target_id).await?;
    Ok(rows.into_iter().map(ProviderHealth::from).collect())
}

#[cfg(test)]
mod bench;
#[cfg(test)]
mod tests;

/// 跨 provider 测试共享的 env mutex —— `HOME` / `USERPROFILE` /
/// `CLAUDE_CONFIG_DIR` / `CODEX_HOME` 都是进程级全局，cargo test 默认
/// 并行跑会让任何一个 provider 测试看到其他 provider 测试设的值。
/// 所有改动这些环境变量的测试必须先 lock 这把锁。
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
