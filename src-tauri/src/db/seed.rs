//! Built-in seed data and init dispatch — Phase 2d.
//!
//! Owns:
//! - `init_database` / `init_database_for_remote_home` / `init_database_with_agents`
//!   that schedule schema creation (in `super::schema::init`) followed by seeding.
//! - `seed_builtin_*` (agents / scan_directories / registries / skill_metadata).
//! - `builtin_*` data (agents per host home, builtin skill tags).
//!
//! Runtime CRUD lives under `super::repos::*` and is bridged at `db/mod.rs`.

use chrono::Utc;
use std::path::Path;

use super::types::*;

const DEFAULT_ENABLED_PLATFORM_IDS: [&str; 5] =
    ["claude-code", "codex", "gemini-cli", "opencode", "kiro"];

// ─── Init ─────────────────────────────────────────────────────────────────────

/// Initialize all database tables (idempotent) and seed built-in agents.
pub async fn init_database(pool: &DbPool) -> Result<(), String> {
    init_database_with_agents(pool, builtin_agents()).await
}

pub async fn init_database_for_remote_home(pool: &DbPool, remote_home: &str) -> Result<(), String> {
    init_database_with_agents(pool, builtin_agents_for_posix_home(remote_home)).await
}

async fn init_database_with_agents(
    pool: &DbPool,
    builtin_agents: Vec<Agent>,
) -> Result<(), String> {
    super::schema::init(pool).await?;

    // Seed built-in agents (INSERT OR IGNORE so repeated init is safe)
    seed_builtin_agents(pool, &builtin_agents).await?;

    // Seed built-in scan directories from the built-in agent registry.
    seed_builtin_scan_directories(pool, &builtin_agents).await?;

    // Seed built-in skill registries (marketplace sources)
    seed_builtin_registries(pool).await?;

    seed_builtin_skill_metadata(pool).await?;

    Ok(())
}

async fn seed_builtin_agents(pool: &DbPool, agents: &[Agent]) -> Result<(), String> {
    let builtin_ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();

    for agent in agents {
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
async fn seed_builtin_scan_directories(pool: &DbPool, agents: &[Agent]) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    for agent in agents {
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
    let builtin_paths: std::collections::HashSet<String> =
        agents.iter().map(|a| a.global_skills_dir.clone()).collect();
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

pub(crate) fn builtin_skill_tags() -> Vec<(&'static str, &'static str, &'static str, &'static str)>
{
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

/// Returns the list of built-in agents using the current user's home directory.
pub fn builtin_agents() -> Vec<Agent> {
    builtin_agents_for_home(&crate::paths::resolve_home_dir())
}

pub fn builtin_agents_for_posix_home(home: &str) -> Vec<Agent> {
    let home = normalize_posix_home(home);
    let central_skills_dir = posix_join(&home, &[".skillsmanage", "skills"]);
    let universal_skills_dir = posix_join(&home, &[".agents", "skills"]);

    let rewrite_path = |local_path: &str| -> String {
        let normalized = local_path.replace('\\', "/");
        if normalized.ends_with("/.skillsmanage/skills") {
            return central_skills_dir.clone();
        }
        if normalized.ends_with("/.agents/skills") {
            return universal_skills_dir.clone();
        }

        let Some(relative) = normalized
            .rsplit("/.")
            .next()
            .map(|suffix| format!(".{suffix}"))
        else {
            return normalized;
        };

        posix_join(&home, &[relative.as_str()])
    };

    builtin_agents_for_home(Path::new("/__skillport_remote_home__"))
        .into_iter()
        .map(|mut agent| {
            agent.global_skills_dir = rewrite_path(&agent.global_skills_dir);
            if let Some(project_skills_dir) = agent.project_skills_dir.as_deref() {
                agent.project_skills_dir = Some(project_skills_dir.replace('\\', "/"));
            }
            agent
        })
        .collect()
}

fn normalize_posix_home(home: &str) -> String {
    let mut value = home.trim().replace('\\', "/");
    while value.len() > 1 && value.ends_with('/') {
        value.pop();
    }
    if value.is_empty() {
        "/".to_string()
    } else {
        value
    }
}

fn posix_join(home: &str, segments: &[&str]) -> String {
    let mut value = normalize_posix_home(home);
    for segment in segments {
        let segment = segment.trim_matches('/');
        if segment.is_empty() {
            continue;
        }
        if value == "/" {
            value.push_str(segment);
        } else {
            value.push('/');
            value.push_str(segment);
        }
    }
    value
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
            project_skills_dir: Some(".claude/skills".to_string()),
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
            project_skills_dir: Some(".codex/skills".to_string()),
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
            project_skills_dir: Some(".cursor/skills".to_string()),
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
            project_skills_dir: Some(".gemini/skills".to_string()),
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
            project_skills_dir: Some(".opencode/skills".to_string()),
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
            project_skills_dir: Some(".kiro/skills".to_string()),
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
