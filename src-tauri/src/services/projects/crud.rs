//! 项目 CRUD + 扫描入口 (`add` / `list` / `rename` / `pin` / `rescan` /
//! `get_skills` / `remove`)。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::db::{self, DbPool, Project};

use super::scan::rescan_project;
use super::types::{ProjectDto, ProjectSkillDto};

/// 规范化项目路径：canonicalize 失败时退回到原始字符串，避免阻塞 add。
pub fn normalize_project_path(input: &str) -> String {
    let trimmed = input.trim();
    let raw = PathBuf::from(trimmed);
    let resolved = raw.canonicalize().unwrap_or_else(|_| raw.clone());
    let mut value = resolved.to_string_lossy().replace('\\', "/");
    while value.len() > 1 && value.ends_with('/') {
        value.pop();
    }
    value
}

/// sha256(规范化 path) 前 16 字符（hex）。
pub fn project_id_from_path(normalized_path: &str) -> String {
    let digest = Sha256::digest(normalized_path.as_bytes());
    let hex = digest.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    hex[..16].to_string()
}

fn project_name_from_path(normalized_path: &str) -> String {
    Path::new(normalized_path)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("project")
        .to_string()
}

/// 添加项目。path 已存在时返回旧记录（幂等）。
///
/// 注意：本函数仅落库，不触发扫描。扫描由 IPC 层在返回 Project 后异步起一条
/// `rescan_project` 任务执行。
pub async fn add_project_impl(pool: &DbPool, raw_path: &str) -> Result<Project, String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err("Project path cannot be empty".to_string());
    }

    let normalized = normalize_project_path(trimmed);
    if !Path::new(&normalized).is_dir() {
        return Err(format!(
            "Project path '{}' does not exist or is not a directory",
            normalized
        ));
    }

    if let Some(existing) = db::get_project_by_path(pool, &normalized).await? {
        return Ok(existing);
    }

    let project = Project {
        id: project_id_from_path(&normalized),
        path: normalized.clone(),
        name: project_name_from_path(&normalized),
        pinned: false,
        added_at: Utc::now().to_rfc3339(),
        last_scanned_at: None,
    };

    db::insert_project(pool, &project).await?;
    Ok(project)
}

/// 列出所有项目 + skill 数。
pub async fn list_projects_impl(pool: &DbPool) -> Result<Vec<ProjectDto>, String> {
    let projects = db::list_projects(pool).await?;

    let mut dtos = Vec::with_capacity(projects.len());
    for p in projects {
        let skills = db::list_project_skill_installations(pool, &p.id).await?;
        dtos.push(ProjectDto {
            id: p.id,
            path: p.path,
            name: p.name,
            pinned: p.pinned,
            added_at: p.added_at,
            last_scanned_at: p.last_scanned_at,
            skill_count: skills.len(),
        });
    }
    Ok(dtos)
}

pub async fn rename_project_impl(pool: &DbPool, id: &str, name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    if db::get_project_by_id(pool, id).await?.is_none() {
        return Err(format!("Project '{}' not found", id));
    }
    db::update_project_name(pool, id, trimmed).await
}

pub async fn set_project_pinned_impl(
    pool: &DbPool,
    id: &str,
    pinned: bool,
) -> Result<(), String> {
    if db::get_project_by_id(pool, id).await?.is_none() {
        return Err(format!("Project '{}' not found", id));
    }
    db::update_project_pinned(pool, id, pinned).await
}

/// 扫描项目并刷 psi。返回扫到的 skill 数量。
pub async fn rescan_project_impl(pool: &DbPool, id: &str) -> Result<usize, String> {
    rescan_project(pool, id).await
}

/// 获取一个项目下的全部 skill（含 agent display_name 渲染）。
pub async fn get_project_skills_impl(
    pool: &DbPool,
    id: &str,
) -> Result<Vec<ProjectSkillDto>, String> {
    if db::get_project_by_id(pool, id).await?.is_none() {
        return Err(format!("Project '{}' not found", id));
    }

    let psi_rows = db::list_project_skill_installations(pool, id).await?;
    let agents = db::get_all_agents(pool).await?;
    let agent_name: HashMap<String, String> = agents
        .iter()
        .map(|a| (a.id.clone(), a.display_name.clone()))
        .collect();

    // 关联 skills 表拿 name/description（中央 skill 才会有），项目本地的 skill 没有
    // 中央条目时回退到 skill_id / 空 description。后续阶段 3 可以再做强增强。
    let mut dtos = Vec::with_capacity(psi_rows.len());
    for psi in psi_rows {
        let display_name = agent_name
            .get(&psi.agent_id)
            .cloned()
            .unwrap_or_else(|| psi.agent_id.clone());

        let (name, description) = match sqlx::query_as::<_, (String, Option<String>)>(
            "SELECT name, description FROM skills WHERE id = ?",
        )
        .bind(&psi.skill_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        {
            Some((n, d)) => (n, d),
            None => (psi.skill_id.clone(), None),
        };

        dtos.push(ProjectSkillDto {
            project_id: psi.project_id,
            skill_id: psi.skill_id,
            name,
            description,
            agent_id: psi.agent_id,
            agent_display_name: display_name,
            installed_path: psi.installed_path,
            link_type: psi.link_type,
            symlink_target: psi.symlink_target,
        });
    }
    Ok(dtos)
}

/// 移除项目。`uninstall_skills=true` 时遍历 psi 删盘上文件再删表；否则只删表。
/// 装卸链路本身在阶段 3 暴露独立命令，本函数复用其底层 FS 操作（这里只删 symlink
/// 或 copy 的实文件，不走 service 层 install 模块以避免循环依赖）。
pub async fn remove_project_impl(
    pool: &DbPool,
    id: &str,
    uninstall_skills: bool,
) -> Result<(), String> {
    let project = db::get_project_by_id(pool, id)
        .await?
        .ok_or_else(|| format!("Project '{}' not found", id))?;

    if uninstall_skills {
        let psi_rows = db::list_project_skill_installations(pool, &project.id).await?;
        for psi in psi_rows {
            // 删盘上文件失败不要让整个 remove 中断：路径可能已被外部清理，
            // 表里的 psi 行还是要清掉。失败原因吞掉，只 log。
            let target = PathBuf::from(&psi.installed_path);
            let result = if psi.link_type == "symlink" {
                #[cfg(windows)]
                {
                    std::fs::remove_dir(&target).or_else(|_| std::fs::remove_file(&target))
                }
                #[cfg(not(windows))]
                {
                    std::fs::remove_file(&target)
                }
            } else {
                std::fs::remove_dir_all(&target)
            };
            if let Err(e) = result {
                tracing::warn!(
                    project_id = %project.id,
                    path = %target.display(),
                    error = %e,
                    "Failed to remove project skill on project removal; ignoring"
                );
            }
        }
    }

    db::delete_project(pool, &project.id).await
}
