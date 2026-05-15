//! Database types — extracted from `db/legacy.rs` per task_plan.md Phase 2 / T2.1.
//!
//! 仅承载数据结构与公共常量定义，不包含任何业务逻辑或 SQL。所有 struct 通过
//! `pub use types::*;` 在 `crate::db` 顶层重导出，下游引用路径保持 `crate::db::Foo`
//! 不变。
//!
//! 演进：后续散落的 `SkillForAgent` / `DiscoveredSkillRow` / `DiscoveredSkillInsert`
//! 等业务子类型在 Phase 2c 抽 repos/* 时一并迁入。

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

/// Connection pool alias —— 整个 crate 内统一以 `DbPool` 引用 SQLite 池。
pub type DbPool = SqlitePool;

/// 通用平台清单：cross-agent 扫描 / 默认安装目标对照表。
pub const UNIVERSAL_AGENT_IDS: [&str; 12] = [
    "amp",
    "antigravity",
    "cline",
    "codex",
    "cursor",
    "deep-agents",
    "firebender",
    "gemini-cli",
    "copilot",
    "kimi-code-cli",
    "opencode",
    "warp",
];

/// Universal agents share one project-level skills directory.
pub const UNIVERSAL_PROJECT_SKILLS_DIR: &str = ".agents/skills";

/// Preferred raw agent id used when a project-level Universal directory is
/// stored in `project_skill_installations`.
pub const UNIVERSAL_PROJECT_REPRESENTATIVE_AGENT_IDS: [&str; 5] =
    ["codex", "opencode", "gemini-cli", "cursor", "amp"];

/// 占位仓库 ID：`local-unknown` 表示来源未知的本地技能。
pub const LOCAL_UNKNOWN_REPOSITORY_ID: &str = "local-unknown";

/// 占位标签 ID：`uncategorized` 表示尚未归类的技能。
pub const UNCATEGORIZED_TAG_ID: &str = "uncategorized";

// ─── Skill / Installation / Observation ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    pub canonical_path: Option<String>,
    pub is_central: bool,
    pub source: Option<String>,
    pub content: Option<String>,
    pub scanned_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillInstallation {
    pub skill_id: String,
    pub agent_id: String,
    pub installed_path: String,
    pub link_type: String,
    pub symlink_target: Option<String>,
    /// ISO 8601 timestamp of when the skill was first installed.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentSkillObservation {
    pub row_id: String,
    pub agent_id: String,
    pub skill_id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    pub dir_path: String,
    pub source_kind: String,
    pub source_root: String,
    pub link_type: String,
    pub symlink_target: Option<String>,
    pub is_read_only: bool,
    pub scanned_at: String,
}

// ─── Agent ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Agent {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub global_skills_dir: String,
    pub project_skills_dir: Option<String>,
    pub icon_name: Option<String>,
    pub is_detected: bool,
    pub is_builtin: bool,
    pub is_enabled: bool,
}

// ─── Collection ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ─── Repository ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillRepository {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub url: Option<String>,
    pub is_unknown: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRepositoryWithStats {
    #[serde(flatten)]
    pub repository: SkillRepository,
    pub skill_count: i64,
    pub unknown_skill_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRepositoryAssignment {
    pub repository: SkillRepository,
    pub source_path: Option<String>,
    pub is_source_unknown: bool,
}

// ─── Update State ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillUpdateState {
    pub skill_id: String,
    pub source_type: String,
    pub source_url: Option<String>,
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    pub source_path: Option<String>,
    pub last_remote_hash: Option<String>,
    pub latest_remote_hash: Option<String>,
    pub last_checked_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub status: String,
    pub error: Option<String>,
}

// ─── Tags ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillTag {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
    /// 标签所属分组的 id。M3 加入；旧 db 升级时通过 ensure_column 自动加列。
    #[serde(default)]
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TagGroup {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i64,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillTagLink {
    pub skill_id: String,
    pub tag_id: String,
    pub confidence: Option<f64>,
    pub reason: Option<String>,
    pub source: String,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAiTagReview {
    pub skill_id: String,
    pub skill_name: String,
    pub tag: SkillTag,
    pub confidence: f64,
    pub reason: String,
    pub suggested_at: String,
    pub updated_at: String,
}

// ─── Scan Directory ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScanDirectory {
    pub id: i64,
    pub path: String,
    pub label: Option<String>,
    pub is_active: bool,
    pub is_builtin: bool,
    pub added_at: String,
}

// ─── Operation Log ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogEntry {
    pub id: String,
    pub created_at: String,
    pub level: String,
    pub target_kind: String,
    pub target_id: String,
    pub target_label: Option<String>,
    pub category: String,
    pub action: String,
    pub status: String,
    pub subject_type: Option<String>,
    pub subject_id: Option<String>,
    pub subject_label: Option<String>,
    pub summary: String,
    pub error_summary: Option<String>,
    pub details_json: Option<String>,
    pub duration_ms: Option<i64>,
    pub batch_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogFilter {
    pub query: Option<String>,
    pub target_kind: Option<String>,
    pub target_id: Option<String>,
    pub level: Option<String>,
    pub status: Option<String>,
    pub category: Option<String>,
    pub action: Option<String>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogPage {
    pub entries: Vec<OperationLogEntry>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// ─── Saved Views (Central Skills V2 / M2) ────────────────────────────────────

/// 一个 saved view 行。`query` 字段是前端 `CentralViewState` 的 JSON，后端只做
/// 透传，不参与解析。`sort_order` 控制 sidebar 渲染顺序，越小越靠前。
/// 字段命名沿用项目其它表的 snake_case（与 Collection 一致），前端 contract 同步。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SavedView {
    pub id: String,
    pub name: String,
    pub query: String,
    pub sort_order: i64,
    pub icon: Option<String>,
    pub pinned: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewOperationLogEntry {
    pub level: String,
    pub target_kind: String,
    pub target_id: String,
    pub target_label: Option<String>,
    pub category: String,
    pub action: String,
    pub status: String,
    pub subject_type: Option<String>,
    pub subject_id: Option<String>,
    pub subject_label: Option<String>,
    pub summary: String,
    pub error_summary: Option<String>,
    pub details_json: Option<String>,
    pub duration_ms: Option<i64>,
    pub batch_id: Option<String>,
}

// ─── Projects (Stage 1 - 项目级 skill 管理) ───────────────────────────────────

/// 用户手动 add 的项目。id = sha256(规范化 path) 前 16 字符。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: String,
    pub path: String,
    pub name: String,
    pub pinned: bool,
    pub added_at: String,
    pub last_scanned_at: Option<String>,
}

/// 项目下某个 agent 目录中登记的 skill 安装。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectSkillInstallation {
    pub project_id: String,
    pub skill_id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    /// `'central'` | `'project'`：中央库安装或项目原有/手动放入。
    pub source_origin: String,
    pub agent_id: String,
    pub installed_path: String,
    /// `'symlink'` | `'copy'`。
    pub link_type: String,
    pub symlink_target: Option<String>,
    pub created_at: String,
}
