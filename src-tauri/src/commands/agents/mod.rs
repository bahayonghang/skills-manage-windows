use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::db::{self, Agent, DbPool};
use crate::observability::{
    CommandLogPolicy, OperationContext, OperationSubjectKind, OperationTarget, OperationTargetKind,
    ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeIdentifier, SafeOperationResult,
};
use crate::paths::{
    expand_home_path, expand_remote_home_path, path_to_string, platform_global_skills_dir,
    platform_global_skills_dir_for_remote, platform_project_skills_dir,
    platform_project_skills_dir_for_remote, PlatformPathSpec,
};
use crate::targets::{connect_remote_target, remote_parent, ActiveTarget, ConnectedRemoteTarget};
use crate::AppState;

/// An agent enriched with a live `is_detected` flag derived from the file
/// system at query time, rather than from the last scan run.
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWithStatus {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub global_skills_dir: String,
    pub project_skills_dir: Option<String>,
    pub icon_name: Option<String>,
    /// `true` if the agent is considered "installed" on this machine.
    /// Detected by checking whether `global_skills_dir` exists or its parent
    /// directory exists.
    pub is_detected: bool,
    pub is_builtin: bool,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedPlatformPaths {
    pub global_skills_dir: String,
    pub project_skills_dir: Option<String>,
}

fn agent_path_specs(agents: &[Agent]) -> Vec<PlatformPathSpec<'_>> {
    agents
        .iter()
        .map(|agent| PlatformPathSpec {
            agent_id: agent.id.as_str(),
            global_skills_dir: agent.global_skills_dir.as_str(),
            project_skills_dir: agent.project_skills_dir.as_deref(),
        })
        .collect()
}

fn resolved_paths_for_agent(
    agent: &Agent,
    specs: &[PlatformPathSpec<'_>],
    remote_home: Option<&str>,
) -> Result<ResolvedPlatformPaths, String> {
    let (global_skills_dir, project_skills_dir) = match remote_home {
        Some(home) => (
            platform_global_skills_dir_for_remote(&agent.id, specs, home)
                .map_err(|e| e.to_string())?,
            platform_project_skills_dir_for_remote(&agent.id, specs, home)
                .map_err(|e| e.to_string())?,
        ),
        None => (
            path_to_string(
                &platform_global_skills_dir(&agent.id, specs).map_err(|e| e.to_string())?,
            ),
            platform_project_skills_dir(&agent.id, specs)
                .map_err(|e| e.to_string())?
                .map(|path| path_to_string(&path)),
        ),
    };

    Ok(ResolvedPlatformPaths {
        global_skills_dir,
        project_skills_dir,
    })
}

pub async fn list_platform_paths_impl(
    pool: &DbPool,
    remote_home: Option<&str>,
) -> Result<std::collections::HashMap<String, ResolvedPlatformPaths>, String> {
    let agents = db::get_all_agents(pool).await.map_err(|e| e.to_string())?;
    let specs = agent_path_specs(&agents);
    let mut paths = std::collections::HashMap::with_capacity(agents.len());

    for agent in &agents {
        paths.insert(
            agent.id.clone(),
            resolved_paths_for_agent(agent, &specs, remote_home)?,
        );
    }

    Ok(paths)
}

/// Payload for registering a new user-defined agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAgentConfig {
    /// Optional explicit ID. If omitted or empty, one is derived from
    /// `display_name`.
    pub id: Option<String>,
    /// Human-readable name shown in the UI.
    pub display_name: String,
    /// Agent category - "coding", "lobster", or "other".
    pub category: Option<String>,
    /// Absolute path to the agent's global skills directory.
    pub global_skills_dir: String,
}

/// Payload for updating an existing user-defined agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCustomAgentConfig {
    /// Human-readable name shown in the UI.
    pub display_name: String,
    /// Agent category - "coding", "lobster", or "other".
    pub category: Option<String>,
    /// Absolute path to the agent's global skills directory.
    pub global_skills_dir: String,
}

/// Returns `true` if the agent appears to be installed on the current machine.
///
/// An agent is considered detected if:
/// - Its `global_skills_dir` exists, or
/// - The parent of `global_skills_dir` exists.
pub fn is_agent_detected(global_skills_dir: &str) -> bool {
    let dir = Path::new(global_skills_dir);
    if dir.exists() {
        return true;
    }
    dir.parent().is_some_and(|p| p.exists())
}

fn agent_to_with_status(agent: Agent) -> AgentWithStatus {
    let is_detected = is_agent_detected(&agent.global_skills_dir);
    agent_to_with_status_with_detected(agent, is_detected)
}

fn agent_to_cached_status(agent: Agent) -> AgentWithStatus {
    let is_detected = agent.is_detected;
    agent_to_with_status_with_detected(agent, is_detected)
}

fn agent_to_with_status_with_detected(agent: Agent, is_detected: bool) -> AgentWithStatus {
    AgentWithStatus {
        id: agent.id,
        display_name: agent.display_name,
        category: agent.category,
        global_skills_dir: agent.global_skills_dir,
        project_skills_dir: agent.project_skills_dir,
        icon_name: agent.icon_name,
        is_detected,
        is_builtin: agent.is_builtin,
        is_enabled: agent.is_enabled,
    }
}

/// Return all agents from the DB with live detection status.
pub async fn get_agents_impl(pool: &DbPool) -> Result<Vec<AgentWithStatus>, String> {
    let agents = db::get_all_agents(pool).await.map_err(|e| e.to_string())?;
    Ok(agents.into_iter().map(agent_to_with_status).collect())
}

pub async fn get_agents_cached_impl(pool: &DbPool) -> Result<Vec<AgentWithStatus>, String> {
    let agents = db::get_all_agents(pool).await.map_err(|e| e.to_string())?;
    Ok(agents.into_iter().map(agent_to_cached_status).collect())
}

/// Scan the filesystem to update each agent's `is_detected` flag, then return
/// all agents with their refreshed status.
pub async fn detect_agents_impl(pool: &DbPool) -> Result<Vec<AgentWithStatus>, String> {
    let agents = db::get_all_agents(pool).await.map_err(|e| e.to_string())?;
    let mut result = Vec::with_capacity(agents.len());

    for agent in agents {
        let is_detected = is_agent_detected(&agent.global_skills_dir);
        if db::update_agent_detected(pool, &agent.id, is_detected)
            .await
            .is_err()
        {
            tracing::warn!(
                target: "skillport::agent",
                code = "agent.detection_persist_failed",
                phase = "database",
                "Agent detection state could not be persisted"
            );
        }
        result.push(agent_to_with_status_with_detected(agent, is_detected));
    }

    Ok(result)
}

async fn is_remote_agent_detected(
    connection: &ConnectedRemoteTarget,
    global_skills_dir: &str,
) -> Result<bool, crate::targets::TargetsError> {
    if connection.exists(global_skills_dir).await? {
        return Ok(true);
    }

    let Some(parent) = remote_parent(global_skills_dir) else {
        return Ok(false);
    };
    connection.exists(&parent).await
}

pub async fn detect_remote_agents_impl(
    pool: &DbPool,
    active_target: &ActiveTarget,
) -> Result<Vec<AgentWithStatus>, String> {
    let agents = db::get_all_agents(pool).await.map_err(|e| e.to_string())?;
    let connection = connect_remote_target(active_target)
        .await
        .map_err(|e| e.to_string())?;
    let mut result = Vec::with_capacity(agents.len());

    for agent in agents {
        let is_detected = is_remote_agent_detected(&connection, &agent.global_skills_dir)
            .await
            .map_err(|e| e.to_string())?;
        if db::update_agent_detected(pool, &agent.id, is_detected)
            .await
            .is_err()
        {
            tracing::warn!(
                target: "skillport::agent",
                code = "agent.detection_persist_failed",
                phase = "database",
                "Agent detection state could not be persisted"
            );
        }
        result.push(agent_to_with_status_with_detected(agent, is_detected));
    }

    Ok(result)
}

/// Insert a new user-defined agent and return its representation.
pub async fn add_custom_agent_impl(
    pool: &DbPool,
    config: CustomAgentConfig,
) -> Result<AgentWithStatus, String> {
    add_custom_agent_impl_for_home(pool, config, None).await
}

async fn add_custom_agent_impl_for_home(
    pool: &DbPool,
    config: CustomAgentConfig,
    remote_home: Option<&str>,
) -> Result<AgentWithStatus, String> {
    let id = match config.id.as_deref() {
        Some(s) if !s.trim().is_empty() => s.trim().to_lowercase().replace(' ', "-"),
        _ => {
            let slug = config
                .display_name
                .trim()
                .to_lowercase()
                .replace(' ', "-")
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect::<String>();
            if slug.is_empty() {
                format!("custom-{}", Uuid::new_v4())
            } else {
                format!("custom-{}", slug)
            }
        }
    };

    if id.is_empty() {
        return Err("Agent ID cannot be empty".to_string());
    }

    let category = config.category.unwrap_or_else(|| "other".to_string());
    let global_skills_dir = expand_agent_skills_dir(&config.global_skills_dir, remote_home);

    let agent = Agent {
        id: id.clone(),
        display_name: config.display_name,
        category,
        global_skills_dir,
        project_skills_dir: None,
        icon_name: None,
        is_detected: false,
        is_builtin: false,
        is_enabled: true,
    };

    db::insert_custom_agent(pool, &agent)
        .await
        .map_err(|e| e.to_string())?;

    let persisted = db::get_agent_by_id(pool, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Failed to retrieve newly created agent".to_string())?;

    Ok(agent_to_with_status(persisted))
}

/// Update an existing user-defined (non-builtin) agent and return its updated representation.
pub async fn update_custom_agent_impl(
    pool: &DbPool,
    agent_id: &str,
    config: UpdateCustomAgentConfig,
) -> Result<AgentWithStatus, String> {
    update_custom_agent_impl_for_home(pool, agent_id, config, None).await
}

async fn update_custom_agent_impl_for_home(
    pool: &DbPool,
    agent_id: &str,
    config: UpdateCustomAgentConfig,
    remote_home: Option<&str>,
) -> Result<AgentWithStatus, String> {
    if config.display_name.trim().is_empty() {
        return Err("Agent display name cannot be empty".to_string());
    }
    if config.global_skills_dir.trim().is_empty() {
        return Err("Agent global skills directory cannot be empty".to_string());
    }

    let category = config.category.unwrap_or_else(|| "other".to_string());
    let global_skills_dir = expand_agent_skills_dir(config.global_skills_dir.trim(), remote_home);

    let updated = db::update_custom_agent(
        pool,
        agent_id,
        config.display_name.trim(),
        &category,
        &global_skills_dir,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(agent_to_with_status(updated))
}

fn expand_agent_skills_dir(path: &str, remote_home: Option<&str>) -> String {
    match remote_home {
        Some(home) => expand_remote_home_path(path, home),
        None => path_to_string(&expand_home_path(path)),
    }
}

/// Remove a user-defined (non-builtin) agent by ID.
pub async fn remove_custom_agent_impl(pool: &DbPool, agent_id: &str) -> Result<(), String> {
    db::delete_custom_agent(pool, agent_id)
        .await
        .map_err(|e| e.to_string())
}

/// Update the enabled state for any agent and return the refreshed representation.
pub async fn set_agent_enabled_impl(
    pool: &DbPool,
    agent_id: &str,
    is_enabled: bool,
) -> Result<AgentWithStatus, String> {
    let updated = db::update_agent_enabled(pool, agent_id, is_enabled)
        .await
        .map_err(|e| e.to_string())?;
    Ok(agent_to_with_status(updated))
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn get_agents(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<AgentWithStatus>> {
    crate::ipc_boundary!(
        "get_agents",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            match active_target {
                ActiveTarget::Local => get_agents_impl(&pool).await,
                ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => get_agents_cached_impl(&pool).await,
            }
        }
        .await
    )
}

#[tauri::command]
pub async fn list_platform_paths(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<std::collections::HashMap<String, ResolvedPlatformPaths>> {
    crate::ipc_boundary!(
        "list_platform_paths",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let remote_home = active_target.remote_home();
            list_platform_paths_impl(&pool, remote_home).await
        }
        .await
    )
}

#[tauri::command]
pub async fn detect_agents(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<AgentWithStatus>> {
    crate::ipc_boundary_async!("detect_agents", {
        let request_context = state.resolve_target_context().await?;
        let active_target = request_context.target().clone();
        let audit_target = match &active_target {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("detect_agents")
            .expect("detect_agents must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("detect_agents must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(audit_target),
            |agents: &Vec<AgentWithStatus>| {
                SafeOperationResult::succeeded("Agent detection completed.")
                    .count(SafeDetailKey::AffectedCount, agents.len() as u64)
            },
            || async move {
                let result = match active_target {
                    ActiveTarget::Local => detect_agents_impl(&pool).await,
                    ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                        detect_remote_agents_impl(&pool, &active_target).await
                    }
                };
                result.map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
pub async fn add_custom_agent(
    state: State<'_, AppState>,
    config: CustomAgentConfig,
) -> crate::ipc_error::IpcResult<AgentWithStatus> {
    crate::ipc_boundary_async!("add_custom_agent", {
        let request_context = state.resolve_target_context().await?;
        let active_target = request_context.target().clone();
        let audit_target = match &active_target {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("add_custom_agent")
            .expect("add_custom_agent must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("add_custom_agent must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(audit_target),
            |agent: &AgentWithStatus| {
                SafeOperationResult::succeeded("Custom agent added.")
                    .identifier(SafeDetailKey::Identifier, SafeIdentifier::new(&agent.id))
            },
            || async move {
                add_custom_agent_impl_for_home(&pool, config, active_target.remote_home())
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
pub async fn update_custom_agent(
    state: State<'_, AppState>,
    agent_id: String,
    config: UpdateCustomAgentConfig,
) -> crate::ipc_error::IpcResult<AgentWithStatus> {
    crate::ipc_boundary_async!("update_custom_agent", {
        let request_context = state.resolve_target_context().await?;
        let active_target = request_context.target().clone();
        let audit_target = match &active_target {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("update_custom_agent")
            .expect("update_custom_agent must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("update_custom_agent must be auditable")
        };
        let context = OperationContext::new(audit_target)
            .subject(OperationSubjectKind::Agent, SafeIdentifier::new(&agent_id));
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| SafeOperationResult::succeeded("Custom agent updated."),
            || async move {
                update_custom_agent_impl_for_home(
                    &pool,
                    &agent_id,
                    config,
                    active_target.remote_home(),
                )
                .await
                .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
pub async fn remove_custom_agent(
    state: State<'_, AppState>,
    agent_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("remove_custom_agent", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("remove_custom_agent")
            .expect("remove_custom_agent must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("remove_custom_agent must be auditable")
        };
        let context = OperationContext::new(audit_target)
            .subject(OperationSubjectKind::Agent, SafeIdentifier::new(&agent_id));
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| SafeOperationResult::succeeded("Custom agent removed."),
            || async move {
                remove_custom_agent_impl(&pool, &agent_id)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
pub async fn set_agent_enabled(
    state: State<'_, AppState>,
    agent_id: String,
    is_enabled: bool,
) -> crate::ipc_error::IpcResult<AgentWithStatus> {
    crate::ipc_boundary_async!("set_agent_enabled", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("set_agent_enabled")
            .expect("set_agent_enabled must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("set_agent_enabled must be auditable")
        };
        let context = OperationContext::new(audit_target)
            .subject(OperationSubjectKind::Agent, SafeIdentifier::new(&agent_id));
        let mode = if is_enabled { "enabled" } else { "disabled" };
        crate::observability::run_operation(
            &state,
            definition,
            context,
            move |_| {
                SafeOperationResult::succeeded("Agent enabled state updated.")
                    .stable(SafeDetailKey::Mode, mode)
            },
            || async move {
                set_agent_enabled_impl(&pool, &agent_id, is_enabled)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[cfg(test)]
mod tests;
