//! Agents table CRUD — Phase 2c.
//!
//! Built-in agent seeding lives in `db/legacy.rs::seed_builtin_agents`
//! (created by migration 1 and populated by the post-migration seed step).
//! This file owns the runtime CRUD that `commands/agents.rs` calls.

use crate::db::types::{Agent, DbPool};

/// Retrieve all agents.
pub async fn get_all_agents(pool: &DbPool) -> Result<Vec<Agent>, sqlx::Error> {
    sqlx::query_as::<_, Agent>("SELECT * FROM agents ORDER BY is_builtin DESC, display_name")
        .fetch_all(pool)
        .await
}

/// Retrieve a single agent by ID.
pub async fn get_agent_by_id(pool: &DbPool, agent_id: &str) -> Result<Option<Agent>, sqlx::Error> {
    sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE id = ?")
        .bind(agent_id)
        .fetch_optional(pool)
        .await
}

/// Update the `is_detected` flag for an agent.
pub async fn update_agent_detected(
    pool: &DbPool,
    agent_id: &str,
    is_detected: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE agents SET is_detected = ? WHERE id = ?")
        .bind(is_detected)
        .bind(agent_id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Update the `is_enabled` flag for an agent and return the refreshed row.
pub async fn update_agent_enabled(
    pool: &DbPool,
    agent_id: &str,
    is_enabled: bool,
) -> Result<Agent, sqlx::Error> {
    let existing = get_agent_by_id(pool, agent_id).await?;
    if existing.is_none() {
        return Err(sqlx::Error::InvalidArgument(format!(
            "Agent '{}' not found",
            agent_id
        )));
    }

    sqlx::query("UPDATE agents SET is_enabled = ? WHERE id = ?")
        .bind(is_enabled)
        .bind(agent_id)
        .execute(pool)
        .await?;

    get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| sqlx::Error::InvalidArgument("Failed to retrieve updated agent".to_string()))
}

/// Insert a new custom agent (non-builtin).
pub async fn insert_custom_agent(pool: &DbPool, agent: &Agent) -> Result<(), sqlx::Error> {
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
}

/// Delete a custom (non-builtin) agent by ID. Returns an error if the agent is builtin.
pub async fn delete_custom_agent(pool: &DbPool, agent_id: &str) -> Result<(), sqlx::Error> {
    let agent = get_agent_by_id(pool, agent_id).await?;
    match agent {
        None => Err(sqlx::Error::InvalidArgument(format!(
            "Agent '{}' not found",
            agent_id
        ))),
        Some(a) if a.is_builtin => Err(sqlx::Error::InvalidArgument(format!(
            "Cannot delete built-in agent '{}'",
            agent_id
        ))),
        Some(_) => sqlx::query("DELETE FROM agents WHERE id = ?")
            .bind(agent_id)
            .execute(pool)
            .await
            .map(|_| ()),
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
) -> Result<Agent, sqlx::Error> {
    let agent = get_agent_by_id(pool, agent_id).await?;
    match agent {
        None => {
            return Err(sqlx::Error::InvalidArgument(format!(
                "Agent '{}' not found",
                agent_id
            )))
        }
        Some(a) if a.is_builtin => {
            return Err(sqlx::Error::InvalidArgument(format!(
                "Cannot update built-in agent '{}'",
                agent_id
            )))
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
    .await?;

    get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| sqlx::Error::InvalidArgument("Failed to retrieve updated agent".to_string()))
}
