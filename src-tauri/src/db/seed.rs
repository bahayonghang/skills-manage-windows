//! Built-in seed data and init dispatch; runtime CRUD lives under `super::repos::*`.

mod agents;
mod skill_tags;

use chrono::Utc;

use super::types::*;

pub use agents::{
    builtin_agents, builtin_agents_for_posix_home, is_universal_agent, is_universal_project_agent,
};

#[cfg(test)]
pub(crate) use skill_tags::builtin_skill_tags;

// ─── Init ─────────────────────────────────────────────────────────────────────

/// Initialize all database tables (idempotent) and seed built-in agents.
pub async fn init_database(pool: &DbPool) -> Result<(), sqlx::Error> {
    super::migrations::initialize_pool(pool, builtin_agents()).await
}

pub async fn init_database_for_remote_home(
    pool: &DbPool,
    remote_home: &str,
) -> Result<(), sqlx::Error> {
    super::migrations::initialize_pool(pool, builtin_agents_for_posix_home(remote_home)).await
}

pub(crate) async fn seed_database_with_agents(
    pool: &DbPool,
    builtin_agents: &[Agent],
) -> Result<(), sqlx::Error> {
    // Seed built-in agents (INSERT OR IGNORE so repeated init is safe)
    seed_builtin_agents(pool, builtin_agents).await?;

    // Seed built-in scan directories from the DB rows after upsert so user
    // customizations that are intentionally preserved (Central store path)
    // remain the source of truth on the next startup.
    seed_builtin_scan_directories(pool).await?;

    // Seed built-in skill registries (marketplace sources)
    seed_builtin_registries(pool).await?;

    seed_builtin_skill_metadata(pool).await?;

    Ok(())
}

async fn seed_builtin_agents(pool: &DbPool, agents: &[Agent]) -> Result<(), sqlx::Error> {
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
              global_skills_dir = CASE
                WHEN agents.id = 'central' THEN agents.global_skills_dir
                ELSE excluded.global_skills_dir
              END,
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
        .await?;
    }

    // Remove builtin agents that no longer exist in code
    let all_db_agents: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM agents WHERE is_builtin = 1")
            .fetch_all(pool)
            .await?;

    for (id,) in &all_db_agents {
        if !builtin_ids.contains(&id.as_str()) {
            sqlx::query("DELETE FROM agents WHERE id = ? AND is_builtin = 1")
                .bind(id)
                .execute(pool)
                .await?;
        }
    }

    Ok(())
}

/// Seed `scan_directories` with one row per unique DB built-in
/// `global_skills_dir` path. Rows are marked `is_builtin = 1` and cannot be
/// removed by the user. Reading from DB rather than the static registry keeps a
/// user-selected Central store path from being reset by startup seeding.
async fn seed_builtin_scan_directories(pool: &DbPool) -> Result<(), sqlx::Error> {
    let agents: Vec<Agent> = sqlx::query_as("SELECT * FROM agents WHERE is_builtin = 1")
        .fetch_all(pool)
        .await?;
    let now = Utc::now().to_rfc3339();
    for agent in &agents {
        sqlx::query(
            "INSERT OR IGNORE INTO scan_directories
             (path, label, is_active, is_builtin, added_at)
             VALUES (?, ?, 1, 1, ?)",
        )
        .bind(&agent.global_skills_dir)
        .bind(&agent.display_name)
        .bind(&now)
        .execute(pool)
        .await?;
    }

    // Remove builtin scan directories that no longer exist in code
    let builtin_paths: std::collections::HashSet<String> =
        agents.iter().map(|a| a.global_skills_dir.clone()).collect();
    let all_db_dirs: Vec<(String,)> =
        sqlx::query_as("SELECT path FROM scan_directories WHERE is_builtin = 1")
            .fetch_all(pool)
            .await?;
    for (path,) in &all_db_dirs {
        if !builtin_paths.contains(path) {
            sqlx::query("DELETE FROM scan_directories WHERE path = ? AND is_builtin = 1")
                .bind(path)
                .execute(pool)
                .await?;
        }
    }

    Ok(())
}

async fn seed_builtin_registries(pool: &DbPool) -> Result<(), sqlx::Error> {
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
        .await?;
    }
    Ok(())
}

async fn seed_builtin_skill_metadata(pool: &DbPool) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO skill_repositories
         (id, name, source_type, owner, repo, branch, url, pinned, is_unknown, created_at, updated_at)
         VALUES (?, ?, 'local', NULL, NULL, NULL, NULL, 0, 1, ?, ?)
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
    ?;

    skill_tags::seed_builtin_skill_tags(pool, &now).await?;

    Ok(())
}
