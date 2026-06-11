//! 项目 CRUD + 扫描入口 (`add` / `list` / `rename` / `pin` / `rescan` /
//! `get_skills` / `remove`)。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::db::{self, DbPool, Project, ProjectSkillInstallation};
use crate::fs_util::run_blocking_fs;
use crate::services::installation::centralize::ensure_replaceable_target;
use crate::services::installation::fs_util::copy_dir_all_blocking;
use crate::services::installation::project::project_relative_skills_dir;
use crate::services::installation::{create_symlink, symlink_target_path};

use super::scan::rescan_project;
use super::types::{ProjectDto, ProjectSkillDto, ProjectUsingSkillDto};

/// 规范化项目路径：canonicalize 失败时退回到原始字符串，避免阻塞 add。
pub fn normalize_project_path(input: &str) -> String {
    let trimmed = input.trim();
    let raw = PathBuf::from(trimmed);
    let resolved = raw.canonicalize().unwrap_or_else(|_| raw.clone());
    crate::paths::normalize_stored_path(&resolved.to_string_lossy())
}

/// sha256(规范化 path) 前 16 字符（hex）。
pub fn project_id_from_path(normalized_path: &str) -> String {
    let digest = Sha256::digest(normalized_path.as_bytes());
    let hex = digest
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
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
    let legacy_extended_path = format!("//?/{}", normalized);
    if let Some(mut existing) = db::get_project_by_path(pool, &legacy_extended_path).await? {
        db::update_project_path(pool, &existing.id, &normalized).await?;
        existing.path = normalized;
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

pub async fn set_project_pinned_impl(pool: &DbPool, id: &str, pinned: bool) -> Result<(), String> {
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

    // psi 行保存扫描期元数据；旧 DB 行可能为空，此时再回退到中央 skills 表。
    let mut dtos = Vec::with_capacity(psi_rows.len());
    for psi in psi_rows {
        let display_name = agent_name
            .get(&psi.agent_id)
            .cloned()
            .unwrap_or_else(|| psi.agent_id.clone());

        let (name, description) = if !psi.name.trim().is_empty() {
            (psi.name.clone(), psi.description.clone())
        } else {
            match sqlx::query_as::<_, (String, Option<String>)>(
                "SELECT name, description FROM skills WHERE id = ?",
            )
            .bind(&psi.skill_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?
            {
                Some((n, d)) => (n, d),
                None => (psi.skill_id.clone(), None),
            }
        };

        dtos.push(ProjectSkillDto {
            project_id: psi.project_id,
            skill_id: psi.skill_id,
            name,
            description,
            file_path: psi.file_path,
            source_origin: psi.source_origin,
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
        let project_id_for_log = project.id.clone();
        run_blocking_fs("project skills removal", move || {
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
                        project_id = %project_id_for_log,
                        path = %target.display(),
                        error = %e,
                        "Failed to remove project skill on project removal; ignoring"
                    );
                }
            }
            Ok(())
        })
        .await?;
    }

    db::delete_project(pool, &project.id).await
}

/// 反向查询：一个中央 skill 装在哪些项目下。
///
/// 用于中央 skill 详情页 sidebar 显示「装在哪些项目」section。返回行按
/// 项目 pinned → name → agent_id 排序，前端不需要再排。
pub async fn list_projects_using_skill_impl(
    pool: &DbPool,
    skill_id: &str,
) -> Result<Vec<ProjectUsingSkillDto>, String> {
    let agents = db::get_all_agents(pool).await?;
    let agent_name: HashMap<String, String> = agents
        .iter()
        .map(|a| (a.id.clone(), a.display_name.clone()))
        .collect();

    let rows = sqlx::query_as::<_, (String, String, String, bool, String, String, String)>(
        "SELECT psi.project_id,
                p.name,
                p.path,
                p.pinned,
                psi.agent_id,
                psi.installed_path,
                psi.link_type
           FROM project_skill_installations psi
           JOIN projects p ON p.id = psi.project_id
          WHERE psi.skill_id = ?
          ORDER BY p.pinned DESC, p.name COLLATE NOCASE ASC, psi.agent_id ASC",
    )
    .bind(skill_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(
            |(
                project_id,
                project_name,
                project_path,
                _pinned,
                agent_id,
                installed_path,
                link_type,
            )| {
                let agent_display_name = agent_name
                    .get(&agent_id)
                    .cloned()
                    .unwrap_or_else(|| agent_id.clone());
                ProjectUsingSkillDto {
                    project_id,
                    project_name,
                    project_path,
                    agent_id,
                    agent_display_name,
                    installed_path,
                    link_type,
                }
            },
        )
        .collect())
}

// ─── Install / Uninstall (Stage 3) ───────────────────────────────────────────

/// 将中央 skill 安装到指定项目的某个 agent 目录下。
///
/// 约束：
/// - 中央 skill 必须 `is_central=true` 且 `canonical_path` 非空（防止把孤儿 skill
///   重复 centralize；不复用 `ensure_centralized` 避免阶段 3 触发隐式状态变更）。
/// - target_dir 不存在自动创建。
/// - target_path（最终落点）必须不存在；若已存在符号链接则替换，已存在实目录/文件
///   则拒绝。
/// - `method` 接受 `"symlink"` | `"copy"`，其他值按 `"symlink"` 处理。symlink 创建
///   失败（Windows 未开发者模式等）原样向上抛错误字符串，前端 toast 透传。
pub async fn install_skill_to_project_impl(
    pool: &DbPool,
    project_id: &str,
    skill_id: &str,
    agent_id: &str,
    method: &str,
) -> Result<ProjectSkillInstallation, String> {
    let project = db::get_project_by_id(pool, project_id)
        .await?
        .ok_or_else(|| format!("Project '{}' not found", project_id))?;

    let project_root = PathBuf::from(&project.path);
    if !project_root.is_dir() {
        return Err(format!(
            "Project path '{}' is missing or not a directory",
            project.path
        ));
    }

    if agent_id == "central" {
        return Err("Cannot install a project skill to the central agent itself".to_string());
    }

    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;
    if !agent.is_enabled {
        return Err(format!("Agent '{}' is disabled", agent.display_name));
    }

    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found in central library", skill_id))?;
    if !skill.is_central {
        return Err(format!(
            "Skill '{}' is not centralized; centralize it before installing to a project",
            skill_id
        ));
    }
    let canonical_path = skill
        .canonical_path
        .clone()
        .filter(|p| !p.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "Skill '{}' has no canonical_path; cannot install to a project",
                skill_id
            )
        })?;
    let canonical_dir = PathBuf::from(&canonical_path);
    if !canonical_dir.is_dir() {
        return Err(format!(
            "Central skill directory '{}' does not exist",
            canonical_dir.display()
        ));
    }

    let relative_skills_dir = project_relative_skills_dir(&agent).map_err(|e| e.to_string())?;
    let project_skills_dir = project_root.join(&relative_skills_dir);
    let target_path = project_skills_dir.join(skill_id);

    let project_skills_dir_for_create = project_skills_dir.clone();
    run_blocking_fs("project skills directory creation", move || {
        std::fs::create_dir_all(&project_skills_dir_for_create).map_err(|e| {
            format!(
                "Failed to create project skills directory '{}': {}",
                project_skills_dir_for_create.display(),
                e
            )
        })
    })
    .await?;

    ensure_replaceable_target(&target_path)
        .await
        .map_err(|e| e.to_string())?;

    let resolved_method = if method == "copy" { "copy" } else { "symlink" };
    let (link_type, symlink_target_str) = if resolved_method == "copy" {
        copy_dir_all_blocking(&canonical_dir, &target_path)
            .await
            .map_err(|e| e.to_string())?;
        ("copy".to_string(), None)
    } else {
        let relative_target = symlink_target_path(&project_skills_dir, &canonical_dir);
        let target_for_create = target_path.clone();
        let target_value = relative_target.clone();
        run_blocking_fs("project skill symlink creation", move || {
            create_symlink(&target_value, &target_for_create).map_err(|e| e.to_string())
        })
        .await?;
        (
            "symlink".to_string(),
            Some(relative_target.to_string_lossy().into_owned()),
        )
    };

    let psi = ProjectSkillInstallation {
        project_id: project.id.clone(),
        skill_id: skill_id.to_string(),
        name: skill.name,
        description: skill.description,
        file_path: crate::paths::normalize_stored_path(
            &target_path.join("SKILL.md").to_string_lossy(),
        ),
        source_origin: "central".to_string(),
        agent_id: agent_id.to_string(),
        installed_path: crate::paths::normalize_stored_path(&target_path.to_string_lossy()),
        link_type,
        symlink_target: symlink_target_str,
        created_at: Utc::now().to_rfc3339(),
    };
    db::upsert_project_skill_installation(pool, &psi).await?;

    Ok(psi)
}

/// 从项目某个 agent 目录卸载 skill。
///
/// 行为：
/// - 必须在 psi 中存在；否则 `Err`（不做 fallback 推断）。
/// - `link_type=symlink` → `remove_file`（Unix）/ `remove_dir`（Windows）。
/// - `link_type=copy`   → `remove_dir_all`。
/// - 删 psi 行作为最后一步；FS 操作失败不删表，留给用户自查。
pub async fn uninstall_skill_from_project_impl(
    pool: &DbPool,
    project_id: &str,
    skill_id: &str,
    agent_id: &str,
) -> Result<(), String> {
    let psi = db::get_project_skill_installation(pool, project_id, skill_id, agent_id)
        .await?
        .ok_or_else(|| {
            format!(
                "Skill '{}' is not installed in project '{}' for agent '{}'",
                skill_id, project_id, agent_id
            )
        })?;

    let target = PathBuf::from(&psi.installed_path);
    let link_type = psi.link_type.clone();
    let target_for_remove = target.clone();
    run_blocking_fs("project skill removal", move || {
        if !target_for_remove.exists() && std::fs::symlink_metadata(&target_for_remove).is_err() {
            return Ok(());
        }
        if link_type == "symlink" {
            #[cfg(windows)]
            {
                std::fs::remove_dir(&target_for_remove)
                    .or_else(|_| std::fs::remove_file(&target_for_remove))
                    .map_err(|e| {
                        format!(
                            "Failed to remove symlink '{}': {}",
                            target_for_remove.display(),
                            e
                        )
                    })
            }
            #[cfg(not(windows))]
            {
                std::fs::remove_file(&target_for_remove).map_err(|e| {
                    format!(
                        "Failed to remove symlink '{}': {}",
                        target_for_remove.display(),
                        e
                    )
                })
            }
        } else {
            std::fs::remove_dir_all(&target_for_remove)
                .map_err(|e| format!("Failed to remove '{}': {}", target_for_remove.display(), e))
        }
    })
    .await?;

    db::delete_project_skill_installation(pool, project_id, skill_id, agent_id).await
}
