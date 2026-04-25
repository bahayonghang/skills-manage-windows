use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    FromRow, Row, SqlitePool,
};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

pub type DbPool = SqlitePool;

const DEFAULT_ENABLED_PLATFORM_IDS: [&str; 5] =
    ["claude-code", "codex", "gemini-cli", "opencode", "kiro"];

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

pub const LOCAL_UNKNOWN_REPOSITORY_ID: &str = "local-unknown";
pub const UNCATEGORIZED_TAG_ID: &str = "uncategorized";

// ─── Data Types ──────────────────────────────────────────────────────────────

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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillTag {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScanDirectory {
    pub id: i64,
    pub path: String,
    pub label: Option<String>,
    pub is_active: bool,
    pub is_builtin: bool,
    pub added_at: String,
}

// ─── Pool Creation ────────────────────────────────────────────────────────────

/// Create a production SQLite pool for the given file path with WAL mode enabled.
pub async fn create_pool(db_path: &str) -> Result<DbPool, String> {
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path))
        .map_err(|e| e.to_string())?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
        .map_err(|e| e.to_string())
}

// ─── Init ─────────────────────────────────────────────────────────────────────

/// Initialize all database tables (idempotent) and seed built-in agents.
pub async fn init_database(pool: &DbPool) -> Result<(), String> {
    // Enable WAL mode (no-op for in-memory databases)
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // skills table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skills (
            id             TEXT PRIMARY KEY,
            name           TEXT NOT NULL,
            description    TEXT,
            file_path      TEXT NOT NULL,
            canonical_path TEXT,
            is_central     BOOLEAN NOT NULL DEFAULT 0,
            source         TEXT,
            content        TEXT,
            scanned_at     TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // skill_installations table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_installations (
            skill_id       TEXT NOT NULL,
            agent_id       TEXT NOT NULL,
            installed_path TEXT NOT NULL,
            link_type      TEXT NOT NULL,
            symlink_target TEXT,
            created_at     TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (skill_id, agent_id)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_installations_agent_id
         ON skill_installations(agent_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS agent_skill_observations (
            row_id         TEXT PRIMARY KEY,
            agent_id       TEXT NOT NULL,
            skill_id       TEXT NOT NULL,
            name           TEXT NOT NULL,
            description    TEXT,
            file_path      TEXT NOT NULL,
            dir_path       TEXT NOT NULL,
            source_kind    TEXT NOT NULL,
            source_root    TEXT NOT NULL,
            link_type      TEXT NOT NULL,
            symlink_target TEXT,
            is_read_only   BOOLEAN NOT NULL DEFAULT 0,
            scanned_at     TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Migration: add created_at column to skill_installations for existing databases
    // that were created before this column was introduced. We check via PRAGMA table_info
    // and run a two-step migration only when the column is absent:
    //   1. ALTER TABLE ADD COLUMN created_at TEXT  (nullable, no default expression –
    //      SQLite's ALTER TABLE does not support non-constant default expressions on
    //      some builds, e.g. the Apple-modified SQLite on macOS).
    //   2. UPDATE … SET created_at = datetime('now') WHERE created_at IS NULL  –
    //      backfills existing rows with the current timestamp.
    // New rows written by the application always supply created_at explicitly, so
    // there is no need for a database-level DEFAULT after the migration runs.
    let columns = sqlx::query("PRAGMA table_info(skill_installations)")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let has_created_at = columns.iter().any(|row| {
        row.try_get::<String, _>("name")
            .map(|name| name == "created_at")
            .unwrap_or(false)
    });

    if !has_created_at {
        // Step 1: add the column (nullable, no expression default for compatibility).
        sqlx::query("ALTER TABLE skill_installations ADD COLUMN created_at TEXT")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

        // Step 2: backfill existing rows with the current timestamp.
        sqlx::query(
            "UPDATE skill_installations SET created_at = datetime('now') WHERE created_at IS NULL",
        )
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    // agents table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS agents (
            id                 TEXT PRIMARY KEY,
            display_name       TEXT NOT NULL,
            category           TEXT NOT NULL,
            global_skills_dir  TEXT NOT NULL,
            project_skills_dir TEXT,
            icon_name          TEXT,
            is_detected        BOOLEAN NOT NULL DEFAULT 0,
            is_builtin         BOOLEAN NOT NULL DEFAULT 1,
            is_enabled         BOOLEAN NOT NULL DEFAULT 1
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skills_is_central
         ON skills(is_central)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // collections table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collections (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            description TEXT,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // collection_skills table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collection_skills (
            collection_id TEXT NOT NULL,
            skill_id      TEXT NOT NULL,
            added_at      TEXT NOT NULL,
            PRIMARY KEY (collection_id, skill_id)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // skill_repositories table — local metadata for grouping Central skills by source repo.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_repositories (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            source_type TEXT NOT NULL,
            owner       TEXT,
            repo        TEXT,
            branch      TEXT,
            url         TEXT,
            is_unknown  BOOLEAN NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_repository_members (
            skill_id      TEXT PRIMARY KEY,
            repository_id TEXT NOT NULL,
            source_path   TEXT,
            added_at      TEXT NOT NULL,
            updated_at    TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_repository_members_repository_id
         ON skill_repository_members(repository_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_update_states (
            skill_id           TEXT PRIMARY KEY,
            source_type        TEXT NOT NULL,
            source_url         TEXT,
            ref_name           TEXT,
            source_path        TEXT,
            last_remote_hash   TEXT,
            latest_remote_hash TEXT,
            last_checked_at    TEXT,
            last_updated_at    TEXT,
            status             TEXT NOT NULL,
            error              TEXT
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_update_states_status
         ON skill_update_states(status)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // skill_tags table — local category taxonomy separate from user Collections.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_tags (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE,
            description TEXT,
            color       TEXT,
            is_builtin  BOOLEAN NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_tag_links (
            skill_id    TEXT NOT NULL,
            tag_id      TEXT NOT NULL,
            confidence  REAL,
            reason      TEXT,
            source      TEXT NOT NULL DEFAULT 'manual',
            added_at    TEXT NOT NULL,
            PRIMARY KEY (skill_id, tag_id)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_tag_links_tag_id
         ON skill_tag_links(tag_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_ai_tag_reviews (
            skill_id     TEXT NOT NULL,
            tag_id       TEXT NOT NULL,
            confidence   REAL NOT NULL,
            reason       TEXT,
            status       TEXT NOT NULL DEFAULT 'pending',
            suggested_at TEXT NOT NULL,
            updated_at   TEXT NOT NULL,
            PRIMARY KEY (skill_id, tag_id)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_ai_tag_reviews_status
         ON skill_ai_tag_reviews(status)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // scan_directories table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS scan_directories (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            path       TEXT NOT NULL UNIQUE,
            label      TEXT,
            is_active  BOOLEAN NOT NULL DEFAULT 1,
            is_builtin BOOLEAN NOT NULL DEFAULT 0,
            added_at   TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // settings table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // discovered_skills table — skills found in project-level directories
    // during a "discover project skills" scan.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS discovered_skills (
            id             TEXT PRIMARY KEY,
            name           TEXT NOT NULL,
            description    TEXT,
            file_path      TEXT NOT NULL,
            dir_path       TEXT NOT NULL,
            project_path   TEXT NOT NULL,
            project_name   TEXT NOT NULL,
            platform_id    TEXT NOT NULL,
            discovered_at  TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_discovered_skills_project_path
         ON discovered_skills(project_path)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_discovered_skills_platform_id
         ON discovered_skills(platform_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // skill_registries table — remote skill sources (marketplace)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_registries (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            source_type TEXT NOT NULL,
            url         TEXT NOT NULL,
            is_builtin  BOOLEAN NOT NULL DEFAULT 0,
            is_enabled  BOOLEAN NOT NULL DEFAULT 1,
            last_synced TEXT,
            last_attempted_sync TEXT,
            last_sync_status TEXT NOT NULL DEFAULT 'never',
            last_sync_error TEXT,
            cache_updated_at TEXT,
            cache_expires_at TEXT,
            etag TEXT,
            last_modified TEXT,
            created_at  TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // marketplace_skills table — cached remote skill metadata
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS marketplace_skills (
            id           TEXT PRIMARY KEY,
            registry_id  TEXT NOT NULL,
            name         TEXT NOT NULL,
            description  TEXT,
            download_url TEXT NOT NULL,
            is_installed BOOLEAN NOT NULL DEFAULT 0,
            synced_at    TEXT NOT NULL,
            cache_updated_at TEXT,
            FOREIGN KEY (registry_id) REFERENCES skill_registries(id)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // skill_explanations table — cached AI-generated skill explanations
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_explanations (
            skill_id    TEXT NOT NULL,
            explanation TEXT NOT NULL,
            lang        TEXT NOT NULL DEFAULT 'zh',
            model       TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (skill_id, lang)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    ensure_column(
        pool,
        "skill_registries",
        "last_attempted_sync",
        "ALTER TABLE skill_registries ADD COLUMN last_attempted_sync TEXT",
    )
    .await?;
    ensure_column(
        pool,
        "skill_registries",
        "last_sync_status",
        "ALTER TABLE skill_registries ADD COLUMN last_sync_status TEXT NOT NULL DEFAULT 'never'",
    )
    .await?;
    ensure_column(
        pool,
        "skill_registries",
        "last_sync_error",
        "ALTER TABLE skill_registries ADD COLUMN last_sync_error TEXT",
    )
    .await?;
    ensure_column(
        pool,
        "skill_registries",
        "cache_updated_at",
        "ALTER TABLE skill_registries ADD COLUMN cache_updated_at TEXT",
    )
    .await?;
    ensure_column(
        pool,
        "skill_registries",
        "cache_expires_at",
        "ALTER TABLE skill_registries ADD COLUMN cache_expires_at TEXT",
    )
    .await?;
    ensure_column(
        pool,
        "skill_registries",
        "etag",
        "ALTER TABLE skill_registries ADD COLUMN etag TEXT",
    )
    .await?;
    ensure_column(
        pool,
        "skill_registries",
        "last_modified",
        "ALTER TABLE skill_registries ADD COLUMN last_modified TEXT",
    )
    .await?;
    ensure_column(
        pool,
        "marketplace_skills",
        "cache_updated_at",
        "ALTER TABLE marketplace_skills ADD COLUMN cache_updated_at TEXT",
    )
    .await?;

    // Seed built-in agents (INSERT OR IGNORE so repeated init is safe)
    seed_builtin_agents(pool).await?;

    // Seed built-in scan directories from the built-in agent registry.
    seed_builtin_scan_directories(pool).await?;

    // Seed built-in skill registries (marketplace sources)
    seed_builtin_registries(pool).await?;

    seed_builtin_skill_metadata(pool).await?;

    Ok(())
}

async fn seed_builtin_agents(pool: &DbPool) -> Result<(), String> {
    let agents = builtin_agents();
    let builtin_ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();

    for agent in &agents {
        sqlx::query(
            "INSERT INTO agents
             (id, display_name, category, global_skills_dir, project_skills_dir,
              icon_name, is_detected, is_builtin, is_enabled)
             VALUES (?, ?, ?, ?, ?, ?, 0, 1, ?)
             ON CONFLICT(id) DO UPDATE SET
              display_name = excluded.display_name,
              category = excluded.category,
              global_skills_dir = excluded.global_skills_dir,
              project_skills_dir = excluded.project_skills_dir,
              icon_name = excluded.icon_name",
        )
        .bind(&agent.id)
        .bind(&agent.display_name)
        .bind(&agent.category)
        .bind(&agent.global_skills_dir)
        .bind(&agent.project_skills_dir)
        .bind(&agent.icon_name)
        .bind(agent.is_enabled)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    // Remove builtin agents that no longer exist in code
    let all_db_agents: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM agents WHERE is_builtin = 1")
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;

    for (id,) in &all_db_agents {
        if !builtin_ids.contains(&id.as_str()) {
            sqlx::query("DELETE FROM agents WHERE id = ? AND is_builtin = 1")
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// Seed `scan_directories` with one row per unique `global_skills_dir` path
/// across all built-in agents. Rows are marked `is_builtin = 1` and cannot
/// be removed by the user. `INSERT OR IGNORE` keeps the operation idempotent:
/// Universal agents share `~/.agents/skills`, while Central uses the private
/// `~/.skillsmanage/skills` store.
async fn seed_builtin_scan_directories(pool: &DbPool) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    for agent in builtin_agents() {
        sqlx::query(
            "INSERT OR IGNORE INTO scan_directories
             (path, label, is_active, is_builtin, added_at)
             VALUES (?, ?, 1, 1, ?)",
        )
        .bind(&agent.global_skills_dir)
        .bind(&agent.display_name)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    // Remove builtin scan directories that no longer exist in code
    let builtin_paths: std::collections::HashSet<String> = builtin_agents()
        .into_iter()
        .map(|a| a.global_skills_dir)
        .collect();
    let all_db_dirs: Vec<(String,)> =
        sqlx::query_as("SELECT path FROM scan_directories WHERE is_builtin = 1")
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
    for (path,) in &all_db_dirs {
        if !builtin_paths.contains(path) {
            sqlx::query("DELETE FROM scan_directories WHERE path = ? AND is_builtin = 1")
                .bind(path)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

async fn seed_builtin_registries(pool: &DbPool) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let registries = vec![
        (
            "anthropic",
            "Anthropic",
            "github",
            "https://github.com/anthropics/skills",
        ),
        (
            "openai",
            "OpenAI",
            "github",
            "https://github.com/openai/skills",
        ),
        (
            "baoyu-skills",
            "baoyu-skills",
            "github",
            "https://github.com/jimliu/baoyu-skills",
        ),
    ];
    for (id, name, source_type, url) in registries {
        sqlx::query(
            "INSERT OR IGNORE INTO skill_registries
             (id, name, source_type, url, is_builtin, is_enabled, created_at)
             VALUES (?, ?, ?, ?, 1, 1, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(source_type)
        .bind(url)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn seed_builtin_skill_metadata(pool: &DbPool) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO skill_repositories
         (id, name, source_type, owner, repo, branch, url, is_unknown, created_at, updated_at)
         VALUES (?, ?, 'local', NULL, NULL, NULL, NULL, 1, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           name = excluded.name,
           source_type = excluded.source_type,
           is_unknown = excluded.is_unknown,
           updated_at = excluded.updated_at",
    )
    .bind(LOCAL_UNKNOWN_REPOSITORY_ID)
    .bind("本地 / 未知来源")
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    for (id, name, description, color) in builtin_skill_tags() {
        sqlx::query(
            "INSERT INTO skill_tags
             (id, name, description, color, is_builtin, created_at, updated_at)
             VALUES (?, ?, ?, ?, 1, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               description = excluded.description,
               color = excluded.color,
               is_builtin = excluded.is_builtin,
               updated_at = excluded.updated_at",
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(color)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn builtin_skill_tags() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        (
            "programming-agent-engineering",
            "编程与 Agent 工程",
            "Coding agents, developer tools, automation, and agent engineering skills.",
            "#7c3aed",
        ),
        (
            "frontend-visual-design",
            "前端与视觉设计",
            "Frontend, UI, UX, visual generation, and interaction design skills.",
            "#2563eb",
        ),
        (
            "academic-research-writing",
            "学术研究与写作",
            "Paper search, academic writing, slides, and research workflows.",
            "#0891b2",
        ),
        (
            "data-analysis-finance",
            "数据分析与金融",
            "Data analysis, quantitative finance, markets, and reporting skills.",
            "#059669",
        ),
        (
            "biomed-research-databases",
            "生物医药与科研数据库",
            "Biomedical, chemistry, omics, and scientific database skills.",
            "#16a34a",
        ),
        (
            "docs-office-knowledge",
            "文档办公与知识管理",
            "Documents, office workflows, notes, and knowledge management skills.",
            "#f59e0b",
        ),
        (
            "business-bid-policy",
            "业务/投标/政策",
            "Business writing, bids, policy briefs, and enterprise workflows.",
            "#dc2626",
        ),
        (
            "ops-security-release",
            "运行维护/安全/发布",
            "Ops, security, release, CI, and production maintenance skills.",
            "#475569",
        ),
        (
            UNCATEGORIZED_TAG_ID,
            "未分类",
            "Fallback category for skills that still need review.",
            "#71717a",
        ),
    ]
}

async fn ensure_column(
    pool: &DbPool,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<(), String> {
    let pragma = format!("PRAGMA table_info({table})");
    let rows = sqlx::query(&pragma)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let has_column = rows.iter().any(|row| {
        use sqlx::Row;
        row.get::<String, _>("name") == column
    });

    if !has_column {
        sqlx::query(alter_sql)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Returns the list of built-in agents using the current user's home directory.
pub fn builtin_agents() -> Vec<Agent> {
    builtin_agents_for_home(&crate::paths::resolve_home_dir())
}

fn is_builtin_agent_enabled_by_default(agent_id: &str, category: &str) -> bool {
    if category == "central" {
        return true;
    }

    if category == "lobster" {
        return false;
    }

    DEFAULT_ENABLED_PLATFORM_IDS.contains(&agent_id)
}

pub fn is_universal_agent(agent_id: &str) -> bool {
    UNIVERSAL_AGENT_IDS.contains(&agent_id)
}

fn builtin_agents_for_home(home: &Path) -> Vec<Agent> {
    let central_skills_dir = crate::paths::central_skills_dir_from_home(home)
        .to_string_lossy()
        .into_owned();
    let universal_skills_dir = crate::paths::universal_skills_dir_from_home(home)
        .to_string_lossy()
        .into_owned();

    let skill_dir = |segments: &[&str]| -> String {
        segments
            .iter()
            .fold(home.to_path_buf(), |path, segment| path.join(segment))
            .to_string_lossy()
            .into_owned()
    };

    let agent_skill_dir = |agent_id: &str, segments: &[&str]| -> String {
        if is_universal_agent(agent_id) {
            return universal_skills_dir.clone();
        }

        skill_dir(segments)
    };

    vec![
        // ── Coding platforms ─────────────────────────────────────────────────
        Agent {
            id: "claude-code".to_string(),
            display_name: "Claude Code".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".claude", "skills"]),
            project_skills_dir: None,
            icon_name: Some("claude".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("claude-code", "coding"),
        },
        Agent {
            id: "codex".to_string(),
            display_name: "Codex CLI".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("codex", &[".codex", "skills"]),
            project_skills_dir: None,
            icon_name: Some("codex".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("codex", "coding"),
        },
        Agent {
            id: "cursor".to_string(),
            display_name: "Cursor".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("cursor", &[".cursor", "skills"]),
            project_skills_dir: None,
            icon_name: Some("cursor".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("cursor", "coding"),
        },
        Agent {
            id: "gemini-cli".to_string(),
            display_name: "Gemini CLI".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("gemini-cli", &[".gemini", "skills"]),
            project_skills_dir: None,
            icon_name: Some("gemini".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("gemini-cli", "coding"),
        },
        Agent {
            id: "trae".to_string(),
            display_name: "Trae".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".trae", "skills"]),
            project_skills_dir: None,
            icon_name: Some("trae".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("trae", "coding"),
        },
        Agent {
            id: "factory-droid".to_string(),
            display_name: "Factory Droid".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".factory", "skills"]),
            project_skills_dir: None,
            icon_name: Some("factory".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("factory-droid", "coding"),
        },
        Agent {
            id: "junie".to_string(),
            display_name: "Junie".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".junie", "skills"]),
            project_skills_dir: None,
            icon_name: Some("junie".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("junie", "coding"),
        },
        Agent {
            id: "qwen".to_string(),
            display_name: "Qwen".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".qwen", "skills"]),
            project_skills_dir: None,
            icon_name: Some("qwen".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("qwen", "coding"),
        },
        Agent {
            id: "trae-cn".to_string(),
            display_name: "Trae CN".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".trae-cn", "skills"]),
            project_skills_dir: None,
            icon_name: Some("trae-cn".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("trae-cn", "coding"),
        },
        Agent {
            id: "windsurf".to_string(),
            display_name: "Windsurf".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".windsurf", "skills"]),
            project_skills_dir: None,
            icon_name: Some("windsurf".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("windsurf", "coding"),
        },
        Agent {
            id: "qoder".to_string(),
            display_name: "Qoder".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".qoder", "skills"]),
            project_skills_dir: None,
            icon_name: Some("qoder".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("qoder", "coding"),
        },
        Agent {
            id: "augment".to_string(),
            display_name: "Augment".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".augment", "skills"]),
            project_skills_dir: None,
            icon_name: Some("augment".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("augment", "coding"),
        },
        Agent {
            id: "opencode".to_string(),
            display_name: "OpenCode".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("opencode", &[".opencode", "skills"]),
            project_skills_dir: None,
            icon_name: Some("opencode".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("opencode", "coding"),
        },
        Agent {
            id: "kilocode".to_string(),
            display_name: "KiloCode".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".kilocode", "skills"]),
            project_skills_dir: None,
            icon_name: Some("kilocode".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("kilocode", "coding"),
        },
        Agent {
            id: "ob1".to_string(),
            display_name: "OB1".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".ob1", "skills"]),
            project_skills_dir: None,
            icon_name: Some("ob1".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("ob1", "coding"),
        },
        Agent {
            id: "amp".to_string(),
            display_name: "Amp".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("amp", &[".amp", "skills"]),
            project_skills_dir: None,
            icon_name: Some("amp".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("amp", "coding"),
        },
        Agent {
            id: "kiro".to_string(),
            display_name: "Kiro".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".kiro", "skills"]),
            project_skills_dir: None,
            icon_name: Some("kiro".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("kiro", "coding"),
        },
        Agent {
            id: "codebuddy".to_string(),
            display_name: "CodeBuddy".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".codebuddy", "skills"]),
            project_skills_dir: None,
            icon_name: Some("codebuddy".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("codebuddy", "coding"),
        },
        Agent {
            id: "hermes".to_string(),
            display_name: "Hermes".to_string(),
            category: "lobster".to_string(),
            global_skills_dir: skill_dir(&[".hermes", "skills"]),
            project_skills_dir: None,
            icon_name: Some("hermes".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("hermes", "lobster"),
        },
        Agent {
            id: "copilot".to_string(),
            display_name: "GitHub Copilot".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("copilot", &[".copilot", "skills"]),
            project_skills_dir: None,
            icon_name: Some("copilot".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("copilot", "coding"),
        },
        Agent {
            id: "antigravity".to_string(),
            display_name: "Antigravity".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("antigravity", &[".antigravity", "skills"]),
            project_skills_dir: None,
            icon_name: Some("antigravity".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("antigravity", "coding"),
        },
        Agent {
            id: "cline".to_string(),
            display_name: "Cline".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("cline", &[".cline", "skills"]),
            project_skills_dir: None,
            icon_name: Some("cline".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("cline", "coding"),
        },
        Agent {
            id: "deep-agents".to_string(),
            display_name: "Deep Agents".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("deep-agents", &[".deep-agents", "skills"]),
            project_skills_dir: None,
            icon_name: Some("deep-agents".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("deep-agents", "coding"),
        },
        Agent {
            id: "firebender".to_string(),
            display_name: "Firebender".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("firebender", &[".firebender", "skills"]),
            project_skills_dir: None,
            icon_name: Some("firebender".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("firebender", "coding"),
        },
        Agent {
            id: "kimi-code-cli".to_string(),
            display_name: "Kimi Code CLI".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("kimi-code-cli", &[".kimi-code", "skills"]),
            project_skills_dir: None,
            icon_name: Some("kimi-code".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("kimi-code-cli", "coding"),
        },
        Agent {
            id: "warp".to_string(),
            display_name: "Warp".to_string(),
            category: "coding".to_string(),
            global_skills_dir: agent_skill_dir("warp", &[".warp", "skills"]),
            project_skills_dir: None,
            icon_name: Some("warp".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("warp", "coding"),
        },
        Agent {
            id: "aider".to_string(),
            display_name: "Aider".to_string(),
            category: "coding".to_string(),
            global_skills_dir: skill_dir(&[".aider", "skills"]),
            project_skills_dir: None,
            icon_name: Some("aider".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("aider", "coding"),
        },
        // ── Lobster platforms ────────────────────────────────────────────────
        Agent {
            id: "openclaw".to_string(),
            display_name: "OpenClaw".to_string(),
            category: "lobster".to_string(),
            global_skills_dir: skill_dir(&[".openclaw", "skills"]),
            project_skills_dir: None,
            icon_name: Some("openclaw".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("openclaw", "lobster"),
        },
        Agent {
            id: "qclaw".to_string(),
            display_name: "QClaw".to_string(),
            category: "lobster".to_string(),
            global_skills_dir: skill_dir(&[".qclaw", "skills"]),
            project_skills_dir: None,
            icon_name: Some("qclaw".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("qclaw", "lobster"),
        },
        Agent {
            id: "easyclaw".to_string(),
            display_name: "EasyClaw".to_string(),
            category: "lobster".to_string(),
            global_skills_dir: skill_dir(&[".easyclaw", "skills"]),
            project_skills_dir: None,
            icon_name: Some("easyclaw".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("easyclaw", "lobster"),
        },
        Agent {
            id: "autoclaw".to_string(),
            display_name: "AutoClaw".to_string(),
            category: "lobster".to_string(),
            global_skills_dir: skill_dir(&[".openclaw-autoclaw", "skills"]),
            project_skills_dir: None,
            icon_name: Some("autoclaw".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("autoclaw", "lobster"),
        },
        Agent {
            id: "workbuddy".to_string(),
            display_name: "WorkBuddy".to_string(),
            category: "lobster".to_string(),
            global_skills_dir: skill_dir(&[".workbuddy", "skills-marketplace", "skills"]),
            project_skills_dir: None,
            icon_name: Some("workbuddy".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("workbuddy", "lobster"),
        },
        // ── Central Skills ────────────────────────────────────────────────────
        Agent {
            id: "central".to_string(),
            display_name: "Central Skills".to_string(),
            category: "central".to_string(),
            global_skills_dir: central_skills_dir.clone(),
            project_skills_dir: None,
            icon_name: Some("central".to_string()),
            is_detected: false,
            is_builtin: true,
            is_enabled: is_builtin_agent_enabled_by_default("central", "central"),
        },
    ]
}

// ─── Skills ───────────────────────────────────────────────────────────────────

/// Insert or update a skill record.
///
/// Uses `ON CONFLICT DO UPDATE` to preserve the private Central record if a
/// platform scan later observes the same skill id in an agent directory.
/// Once a skill is flagged as central it must never be downgraded to non-central
/// or have its canonical file path overwritten by a platform copy.
pub async fn upsert_skill(pool: &DbPool, skill: &Skill) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO skills
         (id, name, description, file_path, canonical_path, is_central, source, content, scanned_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           name           = CASE
                              WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.name
                              ELSE excluded.name
                            END,
           description    = CASE
                              WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.description
                              ELSE excluded.description
                            END,
           file_path      = CASE
                              WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.file_path
                              ELSE excluded.file_path
                            END,
           canonical_path = CASE
                              WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.canonical_path
                              ELSE COALESCE(excluded.canonical_path, skills.canonical_path)
                            END,
           is_central     = MAX(skills.is_central, excluded.is_central),
           source         = CASE
                              WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.source
                              ELSE excluded.source
                            END,
           content        = CASE
                              WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.content
                              ELSE excluded.content
                            END,
           scanned_at     = excluded.scanned_at",
    )
    .bind(&skill.id)
    .bind(&skill.name)
    .bind(&skill.description)
    .bind(&skill.file_path)
    .bind(&skill.canonical_path)
    .bind(skill.is_central)
    .bind(&skill.source)
    .bind(&skill.content)
    .bind(&skill.scanned_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn observation_to_skill(observation: AgentSkillObservation) -> Skill {
    Skill {
        id: observation.skill_id,
        name: observation.name,
        description: observation.description,
        file_path: observation.file_path,
        canonical_path: None,
        is_central: false,
        source: Some(observation.link_type),
        content: None,
        scanned_at: observation.scanned_at,
    }
}

/// Retrieve all skills installed for a given agent.
pub async fn get_skills_by_agent(pool: &DbPool, agent_id: &str) -> Result<Vec<Skill>, String> {
    if agent_id == "claude-code" {
        let observations = get_agent_skill_observations(pool, agent_id).await?;
        if !observations.is_empty() {
            return Ok(observations.into_iter().map(observation_to_skill).collect());
        }
    }

    sqlx::query_as::<_, Skill>(
        "SELECT s.* FROM skills s
         JOIN skill_installations si ON s.id = si.skill_id
         WHERE si.agent_id = ?",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

/// A skill enriched with the installation-specific fields for a given agent.
///
/// Returned by `get_skills_for_agent`. The extra fields come from the
/// `skill_installations` row and allow the frontend `SkillCard` to display
/// the correct source indicator without a second round-trip.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillForAgent {
    pub id: String,
    /// Stable row identity for source-specific detail routing.
    pub row_id: String,
    pub name: String,
    pub description: Option<String>,
    /// Absolute path to the `SKILL.md` file.
    pub file_path: String,
    /// Absolute path to the skill directory as installed for this agent
    /// (i.e., `skill_installations.installed_path`).
    pub dir_path: String,
    /// How the skill is linked: "symlink", "copy", or "native".
    pub link_type: String,
    /// Symlink target path, if `link_type` is "symlink".
    pub symlink_target: Option<String>,
    pub is_central: bool,
    pub source_kind: Option<String>,
    pub source_root: Option<String>,
    pub is_read_only: bool,
    pub conflict_group: Option<String>,
    pub conflict_count: i64,
}

/// Retrieve skills installed for a given agent, enriched with installation
/// metadata (`dir_path`, `link_type`, `symlink_target`) required by the
/// platform-view skill cards.
pub async fn get_skills_for_agent(
    pool: &DbPool,
    agent_id: &str,
) -> Result<Vec<SkillForAgent>, String> {
    if agent_id == "claude-code" {
        let observations = get_agent_skill_observations(pool, agent_id).await?;
        if !observations.is_empty() {
            let mut conflict_counts = HashMap::new();
            for observation in &observations {
                *conflict_counts
                    .entry(observation.skill_id.clone())
                    .or_insert(0_i64) += 1;
            }

            return Ok(observations
                .into_iter()
                .map(|observation| {
                    let conflict_count = conflict_counts
                        .get(&observation.skill_id)
                        .copied()
                        .unwrap_or(0);
                    let mut skill = observation_to_skill_for_agent(observation);
                    if conflict_count > 1 {
                        skill.conflict_group = Some(claude_conflict_group(agent_id, &skill.id));
                        skill.conflict_count = conflict_count;
                    }
                    skill
                })
                .collect());
        }
    }

    sqlx::query_as::<_, SkillForAgent>(
        "SELECT s.id,
                s.id AS row_id,
                s.name,
                s.description,
                s.file_path,
                si.installed_path AS dir_path,
                si.link_type,
                si.symlink_target,
                s.is_central,
                NULL AS source_kind,
                NULL AS source_root,
                0 AS is_read_only,
                NULL AS conflict_group,
                0 AS conflict_count
         FROM skills s
         JOIN skill_installations si ON s.id = si.skill_id
         WHERE si.agent_id = ?",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

fn observation_to_skill_for_agent(observation: AgentSkillObservation) -> SkillForAgent {
    SkillForAgent {
        id: observation.skill_id,
        row_id: observation.row_id,
        name: observation.name,
        description: observation.description,
        file_path: observation.file_path,
        dir_path: observation.dir_path,
        link_type: observation.link_type,
        symlink_target: observation.symlink_target,
        is_central: false,
        source_kind: Some(observation.source_kind),
        source_root: Some(observation.source_root),
        is_read_only: observation.is_read_only,
        conflict_group: None,
        conflict_count: 0,
    }
}

fn claude_conflict_group(agent_id: &str, skill_id: &str) -> String {
    format!("{agent_id}::{skill_id}")
}

pub async fn upsert_agent_skill_observation(
    pool: &DbPool,
    observation: &AgentSkillObservation,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO agent_skill_observations
         (row_id, agent_id, skill_id, name, description, file_path, dir_path,
          source_kind, source_root, link_type, symlink_target, is_read_only, scanned_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(row_id) DO UPDATE SET
           agent_id       = excluded.agent_id,
           skill_id       = excluded.skill_id,
           name           = excluded.name,
           description    = excluded.description,
           file_path      = excluded.file_path,
           dir_path       = excluded.dir_path,
           source_kind    = excluded.source_kind,
           source_root    = excluded.source_root,
           link_type      = excluded.link_type,
           symlink_target = excluded.symlink_target,
           is_read_only   = excluded.is_read_only,
           scanned_at     = excluded.scanned_at",
    )
    .bind(&observation.row_id)
    .bind(&observation.agent_id)
    .bind(&observation.skill_id)
    .bind(&observation.name)
    .bind(&observation.description)
    .bind(&observation.file_path)
    .bind(&observation.dir_path)
    .bind(&observation.source_kind)
    .bind(&observation.source_root)
    .bind(&observation.link_type)
    .bind(&observation.symlink_target)
    .bind(observation.is_read_only)
    .bind(&observation.scanned_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub async fn get_agent_skill_observations(
    pool: &DbPool,
    agent_id: &str,
) -> Result<Vec<AgentSkillObservation>, String> {
    sqlx::query_as::<_, AgentSkillObservation>(
        "SELECT * FROM agent_skill_observations
         WHERE agent_id = ?
         ORDER BY name, dir_path",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

/// Retrieve all Central Skills (is_central = true).
pub async fn get_central_skills(pool: &DbPool) -> Result<Vec<Skill>, String> {
    sqlx::query_as::<_, Skill>("SELECT * FROM skills WHERE is_central = 1")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve a skill by its ID.
pub async fn get_skill_by_id(pool: &DbPool, skill_id: &str) -> Result<Option<Skill>, String> {
    sqlx::query_as::<_, Skill>("SELECT * FROM skills WHERE id = ?")
        .bind(skill_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a skill and all its installation records.
pub async fn delete_skill(pool: &DbPool, skill_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM skill_update_states WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_repository_members WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM collection_skills WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_tag_links WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_ai_tag_reviews WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_explanations WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_installations WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skills WHERE id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ─── Skill Installations ──────────────────────────────────────────────────────

/// Insert or update a skill installation record.
///
/// On conflict (same skill_id + agent_id), updates the mutable fields
/// (installed_path, link_type, symlink_target) but **preserves the original
/// `created_at`** so the installation timestamp reflects when the skill was
/// first installed, not when it was last re-scanned.
pub async fn upsert_skill_installation(
    pool: &DbPool,
    installation: &SkillInstallation,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO skill_installations
         (skill_id, agent_id, installed_path, link_type, symlink_target, created_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(skill_id, agent_id) DO UPDATE SET
           installed_path = excluded.installed_path,
           link_type      = excluded.link_type,
           symlink_target = excluded.symlink_target",
    )
    .bind(&installation.skill_id)
    .bind(&installation.agent_id)
    .bind(&installation.installed_path)
    .bind(&installation.link_type)
    .bind(&installation.symlink_target)
    .bind(&installation.created_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Delete an installation record for a specific skill+agent pair.
pub async fn delete_skill_installation(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
) -> Result<(), String> {
    sqlx::query("DELETE FROM skill_installations WHERE skill_id = ? AND agent_id = ?")
        .bind(skill_id)
        .bind(agent_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Remove installation records for a given agent where the skill ID is NOT in
/// `found_skill_ids`. Pass an empty slice to remove ALL installations for the
/// agent (used when the agent's skills directory no longer exists).
pub async fn delete_stale_skill_installations(
    pool: &DbPool,
    agent_id: &str,
    found_skill_ids: &[String],
) -> Result<(), String> {
    if found_skill_ids.is_empty() {
        return sqlx::query("DELETE FROM skill_installations WHERE agent_id = ?")
            .bind(agent_id)
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string());
    }

    let placeholders = found_skill_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "DELETE FROM skill_installations WHERE agent_id = ? AND skill_id NOT IN ({})",
        placeholders
    );

    let mut q = sqlx::query(&sql).bind(agent_id);
    for id in found_skill_ids {
        q = q.bind(id.as_str());
    }
    q.execute(pool).await.map(|_| ()).map_err(|e| e.to_string())
}

pub async fn delete_stale_agent_skill_observations(
    pool: &DbPool,
    agent_id: &str,
    found_row_ids: &[String],
) -> Result<(), String> {
    if found_row_ids.is_empty() {
        return sqlx::query("DELETE FROM agent_skill_observations WHERE agent_id = ?")
            .bind(agent_id)
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string());
    }

    let placeholders = found_row_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "DELETE FROM agent_skill_observations WHERE agent_id = ? AND row_id NOT IN ({})",
        placeholders
    );

    let mut q = sqlx::query(&sql).bind(agent_id);
    for row_id in found_row_ids {
        q = q.bind(row_id.as_str());
    }

    q.execute(pool).await.map(|_| ()).map_err(|e| e.to_string())
}

/// Delete skills whose IDs are NOT in `found_skill_ids`. Also cascades to
/// remove any orphaned `skill_installations` rows for those skills.
///
/// This is the global reconciliation step run after a full scan to purge rows
/// for skills that no longer exist on disk in any scanned scope.
///
/// Pass an empty slice to delete ALL skills (used only when every scanned
/// directory is empty or missing).
pub async fn delete_skills_not_in_scope(
    pool: &DbPool,
    found_skill_ids: &[String],
) -> Result<(), String> {
    if found_skill_ids.is_empty() {
        // Nothing found — delete all installation records first, then all skills.
        sqlx::query("DELETE FROM skill_update_states")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        sqlx::query("DELETE FROM skill_repository_members")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        sqlx::query("DELETE FROM skill_tag_links")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        sqlx::query("DELETE FROM skill_installations")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        return sqlx::query("DELETE FROM skills")
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string());
    }

    let placeholders = found_skill_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");

    // Cascade: remove installation rows for skills that are no longer on disk.
    let repo_sql = format!(
        "DELETE FROM skill_repository_members WHERE skill_id NOT IN ({})",
        placeholders
    );
    let mut repo_q = sqlx::query(&repo_sql);
    for id in found_skill_ids {
        repo_q = repo_q.bind(id.as_str());
    }
    repo_q.execute(pool).await.map_err(|e| e.to_string())?;

    let update_state_sql = format!(
        "DELETE FROM skill_update_states WHERE skill_id NOT IN ({})",
        placeholders
    );
    let mut update_state_q = sqlx::query(&update_state_sql);
    for id in found_skill_ids {
        update_state_q = update_state_q.bind(id.as_str());
    }
    update_state_q
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    let tag_sql = format!(
        "DELETE FROM skill_tag_links WHERE skill_id NOT IN ({})",
        placeholders
    );
    let mut tag_q = sqlx::query(&tag_sql);
    for id in found_skill_ids {
        tag_q = tag_q.bind(id.as_str());
    }
    tag_q.execute(pool).await.map_err(|e| e.to_string())?;

    let install_sql = format!(
        "DELETE FROM skill_installations WHERE skill_id NOT IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&install_sql);
    for id in found_skill_ids {
        q = q.bind(id.as_str());
    }
    q.execute(pool).await.map_err(|e| e.to_string())?;

    // Remove the stale skills themselves.
    let skill_sql = format!("DELETE FROM skills WHERE id NOT IN ({})", placeholders);
    let mut q2 = sqlx::query(&skill_sql);
    for id in found_skill_ids {
        q2 = q2.bind(id.as_str());
    }
    q2.execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Retrieve all installation records for a given skill.
pub async fn get_skill_installations(
    pool: &DbPool,
    skill_id: &str,
) -> Result<Vec<SkillInstallation>, String> {
    sqlx::query_as::<_, SkillInstallation>("SELECT * FROM skill_installations WHERE skill_id = ?")
        .bind(skill_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve cached installation counts for every agent.
pub async fn get_skill_counts_by_agent(pool: &DbPool) -> Result<HashMap<String, usize>, String> {
    let rows = sqlx::query(
        "SELECT agent_id, COUNT(*) AS cnt
         FROM skill_installations
         GROUP BY agent_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut counts = HashMap::with_capacity(rows.len());
    for row in rows {
        let agent_id: String = row.try_get("agent_id").map_err(|e| e.to_string())?;
        let count: i64 = row.try_get("cnt").map_err(|e| e.to_string())?;
        counts.insert(agent_id, count.max(0) as usize);
    }

    Ok(counts)
}

/// Retrieve the total number of collections.
pub async fn get_collection_count(pool: &DbPool) -> Result<usize, String> {
    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM collections")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let count: i64 = row.try_get("cnt").map_err(|e| e.to_string())?;
    Ok(count.max(0) as usize)
}

/// Retrieve the total number of discovered skills.
pub async fn get_discovered_skill_count(pool: &DbPool) -> Result<usize, String> {
    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM discovered_skills")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let count: i64 = row.try_get("cnt").map_err(|e| e.to_string())?;
    Ok(count.max(0) as usize)
}

// ─── Agents ───────────────────────────────────────────────────────────────────

/// Retrieve all agents.
pub async fn get_all_agents(pool: &DbPool) -> Result<Vec<Agent>, String> {
    sqlx::query_as::<_, Agent>("SELECT * FROM agents ORDER BY is_builtin DESC, display_name")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve a single agent by ID.
pub async fn get_agent_by_id(pool: &DbPool, agent_id: &str) -> Result<Option<Agent>, String> {
    sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE id = ?")
        .bind(agent_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Update the `is_detected` flag for an agent.
pub async fn update_agent_detected(
    pool: &DbPool,
    agent_id: &str,
    is_detected: bool,
) -> Result<(), String> {
    sqlx::query("UPDATE agents SET is_detected = ? WHERE id = ?")
        .bind(is_detected)
        .bind(agent_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Update the `is_enabled` flag for an agent and return the refreshed row.
pub async fn update_agent_enabled(
    pool: &DbPool,
    agent_id: &str,
    is_enabled: bool,
) -> Result<Agent, String> {
    let existing = get_agent_by_id(pool, agent_id).await?;
    if existing.is_none() {
        return Err(format!("Agent '{}' not found", agent_id));
    }

    sqlx::query("UPDATE agents SET is_enabled = ? WHERE id = ?")
        .bind(is_enabled)
        .bind(agent_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| "Failed to retrieve updated agent".to_string())
}

/// Insert a new custom agent (non-builtin).
pub async fn insert_custom_agent(pool: &DbPool, agent: &Agent) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO agents
         (id, display_name, category, global_skills_dir, project_skills_dir,
          icon_name, is_detected, is_builtin, is_enabled)
         VALUES (?, ?, ?, ?, ?, ?, ?, 0, 1)",
    )
    .bind(&agent.id)
    .bind(&agent.display_name)
    .bind(&agent.category)
    .bind(&agent.global_skills_dir)
    .bind(&agent.project_skills_dir)
    .bind(&agent.icon_name)
    .bind(agent.is_detected)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Delete a custom (non-builtin) agent by ID. Returns an error if the agent is builtin.
pub async fn delete_custom_agent(pool: &DbPool, agent_id: &str) -> Result<(), String> {
    let agent = get_agent_by_id(pool, agent_id).await?;
    match agent {
        None => Err(format!("Agent '{}' not found", agent_id)),
        Some(a) if a.is_builtin => Err(format!("Cannot delete built-in agent '{}'", agent_id)),
        Some(_) => sqlx::query("DELETE FROM agents WHERE id = ?")
            .bind(agent_id)
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string()),
    }
}

/// Update a custom (non-builtin) agent's mutable fields.
/// Returns the updated agent record, or an error if the agent is builtin or not found.
pub async fn update_custom_agent(
    pool: &DbPool,
    agent_id: &str,
    display_name: &str,
    category: &str,
    global_skills_dir: &str,
) -> Result<Agent, String> {
    let agent = get_agent_by_id(pool, agent_id).await?;
    match agent {
        None => return Err(format!("Agent '{}' not found", agent_id)),
        Some(a) if a.is_builtin => {
            return Err(format!("Cannot update built-in agent '{}'", agent_id))
        }
        Some(_) => {}
    }

    sqlx::query(
        "UPDATE agents SET display_name = ?, category = ?, global_skills_dir = ? WHERE id = ?",
    )
    .bind(display_name)
    .bind(category)
    .bind(global_skills_dir)
    .bind(agent_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| "Failed to retrieve updated agent".to_string())
}

// ─── Collections ──────────────────────────────────────────────────────────────

/// Create a new collection and return it.
pub async fn create_collection(
    pool: &DbPool,
    name: &str,
    description: Option<&str>,
) -> Result<Collection, String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(description)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    get_collection_by_id(pool, &id)
        .await?
        .ok_or_else(|| "Failed to retrieve newly created collection".to_string())
}

/// Retrieve all collections.
pub async fn get_all_collections(pool: &DbPool) -> Result<Vec<Collection>, String> {
    sqlx::query_as::<_, Collection>("SELECT * FROM collections ORDER BY created_at")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve a collection by ID.
pub async fn get_collection_by_id(
    pool: &DbPool,
    collection_id: &str,
) -> Result<Option<Collection>, String> {
    sqlx::query_as::<_, Collection>("SELECT * FROM collections WHERE id = ?")
        .bind(collection_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Update a collection's name and/or description.
pub async fn update_collection(
    pool: &DbPool,
    collection_id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE collections SET name = ?, description = ?, updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(description)
        .bind(&now)
        .bind(collection_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Delete a collection and all its skill memberships.
pub async fn delete_collection(pool: &DbPool, collection_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM collection_skills WHERE collection_id = ?")
        .bind(collection_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(collection_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Add a skill to a collection (idempotent).
pub async fn add_skill_to_collection(
    pool: &DbPool,
    collection_id: &str,
    skill_id: &str,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT OR IGNORE INTO collection_skills (collection_id, skill_id, added_at)
         VALUES (?, ?, ?)",
    )
    .bind(collection_id)
    .bind(skill_id)
    .bind(&now)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Remove a skill from a collection.
pub async fn remove_skill_from_collection(
    pool: &DbPool,
    collection_id: &str,
    skill_id: &str,
) -> Result<(), String> {
    sqlx::query("DELETE FROM collection_skills WHERE collection_id = ? AND skill_id = ?")
        .bind(collection_id)
        .bind(skill_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Retrieve all skills belonging to a collection.
pub async fn get_collection_skills(
    pool: &DbPool,
    collection_id: &str,
) -> Result<Vec<Skill>, String> {
    sqlx::query_as::<_, Skill>(
        "SELECT s.* FROM skills s
         JOIN collection_skills cs ON s.id = cs.skill_id
         WHERE cs.collection_id = ?
         ORDER BY cs.added_at",
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

/// Retrieve all collections that contain a given skill.
pub async fn get_skill_collections(
    pool: &DbPool,
    skill_id: &str,
) -> Result<Vec<Collection>, String> {
    sqlx::query_as::<_, Collection>(
        "SELECT c.* FROM collections c
         JOIN collection_skills cs ON c.id = cs.collection_id
         WHERE cs.skill_id = ?
         ORDER BY cs.added_at",
    )
    .bind(skill_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

// ─── Skill Repository Metadata ───────────────────────────────────────────────

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn normalize_repository_component(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub fn github_repository_id(owner: &str, repo: &str, branch: &str) -> String {
    format!(
        "github:{}-{}-{}",
        normalize_repository_component(owner),
        normalize_repository_component(repo),
        normalize_repository_component(branch)
    )
}

pub async fn get_local_unknown_repository(pool: &DbPool) -> Result<SkillRepository, String> {
    get_skill_repository_by_id(pool, LOCAL_UNKNOWN_REPOSITORY_ID)
        .await?
        .ok_or_else(|| "Local unknown repository metadata is not initialized".to_string())
}

pub async fn get_skill_repository_by_id(
    pool: &DbPool,
    repository_id: &str,
) -> Result<Option<SkillRepository>, String> {
    sqlx::query_as::<_, SkillRepository>("SELECT * FROM skill_repositories WHERE id = ?")
        .bind(repository_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
pub async fn create_or_update_skill_repository(
    pool: &DbPool,
    id: Option<&str>,
    name: &str,
    source_type: &str,
    owner: Option<&str>,
    repo: Option<&str>,
    branch: Option<&str>,
    url: Option<&str>,
    is_unknown: bool,
) -> Result<SkillRepository, String> {
    let normalized_id = id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = now_rfc3339();

    sqlx::query(
        "INSERT INTO skill_repositories
         (id, name, source_type, owner, repo, branch, url, is_unknown, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           name = excluded.name,
           source_type = excluded.source_type,
           owner = excluded.owner,
           repo = excluded.repo,
           branch = excluded.branch,
           url = excluded.url,
           is_unknown = excluded.is_unknown,
           updated_at = excluded.updated_at",
    )
    .bind(&normalized_id)
    .bind(name)
    .bind(source_type)
    .bind(owner)
    .bind(repo)
    .bind(branch)
    .bind(url)
    .bind(is_unknown)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    get_skill_repository_by_id(pool, &normalized_id)
        .await?
        .ok_or_else(|| "Failed to retrieve repository metadata".to_string())
}

#[allow(clippy::too_many_arguments)]
pub async fn assign_github_repository_to_skill(
    pool: &DbPool,
    owner: &str,
    repo: &str,
    branch: &str,
    url: &str,
    skill_id: &str,
    source_path: &str,
) -> Result<SkillRepository, String> {
    let repository_id = github_repository_id(owner, repo, branch);
    let name = format!("{owner}/{repo}");
    let repository = create_or_update_skill_repository(
        pool,
        Some(&repository_id),
        &name,
        "github",
        Some(owner),
        Some(repo),
        Some(branch),
        Some(url),
        false,
    )
    .await?;
    assign_skills_to_repository(
        pool,
        &repository.id,
        &[skill_id.to_string()],
        Some(source_path),
    )
    .await?;
    Ok(repository)
}

pub async fn assign_skills_to_repository(
    pool: &DbPool,
    repository_id: &str,
    skill_ids: &[String],
    source_path: Option<&str>,
) -> Result<(), String> {
    let existing = get_skill_repository_by_id(pool, repository_id).await?;
    if existing.is_none() {
        return Err(format!("Repository '{}' not found", repository_id));
    }

    let now = now_rfc3339();
    for skill_id in skill_ids {
        sqlx::query(
            "INSERT INTO skill_repository_members
             (skill_id, repository_id, source_path, added_at, updated_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(skill_id) DO UPDATE SET
               repository_id = excluded.repository_id,
               source_path = COALESCE(excluded.source_path, skill_repository_members.source_path),
               updated_at = excluded.updated_at",
        )
        .bind(skill_id)
        .bind(repository_id)
        .bind(source_path)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub async fn get_skill_repository_assignment(
    pool: &DbPool,
    skill_id: &str,
) -> Result<SkillRepositoryAssignment, String> {
    let assigned = sqlx::query_as::<_, SkillRepository>(
        "SELECT r.* FROM skill_repositories r
         JOIN skill_repository_members m ON r.id = m.repository_id
         WHERE m.skill_id = ?",
    )
    .bind(skill_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(repository) = assigned {
        let source_path = sqlx::query_scalar::<_, Option<String>>(
            "SELECT source_path FROM skill_repository_members WHERE skill_id = ?",
        )
        .bind(skill_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .flatten();
        return Ok(SkillRepositoryAssignment {
            is_source_unknown: repository.is_unknown,
            repository,
            source_path,
        });
    }

    Ok(SkillRepositoryAssignment {
        repository: get_local_unknown_repository(pool).await?,
        source_path: None,
        is_source_unknown: true,
    })
}

pub async fn get_skill_repositories_with_stats(
    pool: &DbPool,
) -> Result<Vec<SkillRepositoryWithStats>, String> {
    let repositories = sqlx::query_as::<_, SkillRepository>(
        "SELECT * FROM skill_repositories ORDER BY is_unknown DESC, name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let unknown_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills s
         LEFT JOIN skill_repository_members m ON s.id = m.skill_id
         WHERE s.is_central = 1 AND m.skill_id IS NULL",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut result = Vec::with_capacity(repositories.len());
    for repository in repositories {
        let member_count = if repository.id == LOCAL_UNKNOWN_REPOSITORY_ID {
            unknown_count
        } else {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM skill_repository_members WHERE repository_id = ?",
            )
            .bind(&repository.id)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?
        };
        result.push(SkillRepositoryWithStats {
            unknown_skill_count: if repository.id == LOCAL_UNKNOWN_REPOSITORY_ID {
                unknown_count
            } else {
                0
            },
            repository,
            skill_count: member_count,
        });
    }

    Ok(result)
}

// ─── Skill Tags ──────────────────────────────────────────────────────────────

pub async fn get_skill_update_states(pool: &DbPool) -> Result<Vec<SkillUpdateState>, String> {
    sqlx::query_as::<_, SkillUpdateState>(
        "SELECT * FROM skill_update_states ORDER BY last_checked_at DESC, skill_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn get_skill_update_states_for_skills(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<Vec<SkillUpdateState>, String> {
    if skill_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = skill_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT * FROM skill_update_states WHERE skill_id IN ({})",
        placeholders
    );
    let mut query = sqlx::query_as::<_, SkillUpdateState>(&sql);
    for skill_id in skill_ids {
        query = query.bind(skill_id);
    }

    query.fetch_all(pool).await.map_err(|e| e.to_string())
}

pub async fn upsert_skill_update_state(
    pool: &DbPool,
    state: &SkillUpdateState,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO skill_update_states
         (skill_id, source_type, source_url, ref_name, source_path, last_remote_hash,
          latest_remote_hash, last_checked_at, last_updated_at, status, error)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(skill_id) DO UPDATE SET
           source_type        = excluded.source_type,
           source_url         = excluded.source_url,
           ref_name           = excluded.ref_name,
           source_path        = excluded.source_path,
           last_remote_hash   = COALESCE(excluded.last_remote_hash, skill_update_states.last_remote_hash),
           latest_remote_hash = excluded.latest_remote_hash,
           last_checked_at    = excluded.last_checked_at,
           last_updated_at    = COALESCE(excluded.last_updated_at, skill_update_states.last_updated_at),
           status             = excluded.status,
           error              = excluded.error",
    )
    .bind(&state.skill_id)
    .bind(&state.source_type)
    .bind(&state.source_url)
    .bind(&state.ref_name)
    .bind(&state.source_path)
    .bind(&state.last_remote_hash)
    .bind(&state.latest_remote_hash)
    .bind(&state.last_checked_at)
    .bind(&state.last_updated_at)
    .bind(&state.status)
    .bind(&state.error)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub async fn get_skill_tags(pool: &DbPool) -> Result<Vec<SkillTag>, String> {
    sqlx::query_as::<_, SkillTag>("SELECT * FROM skill_tags ORDER BY is_builtin DESC, name")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_skill_tag_by_id(pool: &DbPool, tag_id: &str) -> Result<Option<SkillTag>, String> {
    sqlx::query_as::<_, SkillTag>("SELECT * FROM skill_tags WHERE id = ?")
        .bind(tag_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_skill_tag_by_name(pool: &DbPool, name: &str) -> Result<Option<SkillTag>, String> {
    sqlx::query_as::<_, SkillTag>("SELECT * FROM skill_tags WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_skill_tag(
    pool: &DbPool,
    name: &str,
    description: Option<&str>,
    color: Option<&str>,
) -> Result<SkillTag, String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Tag name is required".to_string());
    }

    if let Some(existing) = get_skill_tag_by_name(pool, trimmed_name).await? {
        return Ok(existing);
    }

    let id = normalize_repository_component(trimmed_name);
    let tag_id = if id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        id
    };
    let now = now_rfc3339();

    sqlx::query(
        "INSERT INTO skill_tags (id, name, description, color, is_builtin, created_at, updated_at)
         VALUES (?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(&tag_id)
    .bind(trimmed_name)
    .bind(description)
    .bind(color)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    get_skill_tag_by_id(pool, &tag_id)
        .await?
        .ok_or_else(|| "Failed to retrieve created tag".to_string())
}

pub async fn assign_skill_tags(
    pool: &DbPool,
    skill_ids: &[String],
    tag_ids: &[String],
    source: &str,
    confidence: Option<f64>,
    reason: Option<&str>,
) -> Result<(), String> {
    let now = now_rfc3339();
    for tag_id in tag_ids {
        if get_skill_tag_by_id(pool, tag_id).await?.is_none() {
            return Err(format!("Tag '{}' not found", tag_id));
        }
    }

    for skill_id in skill_ids {
        for tag_id in tag_ids {
            sqlx::query(
                "INSERT INTO skill_tag_links
                 (skill_id, tag_id, confidence, reason, source, added_at)
                 VALUES (?, ?, ?, ?, ?, ?)
                 ON CONFLICT(skill_id, tag_id) DO UPDATE SET
                   confidence = excluded.confidence,
                   reason = excluded.reason,
                   source = excluded.source",
            )
            .bind(skill_id)
            .bind(tag_id)
            .bind(confidence)
            .bind(reason)
            .bind(source)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

pub async fn replace_skill_ai_tags(
    pool: &DbPool,
    skill_id: &str,
    suggestions: &[(String, f64, String)],
) -> Result<(), String> {
    sqlx::query("DELETE FROM skill_tag_links WHERE skill_id = ? AND source = 'ai'")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    for (tag_id, confidence, reason) in suggestions {
        assign_skill_tags(
            pool,
            &[skill_id.to_string()],
            std::slice::from_ref(tag_id),
            "ai",
            Some(*confidence),
            Some(reason),
        )
        .await?;
    }

    Ok(())
}

pub async fn replace_pending_ai_tag_reviews(
    pool: &DbPool,
    skill_id: &str,
    suggestions: &[(String, f64, String)],
) -> Result<(), String> {
    let now = now_rfc3339();
    sqlx::query("DELETE FROM skill_ai_tag_reviews WHERE skill_id = ? AND status = 'pending'")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    for (tag_id, confidence, reason) in suggestions {
        if get_skill_tag_by_id(pool, tag_id).await?.is_none() {
            return Err(format!("Tag '{}' not found", tag_id));
        }

        sqlx::query(
            "INSERT INTO skill_ai_tag_reviews
             (skill_id, tag_id, confidence, reason, status, suggested_at, updated_at)
             VALUES (?, ?, ?, ?, 'pending', ?, ?)
             ON CONFLICT(skill_id, tag_id) DO UPDATE SET
               confidence = excluded.confidence,
               reason = excluded.reason,
               status = 'pending',
               updated_at = excluded.updated_at",
        )
        .bind(skill_id)
        .bind(tag_id)
        .bind(confidence)
        .bind(reason)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub async fn get_pending_ai_tag_reviews(pool: &DbPool) -> Result<Vec<SkillAiTagReview>, String> {
    let rows = sqlx::query(
        "SELECT
           r.skill_id,
           COALESCE(s.name, r.skill_id) AS skill_name,
           t.id AS tag_id,
           t.name AS tag_name,
           t.description AS tag_description,
           t.color AS tag_color,
           t.is_builtin AS tag_is_builtin,
           t.created_at AS tag_created_at,
           t.updated_at AS tag_updated_at,
           r.confidence,
           r.reason,
           r.suggested_at,
           r.updated_at
         FROM skill_ai_tag_reviews r
         JOIN skill_tags t ON t.id = r.tag_id
         LEFT JOIN skills s ON s.id = r.skill_id
         WHERE r.status = 'pending'
         ORDER BY r.updated_at DESC, skill_name, t.name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.into_iter()
        .map(|row| {
            Ok(SkillAiTagReview {
                skill_id: row.get("skill_id"),
                skill_name: row.get("skill_name"),
                tag: SkillTag {
                    id: row.get("tag_id"),
                    name: row.get("tag_name"),
                    description: row.get("tag_description"),
                    color: row.get("tag_color"),
                    is_builtin: row.get("tag_is_builtin"),
                    created_at: row.get("tag_created_at"),
                    updated_at: row.get("tag_updated_at"),
                },
                confidence: row.get("confidence"),
                reason: row
                    .get::<Option<String>, _>("reason")
                    .unwrap_or_else(|| "AI 低置信度建议".to_string()),
                suggested_at: row.get("suggested_at"),
                updated_at: row.get("updated_at"),
            })
        })
        .collect()
}

pub async fn accept_ai_tag_reviews(
    pool: &DbPool,
    skill_id: &str,
    tag_ids: &[String],
) -> Result<(), String> {
    if tag_ids.is_empty() {
        return Err("No review tags selected".to_string());
    }

    for tag_id in tag_ids {
        let review = sqlx::query(
            "SELECT confidence, reason
             FROM skill_ai_tag_reviews
             WHERE skill_id = ? AND tag_id = ? AND status = 'pending'",
        )
        .bind(skill_id)
        .bind(tag_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

        let confidence = review
            .as_ref()
            .map(|row| row.get::<f64, _>("confidence"))
            .unwrap_or(1.0);
        let reason = review
            .as_ref()
            .and_then(|row| row.get::<Option<String>, _>("reason"))
            .unwrap_or_else(|| "人工复核确认".to_string());

        assign_skill_tags(
            pool,
            &[skill_id.to_string()],
            std::slice::from_ref(tag_id),
            "ai",
            Some(confidence),
            Some(&reason),
        )
        .await?;
    }

    let now = now_rfc3339();
    for tag_id in tag_ids {
        sqlx::query(
            "UPDATE skill_ai_tag_reviews
             SET status = 'accepted', updated_at = ?
             WHERE skill_id = ? AND tag_id = ? AND status = 'pending'",
        )
        .bind(&now)
        .bind(skill_id)
        .bind(tag_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    sqlx::query(
        "UPDATE skill_ai_tag_reviews
         SET status = 'skipped', updated_at = ?
         WHERE skill_id = ? AND status = 'pending'",
    )
    .bind(&now)
    .bind(skill_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn skip_ai_tag_reviews(pool: &DbPool, skill_id: &str) -> Result<(), String> {
    let now = now_rfc3339();
    sqlx::query(
        "UPDATE skill_ai_tag_reviews
         SET status = 'skipped', updated_at = ?
         WHERE skill_id = ? AND status = 'pending'",
    )
    .bind(&now)
    .bind(skill_id)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub async fn get_skill_tags_for_skill(
    pool: &DbPool,
    skill_id: &str,
) -> Result<Vec<SkillTag>, String> {
    sqlx::query_as::<_, SkillTag>(
        "SELECT t.* FROM skill_tags t
         JOIN skill_tag_links l ON t.id = l.tag_id
         WHERE l.skill_id = ?
         ORDER BY t.is_builtin DESC, t.name",
    )
    .bind(skill_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

// ─── Scan Directories ─────────────────────────────────────────────────────────

/// Retrieve all scan directories.
pub async fn get_scan_directories(pool: &DbPool) -> Result<Vec<ScanDirectory>, String> {
    sqlx::query_as::<_, ScanDirectory>(
        "SELECT * FROM scan_directories ORDER BY is_builtin DESC, added_at",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

/// Add a new scan directory entry (non-builtin by default).
pub async fn add_scan_directory(
    pool: &DbPool,
    path: &str,
    label: Option<&str>,
) -> Result<ScanDirectory, String> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO scan_directories (path, label, is_active, is_builtin, added_at)
         VALUES (?, ?, 1, 0, ?)",
    )
    .bind(path)
    .bind(label)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query_as::<_, ScanDirectory>("SELECT * FROM scan_directories WHERE path = ?")
        .bind(path)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Remove a scan directory. Returns an error if the directory is builtin.
pub async fn remove_scan_directory(pool: &DbPool, path: &str) -> Result<(), String> {
    let row = sqlx::query("SELECT is_builtin FROM scan_directories WHERE path = ?")
        .bind(path)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

    match row {
        None => Err(format!("Scan directory '{}' not found", path)),
        Some(r) => {
            let is_builtin: bool = r.try_get("is_builtin").map_err(|e| e.to_string())?;
            if is_builtin {
                return Err(format!("Cannot remove built-in scan directory '{}'", path));
            }
            sqlx::query("DELETE FROM scan_directories WHERE path = ?")
                .bind(path)
                .execute(pool)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
    }
}

/// Toggle the `is_active` flag on a scan directory.
pub async fn toggle_scan_directory(
    pool: &DbPool,
    path: &str,
    is_active: bool,
) -> Result<(), String> {
    sqlx::query("UPDATE scan_directories SET is_active = ? WHERE path = ?")
        .bind(is_active)
        .bind(path)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ─── Discovered Skills ────────────────────────────────────────────────────────

/// A skill discovered in a project-level directory during a full-disk scan.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DiscoveredSkillRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    pub dir_path: String,
    pub project_path: String,
    pub project_name: String,
    pub platform_id: String,
    pub discovered_at: String,
}

/// Insert a discovered skill record.
#[allow(clippy::too_many_arguments)]
pub async fn insert_discovered_skill(
    pool: &DbPool,
    id: &str,
    name: &str,
    description: Option<&str>,
    file_path: &str,
    dir_path: &str,
    project_path: &str,
    project_name: &str,
    platform_id: &str,
    discovered_at: &str,
) -> Result<(), String> {
    sqlx::query(
        "INSERT OR IGNORE INTO discovered_skills
         (id, name, description, file_path, dir_path, project_path, project_name, platform_id, discovered_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(file_path)
    .bind(dir_path)
    .bind(project_path)
    .bind(project_name)
    .bind(platform_id)
    .bind(discovered_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Retrieve a discovered skill by its qualified ID.
pub async fn get_discovered_skill_by_id(
    pool: &DbPool,
    id: &str,
) -> Result<Option<DiscoveredSkillRow>, String> {
    sqlx::query_as::<_, DiscoveredSkillRow>("SELECT * FROM discovered_skills WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve all discovered skills.
pub async fn get_all_discovered_skills(pool: &DbPool) -> Result<Vec<DiscoveredSkillRow>, String> {
    sqlx::query_as::<_, DiscoveredSkillRow>(
        "SELECT * FROM discovered_skills ORDER BY project_name, platform_id, name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

/// Delete a discovered skill by ID.
pub async fn delete_discovered_skill(pool: &DbPool, id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM discovered_skills WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Clear all discovered skills.
pub async fn clear_all_discovered_skills(pool: &DbPool) -> Result<(), String> {
    sqlx::query("DELETE FROM discovered_skills")
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Get count of discovered projects (distinct project_path values).
pub async fn get_discovered_project_count(pool: &DbPool) -> Result<i64, String> {
    let row = sqlx::query("SELECT COUNT(DISTINCT project_path) AS cnt FROM discovered_skills")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    row.try_get::<i64, _>("cnt").map_err(|e| e.to_string())
}

// ─── Settings ─────────────────────────────────────────────────────────────────

/// Get a setting value by key.
pub async fn get_setting(pool: &DbPool, key: &str) -> Result<Option<String>, String> {
    let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(row.map(|r| r.get::<String, _>("value")))
}

/// Set (upsert) a setting value.
pub async fn set_setting(pool: &DbPool, key: &str, value: &str) -> Result<(), String> {
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Create an in-memory SQLite pool and initialize the schema.
    async fn setup_test_db() -> DbPool {
        let pool = SqlitePool::connect(":memory:")
            .await
            .expect("Failed to create in-memory SQLite pool");
        init_database(&pool)
            .await
            .expect("Failed to initialize test database");
        pool
    }

    // ── Init ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_init_creates_all_tables() {
        let pool = setup_test_db().await;

        // Verify all core tables exist by counting rows (empty is fine)
        let tables = [
            "skills",
            "skill_installations",
            "agent_skill_observations",
            "agents",
            "collections",
            "collection_skills",
            "skill_repositories",
            "skill_repository_members",
            "skill_update_states",
            "skill_tags",
            "skill_tag_links",
            "skill_ai_tag_reviews",
            "skill_explanations",
            "scan_directories",
            "settings",
        ];
        for table in &tables {
            let result = sqlx::query(&format!("SELECT COUNT(*) as cnt FROM {}", table))
                .fetch_one(&pool)
                .await;
            assert!(result.is_ok(), "Table '{}' should exist", table);
        }
    }

    #[tokio::test]
    async fn test_init_is_idempotent() {
        let pool = setup_test_db().await;
        // Calling init_database again should not fail
        let result = init_database(&pool).await;
        assert!(result.is_ok(), "Second init should be idempotent");
    }

    #[tokio::test]
    async fn test_builtin_agents_seeded() {
        let pool = setup_test_db().await;
        let agents = get_all_agents(&pool).await.unwrap();
        assert_eq!(agents.len(), 33, "Should have exactly 33 built-in agents");

        let ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();
        // Coding platforms
        assert!(ids.contains(&"claude-code"));
        assert!(ids.contains(&"codex"));
        assert!(ids.contains(&"cursor"));
        assert!(ids.contains(&"gemini-cli"));
        assert!(ids.contains(&"trae"));
        assert!(ids.contains(&"factory-droid"));
        assert!(ids.contains(&"junie"));
        assert!(ids.contains(&"qwen"));
        assert!(ids.contains(&"trae-cn"));
        assert!(ids.contains(&"windsurf"));
        assert!(ids.contains(&"qoder"));
        assert!(ids.contains(&"augment"));
        assert!(ids.contains(&"opencode"));
        assert!(ids.contains(&"kilocode"));
        assert!(ids.contains(&"ob1"));
        assert!(ids.contains(&"amp"));
        assert!(ids.contains(&"antigravity"));
        assert!(ids.contains(&"cline"));
        assert!(ids.contains(&"deep-agents"));
        assert!(ids.contains(&"firebender"));
        assert!(ids.contains(&"kiro"));
        assert!(ids.contains(&"kimi-code-cli"));
        assert!(ids.contains(&"codebuddy"));
        assert!(ids.contains(&"hermes"));
        assert!(ids.contains(&"copilot"));
        assert!(ids.contains(&"warp"));
        assert!(ids.contains(&"aider"));
        // Lobster platforms
        assert!(ids.contains(&"openclaw"));
        assert!(ids.contains(&"qclaw"));
        assert!(ids.contains(&"easyclaw"));
        assert!(ids.contains(&"autoclaw"));
        assert!(ids.contains(&"workbuddy"));
        // Central
        assert!(ids.contains(&"central"));
    }

    #[tokio::test]
    async fn test_builtin_agents_are_marked_builtin() {
        let pool = setup_test_db().await;
        let agents = get_all_agents(&pool).await.unwrap();
        for agent in &agents {
            assert!(agent.is_builtin, "All seeded agents should be builtin");
        }
    }

    #[tokio::test]
    async fn test_universal_agents_share_universal_skills_dir() {
        let pool = setup_test_db().await;
        let agents = get_all_agents(&pool).await.unwrap();
        let central = agents
            .iter()
            .find(|agent| agent.id == "central")
            .expect("central agent should exist");
        let universal_dir = crate::paths::universal_skills_dir();

        assert!(
            crate::paths::paths_equivalent(
                Path::new(&central.global_skills_dir),
                &crate::paths::central_skills_dir()
            ),
            "central should use the private SkillPort skills directory"
        );
        assert!(
            !crate::paths::paths_equivalent(Path::new(&central.global_skills_dir), &universal_dir),
            "central should not share the Universal Agents directory"
        );

        for agent_id in UNIVERSAL_AGENT_IDS {
            let agent = agents
                .iter()
                .find(|agent| agent.id == agent_id)
                .unwrap_or_else(|| panic!("missing universal agent {agent_id}"));
            assert!(
                crate::paths::paths_equivalent(Path::new(&agent.global_skills_dir), &universal_dir),
                "{agent_id} should use the Universal Agents skills directory"
            );
        }
    }

    #[tokio::test]
    async fn test_builtin_agents_seed_default_enabled_subset() {
        let pool = setup_test_db().await;
        let agents = get_all_agents(&pool).await.unwrap();

        let enabled_ids: std::collections::HashSet<&str> = agents
            .iter()
            .filter(|agent| agent.is_enabled)
            .map(|agent| agent.id.as_str())
            .collect();

        let expected_enabled_ids = std::collections::HashSet::from([
            "claude-code",
            "codex",
            "gemini-cli",
            "opencode",
            "kiro",
            "central",
        ]);

        assert_eq!(enabled_ids, expected_enabled_ids);
    }

    #[tokio::test]
    async fn test_init_does_not_duplicate_agents_on_reinit() {
        let pool = setup_test_db().await;
        init_database(&pool).await.unwrap(); // Call a second time
        let agents = get_all_agents(&pool).await.unwrap();
        assert_eq!(agents.len(), 33, "Reinit must not duplicate agents");
    }

    // ── Skills ────────────────────────────────────────────────────────────────

    fn make_skill(id: &str, name: &str, is_central: bool) -> Skill {
        Skill {
            id: id.to_string(),
            name: name.to_string(),
            description: Some(format!("Description for {}", name)),
            file_path: format!("/tmp/{}/SKILL.md", id),
            canonical_path: if is_central {
                Some(format!("/tmp/.skillsmanage/skills/{}", id))
            } else {
                None
            },
            is_central,
            source: None,
            content: Some("# Test Skill\n\nContent here.".to_string()),
            scanned_at: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_upsert_skill_insert() {
        let pool = setup_test_db().await;
        let skill = make_skill("test-skill", "Test Skill", false);
        upsert_skill(&pool, &skill).await.unwrap();

        let retrieved = get_skill_by_id(&pool, "test-skill").await.unwrap();
        assert!(retrieved.is_some());
        let s = retrieved.unwrap();
        assert_eq!(s.name, "Test Skill");
        assert!(!s.is_central);
    }

    #[tokio::test]
    async fn test_upsert_skill_update() {
        let pool = setup_test_db().await;
        let mut skill = make_skill("skill-1", "Original Name", false);
        upsert_skill(&pool, &skill).await.unwrap();

        skill.name = "Updated Name".to_string();
        upsert_skill(&pool, &skill).await.unwrap();

        let retrieved = get_skill_by_id(&pool, "skill-1").await.unwrap().unwrap();
        assert_eq!(retrieved.name, "Updated Name");
    }

    #[tokio::test]
    async fn test_upsert_skill_preserves_central_record_when_platform_copy_is_seen_later() {
        let pool = setup_test_db().await;
        let mut central = make_skill("shared-skill", "Central Truth", true);
        central.file_path = "/tmp/.skillsmanage/skills/shared-skill/SKILL.md".to_string();
        central.canonical_path = Some("/tmp/.skillsmanage/skills/shared-skill".to_string());
        central.source = Some("native".to_string());
        upsert_skill(&pool, &central).await.unwrap();

        let mut platform = make_skill("shared-skill", "Platform Copy", false);
        platform.file_path = "/tmp/.agents/skills/shared-skill/SKILL.md".to_string();
        platform.canonical_path = Some("/tmp/.agents/skills/shared-skill".to_string());
        platform.source = Some("copy".to_string());
        upsert_skill(&pool, &platform).await.unwrap();

        let retrieved = get_skill_by_id(&pool, "shared-skill")
            .await
            .unwrap()
            .unwrap();
        assert!(retrieved.is_central);
        assert_eq!(retrieved.name, "Central Truth");
        assert_eq!(
            retrieved.file_path,
            "/tmp/.skillsmanage/skills/shared-skill/SKILL.md"
        );
        assert_eq!(
            retrieved.canonical_path.as_deref(),
            Some("/tmp/.skillsmanage/skills/shared-skill")
        );
        assert_eq!(retrieved.source.as_deref(), Some("native"));
    }

    #[tokio::test]
    async fn test_get_skill_by_id_not_found() {
        let pool = setup_test_db().await;
        let result = get_skill_by_id(&pool, "nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_central_skills() {
        let pool = setup_test_db().await;
        upsert_skill(&pool, &make_skill("central-1", "Central One", true))
            .await
            .unwrap();
        upsert_skill(&pool, &make_skill("non-central", "Not Central", false))
            .await
            .unwrap();

        let central = get_central_skills(&pool).await.unwrap();
        assert_eq!(central.len(), 1);
        assert_eq!(central[0].id, "central-1");
    }

    #[tokio::test]
    async fn test_upsert_skill_update_state() {
        let pool = setup_test_db().await;
        let state = SkillUpdateState {
            skill_id: "central-1".to_string(),
            source_type: "github".to_string(),
            source_url: Some("https://github.com/example/skills".to_string()),
            ref_name: Some("main".to_string()),
            source_path: Some("skills/central-1".to_string()),
            last_remote_hash: Some("fnv1a64:old".to_string()),
            latest_remote_hash: Some("fnv1a64:new".to_string()),
            last_checked_at: Some("2026-04-25T00:00:00Z".to_string()),
            last_updated_at: None,
            status: "update_available".to_string(),
            error: None,
        };

        upsert_skill_update_state(&pool, &state).await.unwrap();
        let states = get_skill_update_states_for_skills(&pool, &["central-1".to_string()])
            .await
            .unwrap();

        assert_eq!(states.len(), 1);
        assert_eq!(states[0].skill_id, "central-1");
        assert_eq!(states[0].status, "update_available");
        assert_eq!(states[0].latest_remote_hash.as_deref(), Some("fnv1a64:new"));
    }

    #[tokio::test]
    async fn test_delete_skill() {
        let pool = setup_test_db().await;
        let skill = make_skill("to-delete", "Delete Me", false);
        upsert_skill(&pool, &skill).await.unwrap();

        let repository = create_or_update_skill_repository(
            &pool,
            Some("repo-delete-test"),
            "Delete Test Repo",
            "github",
            Some("owner"),
            Some("repo"),
            Some("main"),
            Some("https://example.com/owner/repo"),
            false,
        )
        .await
        .unwrap();
        assign_skills_to_repository(
            &pool,
            &repository.id,
            &["to-delete".to_string()],
            Some("skills/to-delete"),
        )
        .await
        .unwrap();

        let tag = create_skill_tag(&pool, "Delete Tag", None, None)
            .await
            .unwrap();
        assign_skill_tags(
            &pool,
            &["to-delete".to_string()],
            std::slice::from_ref(&tag.id),
            "manual",
            Some(1.0),
            Some("delete test"),
        )
        .await
        .unwrap();
        replace_pending_ai_tag_reviews(
            &pool,
            "to-delete",
            &[(tag.id.clone(), 0.42, "review".to_string())],
        )
        .await
        .unwrap();
        upsert_skill_update_state(
            &pool,
            &SkillUpdateState {
                skill_id: "to-delete".to_string(),
                source_type: "github".to_string(),
                source_url: Some("https://github.com/owner/repo".to_string()),
                ref_name: Some("main".to_string()),
                source_path: Some("skills/to-delete".to_string()),
                last_remote_hash: Some("fnv1a64:old".to_string()),
                latest_remote_hash: Some("fnv1a64:new".to_string()),
                last_checked_at: Some(Utc::now().to_rfc3339()),
                last_updated_at: None,
                status: "update_available".to_string(),
                error: None,
            },
        )
        .await
        .unwrap();

        let collection = create_collection(&pool, "Delete Collection", None)
            .await
            .unwrap();
        add_skill_to_collection(&pool, &collection.id, "to-delete")
            .await
            .unwrap();
        upsert_skill_installation(&pool, &make_installation("to-delete", "cursor", "copy"))
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO skill_explanations (skill_id, explanation, lang, model, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("to-delete")
        .bind("Explanation")
        .bind("zh")
        .bind("test-model")
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        delete_skill(&pool, "to-delete").await.unwrap();
        let result = get_skill_by_id(&pool, "to-delete").await.unwrap();
        assert!(result.is_none());

        for table in [
            "skill_repository_members",
            "skill_update_states",
            "skill_tag_links",
            "skill_ai_tag_reviews",
            "skill_explanations",
            "collection_skills",
            "skill_installations",
        ] {
            let query = format!("SELECT COUNT(*) FROM {table} WHERE skill_id = ?");
            let count = sqlx::query_scalar::<_, i64>(&query)
                .bind("to-delete")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(count, 0, "{table} rows must be deleted");
        }
    }

    // ── Skill Installations ───────────────────────────────────────────────────

    fn make_installation(skill_id: &str, agent_id: &str, link_type: &str) -> SkillInstallation {
        SkillInstallation {
            skill_id: skill_id.to_string(),
            agent_id: agent_id.to_string(),
            installed_path: format!("/tmp/{}/{}", agent_id, skill_id),
            link_type: link_type.to_string(),
            symlink_target: if link_type == "symlink" {
                Some(format!("/tmp/.agents/skills/{}", skill_id))
            } else {
                None
            },
            created_at: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_upsert_and_get_skill_installation() {
        let pool = setup_test_db().await;
        let skill = make_skill("my-skill", "My Skill", false);
        upsert_skill(&pool, &skill).await.unwrap();

        let inst = make_installation("my-skill", "claude-code", "symlink");
        upsert_skill_installation(&pool, &inst).await.unwrap();

        let installations = get_skill_installations(&pool, "my-skill").await.unwrap();
        assert_eq!(installations.len(), 1);
        assert_eq!(installations[0].agent_id, "claude-code");
        assert_eq!(installations[0].link_type, "symlink");
    }

    #[tokio::test]
    async fn test_delete_skill_installation() {
        let pool = setup_test_db().await;
        let skill = make_skill("del-skill", "Del Skill", false);
        upsert_skill(&pool, &skill).await.unwrap();
        upsert_skill_installation(&pool, &make_installation("del-skill", "cursor", "copy"))
            .await
            .unwrap();

        delete_skill_installation(&pool, "del-skill", "cursor")
            .await
            .unwrap();

        let installations = get_skill_installations(&pool, "del-skill").await.unwrap();
        assert!(installations.is_empty());
    }

    #[tokio::test]
    async fn test_get_skills_by_agent() {
        let pool = setup_test_db().await;
        let skill_a = make_skill("skill-a", "Skill A", false);
        let skill_b = make_skill("skill-b", "Skill B", false);
        upsert_skill(&pool, &skill_a).await.unwrap();
        upsert_skill(&pool, &skill_b).await.unwrap();

        upsert_skill_installation(
            &pool,
            &make_installation("skill-a", "claude-code", "symlink"),
        )
        .await
        .unwrap();
        upsert_skill_installation(&pool, &make_installation("skill-b", "cursor", "copy"))
            .await
            .unwrap();

        let claude_skills = get_skills_by_agent(&pool, "claude-code").await.unwrap();
        assert_eq!(claude_skills.len(), 1);
        assert_eq!(claude_skills[0].id, "skill-a");

        let cursor_skills = get_skills_by_agent(&pool, "cursor").await.unwrap();
        assert_eq!(cursor_skills.len(), 1);
        assert_eq!(cursor_skills[0].id, "skill-b");

        let empty = get_skills_by_agent(&pool, "codex").await.unwrap();
        assert!(empty.is_empty());
    }

    // ── Agents ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_agent_by_id() {
        let pool = setup_test_db().await;
        let agent = get_agent_by_id(&pool, "claude-code").await.unwrap();
        assert!(agent.is_some());
        let a = agent.unwrap();
        assert_eq!(a.display_name, "Claude Code");
        assert_eq!(a.category, "coding");
        assert!(a.is_builtin);
    }

    #[tokio::test]
    async fn test_get_agent_by_id_not_found() {
        let pool = setup_test_db().await;
        let agent = get_agent_by_id(&pool, "nonexistent-agent").await.unwrap();
        assert!(agent.is_none());
    }

    #[tokio::test]
    async fn test_update_agent_detected() {
        let pool = setup_test_db().await;
        update_agent_detected(&pool, "cursor", true).await.unwrap();
        let agent = get_agent_by_id(&pool, "cursor").await.unwrap().unwrap();
        assert!(agent.is_detected);

        update_agent_detected(&pool, "cursor", false).await.unwrap();
        let agent = get_agent_by_id(&pool, "cursor").await.unwrap().unwrap();
        assert!(!agent.is_detected);
    }

    #[tokio::test]
    async fn test_update_agent_enabled() {
        let pool = setup_test_db().await;

        let updated = update_agent_enabled(&pool, "claude-code", false)
            .await
            .unwrap();
        assert!(!updated.is_enabled);

        let persisted = get_agent_by_id(&pool, "claude-code")
            .await
            .unwrap()
            .unwrap();
        assert!(!persisted.is_enabled);

        let reenabled = update_agent_enabled(&pool, "claude-code", true)
            .await
            .unwrap();
        assert!(reenabled.is_enabled);
    }

    #[tokio::test]
    async fn test_insert_custom_agent() {
        let pool = setup_test_db().await;
        let custom = Agent {
            id: "my-custom-agent".to_string(),
            display_name: "My Custom Agent".to_string(),
            category: "other".to_string(),
            global_skills_dir: "/tmp/custom/skills".to_string(),
            project_skills_dir: None,
            icon_name: None,
            is_detected: false,
            is_builtin: false,
            is_enabled: true,
        };
        insert_custom_agent(&pool, &custom).await.unwrap();

        let all = get_all_agents(&pool).await.unwrap();
        assert_eq!(all.len(), 34, "Should have 33 builtins + 1 custom");

        let retrieved = get_agent_by_id(&pool, "my-custom-agent")
            .await
            .unwrap()
            .unwrap();
        assert!(!retrieved.is_builtin);
        assert_eq!(retrieved.display_name, "My Custom Agent");
    }

    #[tokio::test]
    async fn test_delete_custom_agent() {
        let pool = setup_test_db().await;
        let custom = Agent {
            id: "deletable-agent".to_string(),
            display_name: "Deletable".to_string(),
            category: "other".to_string(),
            global_skills_dir: "/tmp/deletable/skills".to_string(),
            project_skills_dir: None,
            icon_name: None,
            is_detected: false,
            is_builtin: false,
            is_enabled: true,
        };
        insert_custom_agent(&pool, &custom).await.unwrap();
        delete_custom_agent(&pool, "deletable-agent").await.unwrap();

        let retrieved = get_agent_by_id(&pool, "deletable-agent").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_cannot_delete_builtin_agent() {
        let pool = setup_test_db().await;
        let result = delete_custom_agent(&pool, "claude-code").await;
        assert!(
            result.is_err(),
            "Should not be able to delete built-in agent"
        );
    }

    #[tokio::test]
    async fn test_workbuddy_scans_correct_directory() {
        let pool = setup_test_db().await;
        let wb = get_agent_by_id(&pool, "workbuddy")
            .await
            .unwrap()
            .expect("WorkBuddy agent should exist");
        assert_eq!(wb.display_name, "WorkBuddy");
        let expected_suffix = Path::new(".workbuddy")
            .join("skills-marketplace")
            .join("skills");
        assert!(
            Path::new(&wb.global_skills_dir).ends_with(&expected_suffix),
            "WorkBuddy should scan ~/.workbuddy/skills-marketplace/skills, got: {}",
            wb.global_skills_dir
        );
    }

    #[tokio::test]
    async fn test_autoclaw_is_separate_from_workbuddy() {
        let pool = setup_test_db().await;
        let ac = get_agent_by_id(&pool, "autoclaw")
            .await
            .unwrap()
            .expect("AutoClaw agent should exist");
        assert_eq!(ac.display_name, "AutoClaw");
        assert_eq!(ac.category, "lobster");
        let expected_suffix = Path::new(".openclaw-autoclaw").join("skills");
        assert!(
            Path::new(&ac.global_skills_dir).ends_with(&expected_suffix),
            "AutoClaw should scan ~/.openclaw-autoclaw/skills, got: {}",
            ac.global_skills_dir
        );
        // Verify AutoClaw and WorkBuddy are distinct entries
        assert_ne!(ac.id, "workbuddy");
        assert_ne!(
            ac.global_skills_dir,
            get_agent_by_id(&pool, "workbuddy")
                .await
                .unwrap()
                .unwrap()
                .global_skills_dir
        );
    }

    // ── Collections ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_collection() {
        let pool = setup_test_db().await;
        let col = create_collection(&pool, "My Collection", Some("A test collection"))
            .await
            .unwrap();
        assert!(!col.id.is_empty());
        assert_eq!(col.name, "My Collection");
        assert_eq!(col.description.as_deref(), Some("A test collection"));
    }

    #[tokio::test]
    async fn test_get_all_collections() {
        let pool = setup_test_db().await;
        create_collection(&pool, "Collection A", None)
            .await
            .unwrap();
        create_collection(&pool, "Collection B", Some("Desc"))
            .await
            .unwrap();

        let all = get_all_collections(&pool).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_update_collection() {
        let pool = setup_test_db().await;
        let col = create_collection(&pool, "Old Name", None).await.unwrap();
        update_collection(&pool, &col.id, "New Name", Some("New desc"))
            .await
            .unwrap();

        let updated = get_collection_by_id(&pool, &col.id).await.unwrap().unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.description.as_deref(), Some("New desc"));
    }

    #[tokio::test]
    async fn test_delete_collection() {
        let pool = setup_test_db().await;
        let col = create_collection(&pool, "To Delete", None).await.unwrap();
        delete_collection(&pool, &col.id).await.unwrap();

        let retrieved = get_collection_by_id(&pool, &col.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_add_and_remove_skill_from_collection() {
        let pool = setup_test_db().await;
        let skill = make_skill("collection-skill", "Collection Skill", false);
        upsert_skill(&pool, &skill).await.unwrap();
        let col = create_collection(&pool, "Test Col", None).await.unwrap();

        add_skill_to_collection(&pool, &col.id, "collection-skill")
            .await
            .unwrap();

        let skills = get_collection_skills(&pool, &col.id).await.unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].id, "collection-skill");

        remove_skill_from_collection(&pool, &col.id, "collection-skill")
            .await
            .unwrap();

        let skills_after = get_collection_skills(&pool, &col.id).await.unwrap();
        assert!(skills_after.is_empty());
    }

    #[tokio::test]
    async fn test_add_skill_to_collection_is_idempotent() {
        let pool = setup_test_db().await;
        let skill = make_skill("idem-skill", "Idem Skill", false);
        upsert_skill(&pool, &skill).await.unwrap();
        let col = create_collection(&pool, "Idem Col", None).await.unwrap();

        add_skill_to_collection(&pool, &col.id, "idem-skill")
            .await
            .unwrap();
        add_skill_to_collection(&pool, &col.id, "idem-skill")
            .await
            .unwrap();

        let skills = get_collection_skills(&pool, &col.id).await.unwrap();
        assert_eq!(skills.len(), 1, "Duplicate add should be a no-op");
    }

    #[tokio::test]
    async fn test_delete_collection_also_removes_skill_memberships() {
        let pool = setup_test_db().await;
        let skill = make_skill("cascade-skill", "Cascade Skill", false);
        upsert_skill(&pool, &skill).await.unwrap();
        let col = create_collection(&pool, "Cascade Col", None).await.unwrap();
        add_skill_to_collection(&pool, &col.id, "cascade-skill")
            .await
            .unwrap();

        delete_collection(&pool, &col.id).await.unwrap();

        // The collection_skills row should also be gone
        let rows: Vec<_> = sqlx::query("SELECT * FROM collection_skills WHERE collection_id = ?")
            .bind(&col.id)
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(rows.is_empty(), "Memberships should be cascade-deleted");
    }

    // ── Skill Repositories and Tags ─────────────────────────────────────────

    #[tokio::test]
    async fn test_builtin_skill_metadata_seeded_and_idempotent() {
        let pool = setup_test_db().await;
        init_database(&pool).await.unwrap();

        let repositories = get_skill_repositories_with_stats(&pool).await.unwrap();
        assert!(repositories
            .iter()
            .any(|entry| entry.repository.id == LOCAL_UNKNOWN_REPOSITORY_ID));

        let tags = get_skill_tags(&pool).await.unwrap();
        assert!(
            tags.iter().any(|tag| tag.id == UNCATEGORIZED_TAG_ID),
            "uncategorized tag should be seeded"
        );
        assert_eq!(
            tags.iter().filter(|tag| tag.is_builtin).count(),
            builtin_skill_tags().len()
        );
    }

    #[tokio::test]
    async fn test_assign_github_repository_to_skill_records_source_path() {
        let pool = setup_test_db().await;
        let skill = make_skill("github-skill", "GitHub Skill", true);
        upsert_skill(&pool, &skill).await.unwrap();

        assign_github_repository_to_skill(
            &pool,
            "openai",
            "skills",
            "main",
            "https://github.com/openai/skills",
            "github-skill",
            "skills/.curated/github-skill",
        )
        .await
        .unwrap();

        let assignment = get_skill_repository_assignment(&pool, "github-skill")
            .await
            .unwrap();
        assert_eq!(assignment.repository.source_type, "github");
        assert_eq!(assignment.repository.owner.as_deref(), Some("openai"));
        assert_eq!(
            assignment.source_path.as_deref(),
            Some("skills/.curated/github-skill")
        );
        assert!(!assignment.is_source_unknown);
    }

    #[tokio::test]
    async fn test_unassigned_central_skill_uses_unknown_repository() {
        let pool = setup_test_db().await;
        let skill = make_skill("unknown-skill", "Unknown Skill", true);
        upsert_skill(&pool, &skill).await.unwrap();

        let assignment = get_skill_repository_assignment(&pool, "unknown-skill")
            .await
            .unwrap();
        assert_eq!(assignment.repository.id, LOCAL_UNKNOWN_REPOSITORY_ID);
        assert!(assignment.is_source_unknown);

        let repositories = get_skill_repositories_with_stats(&pool).await.unwrap();
        let unknown = repositories
            .iter()
            .find(|entry| entry.repository.id == LOCAL_UNKNOWN_REPOSITORY_ID)
            .unwrap();
        assert_eq!(unknown.unknown_skill_count, 1);
    }

    #[tokio::test]
    async fn test_assign_skill_tags_supports_multi_tag_binding() {
        let pool = setup_test_db().await;
        let skill = make_skill("tagged-skill", "Tagged Skill", true);
        upsert_skill(&pool, &skill).await.unwrap();
        let custom = create_skill_tag(&pool, "自定义标签", Some("custom"), Some("#111111"))
            .await
            .unwrap();

        assign_skill_tags(
            &pool,
            &["tagged-skill".to_string()],
            &[custom.id.clone(), UNCATEGORIZED_TAG_ID.to_string()],
            "manual",
            None,
            None,
        )
        .await
        .unwrap();

        let tags = get_skill_tags_for_skill(&pool, "tagged-skill")
            .await
            .unwrap();
        let ids = tags.iter().map(|tag| tag.id.as_str()).collect::<Vec<_>>();
        assert!(ids.contains(&custom.id.as_str()));
        assert!(ids.contains(&UNCATEGORIZED_TAG_ID));
    }

    #[tokio::test]
    async fn test_replace_skill_ai_tags_does_not_remove_manual_tags() {
        let pool = setup_test_db().await;
        let skill = make_skill("ai-tagged-skill", "AI Tagged Skill", true);
        upsert_skill(&pool, &skill).await.unwrap();
        let manual = create_skill_tag(&pool, "人工标签", None, None)
            .await
            .unwrap();

        assign_skill_tags(
            &pool,
            &["ai-tagged-skill".to_string()],
            &[manual.id.clone()],
            "manual",
            None,
            None,
        )
        .await
        .unwrap();
        replace_skill_ai_tags(
            &pool,
            "ai-tagged-skill",
            &[(UNCATEGORIZED_TAG_ID.to_string(), 0.7, "AI 建议".to_string())],
        )
        .await
        .unwrap();

        let tags = get_skill_tags_for_skill(&pool, "ai-tagged-skill")
            .await
            .unwrap();
        let ids = tags.iter().map(|tag| tag.id.as_str()).collect::<Vec<_>>();
        assert!(ids.contains(&manual.id.as_str()));
        assert!(ids.contains(&UNCATEGORIZED_TAG_ID));
    }

    // ── Scan Directories ──────────────────────────────────────────────────────

    /// Returns the number of *unique* global_skills_dir paths across all
    /// built-in agents. This is the number of rows that seed_builtin_scan_directories
    /// inserts, with Universal agents sharing ~/.agents/skills and Central using
    /// ~/.skillsmanage/skills.
    fn expected_builtin_scan_dir_count() -> usize {
        let mut paths = std::collections::HashSet::new();
        for agent in builtin_agents() {
            paths.insert(agent.global_skills_dir);
        }
        paths.len()
    }

    #[tokio::test]
    async fn test_builtin_scan_dirs_seeded() {
        let pool = setup_test_db().await;
        let dirs = get_scan_directories(&pool).await.unwrap();
        let builtin_count = expected_builtin_scan_dir_count();

        // Expect exactly one row per unique global_skills_dir across built-in agents.
        assert_eq!(
            dirs.len(),
            builtin_count,
            "Should have {} built-in scan directories after init (got {})",
            builtin_count,
            dirs.len()
        );

        // Every seeded row must be marked as built-in and active.
        for dir in &dirs {
            assert!(
                dir.is_builtin,
                "Seeded scan directory '{}' must have is_builtin=true",
                dir.path
            );
            assert!(
                dir.is_active,
                "Seeded scan directory '{}' must be active by default",
                dir.path
            );
        }

        // The paths must match the unique global_skills_dir values.
        let seeded_paths: std::collections::HashSet<&str> =
            dirs.iter().map(|d| d.path.as_str()).collect();
        for agent in builtin_agents() {
            assert!(
                seeded_paths.contains(agent.global_skills_dir.as_str()),
                "Built-in agent '{}' global_skills_dir '{}' must be in scan_directories",
                agent.id,
                agent.global_skills_dir
            );
        }
    }

    #[tokio::test]
    async fn test_builtin_scan_dirs_seeded_is_idempotent() {
        let pool = setup_test_db().await;
        // Second call to init_database must not create duplicate rows.
        init_database(&pool).await.unwrap();
        let dirs = get_scan_directories(&pool).await.unwrap();
        let builtin_count = expected_builtin_scan_dir_count();
        assert_eq!(
            dirs.len(),
            builtin_count,
            "Repeated init must not create duplicate scan directory rows"
        );
    }

    #[tokio::test]
    async fn test_reinit_updates_stale_builtin_agent_paths() {
        let pool = setup_test_db().await;
        sqlx::query(
            "UPDATE agents SET global_skills_dir = '/tmp/.agents/skills' WHERE id = 'central'",
        )
        .execute(&pool)
        .await
        .unwrap();

        init_database(&pool).await.unwrap();

        let central = get_agent_by_id(&pool, "central")
            .await
            .unwrap()
            .expect("central agent should exist");
        assert_eq!(
            central.global_skills_dir,
            crate::paths::central_skills_dir().to_string_lossy()
        );
    }

    #[tokio::test]
    async fn test_reinit_preserves_existing_builtin_agent_enabled_flags() {
        let pool = setup_test_db().await;

        sqlx::query(
            "UPDATE agents
             SET is_enabled = CASE id
               WHEN 'claude-code' THEN 0
               WHEN 'cursor' THEN 1
               ELSE is_enabled
             END",
        )
        .execute(&pool)
        .await
        .unwrap();

        init_database(&pool).await.unwrap();

        let claude_code = get_agent_by_id(&pool, "claude-code")
            .await
            .unwrap()
            .expect("claude-code should exist");
        let cursor = get_agent_by_id(&pool, "cursor")
            .await
            .unwrap()
            .expect("cursor should exist");

        assert!(!claude_code.is_enabled);
        assert!(cursor.is_enabled);
    }

    #[tokio::test]
    async fn test_reinit_replaces_stale_builtin_scan_directory_paths() {
        let pool = setup_test_db().await;
        sqlx::query("DELETE FROM scan_directories WHERE is_builtin = 1")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO scan_directories (path, label, is_active, is_builtin, added_at)
             VALUES ('/tmp/.agents/skills', 'Central Skills', 1, 1, ?)",
        )
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        init_database(&pool).await.unwrap();

        let dirs = get_scan_directories(&pool).await.unwrap();
        let central_path = crate::paths::central_skills_dir()
            .to_string_lossy()
            .into_owned();
        assert!(
            dirs.iter().any(|dir| dir.path == central_path),
            "reinit should seed the resolved central skills path"
        );
        assert!(
            !dirs.iter().any(|dir| dir.path == "/tmp/.agents/skills"),
            "stale /tmp builtin scan directory should be removed"
        );
    }

    #[tokio::test]
    async fn test_add_scan_directory() {
        let pool = setup_test_db().await;
        let dir = add_scan_directory(&pool, "/tmp/my-project", Some("My Project"))
            .await
            .unwrap();
        assert_eq!(dir.path, "/tmp/my-project");
        assert_eq!(dir.label.as_deref(), Some("My Project"));
        assert!(dir.is_active);
        assert!(!dir.is_builtin);
    }

    #[tokio::test]
    async fn test_get_scan_directories() {
        let pool = setup_test_db().await;
        add_scan_directory(&pool, "/tmp/dir-a", None).await.unwrap();
        add_scan_directory(&pool, "/tmp/dir-b", Some("Dir B"))
            .await
            .unwrap();

        let dirs = get_scan_directories(&pool).await.unwrap();
        // There are N built-in dirs (seeded on init) plus the 2 we just added.
        let builtin_count = expected_builtin_scan_dir_count();
        assert_eq!(dirs.len(), builtin_count + 2);

        // Verify the custom ones are present.
        let paths: Vec<&str> = dirs.iter().map(|d| d.path.as_str()).collect();
        assert!(paths.contains(&"/tmp/dir-a"));
        assert!(paths.contains(&"/tmp/dir-b"));
    }

    #[tokio::test]
    async fn test_remove_scan_directory() {
        let pool = setup_test_db().await;
        add_scan_directory(&pool, "/tmp/removable", None)
            .await
            .unwrap();
        remove_scan_directory(&pool, "/tmp/removable")
            .await
            .unwrap();

        let dirs = get_scan_directories(&pool).await.unwrap();
        // Built-in dirs remain; only the custom one is removed.
        let builtin_count = expected_builtin_scan_dir_count();
        assert_eq!(dirs.len(), builtin_count);
        assert!(!dirs.iter().any(|d| d.path == "/tmp/removable"));
    }

    #[tokio::test]
    async fn test_cannot_remove_builtin_scan_directory() {
        let pool = setup_test_db().await;
        // Manually insert a builtin directory
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO scan_directories (path, is_active, is_builtin, added_at)
             VALUES ('/builtin/path', 1, 1, ?)",
        )
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        let result = remove_scan_directory(&pool, "/builtin/path").await;
        assert!(
            result.is_err(),
            "Should not remove a builtin scan directory"
        );
    }

    #[tokio::test]
    async fn test_remove_nonexistent_scan_directory_returns_error() {
        let pool = setup_test_db().await;
        let result = remove_scan_directory(&pool, "/nonexistent/path").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_toggle_scan_directory() {
        let pool = setup_test_db().await;
        add_scan_directory(&pool, "/tmp/toggle-dir", None)
            .await
            .unwrap();
        toggle_scan_directory(&pool, "/tmp/toggle-dir", false)
            .await
            .unwrap();

        let dirs = get_scan_directories(&pool).await.unwrap();
        let dir = dirs.iter().find(|d| d.path == "/tmp/toggle-dir").unwrap();
        assert!(!dir.is_active);
    }

    // ── Settings ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_set_and_get_setting() {
        let pool = setup_test_db().await;
        set_setting(&pool, "theme", "dark").await.unwrap();
        let value = get_setting(&pool, "theme").await.unwrap();
        assert_eq!(value.as_deref(), Some("dark"));
    }

    #[tokio::test]
    async fn test_get_missing_setting_returns_none() {
        let pool = setup_test_db().await;
        let value = get_setting(&pool, "nonexistent_key").await.unwrap();
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_set_setting_upserts() {
        let pool = setup_test_db().await;
        set_setting(&pool, "lang", "en").await.unwrap();
        set_setting(&pool, "lang", "zh").await.unwrap();
        let value = get_setting(&pool, "lang").await.unwrap();
        assert_eq!(value.as_deref(), Some("zh"));
    }

    #[tokio::test]
    async fn test_multiple_settings() {
        let pool = setup_test_db().await;
        set_setting(&pool, "key1", "val1").await.unwrap();
        set_setting(&pool, "key2", "val2").await.unwrap();
        set_setting(&pool, "key3", "val3").await.unwrap();

        assert_eq!(
            get_setting(&pool, "key1").await.unwrap().as_deref(),
            Some("val1")
        );
        assert_eq!(
            get_setting(&pool, "key2").await.unwrap().as_deref(),
            Some("val2")
        );
        assert_eq!(
            get_setting(&pool, "key3").await.unwrap().as_deref(),
            Some("val3")
        );
    }

    // ── Migration: created_at ─────────────────────────────────────────────────

    /// Verifies that `init_database` adds the `created_at` column to an existing
    /// `skill_installations` table that was created with the old schema (before
    /// the column was introduced), and that existing rows are backfilled.
    #[tokio::test]
    async fn test_migration_adds_created_at_to_skill_installations() {
        // Create a fresh in-memory pool WITHOUT calling init_database first.
        let pool = SqlitePool::connect(":memory:")
            .await
            .expect("Failed to create in-memory SQLite pool");

        // Build the OLD skill_installations schema — no created_at column.
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS skill_installations (
                skill_id       TEXT NOT NULL,
                agent_id       TEXT NOT NULL,
                installed_path TEXT NOT NULL,
                link_type      TEXT NOT NULL,
                symlink_target TEXT,
                PRIMARY KEY (skill_id, agent_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create old skill_installations table");

        // Create the skills table so the FK-style relationship is consistent.
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS skills (
                id             TEXT PRIMARY KEY,
                name           TEXT NOT NULL,
                description    TEXT,
                file_path      TEXT NOT NULL,
                canonical_path TEXT,
                is_central     BOOLEAN NOT NULL DEFAULT 0,
                source         TEXT,
                content        TEXT,
                scanned_at     TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create skills table");

        // Insert a skill row (needed before the installation row references it).
        sqlx::query(
            "INSERT INTO skills (id, name, file_path, is_central, scanned_at)
             VALUES ('legacy-skill', 'Legacy Skill', '/tmp/legacy-skill/SKILL.md', 0, '2024-01-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("Failed to insert legacy skill");

        // Insert an installation row using the OLD schema (no created_at column).
        sqlx::query(
            "INSERT INTO skill_installations (skill_id, agent_id, installed_path, link_type)
             VALUES ('legacy-skill', 'claude-code', '/tmp/claude/legacy-skill', 'symlink')",
        )
        .execute(&pool)
        .await
        .expect("Failed to insert legacy skill_installations row");

        // Run init_database — should detect the missing created_at column and add it.
        init_database(&pool)
            .await
            .expect("init_database should succeed and apply the created_at migration");

        // Confirm the column now exists in PRAGMA table_info.
        let columns = sqlx::query("PRAGMA table_info(skill_installations)")
            .fetch_all(&pool)
            .await
            .expect("PRAGMA table_info should succeed");

        let has_created_at = columns.iter().any(|row| {
            row.try_get::<String, _>("name")
                .map(|name| name == "created_at")
                .unwrap_or(false)
        });
        assert!(
            has_created_at,
            "created_at column must exist in skill_installations after migration"
        );

        // Confirm that the pre-existing row has a non-empty created_at value
        // (backfilled by the DEFAULT (datetime('now')) expression).
        let row = sqlx::query(
            "SELECT created_at FROM skill_installations \
             WHERE skill_id = 'legacy-skill' AND agent_id = 'claude-code'",
        )
        .fetch_one(&pool)
        .await
        .expect("Pre-existing installation row should still be queryable after migration");

        let created_at: String = row
            .try_get("created_at")
            .expect("created_at should be readable from the pre-existing row");
        assert!(
            !created_at.is_empty(),
            "Pre-existing rows must have a non-empty created_at value after migration (got: '{}')",
            created_at
        );
    }

    /// Verifies that calling `init_database` on a fresh database (one that already
    /// includes created_at in the CREATE TABLE) does NOT trigger the ALTER TABLE
    /// migration path — i.e., the second `init_database` call is fully idempotent
    /// and does not fail.
    #[tokio::test]
    async fn test_migration_skipped_when_created_at_already_exists() {
        // setup_test_db calls init_database, which creates the table WITH created_at.
        let pool = setup_test_db().await;

        // A second call to init_database must succeed without error (idempotent).
        let result = init_database(&pool).await;
        assert!(
            result.is_ok(),
            "Second init_database should be idempotent when created_at already exists"
        );

        // Confirm created_at is still present and there's exactly one occurrence.
        let columns = sqlx::query("PRAGMA table_info(skill_installations)")
            .fetch_all(&pool)
            .await
            .unwrap();
        let created_at_count = columns
            .iter()
            .filter(|row| {
                row.try_get::<String, _>("name")
                    .map(|name| name == "created_at")
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(
            created_at_count, 1,
            "created_at column should appear exactly once after repeated init"
        );
    }
}
