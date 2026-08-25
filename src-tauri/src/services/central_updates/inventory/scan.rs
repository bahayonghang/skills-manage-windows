use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::db::{self, Agent, AgentSkillObservation, DbPool, SkillInstallation};
use crate::services::central_updates::CentralUpdatesError;

use super::{DeletedPlatformCopyGroup, PlatformDuplicateGroup};

/// 内核版本：接受 pool 直接传入，便于单元测试。
pub(crate) async fn scan_platform_duplicate_skills_with_pool(
    pool: &DbPool,
    agent_ids: Option<Vec<String>>,
) -> Result<Vec<PlatformDuplicateGroup>, CentralUpdatesError> {
    /*
     * 步骤1：拿 agents
     * 步骤2：对每个 agent 拿 observations
     * 步骤3：同 skill_id 分组，分 writable / plugin readonly
     *        只保留两类都存在的组
     */
    let agents = db::get_all_agents(pool).await?;
    let target_agent_ids: HashSet<String> = match agent_ids {
        Some(ids) => ids.into_iter().collect(),
        None => agents.iter().map(|a| a.id.clone()).collect(),
    };

    let mut groups = Vec::new();
    for agent in agents {
        if !target_agent_ids.contains(&agent.id) {
            continue;
        }
        let observations = db::get_agent_skill_observations(pool, &agent.id).await?;
        groups.extend(group_platform_duplicate_skills(&agent.id, &observations));
    }
    // 稳定排序，便于前端展示
    groups.sort_by(|a, b| {
        a.agent_id
            .cmp(&b.agent_id)
            .then_with(|| a.skill_id.cmp(&b.skill_id))
    });
    Ok(groups)
}

/// 内核版本：扫描平台目录中 Central 已不存在但仍可删除的本平台副本。
///
/// `cli_lock_protect` 仅在 Local target 为 true：读取本机 Skills CLI lock，
/// 排除 lock 拥有的 canonical 目录及指向它们的 symlink/junction。远程扫描
/// 不得使用本机 lock 证据（零 CLI 保护）。禁止仅因路径位于
/// `~/.agents/skills/` 就整棵排除——无 lock 条目的副本仍是合法 leftover。
pub(crate) async fn scan_deleted_platform_copies_with_pool(
    pool: &DbPool,
    agent_ids: Option<Vec<String>>,
    cli_lock_protect: bool,
) -> Result<Vec<DeletedPlatformCopyGroup>, CentralUpdatesError> {
    let ownership = if cli_lock_protect {
        Some(load_local_cli_lock_ownership()?)
    } else {
        None
    };
    scan_deleted_platform_copies_with_ownership(
        pool,
        agent_ids,
        ownership.as_ref(),
        &crate::paths::universal_skills_dir(),
    )
    .await
}

/// Scan leftover copies against an injected lock-ownership snapshot.
///
/// `ownership = None` is the remote / unprotected path. Tests inject a parsed
/// lock and a temp Universal root so they never read the machine HOME lock.
pub(crate) async fn scan_deleted_platform_copies_with_ownership(
    pool: &DbPool,
    agent_ids: Option<Vec<String>>,
    cli_ownership: Option<&crate::services::skills_cli::CliLockOwnership>,
    universal_skills_dir: &std::path::Path,
) -> Result<Vec<DeletedPlatformCopyGroup>, CentralUpdatesError> {
    let agents = db::get_all_agents(pool).await?;
    let target_agent_ids: HashSet<String> = match agent_ids {
        Some(ids) => ids.into_iter().collect(),
        None => agents
            .iter()
            .filter(|agent| agent.id != "central")
            .map(|agent| agent.id.clone())
            .collect(),
    };
    let central_skill_ids = db::get_central_skills(pool)
        .await?
        .into_iter()
        .map(|skill| skill.id)
        .collect::<HashSet<_>>();
    let is_cli_protected = |path: &str, agent: &crate::db::Agent| {
        let Some(ownership) = cli_ownership else {
            return false;
        };
        crate::services::skills_cli::classify_local_path_origin(
            std::path::Path::new(path),
            universal_skills_dir,
            ownership,
        ) == crate::services::skills_cli::LinkOrigin::SkillsCli
            || crate::services::skills_cli::is_mapped_agent_lock_copy(
                std::path::Path::new(path),
                std::path::Path::new(&agent.global_skills_dir),
                &agent.id,
                ownership,
            )
    };
    let mut grouped: HashMap<(String, String), (String, Vec<String>)> = HashMap::new();
    for agent in agents {
        if agent.id == "central" || !target_agent_ids.contains(&agent.id) {
            continue;
        }

        let observations = db::get_agent_skill_observations(pool, &agent.id).await?;
        for obs in observations {
            if !is_deleted_observation_candidate(&obs, &central_skill_ids) {
                continue;
            }
            if !is_candidate_path_within_agent_root(&agent, &obs.dir_path) {
                continue;
            }
            if is_cli_protected(&obs.dir_path, &agent) {
                continue;
            }
            if !is_candidate_entry_deletable_shape(&obs.dir_path) {
                continue;
            }
            push_deleted_candidate(
                &mut grouped,
                &obs.agent_id,
                &obs.skill_id,
                &obs.name,
                &obs.dir_path,
            );
        }

        let installations = sqlx::query_as::<_, SkillInstallation>(
            "SELECT si.* FROM skill_installations si
             LEFT JOIN skills s ON s.id = si.skill_id AND s.is_central = 1
             WHERE si.agent_id = ? AND s.id IS NULL",
        )
        .bind(&agent.id)
        .fetch_all(pool)
        .await?;
        for installation in installations {
            if installation.link_type == "native" {
                continue;
            }
            if !is_candidate_path_within_agent_root(&agent, &installation.installed_path) {
                continue;
            }
            if is_cli_protected(&installation.installed_path, &agent) {
                continue;
            }
            if !is_candidate_entry_deletable_shape(&installation.installed_path) {
                continue;
            }
            let skill_name = deleted_installation_skill_name(pool, &installation.skill_id).await?;
            push_deleted_candidate(
                &mut grouped,
                &installation.agent_id,
                &installation.skill_id,
                &skill_name,
                &installation.installed_path,
            );
        }
    }

    let mut groups = grouped
        .into_iter()
        .filter_map(|((agent_id, skill_id), (skill_name, writable_paths))| {
            (!writable_paths.is_empty()).then_some(DeletedPlatformCopyGroup {
                agent_id,
                skill_id,
                skill_name,
                writable_paths,
            })
        })
        .collect::<Vec<_>>();
    groups.sort_by(|a, b| {
        a.agent_id
            .cmp(&b.agent_id)
            .then_with(|| a.skill_id.cmp(&b.skill_id))
    });
    Ok(groups)
}

/// Load the Local machine's Skills CLI lock evidence for leftover exclusion.
///
/// A missing lock yields empty ownership; an unreadable lock fails the scan so
/// cleanup never deletes paths whose CLI ownership is undeterminable.
fn load_local_cli_lock_ownership(
) -> Result<crate::services::skills_cli::CliLockOwnership, CentralUpdatesError> {
    use crate::services::skills_cli::{load_cli_lock_ownership, skills_cli_lock_path};
    let home = crate::paths::resolve_home_dir();
    let lock_path = skills_cli_lock_path(&home);
    match load_cli_lock_ownership(&lock_path) {
        Ok(ownership) => Ok(ownership),
        Err(crate::services::skills_cli::SkillsCliError::Io { source, .. }) => {
            Err(CentralUpdatesError::Io {
                context: "read Skills CLI lock".to_string(),
                source,
            })
        }
        Err(_) => Err(CentralUpdatesError::Batch(
            "Skills CLI lock ownership could not be established.".to_string(),
        )),
    }
}

/// 纯函数：从某 agent 的 observations 中分出 writable + plugin readonly 同时存在的组。
/// 抽出来方便单元测试避开 DB。
pub(crate) fn group_platform_duplicate_skills(
    agent_id: &str,
    observations: &[AgentSkillObservation],
) -> Vec<PlatformDuplicateGroup> {
    let mut by_skill: HashMap<String, (Vec<String>, Vec<String>, String)> = HashMap::new();
    for obs in observations {
        let entry = by_skill
            .entry(obs.skill_id.clone())
            .or_insert_with(|| (Vec::new(), Vec::new(), obs.name.clone()));
        let is_plugin = obs.source_kind == "plugin" || obs.is_read_only;
        if is_plugin {
            entry.1.push(obs.dir_path.clone());
        } else {
            entry.0.push(obs.dir_path.clone());
        }
    }
    let mut groups = Vec::new();
    for (skill_id, (writable, plugin, name)) in by_skill {
        if writable.is_empty() || plugin.is_empty() {
            continue;
        }
        groups.push(PlatformDuplicateGroup {
            agent_id: agent_id.to_string(),
            skill_id,
            skill_name: name,
            writable_paths: writable,
            plugin_paths: plugin,
        });
    }
    groups
}

#[cfg(test)]
pub(crate) fn group_deleted_platform_copies(
    observations: &[AgentSkillObservation],
    central_skill_ids: &HashSet<String>,
) -> Vec<DeletedPlatformCopyGroup> {
    let mut grouped: HashMap<(String, String), (String, Vec<String>)> = HashMap::new();
    for obs in observations {
        if !is_deleted_observation_candidate(obs, central_skill_ids) {
            continue;
        }
        push_deleted_candidate(
            &mut grouped,
            &obs.agent_id,
            &obs.skill_id,
            &obs.name,
            &obs.dir_path,
        );
    }
    grouped
        .into_iter()
        .filter_map(|((agent_id, skill_id), (skill_name, writable_paths))| {
            (!writable_paths.is_empty()).then_some(DeletedPlatformCopyGroup {
                agent_id,
                skill_id,
                skill_name,
                writable_paths,
            })
        })
        .collect()
}

fn is_deleted_observation_candidate(
    obs: &AgentSkillObservation,
    central_skill_ids: &HashSet<String>,
) -> bool {
    !central_skill_ids.contains(&obs.skill_id)
        && !obs.is_read_only
        && obs.source_kind != "plugin"
        && obs.link_type != "native"
}

fn push_deleted_candidate(
    grouped: &mut HashMap<(String, String), (String, Vec<String>)>,
    agent_id: &str,
    skill_id: &str,
    skill_name: &str,
    path: &str,
) {
    let entry = grouped
        .entry((agent_id.to_string(), skill_id.to_string()))
        .or_insert_with(|| (skill_name.to_string(), Vec::new()));
    if !entry.1.iter().any(|existing| paths_match(existing, path)) {
        entry.1.push(path.to_string());
    }
}

async fn deleted_installation_skill_name(
    pool: &DbPool,
    skill_id: &str,
) -> Result<String, CentralUpdatesError> {
    Ok(db::get_skill_by_id(pool, skill_id)
        .await?
        .map(|skill| skill.name)
        .unwrap_or_else(|| skill_id.to_string()))
}

fn paths_match(left: &str, right: &str) -> bool {
    let left = Path::new(left);
    let right = Path::new(right);
    crate::paths::paths_equivalent(left, right)
}

fn is_candidate_path_within_agent_root(agent: &Agent, path: &str) -> bool {
    let root = Path::new(&agent.global_skills_dir);
    let child = Path::new(path);
    if crate::paths::paths_equivalent(root, child) {
        return false;
    }
    let root_cmp = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let Some(child_parent) = child.parent() else {
        return false;
    };
    let child_parent_cmp = child_parent
        .canonicalize()
        .unwrap_or_else(|_| child_parent.to_path_buf());
    child_parent_cmp.starts_with(root_cmp)
}

fn is_candidate_entry_deletable_shape(path: &str) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_dir() => true,
        Ok(_) => false,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(_) => false,
    }
}
