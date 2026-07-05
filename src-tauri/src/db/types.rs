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

/// Global Universal Agents targets share the user-level `~/.agents/skills` directory.
///
/// Google-branded global targets are intentionally not part of this set:
/// Antigravity uses `~/.gemini/antigravity/skills`, Antigravity CLI uses
/// `~/.gemini/antigravity-cli/skills`, and legacy Gemini CLI carries the
/// shared Google `~/.gemini/skills` target.
pub const UNIVERSAL_AGENT_IDS: [&str; 10] = [
    "amp",
    "cline",
    "codex",
    "cursor",
    "deep-agents",
    "firebender",
    "copilot",
    "kimi-code-cli",
    "opencode",
    "warp",
];

/// Universal agents share one project-level skills directory.
pub const UNIVERSAL_PROJECT_SKILLS_DIR: &str = crate::paths::UNIVERSAL_SKILLS_REL;

/// Workspace-level Universal Agents targets share `<workspace>/.agents/skills`.
pub const UNIVERSAL_PROJECT_AGENT_IDS: [&str; 13] = [
    "amp",
    "antigravity",
    "antigravity-cli",
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

/// Preferred raw agent id used when a project-level Universal directory is
/// stored in `project_skill_installations`.
pub const UNIVERSAL_PROJECT_REPRESENTATIVE_AGENT_IDS: [&str; 7] = [
    "codex",
    "opencode",
    "antigravity-cli",
    "antigravity",
    "gemini-cli",
    "cursor",
    "amp",
];

/// 占位仓库 ID：`local-unknown` 表示来源未知的本地技能。
pub const LOCAL_UNKNOWN_REPOSITORY_ID: &str = "local-unknown";

/// 占位标签 ID：`uncategorized` 表示尚未归类的技能。
pub const UNCATEGORIZED_TAG_ID: &str = "uncategorized";

/// 唯一保留的普通内置标签 ID。
pub const ACADEMIC_RESEARCH_WRITING_TAG_ID: &str = "academic-research-writing";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    Native,
    Symlink,
    Copy,
    Writable,
}

impl LinkType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Symlink => "symlink",
            Self::Copy => "copy",
            Self::Writable => "writable",
        }
    }
}

impl std::fmt::Display for LinkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for LinkType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "native" => Self::Native,
            "symlink" => Self::Symlink,
            "copy" => Self::Copy,
            "writable" => Self::Writable,
            other => {
                return Err(format!(
                    "Unsupported link_type '{other}'. Expected one of: native, symlink, copy, writable."
                ))
            }
        })
    }
}

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
    pub fs_created_at: Option<String>,
    pub fs_updated_at: Option<String>,
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
    pub fs_created_at: Option<String>,
    pub fs_updated_at: Option<String>,
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
    pub pinned: bool,
    pub is_unknown: bool,
    pub created_at: String,
    pub updated_at: String,
    /// repo 级最后一次 inventory refresh 的时间戳（ISO-8601）。Phase P2 引入。
    /// 旧 DB 升级时通过 ensure_column 安全加列，默认为 NULL。
    #[serde(default)]
    pub last_synced_at: Option<String>,
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

#[derive(Debug, Clone)]
pub struct SkillRepositoryMember {
    pub skill_id: String,
    pub source_path: Option<String>,
    pub repository: SkillRepository,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillRepositorySyncSkip {
    pub repository_id: String,
    pub source_path: String,
    pub skill_id: String,
    pub skill_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_seen_at: String,
}

/// Pending remote addition discovered during inventory refresh — Phase P2.
///
/// 由 `refresh_skill_update_inventory` 写入，关闭"更新中心"也不丢；apply 阶段
/// 走 import / skip / unskip 任意分支后由对应分支删掉对应行。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillRepositoryPendingAddition {
    pub repository_id: String,
    pub source_path: String,
    pub skill_id: String,
    pub skill_name: String,
    pub conflict_existing_skill_id: Option<String>,
    pub discovered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillUpdateInventoryRun {
    pub inventory_id: String,
    pub scope_kind: String,
    pub mode: String,
    pub skill_ids_json: Option<String>,
    pub repository_ids_json: Option<String>,
    pub agent_ids_json: Option<String>,
    pub cache_policy: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillUpdateInventoryEntry {
    pub inventory_id: String,
    pub bucket: String,
    pub entity_key: String,
    pub skill_id: Option<String>,
    pub skill_name: Option<String>,
    pub repository_id: Option<String>,
    pub source_type: Option<String>,
    pub source_url: Option<String>,
    pub ref_name: Option<String>,
    pub source_path: Option<String>,
    pub agent_id: Option<String>,
    pub local_hash: Option<String>,
    pub baseline_hash: Option<String>,
    pub remote_hash: Option<String>,
    pub local_version: Option<String>,
    pub remote_version: Option<String>,
    pub cache_policy: String,
    pub cache_hit: bool,
    pub snapshot_fetched_at: Option<String>,
    pub generated_at: String,
    pub payload_json: String,
    pub error: Option<String>,
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
