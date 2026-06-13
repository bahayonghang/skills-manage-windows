use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::db;
pub(super) use crate::skill_time::skill_filesystem_timestamps;

use super::error::CentralSkillsError;
use super::types::SkillInstallationDetail;

/// Run a synchronous filesystem task on the blocking-thread pool with
/// Central-Skills-domain errors. Thin typed wrapper over
/// [`crate::fs_util::run_blocking_fs_with`].
pub(super) async fn run_blocking_fs<T, F>(
    label: &'static str,
    task: F,
) -> Result<T, CentralSkillsError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, CentralSkillsError> + Send + 'static,
{
    crate::fs_util::run_blocking_fs_with(label, task, CentralSkillsError::task_join).await
}

pub(super) fn skill_dir_path(skill: &db::Skill) -> String {
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

pub(super) fn claude_conflict_metadata(
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

pub(super) fn installation_details(
    installations: Vec<db::SkillInstallation>,
) -> Vec<SkillInstallationDetail> {
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

pub(super) fn shared_root_agent_ids(agents: &[db::Agent]) -> Vec<String> {
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

pub(super) fn append_missing_agents(linked_agents: &mut Vec<String>, extra_agents: &[String]) {
    let mut seen: HashSet<String> = linked_agents.iter().cloned().collect();
    for agent_id in extra_agents {
        if seen.insert(agent_id.clone()) {
            linked_agents.push(agent_id.clone());
        }
    }
}

pub(super) fn unique_agent_ids(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for id in ids {
        if seen.insert(id.clone()) {
            result.push(id);
        }
    }
    result
}
