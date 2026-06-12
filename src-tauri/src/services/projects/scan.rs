//! 单点扫描：遍历已启用 agent，对每个项目内 skill 目录扫盘并落 psi。
//!
//! 与旧 Discover 的 `services::discovery::scan` 的区别：
//! - 不再做全盘递归遍历，只在「项目根 / agent.project_skills_dir」下做一级扫描
//! - 不再走 `discovered_skills` 表，直接 UPSERT 到 `project_skill_installations`
//! - reconcile 范围限制在当前 project_id，不影响其它项目

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::db::{self, DbPool, ProjectSkillInstallation};
use crate::services::installation::project::project_relative_skills_dir;
use crate::services::scanner::scan_directory;

use super::error::ProjectsError;

const UNIVERSAL_LEGACY_PROJECT_SKILLS_DIRS: [&str; 2] = [".codex/skills", ".opencode/skills"];

#[derive(Clone)]
struct ProjectScanTarget {
    agent: db::Agent,
    rel: PathBuf,
    priority: usize,
}

/// 检测目录项的 link_type。
///
/// 与 `services::scanner::detect_link_type` 不同：项目场景下永远不可能是 `native`
/// （native 是中央目录专属），因此只区分 `symlink` 和 `copy`。
fn detect_project_link_type(path: &Path) -> (String, Option<String>) {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            let target = std::fs::read_link(path)
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()));
            ("symlink".to_string(), target)
        }
        _ => ("copy".to_string(), None),
    }
}

fn select_universal_representative(agents: &[db::Agent]) -> Option<db::Agent> {
    for preferred_id in db::UNIVERSAL_PROJECT_REPRESENTATIVE_AGENT_IDS {
        if let Some(agent) = agents.iter().find(|agent| agent.id == preferred_id) {
            return Some(agent.clone());
        }
    }

    agents
        .iter()
        .find(|agent| db::is_universal_project_agent(&agent.id))
        .cloned()
}

fn build_project_scan_targets(enabled_agents: &[db::Agent]) -> Vec<ProjectScanTarget> {
    let mut targets = Vec::new();
    let mut seen = HashSet::<(String, PathBuf)>::new();
    let universal_representative = select_universal_representative(enabled_agents);

    for agent in enabled_agents {
        if agent.id == "central" {
            continue;
        }

        if db::is_universal_project_agent(&agent.id) {
            continue;
        }

        let rel = match project_relative_skills_dir(agent) {
            Ok(rel) => rel,
            Err(_) => continue,
        };
        let key = (agent.id.clone(), rel.clone());
        if seen.insert(key) {
            targets.push(ProjectScanTarget {
                agent: agent.clone(),
                rel,
                priority: 0,
            });
        }
    }

    let Some(universal_agent) = universal_representative else {
        return targets;
    };

    let canonical = PathBuf::from(db::UNIVERSAL_PROJECT_SKILLS_DIR);
    if seen.insert((universal_agent.id.clone(), canonical.clone())) {
        targets.push(ProjectScanTarget {
            agent: universal_agent.clone(),
            rel: canonical,
            priority: 0,
        });
    }

    for (index, legacy_rel) in UNIVERSAL_LEGACY_PROJECT_SKILLS_DIRS.iter().enumerate() {
        let rel = PathBuf::from(legacy_rel);
        if seen.insert((universal_agent.id.clone(), rel.clone())) {
            targets.push(ProjectScanTarget {
                agent: universal_agent.clone(),
                rel,
                priority: index + 1,
            });
        }
    }

    targets
}

fn scan_project_target(
    project_id: &str,
    project_root: &Path,
    target: &ProjectScanTarget,
    now: &str,
) -> Vec<ProjectSkillInstallation> {
    let skill_dir = project_root.join(&target.rel);
    if !skill_dir.exists() {
        return Vec::new();
    }

    // is_central=false：项目级目录下永远不是中央存储。
    let skills = scan_directory(&skill_dir, false);
    skills
        .into_iter()
        .map(|s| {
            // 用 symlink_metadata 重新核一遍 link_type，scan_directory 也做了但拿不到
            // 我们想要的精确字段；这里就近读一次保证 psi 字段权威。
            let entry_path = Path::new(&s.dir_path);
            let (link_type, symlink_target) = detect_project_link_type(entry_path);

            ProjectSkillInstallation {
                project_id: project_id.to_string(),
                skill_id: s.id,
                name: s.name,
                description: s.description,
                file_path: crate::paths::normalize_stored_path(&s.file_path),
                source_origin: "project".to_string(),
                agent_id: target.agent.id.clone(),
                installed_path: crate::paths::normalize_stored_path(&s.dir_path),
                link_type,
                symlink_target,
                created_at: now.to_string(),
            }
        })
        .collect()
}

/// 阻塞地扫一个项目根：遍历已启用 agent，逐个 agent 的 project_skills_dir 做扫描。
///
/// 返回该项目下所有 (psi_row, agent_id_seen) 的列表，调用方负责落库与 reconcile。
fn scan_project_blocking(
    project_id: String,
    project_root: PathBuf,
    enabled_agents: Vec<db::Agent>,
    now: String,
) -> Vec<ProjectSkillInstallation> {
    let targets = build_project_scan_targets(&enabled_agents);
    let mut found_by_key = HashMap::<(String, String), (usize, ProjectSkillInstallation)>::new();

    for target in targets {
        for psi in scan_project_target(&project_id, &project_root, &target, &now) {
            let key = (psi.skill_id.clone(), psi.agent_id.clone());
            let should_replace = match found_by_key.get(&key) {
                Some((existing_priority, _existing)) => target.priority < *existing_priority,
                None => true,
            };
            if should_replace {
                found_by_key.insert(key, (target.priority, psi));
            }
        }
    }

    found_by_key.into_values().map(|(_, psi)| psi).collect()
}

/// 扫描指定项目并落 psi。
pub async fn rescan_project(pool: &DbPool, project_id: &str) -> Result<usize, ProjectsError> {
    let project = db::get_project_by_id(pool, project_id)
        .await?
        .ok_or_else(|| ProjectsError::ProjectNotFound(project_id.to_string()))?;

    let agents = db::get_all_agents(pool).await?;
    let enabled: Vec<db::Agent> = agents
        .into_iter()
        .filter(|a| a.is_enabled && a.id != "central")
        .collect();

    let project_root = PathBuf::from(&project.path);
    if !project_root.exists() {
        // 项目根盘已不存在：清空 psi 并仍然刷 last_scanned_at，让前端能感知。
        db::delete_stale_project_skill_installations(pool, project_id, &[]).await?;
        let now = Utc::now().to_rfc3339();
        db::update_project_last_scanned(pool, project_id, &now).await?;
        return Ok(0);
    }

    let now = Utc::now().to_rfc3339();
    let project_id_owned = project_id.to_string();
    let now_clone = now.clone();
    let found = crate::fs_util::run_blocking_fs_with(
        "project scan",
        move || {
            Ok(scan_project_blocking(
                project_id_owned,
                project_root,
                enabled,
                now_clone,
            ))
        },
        ProjectsError::task_join,
    )
    .await?;

    let central_skills = central_skill_map_for_project_scan(pool, &found).await?;
    let rows = found
        .iter()
        .cloned()
        .map(|mut psi| {
            if has_central_match(&central_skills, &psi) {
                psi.source_origin = "central".to_string();
            }
            psi
        })
        .collect::<Vec<_>>();

    db::persist_project_skill_scan(pool, project_id, &rows, &now).await?;

    Ok(found.len())
}

async fn central_skill_map_for_project_scan(
    pool: &DbPool,
    rows: &[ProjectSkillInstallation],
) -> Result<HashMap<String, db::Skill>, ProjectsError> {
    let skill_ids = rows
        .iter()
        .filter(|psi| psi.link_type == "symlink")
        .filter_map(|psi| {
            psi.symlink_target
                .as_deref()
                .filter(|target| !target.trim().is_empty())
                .map(|_| psi.skill_id.clone())
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    Ok(db::get_skills_by_ids(pool, &skill_ids)
        .await?
        .into_iter()
        .filter(|(_, skill)| skill.is_central)
        .collect())
}

fn has_central_match(
    central_skills: &HashMap<String, db::Skill>,
    psi: &ProjectSkillInstallation,
) -> bool {
    if psi.link_type != "symlink" {
        return false;
    }

    let target = match psi.symlink_target.as_deref() {
        Some(target) if !target.trim().is_empty() => target,
        _ => return false,
    };
    let central_skill = match central_skills.get(&psi.skill_id) {
        Some(skill) => skill,
        None => return false,
    };
    let Some(canonical_path) = central_skill.canonical_path.as_deref() else {
        return false;
    };

    let installed_parent = Path::new(&psi.installed_path)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let target_path = PathBuf::from(target);
    let target_path = if target_path.is_absolute() {
        target_path
    } else {
        installed_parent.join(target_path)
    };

    crate::paths::paths_equivalent(&target_path, Path::new(&canonical_path))
}
