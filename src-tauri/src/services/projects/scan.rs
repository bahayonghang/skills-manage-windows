//! 单点扫描：遍历已启用 agent，对每个项目内 skill 目录扫盘并落 psi。
//!
//! 与旧 Discover 的 `services::discovery::scan` 的区别：
//! - 不再做全盘递归遍历，只在「项目根 / agent.project_skills_dir」下做一级扫描
//! - 不再走 `discovered_skills` 表，直接 UPSERT 到 `project_skill_installations`
//! - reconcile 范围限制在当前 project_id，不影响其它项目

use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::db::{self, DbPool, ProjectSkillInstallation};
use crate::services::installation::project::project_relative_skills_dir;
use crate::services::scanner::scan_directory;

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

/// 阻塞地扫一个项目根：遍历已启用 agent，逐个 agent 的 project_skills_dir 做扫描。
///
/// 返回该项目下所有 (psi_row, agent_id_seen) 的列表，调用方负责落库与 reconcile。
fn scan_project_blocking(
    project_id: String,
    project_root: PathBuf,
    enabled_agents: Vec<db::Agent>,
    now: String,
) -> Vec<ProjectSkillInstallation> {
    let mut found = Vec::new();
    for agent in &enabled_agents {
        if agent.id == "central" {
            continue;
        }
        let rel = match project_relative_skills_dir(agent) {
            Ok(rel) => rel,
            Err(_) => continue,
        };
        let skill_dir = project_root.join(&rel);
        if !skill_dir.exists() {
            continue;
        }

        // is_central=false：项目级目录下永远不是中央存储。
        let skills = scan_directory(&skill_dir, false);
        for s in skills {
            // 用 symlink_metadata 重新核一遍 link_type，scan_directory 也做了但拿不到
            // 我们想要的精确字段；这里就近读一次保证 psi 字段权威。
            let entry_path = Path::new(&s.dir_path);
            let (link_type, symlink_target) = detect_project_link_type(entry_path);

            found.push(ProjectSkillInstallation {
                project_id: project_id.clone(),
                skill_id: s.id,
                agent_id: agent.id.clone(),
                installed_path: s.dir_path,
                link_type,
                symlink_target,
                created_at: now.clone(),
            });
        }
    }
    found
}

/// 扫描指定项目并落 psi。
pub async fn rescan_project(pool: &DbPool, project_id: &str) -> Result<usize, String> {
    let project = db::get_project_by_id(pool, project_id)
        .await?
        .ok_or_else(|| format!("Project '{}' not found", project_id))?;

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
    let found = tauri::async_runtime::spawn_blocking(move || {
        scan_project_blocking(project_id_owned, project_root, enabled, now_clone)
    })
    .await
    .map_err(|e| format!("Failed to join project scan task: {}", e))?;

    // 先全量 upsert，再 reconcile 掉本项目下消失的 psi。
    for psi in &found {
        db::upsert_project_skill_installation(pool, psi).await?;
    }

    let kept_pairs: Vec<(String, String)> = found
        .iter()
        .map(|p| (p.skill_id.clone(), p.agent_id.clone()))
        .collect();
    db::delete_stale_project_skill_installations(pool, project_id, &kept_pairs).await?;

    db::update_project_last_scanned(pool, project_id, &now).await?;

    Ok(found.len())
}
