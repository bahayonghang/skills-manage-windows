use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::State;

use crate::db::{self, Collection, DbPool, SkillForAgent, SkillRepository, SkillTag};
use crate::targets::{connect_ssh_target, ActiveTarget, ConnectedSshTarget, RemoteTargetConfig};
use crate::AppState;

// ─── Types ────────────────────────────────────────────────────────────────────

/// A Central Skill with a list of agent IDs that currently have this skill
/// installed (via symlink or copy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillWithLinks {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    pub canonical_path: Option<String>,
    pub is_central: bool,
    pub source: Option<String>,
    pub scanned_at: String,
    pub created_at: String,
    pub updated_at: String,
    /// Agent IDs that have an installation record for this skill.
    pub linked_agents: Vec<String>,
    /// Agent IDs that use the Central skills directory as their own root.
    pub shared_root_agents: Vec<String>,
    pub repository: Option<SkillRepository>,
    pub tags: Vec<SkillTag>,
    pub source_path: Option<String>,
    pub is_source_unknown: bool,
}

/// An installation record enriched with the `installed_at` timestamp for
/// the skill detail IPC response. This is the frontend-facing version of
/// `db::SkillInstallation` — `created_at` from the DB is exposed as
/// `installed_at` for clarity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInstallationDetail {
    pub skill_id: String,
    pub agent_id: String,
    pub installed_path: String,
    pub link_type: String,
    pub symlink_target: Option<String>,
    /// ISO 8601 timestamp of when the skill was first installed.
    pub installed_at: String,
}

/// A skill with full installation details across all platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDetail {
    pub id: String,
    pub row_id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    pub dir_path: String,
    pub canonical_path: Option<String>,
    pub is_central: bool,
    pub source: Option<String>,
    pub scanned_at: String,
    pub source_kind: Option<String>,
    pub source_root: Option<String>,
    pub is_read_only: bool,
    pub conflict_group: Option<String>,
    pub conflict_count: i64,
    /// All installation records for this skill across agents.
    pub installations: Vec<SkillInstallationDetail>,
    /// Collections this skill currently belongs to.
    pub collections: Vec<Collection>,
    pub repository: Option<SkillRepository>,
    pub tags: Vec<SkillTag>,
    pub source_path: Option<String>,
    pub is_source_unknown: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteCentralSkillResult {
    pub removed_central_path: String,
    pub removed_agent_ids: Vec<String>,
    pub retained_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteCentralSkillPreview {
    pub skill_id: String,
    pub skill_name: String,
    pub central_path: String,
    pub copy_installations: Vec<SkillInstallationDetail>,
    pub auto_removed_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedCentralSkillDelete {
    pub skill_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteCentralSkillPreviewResult {
    pub previews: Vec<DeleteCentralSkillPreview>,
    pub failed: Vec<FailedCentralSkillDelete>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteCentralSkillRequest {
    pub skill_id: String,
    pub remove_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteCentralSkillSuccess {
    pub skill_id: String,
    pub removed_central_path: String,
    pub removed_agent_ids: Vec<String>,
    pub retained_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteCentralSkillResult {
    pub succeeded: Vec<BatchDeleteCentralSkillSuccess>,
    pub failed: Vec<FailedCentralSkillDelete>,
}

// ─── Tauri Commands ───────────────────────────────────────────────────────────

fn system_time_to_rfc3339(time: SystemTime) -> String {
    let datetime: DateTime<Utc> = time.into();
    datetime.to_rfc3339()
}

fn skill_filesystem_timestamps(skill: &db::Skill) -> (String, String) {
    let directory_metadata = skill
        .canonical_path
        .as_deref()
        .and_then(|path| std::fs::metadata(path).ok());
    let file_metadata = std::fs::metadata(&skill.file_path).ok();

    let created_at = directory_metadata
        .as_ref()
        .or(file_metadata.as_ref())
        .and_then(|metadata| metadata.created().ok())
        .map(system_time_to_rfc3339)
        .unwrap_or_else(|| skill.scanned_at.clone());

    let updated_at = file_metadata
        .as_ref()
        .or(directory_metadata.as_ref())
        .and_then(|metadata| metadata.modified().ok())
        .map(system_time_to_rfc3339)
        .unwrap_or_else(|| skill.scanned_at.clone());

    (created_at, updated_at)
}

fn skill_dir_path(skill: &db::Skill) -> String {
    skill
        .canonical_path
        .clone()
        .or_else(|| {
            Path::new(&skill.file_path)
                .parent()
                .map(|path| path.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| skill.file_path.clone())
}

fn claude_conflict_group(agent_id: &str, skill_id: &str) -> String {
    format!("{agent_id}::{skill_id}")
}

fn claude_conflict_counts(observations: &[db::AgentSkillObservation]) -> HashMap<String, i64> {
    let mut counts = HashMap::new();
    for observation in observations {
        *counts.entry(observation.skill_id.clone()).or_insert(0) += 1;
    }
    counts
}

fn claude_conflict_metadata(
    agent_id: &str,
    skill_id: &str,
    counts: &HashMap<String, i64>,
) -> (Option<String>, i64) {
    let count = counts.get(skill_id).copied().unwrap_or(0);
    if count > 1 {
        (Some(claude_conflict_group(agent_id, skill_id)), count)
    } else {
        (None, 0)
    }
}

fn installation_details(installations: Vec<db::SkillInstallation>) -> Vec<SkillInstallationDetail> {
    installations
        .into_iter()
        .map(|i| SkillInstallationDetail {
            skill_id: i.skill_id,
            agent_id: i.agent_id,
            installed_path: i.installed_path,
            link_type: i.link_type,
            symlink_target: i.symlink_target,
            installed_at: i.created_at,
        })
        .collect()
}

async fn get_claude_observation_detail(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
    row_id: Option<&str>,
) -> Result<Option<SkillDetail>, String> {
    if agent_id != "claude-code" {
        return Ok(None);
    }

    let observations = db::get_agent_skill_observations(pool, agent_id).await?;
    if observations.is_empty() {
        return Ok(None);
    }

    let conflict_counts = claude_conflict_counts(&observations);
    let matches: Vec<db::AgentSkillObservation> = observations
        .into_iter()
        .filter(|observation| observation.skill_id == skill_id)
        .collect();

    if matches.is_empty() {
        return Ok(None);
    }

    let observation = match row_id {
        Some(row_id) => matches
            .into_iter()
            .find(|observation| observation.row_id == row_id)
            .ok_or_else(|| format!("Claude row '{}' not found for skill '{}'", row_id, skill_id))?,
        None if matches.len() == 1 => matches.into_iter().next().expect("single match"),
        None => {
            return Err(format!(
                "Multiple Claude rows found for skill '{}'; row_id is required",
                skill_id
            ))
        }
    };

    let manageable_skill = db::get_skill_by_id(pool, &observation.skill_id).await?;
    let installations = if observation.is_read_only {
        Vec::new()
    } else {
        installation_details(db::get_skill_installations(pool, &observation.skill_id).await?)
    };
    let collections = if observation.is_read_only {
        Vec::new()
    } else {
        db::get_skill_collections(pool, &observation.skill_id).await?
    };
    let repository_assignment = if observation.is_read_only {
        None
    } else {
        Some(db::get_skill_repository_assignment(pool, &observation.skill_id).await?)
    };
    let tags = if observation.is_read_only {
        Vec::new()
    } else {
        db::get_skill_tags_for_skill(pool, &observation.skill_id).await?
    };
    let (conflict_group, conflict_count) =
        claude_conflict_metadata(agent_id, &observation.skill_id, &conflict_counts);

    Ok(Some(SkillDetail {
        row_id: observation.row_id,
        id: observation.skill_id.clone(),
        name: observation.name,
        description: observation.description.or_else(|| {
            manageable_skill
                .as_ref()
                .and_then(|skill| skill.description.clone())
        }),
        file_path: observation.file_path,
        dir_path: observation.dir_path,
        canonical_path: if observation.is_read_only {
            None
        } else {
            manageable_skill
                .as_ref()
                .and_then(|skill| skill.canonical_path.clone())
        },
        is_central: manageable_skill
            .as_ref()
            .map(|skill| skill.is_central)
            .unwrap_or(false),
        source: manageable_skill
            .as_ref()
            .and_then(|skill| skill.source.clone())
            .or_else(|| Some(observation.link_type.clone())),
        scanned_at: observation.scanned_at,
        source_kind: Some(observation.source_kind),
        source_root: Some(observation.source_root),
        is_read_only: observation.is_read_only,
        conflict_group,
        conflict_count,
        installations,
        collections,
        repository: repository_assignment
            .as_ref()
            .map(|assignment| assignment.repository.clone()),
        tags,
        source_path: repository_assignment
            .as_ref()
            .and_then(|assignment| assignment.source_path.clone()),
        is_source_unknown: repository_assignment
            .as_ref()
            .map(|assignment| assignment.is_source_unknown)
            .unwrap_or(true),
    }))
}

async fn get_skill_detail_with_row_impl(
    pool: &DbPool,
    skill_id: &str,
    agent_id: Option<&str>,
    row_id: Option<&str>,
) -> Result<SkillDetail, String> {
    if let Some(agent_id) = agent_id {
        if let Some(detail) =
            get_claude_observation_detail(pool, skill_id, agent_id, row_id).await?
        {
            return Ok(detail);
        }
    }

    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;

    let row_id = skill.id.clone();
    let dir_path = skill_dir_path(&skill);
    let installations = installation_details(db::get_skill_installations(pool, skill_id).await?);
    let collections = db::get_skill_collections(pool, skill_id).await?;
    let repository_assignment = db::get_skill_repository_assignment(pool, skill_id).await?;
    let tags = db::get_skill_tags_for_skill(pool, skill_id).await?;

    Ok(SkillDetail {
        row_id,
        id: skill.id,
        name: skill.name,
        description: skill.description,
        file_path: skill.file_path,
        dir_path,
        canonical_path: skill.canonical_path,
        is_central: skill.is_central,
        source: skill.source,
        scanned_at: skill.scanned_at,
        source_kind: None,
        source_root: None,
        is_read_only: false,
        conflict_group: None,
        conflict_count: 0,
        installations,
        collections,
        repository: Some(repository_assignment.repository),
        tags,
        source_path: repository_assignment.source_path,
        is_source_unknown: repository_assignment.is_source_unknown,
    })
}

/// Testable core implementation of `get_skills_by_agent`.
///
/// Returns skills for the given agent enriched with installation metadata
/// (`dir_path`, `link_type`, `symlink_target`) so the frontend `SkillCard`
/// can display the correct source indicator.
pub async fn get_skills_by_agent_impl(
    pool: &DbPool,
    agent_id: &str,
) -> Result<Vec<SkillForAgent>, String> {
    db::get_skills_for_agent(pool, agent_id).await
}

/// Tauri command: return all skills installed for a given agent, including
/// installation metadata needed by the platform-view skill cards.
#[tauri::command]
pub async fn get_skills_by_agent(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<SkillForAgent>, String> {
    let pool = state.active_db().await?;
    get_skills_by_agent_impl(&pool, &agent_id).await
}

/// Tauri command: return all Central Skills with per-platform link status.
///
/// For each skill in the central skills directory, the response includes a
/// `linked_agents` array listing every agent that has an installation record
/// for that skill (regardless of whether the link type is symlink or copy).
#[tauri::command]
pub async fn get_central_skills(state: State<'_, AppState>) -> Result<Vec<SkillWithLinks>, String> {
    let pool = state.active_db().await?;
    get_central_skills_impl(&pool).await
}

async fn get_central_skills_impl(pool: &DbPool) -> Result<Vec<SkillWithLinks>, String> {
    let skills = db::get_central_skills(pool).await?;
    let agents = db::get_all_agents(pool).await?;
    let shared_root_agents = shared_root_agent_ids(&agents);
    let mut result = Vec::with_capacity(skills.len());
    for skill in skills {
        let installations = db::get_skill_installations(pool, &skill.id).await?;
        let mut linked_agents: Vec<String> =
            installations.into_iter().map(|i| i.agent_id).collect();
        append_missing_agents(&mut linked_agents, &shared_root_agents);
        let (created_at, updated_at) = skill_filesystem_timestamps(&skill);
        let repository_assignment = db::get_skill_repository_assignment(pool, &skill.id).await?;
        let tags = db::get_skill_tags_for_skill(pool, &skill.id).await?;

        result.push(SkillWithLinks {
            id: skill.id,
            name: skill.name,
            description: skill.description,
            file_path: skill.file_path,
            canonical_path: skill.canonical_path,
            is_central: skill.is_central,
            source: skill.source,
            scanned_at: skill.scanned_at,
            created_at,
            updated_at,
            linked_agents,
            shared_root_agents: shared_root_agents.clone(),
            repository: Some(repository_assignment.repository),
            tags,
            source_path: repository_assignment.source_path,
            is_source_unknown: repository_assignment.is_source_unknown,
        });
    }

    Ok(result)
}

fn shared_root_agent_ids(agents: &[db::Agent]) -> Vec<String> {
    let Some(central) = agents.iter().find(|agent| agent.id == "central") else {
        return Vec::new();
    };

    let central_dir = Path::new(&central.global_skills_dir);
    agents
        .iter()
        .filter(|agent| agent.id != "central")
        .filter(|agent| {
            crate::paths::paths_equivalent(Path::new(&agent.global_skills_dir), central_dir)
        })
        .map(|agent| agent.id.clone())
        .collect()
}

fn append_missing_agents(linked_agents: &mut Vec<String>, extra_agents: &[String]) {
    let mut seen: HashSet<String> = linked_agents.iter().cloned().collect();
    for agent_id in extra_agents {
        if seen.insert(agent_id.clone()) {
            linked_agents.push(agent_id.clone());
        }
    }
}

fn unique_agent_ids(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for id in ids {
        if seen.insert(id.clone()) {
            result.push(id);
        }
    }
    result
}

#[cfg(windows)]
fn remove_symlink_path(path: &Path) -> Result<(), String> {
    std::fs::remove_dir(path).map_err(|e| format!("Failed to remove symlink: {}", e))
}

#[cfg(not(windows))]
fn remove_symlink_path(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| format!("Failed to remove symlink: {}", e))
}

fn skill_delete_dir(skill: &db::Skill) -> Result<PathBuf, String> {
    skill
        .canonical_path
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| Path::new(&skill.file_path).parent().map(Path::to_path_buf))
        .ok_or_else(|| format!("Skill '{}' has no canonical directory", skill.id))
}

fn ensure_child_path(root: &Path, child: &Path, label: &str) -> Result<(), String> {
    let root_cmp = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let child_cmp = child.canonicalize().unwrap_or_else(|_| child.to_path_buf());

    if crate::paths::paths_equivalent(&root_cmp, &child_cmp) {
        return Err(format!(
            "Refusing to delete the Central Skills root for {}",
            label
        ));
    }

    if !child_cmp.starts_with(&root_cmp) {
        return Err(format!(
            "Refusing to delete '{}' because it is outside Central Skills root '{}'",
            child.display(),
            root.display()
        ));
    }

    Ok(())
}

fn remove_skill_dir(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => remove_symlink_path(path),
        Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(path).map_err(|e| {
            format!(
                "Failed to remove skill directory '{}': {}",
                path.display(),
                e
            )
        }),
        Ok(_) => Err(format!(
            "Path '{}' is not a directory. Refusing to delete.",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to inspect '{}': {}", path.display(), error)),
    }
}

fn remove_installation_path(installation: &db::SkillInstallation) -> Result<(), String> {
    let path = Path::new(&installation.installed_path);
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => remove_symlink_path(path),
        Ok(metadata) if metadata.is_dir() && installation.link_type == "copy" => {
            std::fs::remove_dir_all(path).map_err(|e| {
                format!(
                    "Failed to remove copied skill directory '{}': {}",
                    path.display(),
                    e
                )
            })
        }
        Ok(metadata) if metadata.is_dir() => Err(format!(
            "Path '{}' is not a managed copy. Refusing to delete.",
            path.display()
        )),
        Ok(_) => Err(format!(
            "Path '{}' is not a directory or symlink. Refusing to delete.",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to inspect '{}': {}", path.display(), error)),
    }
}

fn remote_skill_delete_dir(skill: &db::Skill) -> Result<String, String> {
    skill
        .canonical_path
        .as_deref()
        .map(str::to_string)
        .or_else(|| crate::targets::remote_parent(&skill.file_path))
        .ok_or_else(|| format!("Skill '{}' has no canonical directory", skill.id))
}

fn normalize_remote_path(path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() || !trimmed.starts_with('/') || trimmed.contains('\0') {
        return Err(format!("Invalid remote path '{}'", path));
    }

    let mut segments = Vec::new();
    for segment in trimmed.split('/') {
        match segment {
            "" | "." => {}
            ".." => return Err(format!("Remote path '{}' contains traversal", path)),
            value => segments.push(value),
        }
    }

    if segments.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", segments.join("/")))
    }
}

fn ensure_remote_child_path(root: &str, child: &str, label: &str) -> Result<String, String> {
    let root_cmp = normalize_remote_path(root)?;
    let child_cmp = normalize_remote_path(child)?;

    if root_cmp == "/" {
        return Err(format!(
            "Refusing to delete under remote root for {}",
            label
        ));
    }

    if root_cmp == child_cmp {
        return Err(format!(
            "Refusing to delete the remote root '{}' for {}",
            root_cmp, label
        ));
    }

    let prefix = format!("{}/", root_cmp.trim_end_matches('/'));
    if !child_cmp.starts_with(&prefix) {
        return Err(format!(
            "Refusing to delete '{}' because it is outside remote root '{}'",
            child, root
        ));
    }

    Ok(child_cmp)
}

fn remote_shared_root_agent_ids(agents: &[db::Agent], central_root: &str) -> Vec<String> {
    let Ok(central_root) = normalize_remote_path(central_root) else {
        return Vec::new();
    };

    agents
        .iter()
        .filter(|agent| agent.id != "central")
        .filter_map(|agent| {
            normalize_remote_path(&agent.global_skills_dir)
                .ok()
                .filter(|path| *path == central_root)
                .map(|_| agent.id.clone())
        })
        .collect()
}

async fn preview_delete_central_skill_ssh_impl(
    pool: &DbPool,
    skill_id: &str,
) -> Result<DeleteCentralSkillPreview, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
    if !skill.is_central {
        return Err(format!("Skill '{}' is not a Central skill", skill_id));
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    let central_skill_dir = remote_skill_delete_dir(&skill)?;
    let central_path =
        ensure_remote_child_path(&central.global_skills_dir, &central_skill_dir, skill_id)?;

    let agents = db::get_all_agents(pool).await?;
    let shared_root_agents = remote_shared_root_agent_ids(&agents, &central.global_skills_dir);
    let installations = db::get_skill_installations(pool, skill_id).await?;
    let copy_installations = installation_details(
        installations
            .iter()
            .filter(|installation| installation.link_type == "copy")
            .cloned()
            .collect(),
    );
    let auto_removed_agent_ids = unique_agent_ids(
        installations
            .iter()
            .filter(|installation| installation.agent_id != "central")
            .filter(|installation| installation.link_type != "copy")
            .map(|installation| installation.agent_id.clone())
            .chain(shared_root_agents),
    );

    Ok(DeleteCentralSkillPreview {
        skill_id: skill.id,
        skill_name: skill.name,
        central_path,
        copy_installations,
        auto_removed_agent_ids,
    })
}

pub async fn preview_delete_central_skills_ssh_impl(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<BatchDeleteCentralSkillPreviewResult, String> {
    let mut previews = Vec::new();
    let mut failed = Vec::new();
    let mut seen = HashSet::new();

    for skill_id in skill_ids {
        if !seen.insert(skill_id.clone()) {
            continue;
        }

        match preview_delete_central_skill_ssh_impl(pool, skill_id).await {
            Ok(preview) => previews.push(preview),
            Err(error) => failed.push(FailedCentralSkillDelete {
                skill_id: skill_id.clone(),
                error,
            }),
        }
    }

    Ok(BatchDeleteCentralSkillPreviewResult { previews, failed })
}

async fn remove_remote_installation_path(
    connection: &ConnectedSshTarget,
    installation: &db::SkillInstallation,
    agents_by_id: &HashMap<String, db::Agent>,
) -> Result<(), String> {
    let agent = agents_by_id
        .get(&installation.agent_id)
        .ok_or_else(|| format!("Agent '{}' not found", installation.agent_id))?;
    let path = ensure_remote_child_path(
        &agent.global_skills_dir,
        &installation.installed_path,
        &installation.agent_id,
    )?;
    connection.remove_tree(&path).await
}

pub async fn delete_central_skill_ssh_impl(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    skill_id: &str,
    remove_agent_ids: &[String],
) -> Result<DeleteCentralSkillResult, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
    if !skill.is_central {
        return Err(format!("Skill '{}' is not a Central skill", skill_id));
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    let central_skill_dir = remote_skill_delete_dir(&skill)?;
    let central_path =
        ensure_remote_child_path(&central.global_skills_dir, &central_skill_dir, skill_id)?;

    let remove_agent_set: HashSet<String> = remove_agent_ids.iter().cloned().collect();
    let installations = db::get_skill_installations(pool, skill_id).await?;
    for agent_id in &remove_agent_set {
        let installation = installations
            .iter()
            .find(|item| item.agent_id == *agent_id)
            .ok_or_else(|| format!("Skill '{}' is not installed for '{}'", skill_id, agent_id))?;
        if installation.link_type != "copy" {
            return Err(format!(
                "Only copy installations can be selected for platform deletion: {}",
                agent_id
            ));
        }
    }

    let agents = db::get_all_agents(pool).await?;
    let agents_by_id: HashMap<String, db::Agent> = agents
        .into_iter()
        .map(|agent| (agent.id.clone(), agent))
        .collect();
    let connection = connect_ssh_target(target).await?;

    let mut removed_agent_ids = Vec::new();
    let mut retained_agent_ids = Vec::new();
    for installation in &installations {
        match installation.link_type.as_str() {
            "copy" if remove_agent_set.contains(&installation.agent_id) => {
                remove_remote_installation_path(&connection, installation, &agents_by_id).await?;
                removed_agent_ids.push(installation.agent_id.clone());
            }
            "copy" => retained_agent_ids.push(installation.agent_id.clone()),
            "symlink" => {
                remove_remote_installation_path(&connection, installation, &agents_by_id).await?;
                removed_agent_ids.push(installation.agent_id.clone());
            }
            "native" => {
                removed_agent_ids.push(installation.agent_id.clone());
            }
            _ => {
                retained_agent_ids.push(installation.agent_id.clone());
            }
        }
    }

    connection.remove_tree(&central_path).await?;
    db::delete_skill(pool, skill_id).await?;

    Ok(DeleteCentralSkillResult {
        removed_central_path: central_path,
        removed_agent_ids,
        retained_agent_ids,
    })
}

pub async fn delete_central_skills_ssh_impl(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<BatchDeleteCentralSkillResult, String> {
    let mut ordered_requests: Vec<BatchDeleteCentralSkillRequest> = Vec::new();
    for request in requests {
        if let Some(existing) = ordered_requests
            .iter_mut()
            .find(|existing| existing.skill_id == request.skill_id)
        {
            for agent_id in &request.remove_agent_ids {
                if !existing.remove_agent_ids.contains(agent_id) {
                    existing.remove_agent_ids.push(agent_id.clone());
                }
            }
        } else {
            ordered_requests.push(BatchDeleteCentralSkillRequest {
                skill_id: request.skill_id.clone(),
                remove_agent_ids: unique_agent_ids(request.remove_agent_ids.clone()),
            });
        }
    }

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    for request in ordered_requests {
        match delete_central_skill_ssh_impl(
            pool,
            target,
            &request.skill_id,
            &request.remove_agent_ids,
        )
        .await
        {
            Ok(result) => succeeded.push(BatchDeleteCentralSkillSuccess {
                skill_id: request.skill_id,
                removed_central_path: result.removed_central_path,
                removed_agent_ids: result.removed_agent_ids,
                retained_agent_ids: result.retained_agent_ids,
            }),
            Err(error) => failed.push(FailedCentralSkillDelete {
                skill_id: request.skill_id,
                error,
            }),
        }
    }

    Ok(BatchDeleteCentralSkillResult { succeeded, failed })
}

pub async fn preview_delete_central_skill_impl(
    pool: &DbPool,
    skill_id: &str,
) -> Result<DeleteCentralSkillPreview, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
    if !skill.is_central {
        return Err(format!("Skill '{}' is not a Central skill", skill_id));
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    let central_root = PathBuf::from(&central.global_skills_dir);
    let central_skill_dir = skill_delete_dir(&skill)?;
    ensure_child_path(&central_root, &central_skill_dir, skill_id)?;

    let agents = db::get_all_agents(pool).await?;
    let shared_root_agents = shared_root_agent_ids(&agents);
    let installations = db::get_skill_installations(pool, skill_id).await?;
    let copy_installations = installation_details(
        installations
            .iter()
            .filter(|installation| installation.link_type == "copy")
            .cloned()
            .collect(),
    );
    let auto_removed_agent_ids = unique_agent_ids(
        installations
            .iter()
            .filter(|installation| installation.agent_id != "central")
            .filter(|installation| installation.link_type != "copy")
            .map(|installation| installation.agent_id.clone())
            .chain(shared_root_agents),
    );

    Ok(DeleteCentralSkillPreview {
        skill_id: skill.id,
        skill_name: skill.name,
        central_path: central_skill_dir.to_string_lossy().into_owned(),
        copy_installations,
        auto_removed_agent_ids,
    })
}

pub async fn preview_delete_central_skills_impl(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<BatchDeleteCentralSkillPreviewResult, String> {
    let mut previews = Vec::new();
    let mut failed = Vec::new();
    let mut seen = HashSet::new();

    for skill_id in skill_ids {
        if !seen.insert(skill_id.clone()) {
            continue;
        }

        match preview_delete_central_skill_impl(pool, skill_id).await {
            Ok(preview) => previews.push(preview),
            Err(error) => failed.push(FailedCentralSkillDelete {
                skill_id: skill_id.clone(),
                error,
            }),
        }
    }

    Ok(BatchDeleteCentralSkillPreviewResult { previews, failed })
}

pub async fn delete_central_skill_impl(
    pool: &DbPool,
    skill_id: &str,
    remove_agent_ids: &[String],
) -> Result<DeleteCentralSkillResult, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
    if !skill.is_central {
        return Err(format!("Skill '{}' is not a Central skill", skill_id));
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    let central_root = PathBuf::from(&central.global_skills_dir);
    let central_skill_dir = skill_delete_dir(&skill)?;
    ensure_child_path(&central_root, &central_skill_dir, skill_id)?;

    let remove_agent_set: HashSet<String> = remove_agent_ids.iter().cloned().collect();
    let installations = db::get_skill_installations(pool, skill_id).await?;
    for agent_id in &remove_agent_set {
        let installation = installations
            .iter()
            .find(|item| item.agent_id == *agent_id)
            .ok_or_else(|| format!("Skill '{}' is not installed for '{}'", skill_id, agent_id))?;
        if installation.link_type != "copy" {
            return Err(format!(
                "Only copy installations can be selected for platform deletion: {}",
                agent_id
            ));
        }
    }

    let mut removed_agent_ids = Vec::new();
    let mut retained_agent_ids = Vec::new();
    for installation in &installations {
        match installation.link_type.as_str() {
            "copy" if remove_agent_set.contains(&installation.agent_id) => {
                remove_installation_path(installation)?;
                removed_agent_ids.push(installation.agent_id.clone());
            }
            "copy" => retained_agent_ids.push(installation.agent_id.clone()),
            "symlink" => {
                remove_installation_path(installation)?;
                removed_agent_ids.push(installation.agent_id.clone());
            }
            "native" => {
                removed_agent_ids.push(installation.agent_id.clone());
            }
            _ => {
                retained_agent_ids.push(installation.agent_id.clone());
            }
        }
    }

    remove_skill_dir(&central_skill_dir)?;
    db::delete_skill(pool, skill_id).await?;

    Ok(DeleteCentralSkillResult {
        removed_central_path: central_skill_dir.to_string_lossy().into_owned(),
        removed_agent_ids,
        retained_agent_ids,
    })
}

pub async fn delete_central_skills_impl(
    pool: &DbPool,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<BatchDeleteCentralSkillResult, String> {
    let mut ordered_requests: Vec<BatchDeleteCentralSkillRequest> = Vec::new();
    for request in requests {
        if let Some(existing) = ordered_requests
            .iter_mut()
            .find(|existing| existing.skill_id == request.skill_id)
        {
            for agent_id in &request.remove_agent_ids {
                if !existing.remove_agent_ids.contains(agent_id) {
                    existing.remove_agent_ids.push(agent_id.clone());
                }
            }
        } else {
            ordered_requests.push(BatchDeleteCentralSkillRequest {
                skill_id: request.skill_id.clone(),
                remove_agent_ids: unique_agent_ids(request.remove_agent_ids.clone()),
            });
        }
    }

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    for request in ordered_requests {
        match delete_central_skill_impl(pool, &request.skill_id, &request.remove_agent_ids).await {
            Ok(result) => succeeded.push(BatchDeleteCentralSkillSuccess {
                skill_id: request.skill_id,
                removed_central_path: result.removed_central_path,
                removed_agent_ids: result.removed_agent_ids,
                retained_agent_ids: result.retained_agent_ids,
            }),
            Err(error) => failed.push(FailedCentralSkillDelete {
                skill_id: request.skill_id,
                error,
            }),
        }
    }

    Ok(BatchDeleteCentralSkillResult { succeeded, failed })
}

#[tauri::command]
pub async fn preview_delete_central_skills(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
) -> Result<BatchDeleteCentralSkillPreviewResult, String> {
    let pool = state.active_db().await?;
    match state.active_target().await? {
        ActiveTarget::Local => preview_delete_central_skills_impl(&pool, &skill_ids).await,
        ActiveTarget::Ssh(_) => preview_delete_central_skills_ssh_impl(&pool, &skill_ids).await,
    }
}

#[tauri::command]
pub async fn delete_central_skill(
    state: State<'_, AppState>,
    skill_id: String,
    remove_agent_ids: Vec<String>,
) -> Result<DeleteCentralSkillResult, String> {
    let pool = state.active_db().await?;
    match state.active_target().await? {
        ActiveTarget::Local => delete_central_skill_impl(&pool, &skill_id, &remove_agent_ids).await,
        ActiveTarget::Ssh(target) => {
            delete_central_skill_ssh_impl(&pool, &target, &skill_id, &remove_agent_ids).await
        }
    }
}

#[tauri::command]
pub async fn delete_central_skills(
    state: State<'_, AppState>,
    requests: Vec<BatchDeleteCentralSkillRequest>,
) -> Result<BatchDeleteCentralSkillResult, String> {
    let pool = state.active_db().await?;
    match state.active_target().await? {
        ActiveTarget::Local => delete_central_skills_impl(&pool, &requests).await,
        ActiveTarget::Ssh(target) => {
            delete_central_skills_ssh_impl(&pool, &target, &requests).await
        }
    }
}

/// Tauri command: return detailed information about a skill, including all
/// installation records across agents. Each installation includes `installed_at`
/// (the `created_at` timestamp from the DB, renamed for frontend clarity).
#[tauri::command]
pub async fn get_skill_detail(
    state: State<'_, AppState>,
    skill_id: String,
    agent_id: Option<String>,
    row_id: Option<String>,
) -> Result<SkillDetail, String> {
    let pool = state.active_db().await?;
    get_skill_detail_with_row_impl(&pool, &skill_id, agent_id.as_deref(), row_id.as_deref()).await
}

/// Tauri command: read and return the raw content of a skill's `SKILL.md` file.
#[tauri::command]
pub async fn read_skill_content(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<String, String> {
    let pool = state.active_db().await?;
    let skill = db::get_skill_by_id(&pool, &skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;

    match state.active_target().await? {
        ActiveTarget::Local => std::fs::read_to_string(&skill.file_path)
            .map_err(|e| format!("Failed to read '{}': {}", skill.file_path, e)),
        ActiveTarget::Ssh(target) => {
            let connection = connect_ssh_target(&target).await?;
            let bytes = connection.read_file(&skill.file_path).await?;
            String::from_utf8(bytes).map_err(|e| {
                format!(
                    "Remote file '{}' is not valid UTF-8: {}",
                    skill.file_path, e
                )
            })
        }
    }
}

#[tauri::command]
pub async fn read_file_by_path(state: State<'_, AppState>, path: String) -> Result<String, String> {
    match state.active_target().await? {
        ActiveTarget::Local => read_file_by_path_impl(&path),
        ActiveTarget::Ssh(target) => {
            let connection = connect_ssh_target(&target).await?;
            let bytes = connection.read_file(&path).await?;
            String::from_utf8(bytes)
                .map_err(|e| format!("Remote file '{}' is not valid UTF-8: {}", path, e))
        }
    }
}

fn read_file_by_path_impl(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path, e))
}

#[tauri::command]
pub async fn open_in_file_manager(state: State<'_, AppState>, path: String) -> Result<(), String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Err("Remote paths cannot be opened in the local file manager. Copy the remote path instead.".to_string());
    }
    open_in_file_manager_checked_impl(&path)
}

fn open_in_file_manager_checked_impl(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    open_in_file_manager_impl(path)
}

fn open_in_file_manager_impl(path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }

    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, AgentSkillObservation, Skill, SkillInstallation};
    use chrono::Utc;
    use sqlx::SqlitePool;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        db::init_database(&pool).await.unwrap();
        pool
    }

    async fn set_test_central_root(pool: &SqlitePool, root: &Path) {
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind(root.to_string_lossy().into_owned())
            .execute(pool)
            .await
            .unwrap();
    }

    fn write_test_skill_dir(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            "---\nname: test-skill\ndescription: test skill\n---\n",
        )
        .unwrap();
    }

    fn make_central_skill_at(id: &str, name: &str, dir: &Path) -> Skill {
        Skill {
            id: id.to_string(),
            name: name.to_string(),
            description: Some(format!("Desc for {}", name)),
            file_path: dir.join("SKILL.md").to_string_lossy().into_owned(),
            canonical_path: Some(dir.to_string_lossy().into_owned()),
            is_central: true,
            source: Some("native".to_string()),
            content: None,
            scanned_at: Utc::now().to_rfc3339(),
        }
    }

    fn make_installation_at(
        skill_id: &str,
        agent_id: &str,
        dir: &Path,
        link_type: &str,
        symlink_target: Option<&Path>,
    ) -> SkillInstallation {
        SkillInstallation {
            skill_id: skill_id.to_string(),
            agent_id: agent_id.to_string(),
            installed_path: dir.to_string_lossy().into_owned(),
            link_type: link_type.to_string(),
            symlink_target: symlink_target.map(|path| path.to_string_lossy().into_owned()),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    fn make_skill(id: &str, name: &str, is_central: bool) -> Skill {
        Skill {
            id: id.to_string(),
            name: name.to_string(),
            description: Some(format!("Desc for {}", name)),
            file_path: format!("/tmp/{}/SKILL.md", id),
            canonical_path: if is_central {
                Some(format!("/tmp/central/{}", id))
            } else {
                None
            },
            is_central,
            source: if is_central {
                Some("native".to_string())
            } else {
                Some("copy".to_string())
            },
            content: None,
            scanned_at: Utc::now().to_rfc3339(),
        }
    }

    fn make_remote_central_skill(id: &str, dir: &str) -> Skill {
        Skill {
            id: id.to_string(),
            name: id.to_string(),
            description: Some(format!("Desc for {}", id)),
            file_path: format!("{}/SKILL.md", dir.trim_end_matches('/')),
            canonical_path: Some(dir.to_string()),
            is_central: true,
            source: Some("native".to_string()),
            content: None,
            scanned_at: Utc::now().to_rfc3339(),
        }
    }

    fn make_remote_installation(
        skill_id: &str,
        agent_id: &str,
        installed_path: &str,
        link_type: &str,
    ) -> SkillInstallation {
        SkillInstallation {
            skill_id: skill_id.to_string(),
            agent_id: agent_id.to_string(),
            installed_path: installed_path.to_string(),
            link_type: link_type.to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        }
    }

    fn make_observation(
        row_id: &str,
        skill_id: &str,
        name: &str,
        dir_path: &str,
        source_kind: &str,
        read_only: bool,
    ) -> AgentSkillObservation {
        AgentSkillObservation {
            row_id: row_id.to_string(),
            agent_id: "claude-code".to_string(),
            skill_id: skill_id.to_string(),
            name: name.to_string(),
            description: Some(format!("{source_kind} copy")),
            file_path: format!("{dir_path}/SKILL.md"),
            dir_path: dir_path.to_string(),
            source_kind: source_kind.to_string(),
            source_root: if source_kind == "user" {
                "/tmp/.claude/skills".to_string()
            } else {
                "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0".to_string()
            },
            link_type: "copy".to_string(),
            symlink_target: None,
            is_read_only: read_only,
            scanned_at: Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn test_remote_child_path_guard_normalizes_and_rejects_unsafe_paths() {
        assert_eq!(
            ensure_remote_child_path(
                "/home/alice/.skillsmanage/skills/",
                "/home/alice/.skillsmanage/skills/demo",
                "demo",
            )
            .unwrap(),
            "/home/alice/.skillsmanage/skills/demo"
        );

        assert!(ensure_remote_child_path(
            "/home/alice/.skillsmanage/skills",
            "/home/alice/.skillsmanage/skills",
            "root",
        )
        .is_err());
        assert!(ensure_remote_child_path(
            "/home/alice/.skillsmanage/skills",
            "/home/alice/other/demo",
            "outside",
        )
        .is_err());
        assert!(ensure_remote_child_path(
            "/home/alice/.skillsmanage/skills",
            "/home/alice/.skillsmanage/skills/../other",
            "traversal",
        )
        .is_err());
    }

    #[tokio::test]
    async fn test_preview_remote_delete_uses_remote_paths_and_installations() {
        let pool = setup_test_db().await;
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind("/home/alice/.skillsmanage/skills")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
            .bind("/home/alice/.agents/skills")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
            .bind("/home/alice/.claude/skills")
            .execute(&pool)
            .await
            .unwrap();

        db::upsert_skill(
            &pool,
            &make_remote_central_skill(
                "remote-delete",
                "/home/alice/.skillsmanage/skills/remote-delete",
            ),
        )
        .await
        .unwrap();
        db::upsert_skill_installation(
            &pool,
            &make_remote_installation(
                "remote-delete",
                "cursor",
                "/home/alice/.agents/skills/remote-delete",
                "copy",
            ),
        )
        .await
        .unwrap();
        db::upsert_skill_installation(
            &pool,
            &make_remote_installation(
                "remote-delete",
                "claude-code",
                "/home/alice/.claude/skills/remote-delete",
                "symlink",
            ),
        )
        .await
        .unwrap();
        db::upsert_skill_installation(
            &pool,
            &make_remote_installation(
                "remote-delete",
                "central",
                "/home/alice/.skillsmanage/skills/remote-delete",
                "native",
            ),
        )
        .await
        .unwrap();

        let result = preview_delete_central_skills_ssh_impl(&pool, &["remote-delete".to_string()])
            .await
            .unwrap();

        assert!(result.failed.is_empty());
        assert_eq!(
            result.previews[0].central_path,
            "/home/alice/.skillsmanage/skills/remote-delete"
        );
        assert_eq!(result.previews[0].copy_installations[0].agent_id, "cursor");
        assert_eq!(
            result.previews[0].auto_removed_agent_ids,
            vec!["claude-code"]
        );
    }

    #[tokio::test]
    async fn test_preview_remote_delete_rejects_central_path_outside_remote_root() {
        let pool = setup_test_db().await;
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind("/home/alice/.skillsmanage/skills")
            .execute(&pool)
            .await
            .unwrap();
        db::upsert_skill(
            &pool,
            &make_remote_central_skill("outside-remote", "/tmp/outside-remote"),
        )
        .await
        .unwrap();

        let result = preview_delete_central_skills_ssh_impl(&pool, &["outside-remote".to_string()])
            .await
            .unwrap();

        assert!(result.previews.is_empty());
        assert_eq!(result.failed[0].skill_id, "outside-remote");
        assert!(result.failed[0].error.contains("outside remote root"));
    }

    // ── get_skills_by_agent ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_skills_by_agent_returns_correct_skills() {
        let pool = setup_test_db().await;

        let skill_a = make_skill("skill-a", "Skill A", false);
        let skill_b = make_skill("skill-b", "Skill B", false);
        db::upsert_skill(&pool, &skill_a).await.unwrap();
        db::upsert_skill(&pool, &skill_b).await.unwrap();

        db::upsert_skill_installation(
            &pool,
            &SkillInstallation {
                skill_id: "skill-a".to_string(),
                agent_id: "claude-code".to_string(),
                installed_path: "/tmp/claude/skill-a/SKILL.md".to_string(),
                link_type: "symlink".to_string(),
                symlink_target: Some("/tmp/central/skill-a".to_string()),
                created_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .unwrap();

        let skills = db::get_skills_by_agent(&pool, "claude-code").await.unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].id, "skill-a");
    }

    #[tokio::test]
    async fn test_get_skills_by_agent_empty_for_unknown_agent() {
        let pool = setup_test_db().await;
        let skills = db::get_skills_by_agent(&pool, "nonexistent-agent")
            .await
            .unwrap();
        assert!(skills.is_empty());
    }

    // ── get_central_skills ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_central_skills_includes_linked_agents() {
        let pool = setup_test_db().await;

        let central_skill = make_skill("central-a", "Central A", true);
        db::upsert_skill(&pool, &central_skill).await.unwrap();

        // Install to claude-code and cursor.
        for agent_id in &["claude-code", "cursor"] {
            db::upsert_skill_installation(
                &pool,
                &SkillInstallation {
                    skill_id: "central-a".to_string(),
                    agent_id: agent_id.to_string(),
                    installed_path: format!("/tmp/{}/central-a/SKILL.md", agent_id),
                    link_type: "symlink".to_string(),
                    symlink_target: Some("/tmp/central/central-a".to_string()),
                    created_at: Utc::now().to_rfc3339(),
                },
            )
            .await
            .unwrap();
        }

        let skills_with_links = get_central_skills_impl(&pool).await.unwrap();
        assert_eq!(skills_with_links.len(), 1);

        let mut linked = skills_with_links[0].linked_agents.clone();
        linked.sort();
        let mut expected_linked: Vec<String> =
            vec!["claude-code".to_string(), "cursor".to_string()];
        expected_linked.sort();
        assert_eq!(linked, expected_linked);

        let mut shared = skills_with_links[0].shared_root_agents.clone();
        shared.sort();
        assert!(shared.is_empty());
    }

    #[tokio::test]
    async fn test_get_central_skills_no_links() {
        let pool = setup_test_db().await;

        let central_skill = make_skill("central-solo", "Solo Central", true);
        db::upsert_skill(&pool, &central_skill).await.unwrap();

        let skills_with_links = get_central_skills_impl(&pool).await.unwrap();
        assert_eq!(skills_with_links.len(), 1);
        let mut linked = skills_with_links[0].linked_agents.clone();
        linked.sort();
        assert!(linked.is_empty());

        let mut shared = skills_with_links[0].shared_root_agents.clone();
        shared.sort();
        assert!(shared.is_empty());
    }

    #[tokio::test]
    async fn test_get_central_skills_ignores_claude_plugin_observations() {
        let pool = setup_test_db().await;

        let central_skill = make_skill("shared-skill", "Shared Skill", true);
        db::upsert_skill(&pool, &central_skill).await.unwrap();
        db::upsert_agent_skill_observation(
            &pool,
            &make_observation(
                "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
                "shared-skill",
                "Shared Skill",
                "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
                "plugin",
                true,
            ),
        )
        .await
        .unwrap();

        let skills_with_links = get_central_skills_impl(&pool).await.unwrap();
        assert_eq!(skills_with_links.len(), 1);
        assert_eq!(
            {
                let mut linked = skills_with_links[0].linked_agents.clone();
                linked.sort();
                linked
            },
            Vec::<String>::new(),
            "plugin observations must not pollute linked_agents state"
        );
        let mut shared = skills_with_links[0].shared_root_agents.clone();
        shared.sort();
        assert!(shared.is_empty());
    }

    #[tokio::test]
    async fn test_get_central_skills_excludes_non_central() {
        let pool = setup_test_db().await;

        let central = make_skill("c-skill", "Central", true);
        let non_central = make_skill("nc-skill", "Non-Central", false);
        db::upsert_skill(&pool, &central).await.unwrap();
        db::upsert_skill(&pool, &non_central).await.unwrap();

        let skills_with_links = get_central_skills_impl(&pool).await.unwrap();
        assert_eq!(
            skills_with_links.len(),
            1,
            "only central skills should be returned"
        );
        assert_eq!(skills_with_links[0].id, "c-skill");
    }

    // ── get_skill_detail ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_delete_central_skill_rejects_non_central_skill() {
        let pool = setup_test_db().await;
        let skill = make_skill("plain-skill", "Plain Skill", false);
        db::upsert_skill(&pool, &skill).await.unwrap();

        let error = delete_central_skill_impl(&pool, "plain-skill", &[])
            .await
            .unwrap_err();

        assert!(error.contains("is not a Central skill"));
    }

    #[tokio::test]
    async fn test_delete_central_skill_rejects_path_outside_central_root() {
        let pool = setup_test_db().await;
        let temp = TempDir::new().unwrap();
        let central_root = temp.path().join("central");
        let outside_dir = temp.path().join("outside").join("outside-skill");
        fs::create_dir_all(&central_root).unwrap();
        write_test_skill_dir(&outside_dir);
        set_test_central_root(&pool, &central_root).await;

        let skill = make_central_skill_at("outside-skill", "Outside Skill", &outside_dir);
        db::upsert_skill(&pool, &skill).await.unwrap();

        let error = delete_central_skill_impl(&pool, "outside-skill", &[])
            .await
            .unwrap_err();

        assert!(error.contains("outside Central Skills root"));
        assert!(outside_dir.exists());
        assert!(db::get_skill_by_id(&pool, "outside-skill")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn test_delete_central_skill_removes_selected_copy_and_retains_unselected_copy() {
        let pool = setup_test_db().await;
        let temp = TempDir::new().unwrap();
        let central_root = temp.path().join("central");
        let central_dir = central_root.join("central-delete");
        let removed_copy_dir = temp.path().join("cursor").join("central-delete");
        let retained_copy_dir = temp.path().join("claude").join("central-delete");
        let missing_symlink_path = temp.path().join("codex").join("central-delete");
        fs::create_dir_all(&central_root).unwrap();
        write_test_skill_dir(&central_dir);
        write_test_skill_dir(&removed_copy_dir);
        write_test_skill_dir(&retained_copy_dir);
        set_test_central_root(&pool, &central_root).await;

        let skill = make_central_skill_at("central-delete", "Central Delete", &central_dir);
        db::upsert_skill(&pool, &skill).await.unwrap();
        db::upsert_skill_installation(
            &pool,
            &make_installation_at("central-delete", "cursor", &removed_copy_dir, "copy", None),
        )
        .await
        .unwrap();
        db::upsert_skill_installation(
            &pool,
            &make_installation_at(
                "central-delete",
                "claude-code",
                &retained_copy_dir,
                "copy",
                None,
            ),
        )
        .await
        .unwrap();
        db::upsert_skill_installation(
            &pool,
            &make_installation_at(
                "central-delete",
                "codex",
                &missing_symlink_path,
                "symlink",
                Some(&central_dir),
            ),
        )
        .await
        .unwrap();

        let result = delete_central_skill_impl(&pool, "central-delete", &["cursor".to_string()])
            .await
            .unwrap();

        assert_eq!(
            result.removed_central_path,
            central_dir.to_string_lossy().into_owned()
        );
        let mut removed_agent_ids = result.removed_agent_ids;
        removed_agent_ids.sort();
        assert_eq!(
            removed_agent_ids,
            vec!["codex".to_string(), "cursor".to_string()]
        );
        assert_eq!(result.retained_agent_ids, vec!["claude-code".to_string()]);
        assert!(!central_dir.exists());
        assert!(!removed_copy_dir.exists());
        assert!(retained_copy_dir.exists());
        assert!(db::get_skill_by_id(&pool, "central-delete")
            .await
            .unwrap()
            .is_none());
        assert!(db::get_skill_installations(&pool, "central-delete")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn test_preview_delete_central_skills_reports_copies_and_preview_failures() {
        let pool = setup_test_db().await;
        let temp = TempDir::new().unwrap();
        let central_root = temp.path().join("central");
        let central_dir = central_root.join("preview-delete");
        let copy_dir = temp.path().join("cursor").join("preview-delete");
        let missing_symlink_path = temp.path().join("codex").join("preview-delete");
        fs::create_dir_all(&central_root).unwrap();
        write_test_skill_dir(&central_dir);
        write_test_skill_dir(&copy_dir);
        set_test_central_root(&pool, &central_root).await;

        let skill = make_central_skill_at("preview-delete", "Preview Delete", &central_dir);
        db::upsert_skill(&pool, &skill).await.unwrap();
        db::upsert_skill_installation(
            &pool,
            &make_installation_at("preview-delete", "cursor", &copy_dir, "copy", None),
        )
        .await
        .unwrap();
        db::upsert_skill_installation(
            &pool,
            &make_installation_at(
                "preview-delete",
                "codex",
                &missing_symlink_path,
                "symlink",
                Some(&central_dir),
            ),
        )
        .await
        .unwrap();

        let result = preview_delete_central_skills_impl(
            &pool,
            &["preview-delete".to_string(), "missing-delete".to_string()],
        )
        .await
        .unwrap();

        assert_eq!(result.previews.len(), 1);
        assert_eq!(result.previews[0].skill_id, "preview-delete");
        assert_eq!(result.previews[0].copy_installations.len(), 1);
        assert_eq!(result.previews[0].copy_installations[0].agent_id, "cursor");
        assert_eq!(result.previews[0].auto_removed_agent_ids, vec!["codex"]);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].skill_id, "missing-delete");
    }

    #[tokio::test]
    async fn test_preview_delete_central_skills_reports_auto_links_without_central_self() {
        let pool = setup_test_db().await;
        let temp = TempDir::new().unwrap();
        let central_root = temp.path().join("central");
        let central_dir = central_root.join("linked-delete");
        let symlink_path = temp.path().join("codex").join("linked-delete");
        fs::create_dir_all(&central_root).unwrap();
        write_test_skill_dir(&central_dir);
        set_test_central_root(&pool, &central_root).await;

        let skill = make_central_skill_at("linked-delete", "Linked Delete", &central_dir);
        db::upsert_skill(&pool, &skill).await.unwrap();
        db::upsert_skill_installation(
            &pool,
            &make_installation_at("linked-delete", "central", &central_dir, "native", None),
        )
        .await
        .unwrap();
        db::upsert_skill_installation(
            &pool,
            &make_installation_at(
                "linked-delete",
                "codex",
                &symlink_path,
                "symlink",
                Some(&central_dir),
            ),
        )
        .await
        .unwrap();

        let result =
            preview_delete_central_skills_impl(&pool, &["linked-delete".to_string()])
                .await
                .unwrap();

        assert!(result.failed.is_empty());
        assert_eq!(result.previews.len(), 1);
        assert!(result.previews[0].copy_installations.is_empty());
        assert_eq!(result.previews[0].auto_removed_agent_ids, vec!["codex"]);
    }

    #[tokio::test]
    async fn test_batch_delete_central_skills_keeps_partial_failures_isolated() {
        let pool = setup_test_db().await;
        let temp = TempDir::new().unwrap();
        let central_root = temp.path().join("central");
        let valid_dir = central_root.join("valid-delete");
        let outside_dir = temp.path().join("outside").join("unsafe-delete");
        fs::create_dir_all(&central_root).unwrap();
        write_test_skill_dir(&valid_dir);
        write_test_skill_dir(&outside_dir);
        set_test_central_root(&pool, &central_root).await;

        db::upsert_skill(
            &pool,
            &make_central_skill_at("valid-delete", "Valid Delete", &valid_dir),
        )
        .await
        .unwrap();
        db::upsert_skill(
            &pool,
            &make_central_skill_at("unsafe-delete", "Unsafe Delete", &outside_dir),
        )
        .await
        .unwrap();

        let result = delete_central_skills_impl(
            &pool,
            &[
                BatchDeleteCentralSkillRequest {
                    skill_id: "valid-delete".to_string(),
                    remove_agent_ids: Vec::new(),
                },
                BatchDeleteCentralSkillRequest {
                    skill_id: "unsafe-delete".to_string(),
                    remove_agent_ids: Vec::new(),
                },
            ],
        )
        .await
        .unwrap();

        assert_eq!(result.succeeded.len(), 1);
        assert_eq!(result.succeeded[0].skill_id, "valid-delete");
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].skill_id, "unsafe-delete");
        assert!(!valid_dir.exists());
        assert!(outside_dir.exists());
        assert!(db::get_skill_by_id(&pool, "valid-delete")
            .await
            .unwrap()
            .is_none());
        assert!(db::get_skill_by_id(&pool, "unsafe-delete")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn test_batch_delete_central_skills_dedupes_and_merges_copy_agents() {
        let pool = setup_test_db().await;
        let temp = TempDir::new().unwrap();
        let central_root = temp.path().join("central");
        let central_dir = central_root.join("dedupe-delete");
        let cursor_copy_dir = temp.path().join("cursor").join("dedupe-delete");
        let claude_copy_dir = temp.path().join("claude").join("dedupe-delete");
        fs::create_dir_all(&central_root).unwrap();
        write_test_skill_dir(&central_dir);
        write_test_skill_dir(&cursor_copy_dir);
        write_test_skill_dir(&claude_copy_dir);
        set_test_central_root(&pool, &central_root).await;

        db::upsert_skill(
            &pool,
            &make_central_skill_at("dedupe-delete", "Dedupe Delete", &central_dir),
        )
        .await
        .unwrap();
        db::upsert_skill_installation(
            &pool,
            &make_installation_at("dedupe-delete", "cursor", &cursor_copy_dir, "copy", None),
        )
        .await
        .unwrap();
        db::upsert_skill_installation(
            &pool,
            &make_installation_at(
                "dedupe-delete",
                "claude-code",
                &claude_copy_dir,
                "copy",
                None,
            ),
        )
        .await
        .unwrap();

        let result = delete_central_skills_impl(
            &pool,
            &[
                BatchDeleteCentralSkillRequest {
                    skill_id: "dedupe-delete".to_string(),
                    remove_agent_ids: vec!["cursor".to_string()],
                },
                BatchDeleteCentralSkillRequest {
                    skill_id: "dedupe-delete".to_string(),
                    remove_agent_ids: vec!["claude-code".to_string(), "cursor".to_string()],
                },
            ],
        )
        .await
        .unwrap();

        assert_eq!(result.succeeded.len(), 1);
        assert!(result.failed.is_empty());
        let mut removed_agent_ids = result.succeeded[0].removed_agent_ids.clone();
        removed_agent_ids.sort();
        assert_eq!(
            removed_agent_ids,
            vec!["claude-code".to_string(), "cursor".to_string()]
        );
        assert!(!central_dir.exists());
        assert!(!cursor_copy_dir.exists());
        assert!(!claude_copy_dir.exists());
    }

    #[tokio::test]
    async fn test_get_skill_detail_returns_installations() {
        let pool = setup_test_db().await;

        let skill = make_skill("detail-skill", "Detail Skill", false);
        db::upsert_skill(&pool, &skill).await.unwrap();

        let now = Utc::now().to_rfc3339();
        db::upsert_skill_installation(
            &pool,
            &SkillInstallation {
                skill_id: "detail-skill".to_string(),
                agent_id: "claude-code".to_string(),
                installed_path: "/tmp/claude/detail-skill/SKILL.md".to_string(),
                link_type: "copy".to_string(),
                symlink_target: None,
                created_at: now.clone(),
            },
        )
        .await
        .unwrap();

        let detail = get_skill_detail_impl(&pool, "detail-skill").await.unwrap();
        assert_eq!(detail.id, "detail-skill");
        assert_eq!(detail.installations.len(), 1);
        assert_eq!(detail.installations[0].agent_id, "claude-code");
        // installed_at should be populated from created_at
        assert!(
            !detail.installations[0].installed_at.is_empty(),
            "installed_at must be set"
        );
        assert!(
            detail.collections.is_empty(),
            "skill should have no collections by default"
        );
    }

    #[tokio::test]
    async fn test_get_skill_detail_returns_collections() {
        let pool = setup_test_db().await;

        let skill = make_skill("detail-skill", "Detail Skill", false);
        db::upsert_skill(&pool, &skill).await.unwrap();

        let alpha = db::create_collection(&pool, "Alpha", Some("First collection"))
            .await
            .unwrap();
        let beta = db::create_collection(&pool, "Beta", None).await.unwrap();

        db::add_skill_to_collection(&pool, &alpha.id, "detail-skill")
            .await
            .unwrap();
        db::add_skill_to_collection(&pool, &beta.id, "detail-skill")
            .await
            .unwrap();

        let detail = get_skill_detail_impl(&pool, "detail-skill").await.unwrap();
        let collection_names: Vec<&str> =
            detail.collections.iter().map(|c| c.name.as_str()).collect();

        assert_eq!(collection_names, vec!["Alpha", "Beta"]);
    }

    #[tokio::test]
    async fn test_get_skill_detail_not_found() {
        let pool = setup_test_db().await;
        let result = get_skill_detail_impl(&pool, "nonexistent").await;
        assert!(result.is_err(), "should error for unknown skill_id");
    }

    // ── read_skill_content ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_read_skill_content_returns_file_content() {
        let tmp = TempDir::new().unwrap();
        let pool = setup_test_db().await;

        let skill_dir = tmp.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        let skill_md_path = skill_dir.join("SKILL.md");
        let expected_content = "---\nname: My Skill\n---\n\n# My Skill\n\nContent here.";
        fs::write(&skill_md_path, expected_content).unwrap();

        let skill = Skill {
            id: "my-skill".to_string(),
            name: "My Skill".to_string(),
            description: None,
            file_path: skill_md_path.to_string_lossy().into_owned(),
            canonical_path: None,
            is_central: false,
            source: None,
            content: None,
            scanned_at: Utc::now().to_rfc3339(),
        };
        db::upsert_skill(&pool, &skill).await.unwrap();

        let content = read_skill_content_impl(&pool, "my-skill").await.unwrap();
        assert_eq!(content, expected_content);
    }

    #[tokio::test]
    async fn test_read_skill_content_file_not_found() {
        let pool = setup_test_db().await;

        let skill = Skill {
            id: "missing-file-skill".to_string(),
            name: "Missing File".to_string(),
            description: None,
            file_path: "/nonexistent/SKILL.md".to_string(),
            canonical_path: None,
            is_central: false,
            source: None,
            content: None,
            scanned_at: Utc::now().to_rfc3339(),
        };
        db::upsert_skill(&pool, &skill).await.unwrap();

        let result = read_skill_content_impl(&pool, "missing-file-skill").await;
        assert!(result.is_err(), "should error when file does not exist");
    }

    // ── Testable core implementations (without Tauri State) ───────────────────

    async fn get_central_skills_impl(pool: &SqlitePool) -> Result<Vec<SkillWithLinks>, String> {
        let skills = db::get_central_skills(pool).await?;
        let agents = db::get_all_agents(pool).await?;
        let shared_root_agents = shared_root_agent_ids(&agents);
        let mut result = Vec::with_capacity(skills.len());
        for skill in skills {
            let installations = db::get_skill_installations(pool, &skill.id).await?;
            let mut linked_agents: Vec<String> =
                installations.into_iter().map(|i| i.agent_id).collect();
            append_missing_agents(&mut linked_agents, &shared_root_agents);
            let (created_at, updated_at) = skill_filesystem_timestamps(&skill);
            let repository_assignment =
                db::get_skill_repository_assignment(pool, &skill.id).await?;
            let tags = db::get_skill_tags_for_skill(pool, &skill.id).await?;
            result.push(SkillWithLinks {
                id: skill.id,
                name: skill.name,
                description: skill.description,
                file_path: skill.file_path,
                canonical_path: skill.canonical_path,
                is_central: skill.is_central,
                source: skill.source,
                scanned_at: skill.scanned_at,
                created_at,
                updated_at,
                linked_agents,
                shared_root_agents: shared_root_agents.clone(),
                repository: Some(repository_assignment.repository),
                tags,
                source_path: repository_assignment.source_path,
                is_source_unknown: repository_assignment.is_source_unknown,
            });
        }
        Ok(result)
    }

    async fn get_skill_detail_impl(
        pool: &SqlitePool,
        skill_id: &str,
    ) -> Result<SkillDetail, String> {
        super::get_skill_detail_with_row_impl(pool, skill_id, None, None).await
    }

    async fn read_skill_content_impl(pool: &SqlitePool, skill_id: &str) -> Result<String, String> {
        let skill = db::get_skill_by_id(pool, skill_id)
            .await?
            .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
        std::fs::read_to_string(&skill.file_path)
            .map_err(|e| format!("Failed to read '{}': {}", skill.file_path, e))
    }

    // ── Regression: get_skills_by_agent_impl returns installation metadata ─────

    /// `get_skills_by_agent_impl` must return `SkillForAgent` objects that
    /// include `link_type`, `dir_path`, and `symlink_target` from the
    /// installation record so the frontend `SkillCard` can show the correct
    /// source indicator.
    #[tokio::test]
    async fn test_get_skills_by_agent_impl_includes_installation_metadata() {
        let pool = setup_test_db().await;

        let skill = make_skill("meta-skill", "Meta Skill", false);
        db::upsert_skill(&pool, &skill).await.unwrap();

        db::upsert_skill_installation(
            &pool,
            &SkillInstallation {
                skill_id: "meta-skill".to_string(),
                agent_id: "claude-code".to_string(),
                installed_path: "/tmp/claude/meta-skill".to_string(),
                link_type: "symlink".to_string(),
                symlink_target: Some("/tmp/central/meta-skill".to_string()),
                created_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .unwrap();

        let skills = get_skills_by_agent_impl(&pool, "claude-code")
            .await
            .unwrap();
        assert_eq!(skills.len(), 1, "should find one skill for claude-code");

        let s = &skills[0];
        assert_eq!(s.id, "meta-skill");
        assert_eq!(
            s.link_type, "symlink",
            "link_type must come from installation record"
        );
        assert_eq!(
            s.dir_path, "/tmp/claude/meta-skill",
            "dir_path must be installed_path from installation record"
        );
        assert_eq!(
            s.symlink_target.as_deref(),
            Some("/tmp/central/meta-skill"),
            "symlink_target must be forwarded from installation record"
        );
    }

    #[tokio::test]
    async fn test_get_skills_by_agent_impl_empty_for_unknown_agent() {
        let pool = setup_test_db().await;
        let skills = get_skills_by_agent_impl(&pool, "nobody").await.unwrap();
        assert!(
            skills.is_empty(),
            "no skills for an agent with no installations"
        );
    }

    #[tokio::test]
    async fn test_get_skills_by_agent_impl_claude_uses_observations_for_duplicate_rows() {
        let pool = setup_test_db().await;

        db::upsert_agent_skill_observation(
            &pool,
            &make_observation(
                "claude-code::/tmp/.claude/skills/shared-skill",
                "shared-skill",
                "Shared Skill",
                "/tmp/.claude/skills/shared-skill",
                "user",
                false,
            ),
        )
        .await
        .unwrap();
        db::upsert_agent_skill_observation(
            &pool,
            &make_observation(
                "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
                "shared-skill",
                "Shared Skill",
                "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
                "plugin",
                true,
            ),
        )
        .await
        .unwrap();

        let mut skills = get_skills_by_agent_impl(&pool, "claude-code")
            .await
            .unwrap();
        skills.sort_by(|a, b| a.dir_path.cmp(&b.dir_path));

        assert_eq!(
            skills.len(),
            2,
            "Claude queries should surface duplicate logical skills from different sources"
        );
        assert_eq!(skills[0].id, "shared-skill");
        assert_eq!(skills[1].id, "shared-skill");
        assert_ne!(skills[0].dir_path, skills[1].dir_path);
    }

    #[tokio::test]
    async fn test_get_skills_by_agent_impl_claude_includes_source_identity_and_conflict_grouping() {
        let pool = setup_test_db().await;

        db::upsert_agent_skill_observation(
            &pool,
            &make_observation(
                "claude-code::/tmp/.claude/skills/shared-skill",
                "shared-skill",
                "Shared Skill",
                "/tmp/.claude/skills/shared-skill",
                "user",
                false,
            ),
        )
        .await
        .unwrap();
        db::upsert_agent_skill_observation(
            &pool,
            &make_observation(
                "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
                "shared-skill",
                "Shared Skill",
                "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
                "plugin",
                true,
            ),
        )
        .await
        .unwrap();

        let mut skills = get_skills_by_agent_impl(&pool, "claude-code")
            .await
            .unwrap();
        skills.sort_by(|a, b| a.dir_path.cmp(&b.dir_path));

        assert_eq!(skills.len(), 2);
        assert_eq!(
            skills[0].row_id,
            "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill"
        );
        assert_eq!(
            skills[1].row_id,
            "claude-code::/tmp/.claude/skills/shared-skill"
        );
        assert_eq!(skills[0].source_kind.as_deref(), Some("plugin"));
        assert_eq!(skills[1].source_kind.as_deref(), Some("user"));
        assert_eq!(
            skills[0].source_root.as_deref(),
            Some("/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0")
        );
        assert_eq!(
            skills[1].source_root.as_deref(),
            Some("/tmp/.claude/skills")
        );
        assert!(skills[0].is_read_only);
        assert!(!skills[1].is_read_only);
        assert_eq!(
            skills[0].conflict_group.as_deref(),
            Some("claude-code::shared-skill")
        );
        assert_eq!(
            skills[1].conflict_group.as_deref(),
            Some("claude-code::shared-skill")
        );
        assert_eq!(skills[0].conflict_count, 2);
        assert_eq!(skills[1].conflict_count, 2);
    }

    #[tokio::test]
    async fn test_get_skill_detail_with_row_impl_claude_plugin_row_uses_selected_observation() {
        let pool = setup_test_db().await;

        let skill = make_skill("shared-skill", "Shared Skill", false);
        db::upsert_skill(&pool, &skill).await.unwrap();
        db::upsert_skill_installation(
            &pool,
            &SkillInstallation {
                skill_id: "shared-skill".to_string(),
                agent_id: "claude-code".to_string(),
                installed_path: "/tmp/.claude/skills/shared-skill".to_string(),
                link_type: "copy".to_string(),
                symlink_target: None,
                created_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .unwrap();

        let collection = db::create_collection(&pool, "Alpha", None).await.unwrap();
        db::add_skill_to_collection(&pool, &collection.id, "shared-skill")
            .await
            .unwrap();

        db::upsert_agent_skill_observation(
            &pool,
            &make_observation(
                "claude-code::/tmp/.claude/skills/shared-skill",
                "shared-skill",
                "Shared Skill",
                "/tmp/.claude/skills/shared-skill",
                "user",
                false,
            ),
        )
        .await
        .unwrap();
        db::upsert_agent_skill_observation(
            &pool,
            &make_observation(
                "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
                "shared-skill",
                "Shared Skill",
                "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
                "plugin",
                true,
            ),
        )
        .await
        .unwrap();

        let detail = get_skill_detail_with_row_impl(
            &pool,
            "shared-skill",
            Some("claude-code"),
            Some("claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill"),
        )
        .await
        .unwrap();

        assert_eq!(
            detail.row_id,
            "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill"
        );
        assert_eq!(
            detail.dir_path,
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill"
        );
        assert_eq!(
            detail.file_path,
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill/SKILL.md"
        );
        assert_eq!(detail.source_kind.as_deref(), Some("plugin"));
        assert_eq!(
            detail.source_root.as_deref(),
            Some("/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0")
        );
        assert!(detail.is_read_only);
        assert_eq!(detail.conflict_count, 2);
        assert_eq!(
            detail.conflict_group.as_deref(),
            Some("claude-code::shared-skill")
        );
        assert!(
            detail.installations.is_empty(),
            "plugin detail should not expose manageable installations"
        );
        assert!(
            detail.collections.is_empty(),
            "plugin detail should not expose collection management state"
        );
    }

    #[tokio::test]
    async fn test_get_skill_detail_with_row_impl_claude_user_row_keeps_manageable_state() {
        let pool = setup_test_db().await;

        let skill = make_skill("shared-skill", "Shared Skill", false);
        db::upsert_skill(&pool, &skill).await.unwrap();
        db::upsert_skill_installation(
            &pool,
            &SkillInstallation {
                skill_id: "shared-skill".to_string(),
                agent_id: "claude-code".to_string(),
                installed_path: "/tmp/.claude/skills/shared-skill".to_string(),
                link_type: "copy".to_string(),
                symlink_target: None,
                created_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .unwrap();

        let collection = db::create_collection(&pool, "Alpha", None).await.unwrap();
        db::add_skill_to_collection(&pool, &collection.id, "shared-skill")
            .await
            .unwrap();

        db::upsert_agent_skill_observation(
            &pool,
            &make_observation(
                "claude-code::/tmp/.claude/skills/shared-skill",
                "shared-skill",
                "Shared Skill",
                "/tmp/.claude/skills/shared-skill",
                "user",
                false,
            ),
        )
        .await
        .unwrap();
        db::upsert_agent_skill_observation(
            &pool,
            &make_observation(
                "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
                "shared-skill",
                "Shared Skill",
                "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
                "plugin",
                true,
            ),
        )
        .await
        .unwrap();

        let detail = get_skill_detail_with_row_impl(
            &pool,
            "shared-skill",
            Some("claude-code"),
            Some("claude-code::/tmp/.claude/skills/shared-skill"),
        )
        .await
        .unwrap();

        assert_eq!(
            detail.row_id,
            "claude-code::/tmp/.claude/skills/shared-skill"
        );
        assert_eq!(detail.dir_path, "/tmp/.claude/skills/shared-skill");
        assert_eq!(detail.source_kind.as_deref(), Some("user"));
        assert!(!detail.is_read_only);
        assert_eq!(detail.conflict_count, 2);
        assert_eq!(detail.installations.len(), 1);
        assert_eq!(detail.collections.len(), 1);
    }

    #[tokio::test]
    async fn test_get_skills_by_agent_impl_copy_link_type() {
        let pool = setup_test_db().await;

        let skill = make_skill("copy-skill", "Copy Skill", false);
        db::upsert_skill(&pool, &skill).await.unwrap();

        db::upsert_skill_installation(
            &pool,
            &SkillInstallation {
                skill_id: "copy-skill".to_string(),
                agent_id: "cursor".to_string(),
                installed_path: "/tmp/cursor/copy-skill".to_string(),
                link_type: "copy".to_string(),
                symlink_target: None,
                created_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .unwrap();

        let skills = get_skills_by_agent_impl(&pool, "cursor").await.unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].link_type, "copy");
        assert!(
            skills[0].symlink_target.is_none(),
            "copy skills have no symlink target"
        );
    }

    // ── read_file_by_path ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_read_file_by_path_success() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("test-skill.md");
        let content = "---\nname: Test\n---\n\n# Test Skill";
        fs::write(&file_path, content).unwrap();

        let result = read_file_by_path_impl(&file_path.to_string_lossy());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), content);
    }

    #[tokio::test]
    async fn test_read_file_by_path_not_found() {
        let result = read_file_by_path_impl("/nonexistent/file.md");
        assert!(result.is_err());
    }

    // ── open_in_file_manager ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_open_in_file_manager_nonexistent_path() {
        let result = open_in_file_manager_checked_impl("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
    }
}
