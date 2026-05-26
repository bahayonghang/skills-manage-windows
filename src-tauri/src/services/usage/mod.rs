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
pub mod fs_backend;
pub mod providers;

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
    async fn collect(&self, scope: &Scope) -> Result<Vec<SkillCall>, String>;
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
pub async fn refresh(pool: &DbPool, scope: &Scope, force: bool) -> Result<RefreshSummary, String> {
    refresh_with_providers(pool, scope, force, providers::all_providers()).await
}

async fn refresh_with_providers(
    pool: &DbPool,
    scope: &Scope,
    force: bool,
    providers: Vec<Box<dyn UsageProvider>>,
) -> Result<RefreshSummary, String> {
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

    // 4) 事务原子替换
    db::replace_calls_for_target(
        pool,
        &target_id,
        &all_calls,
        &provider_outcomes,
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
    top_skills_limit: usize,
) -> Result<aggregate::UsageOverview, String> {
    let kpis_row = db::get_usage_kpis(pool, target_id).await?;
    let top_skill_rows = db::list_top_skills(pool, target_id, top_skills_limit).await?;
    let cutoff_ms = Utc::now().timestamp_millis() - (16 * 7 * 86_400_000);
    let day_rows = db::list_daily_counts_since(pool, target_id, cutoff_ms).await?;
    let last_scan_ms = db::get_last_scan_ms(pool, target_id).await?;

    Ok(aggregate::UsageOverview {
        kpis: aggregate::UsageKpis {
            total_calls: kpis_row.total_calls,
            unique_skills: kpis_row.unique_skills,
            unique_projects: kpis_row.unique_projects,
            unique_sources: kpis_row.unique_sources,
        },
        top_skills: top_skill_rows
            .into_iter()
            .map(|row| aggregate::SkillCount {
                skill: row.skill,
                count: row.count,
                projects: row.projects,
                sessions: row.sessions,
                last_used_ms: row.last_used_ms,
            })
            .collect(),
        heatmap: aggregate::heatmap_grid_16w_from_daily_counts(
            &day_rows
                .into_iter()
                .map(|row| aggregate::DayCount {
                    date: row.date,
                    count: row.count,
                })
                .collect::<Vec<_>>(),
            Utc::now().timestamp_millis(),
        ),
        last_scan_ms,
    })
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
) -> Result<Vec<ProviderHealth>, String> {
    let rows = db::list_provider_rows(pool, target_id).await?;
    Ok(rows.into_iter().map(ProviderHealth::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_call_serializes_to_camel_case() {
        let call = SkillCall {
            skill: "review".to_string(),
            timestamp_ms: 1_700_000_000_000,
            project: "/tmp/x".to_string(),
            session_id: "s1".to_string(),
            source: "Claude Code".to_string(),
        };
        let json = serde_json::to_string(&call).unwrap();
        // 关键字段必须是 camelCase 才能直接喂给前端 ts 类型 SkillCall
        assert!(json.contains("\"timestampMs\""));
        assert!(json.contains("\"sessionId\""));
        // 不应该出现 snake_case 残留
        assert!(!json.contains("\"timestamp_ms\""));
        assert!(!json.contains("\"session_id\""));
    }

    #[test]
    fn scope_target_id_branches() {
        assert_eq!(Scope::Local.target_id(), "local");
        // Scope::Remote 的 target_id 在 refresh_tests / 远程集成测试中验证；
        // 这里只能拿 Local，因为构造 Remote 需要一条真实 ConnectedRemoteTarget。
    }

    #[test]
    fn join_posix_path_preserves_remote_home_root() {
        assert_eq!(
            super::join_posix_path("/home/alice/", &[".codex", "sessions"]),
            "/home/alice/.codex/sessions"
        );
        assert_eq!(super::join_posix_path("/", &[".claude"]), "/.claude");
    }
}

/// 跨 provider 测试共享的 env mutex —— `HOME` / `USERPROFILE` /
/// `CLAUDE_CONFIG_DIR` / `CODEX_HOME` 都是进程级全局，cargo test 默认
/// 并行跑会让任何一个 provider 测试看到其他 provider 测试设的值。
/// 所有改动这些环境变量的测试必须先 lock 这把锁。
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod refresh_tests {
    use super::*;
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    struct FailingProvider;

    #[async_trait::async_trait]
    impl UsageProvider for FailingProvider {
        fn id(&self) -> &'static str {
            "failing"
        }

        fn display_name(&self) -> &'static str {
            "Failing Provider"
        }

        async fn available(&self, _scope: &Scope) -> bool {
            true
        }

        async fn collect(&self, _scope: &Scope) -> Result<Vec<SkillCall>, String> {
            Err("fixture failure".to_string())
        }
    }

    async fn setup_pool() -> DbPool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        db::init_database(&pool).await.unwrap();
        pool
    }

    fn write_claude_fixture(dir: &TempDir) {
        let history = r#"{"display":"/review","project":"/p1","sessionId":"s1","timestamp":1700000000000}
{"display":"/facts","project":"/p2","sessionId":"s2","timestamp":1700000010000}"#;
        std::fs::write(dir.path().join("history.jsonl"), history).unwrap();
    }

    #[tokio::test]
    async fn refresh_scans_then_caches_within_ttl_then_force_rescans() {
        let _g = ENV_LOCK.lock().unwrap();
        let pool = setup_pool().await;

        let dir = TempDir::new().unwrap();
        write_claude_fixture(&dir);
        std::env::set_var("CLAUDE_CONFIG_DIR", dir.path());

        // 第一次：实际跑扫描
        let s1 = refresh(&pool, &Scope::Local, false).await.unwrap();
        assert!(!s1.cached);
        assert!(
            s1.calls_written >= 2,
            "should have at least 2 calls from claude fixture"
        );
        assert!(
            s1.providers_available >= 1,
            "claude code provider should be available with the fixture"
        );
        let first_scan_ms = s1.scanned_at_ms;

        // 第二次：5min 内 → 缓存命中，不写库
        let s2 = refresh(&pool, &Scope::Local, false).await.unwrap();
        assert!(s2.cached, "second refresh within TTL must be cached");
        assert_eq!(s2.calls_written, 0);
        assert_eq!(s2.scanned_at_ms, first_scan_ms);

        // 第三次 force=true：跳过缓存，重新跑
        let s3 = refresh(&pool, &Scope::Local, true).await.unwrap();
        assert!(!s3.cached, "force=true must skip cache");
        assert!(s3.calls_written >= 2);

        // overview 拉得到数据
        let overview = build_overview(&pool, "local", 50).await.unwrap();
        assert!(overview.kpis.total_calls >= 2);
        assert!(overview.heatmap.len() == 16 * 7);
        assert!(overview.last_scan_ms.is_some());

        // provider health 列表能拉到全部 8 项（含 stub）
        let health = list_provider_health(&pool, "local").await.unwrap();
        assert_eq!(health.len(), 8, "all 8 providers should be recorded");

        std::env::remove_var("CLAUDE_CONFIG_DIR");
    }

    #[tokio::test]
    async fn refresh_marks_available_provider_unavailable_when_collect_fails() {
        let pool = setup_pool().await;

        let summary =
            refresh_with_providers(&pool, &Scope::Local, true, vec![Box::new(FailingProvider)])
                .await
                .unwrap();

        assert!(!summary.cached);
        assert_eq!(summary.calls_written, 0);
        assert_eq!(
            summary.providers_available, 0,
            "collect errors should not count as available providers"
        );

        let health = list_provider_health(&pool, "local").await.unwrap();
        assert_eq!(health.len(), 1);
        assert_eq!(health[0].provider_id, "failing");
        assert!(!health[0].available);
        assert_eq!(health[0].call_count, 0);
    }
}
