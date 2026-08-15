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
pub mod fs_backend;
pub mod providers;

pub use enrichment::UsageSkillMatchStatus;
pub use error::UsageError;

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
    /// `~/.claude/history.jsonl` 是否存在。Stub provider 永远返回 false。
    async fn available(&self, scope: &Scope) -> bool;

    /// 解析日志、返回归一化调用列表。失败时返回 Err 让编排器记录到
    /// 健康表的 `available=false` 但不影响其他 provider；空目录返回
    /// `Ok(vec![])` 而非 Err。
    async fn collect(&self, scope: &Scope) -> Result<Vec<SkillCall>, UsageError>;
}

// ─── Cache & Refresh ─────────────────────────────────────────────────────────

/// 5 分钟缓存 TTL。低于这个阈值的扫描请求会被跳过，除非 force=true。
pub const CACHE_TTL_MS: i64 = 5 * 60 * 1000;

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
/// `force=true` 跳过缓存。失败的 provider 不会让整个 refresh 出错，只会
/// 在 `skill_call_providers` 里被标 available=false。
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

    // 2) 并发跑全部 provider
    let futures = providers.iter().map(|p| async move {
        let avail = p.available(scope).await;
        if !avail {
            return (p.id(), p.display_name(), false, Vec::new());
        }
        match p.collect(scope).await {
            Ok(calls) => (p.id(), p.display_name(), true, calls),
            Err(error) => {
                tracing::warn!(
                    provider = p.id(),
                    error = %error,
                    "usage provider collect failed"
                );
                (p.id(), p.display_name(), false, Vec::new())
            }
        }
    });
    let outcomes = join_all(futures).await;

    // 3) 平展为落库形态
    let mut all_calls: Vec<NewSkillCall> = Vec::new();
    let mut provider_outcomes: Vec<ProviderScanOutcome> = Vec::with_capacity(outcomes.len());
    let mut providers_available = 0i64;
    for (id, name, available, calls) in outcomes {
        if available {
            providers_available += 1;
        }
        provider_outcomes.push(ProviderScanOutcome {
            provider_id: id.to_string(),
            display_name: name.to_string(),
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
            Err(error) => {
                tracing::warn!(error = %error, "usage Skill.md enrichment read failed");
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

/// 构建「从未使用 / 长期未用」报表（只读派生，不写任何表）。
///
/// 双 pool 约定：
/// - `usage_pool`：`skill_calls` / `skill_usage_metadata` 所在池，按
///   `target_id` 隔离（远程 target 的 usage 缓存也落在这里，见
///   `commands::usage` 的既有口径）；`source` 过滤只作用于 calls 聚合。
/// - `skills_pool`：active target 的技能库池（`skills` /
///   `agent_skill_observations` / `skill_installations`）。local target 下
///   两池相同；远程 target 下是 `TargetRegistry::db_for_target` 解析出的
///   缓存池。
///
/// Central 维度按 `skill_usage_metadata.resolved_skill_id` 归属调用；平台维度
/// 以 `agent_skill_observations` 为权威安装表（每次 agent 扫描落盘、含平台
/// 散件与插件源），按 normalized name（`enrichment::normalize_identity` 同一
/// 规则）直查 `skill_calls`。
pub async fn build_unused_report(
    usage_pool: &DbPool,
    skills_pool: &DbPool,
    target_id: &str,
    source: Option<&str>,
    threshold_days: u32,
) -> Result<aggregate::UnusedSkillsReport, UsageError> {
    let now_ms = Utc::now().timestamp_millis();

    // ── Central 维度：skills(is_central=1) × 按 resolved_skill_id 聚合的调用事实
    let central_skills = db::get_central_skills(skills_pool).await?;
    let resolved_aggregates: HashMap<String, db::ResolvedCallAggregateRow> =
        db::list_resolved_call_aggregates(usage_pool, target_id, source)
            .await?
            .into_iter()
            .map(|row| (row.resolved_skill_id.clone(), row))
            .collect();
    let central_ids = central_skills
        .iter()
        .map(|skill| skill.id.clone())
        .collect::<Vec<_>>();
    let mut installations_by_skill =
        db::get_skill_installations_for_skills(skills_pool, &central_ids).await?;

    let mut central = Vec::new();
    for skill in central_skills {
        let stats = resolved_aggregates.get(&skill.id);
        let call_count = stats.map(|row| row.call_count).unwrap_or(0);
        let last_used_ms = stats.and_then(|row| row.last_used_ms);
        let Some(status) =
            aggregate::unused_skill_status(call_count, last_used_ms, now_ms, threshold_days)
        else {
            continue;
        };
        let mut agents = installations_by_skill
            .remove(&skill.id)
            .unwrap_or_default()
            .into_iter()
            .map(|installation| installation.agent_id)
            .collect::<Vec<_>>();
        agents.sort();
        agents.dedup();
        central.push(aggregate::UnusedSkillEntry {
            skill_id: Some(skill.id.clone()),
            name: skill.name.clone(),
            // 有归属聚合行 ⟺ 存在 matched metadata 行（只有 matched 才有 resolved id）
            match_status: if stats.is_some() {
                UsageSkillMatchStatus::Matched
            } else {
                UsageSkillMatchStatus::Unmatched
            },
            origin: aggregate::UnusedSkillOrigin::Central,
            agents,
            installed_path: skill.canonical_path.clone(),
            call_count,
            last_used_ms,
            static_token_estimate: stats.and_then(|row| row.static_token_estimate),
            static_byte_count: stats.and_then(|row| row.static_byte_count),
            status,
        });
    }

    // ── 平台维度：agent_skill_observations 按 normalized name 直查 skill_calls
    let observations = db::list_platform_skill_observations(skills_pool).await?;
    let call_aggregates: HashMap<String, db::NormalizedCallAggregateRow> =
        db::list_normalized_call_aggregates(usage_pool, target_id, source)
            .await?
            .into_iter()
            .map(|row| (row.normalized_skill.clone(), row))
            .collect();
    // metadata 以日志里的原始调用名为键；平台名 normalize 后对齐取身份与静态体量。
    // 同一 normalized 名可能有多个原始名变体，matched 行身份最强，优先保留。
    let mut metadata_by_normalized: HashMap<String, db::SkillUsageMetadataRow> = HashMap::new();
    for row in db::list_usage_metadata(usage_pool, target_id).await? {
        let key = enrichment::normalize_identity(&row.skill);
        let replace = metadata_by_normalized
            .get(&key)
            .map(|existing| row.match_status == "matched" && existing.match_status != "matched")
            .unwrap_or(true);
        if replace {
            metadata_by_normalized.insert(key, row);
        }
    }

    struct PlatformGroup {
        name: String,
        dir_path: String,
        agents: Vec<String>,
    }
    let mut groups: HashMap<String, PlatformGroup> = HashMap::new();
    for observation in observations {
        let key = enrichment::normalize_identity(&observation.name);
        let group = groups.entry(key).or_insert_with(|| PlatformGroup {
            name: observation.name.clone(),
            dir_path: observation.dir_path.clone(),
            agents: Vec::new(),
        });
        if !group.agents.contains(&observation.agent_id) {
            group.agents.push(observation.agent_id.clone());
        }
    }

    let mut platforms = Vec::new();
    for (normalized, group) in groups {
        let stats = call_aggregates.get(&normalized);
        let call_count = stats.map(|row| row.call_count).unwrap_or(0);
        let last_used_ms = stats.and_then(|row| row.last_used_ms);
        let Some(status) =
            aggregate::unused_skill_status(call_count, last_used_ms, now_ms, threshold_days)
        else {
            continue;
        };
        let metadata = metadata_by_normalized.get(&normalized);
        platforms.push(aggregate::UnusedSkillEntry {
            skill_id: metadata.and_then(|row| row.resolved_skill_id.clone()),
            name: group.name,
            match_status: metadata
                .map(|row| UsageSkillMatchStatus::from_db(&row.match_status))
                .unwrap_or(UsageSkillMatchStatus::Unmatched),
            origin: aggregate::UnusedSkillOrigin::Platform,
            agents: group.agents,
            installed_path: Some(group.dir_path),
            call_count,
            last_used_ms,
            static_token_estimate: metadata.and_then(|row| row.static_token_estimate),
            static_byte_count: metadata.and_then(|row| row.static_byte_count),
            status,
        });
    }

    // 输出排序固定（名称升序），阈值/排序/筛选的交互全部留给前端视图层。
    let by_name = |a: &aggregate::UnusedSkillEntry, b: &aggregate::UnusedSkillEntry| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    };
    central.sort_by(by_name);
    platforms.sort_by(by_name);

    Ok(aggregate::UnusedSkillsReport { central, platforms })
}

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
mod tests;

/// 跨 provider 测试共享的 env mutex —— `HOME` / `USERPROFILE` /
/// `CLAUDE_CONFIG_DIR` / `CODEX_HOME` 都是进程级全局，cargo test 默认
/// 并行跑会让任何一个 provider 测试看到其他 provider 测试设的值。
/// 所有改动这些环境变量的测试必须先 lock 这把锁。
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
