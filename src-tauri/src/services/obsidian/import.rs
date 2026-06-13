use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::db::{self, DbPool};
use crate::fs_util::run_blocking_fs_with;
use crate::paths;
use crate::services::installation::{copy_dir_all_blocking, create_symlink, symlink_target_path};
use crate::services::scanner::parse_skill_md;

use super::error::ObsidianError;
use super::types::ObsidianImportResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObsidianInstallMethod {
    Symlink,
    Copy,
}

impl ObsidianInstallMethod {
    fn parse(method: Option<&str>) -> Result<Self, ObsidianError> {
        match method.unwrap_or("symlink") {
            "symlink" | "auto" => Ok(Self::Symlink),
            "copy" => Ok(Self::Copy),
            other => Err(ObsidianError::UnsupportedInstallMethod(other.to_string())),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Symlink => "symlink",
            Self::Copy => "copy",
        }
    }
}

fn skill_dir_name(source_dir: &Path) -> Result<String, ObsidianError> {
    source_dir
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string)
        .ok_or(ObsidianError::SkillDirNameUnavailable)
}

pub async fn import_obsidian_skill_to_central_impl(
    pool: &DbPool,
    dir_path: &str,
) -> Result<ObsidianImportResult, ObsidianError> {
    let central_dir = paths::central_skills_dir();
    let source_dir = Path::new(dir_path).to_path_buf();
    let source_dir_for_check = source_dir.clone();
    let source_is_dir = run_blocking_fs_with(
        "Central import source inspection",
        move || Ok(source_dir_for_check.is_dir()),
        ObsidianError::task_join,
    )
    .await?;
    if !source_is_dir {
        return Err(ObsidianError::SourceDirMissing(
            source_dir.display().to_string(),
        ));
    }

    let skill_dir_name = skill_dir_name(&source_dir)?;
    let target_dir = central_dir.join(&skill_dir_name);

    let target_dir_for_check = target_dir.clone();
    let target_exists = run_blocking_fs_with(
        "Central import target inspection",
        move || Ok(target_dir_for_check.exists()),
        ObsidianError::task_join,
    )
    .await?;
    if target_exists {
        return Err(ObsidianError::CentralSkillExists(skill_dir_name));
    }

    copy_dir_all_blocking(&source_dir, &target_dir).await?;

    let skill_md_path = target_dir.join("SKILL.md");
    let info = parse_skill_md(&skill_md_path);

    if let Some(skill_info) = info {
        let now = Utc::now().to_rfc3339();
        let db_skill = db::Skill {
            id: skill_dir_name.clone(),
            name: skill_info.name,
            description: skill_info.description,
            file_path: skill_md_path.to_string_lossy().into_owned(),
            canonical_path: Some(target_dir.to_string_lossy().into_owned()),
            is_central: true,
            source: Some("copy".to_string()),
            content: None,
            scanned_at: now,
            fs_created_at: None,
            fs_updated_at: None,
        };
        db::upsert_skill(pool, &db_skill).await?;
    }

    Ok(ObsidianImportResult {
        skill_id: skill_dir_name,
        target: "central".to_string(),
    })
}

pub async fn import_obsidian_skill_to_platform_impl(
    pool: &DbPool,
    dir_path: &str,
    agent_id: &str,
    method: Option<&str>,
) -> Result<ObsidianImportResult, ObsidianError> {
    let install_method = ObsidianInstallMethod::parse(method)?;
    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| ObsidianError::AgentNotFound(agent_id.to_string()))?;
    let agent_dir = PathBuf::from(&agent.global_skills_dir);
    let source_dir = Path::new(dir_path).to_path_buf();
    let skill_dir_name = skill_dir_name(&source_dir)?;
    let target_path = agent_dir.join(&skill_dir_name);

    let agent_dir_for_create = agent_dir.clone();
    run_blocking_fs_with(
        "agent skills directory creation",
        move || {
            std::fs::create_dir_all(&agent_dir_for_create)
                .map_err(|e| ObsidianError::io("Failed to create agent skills directory", e))
        },
        ObsidianError::task_join,
    )
    .await?;

    let target_path_for_check = target_path.clone();
    let target_exists = run_blocking_fs_with(
        "platform import target inspection",
        move || {
            Ok(target_path_for_check.exists()
                || std::fs::symlink_metadata(&target_path_for_check).is_ok())
        },
        ObsidianError::task_join,
    )
    .await?;
    if target_exists {
        return Err(ObsidianError::SkillExistsInAgent {
            skill: skill_dir_name,
            agent: agent.display_name,
        });
    }

    match install_method {
        ObsidianInstallMethod::Symlink => {
            let relative_target = symlink_target_path(&agent_dir, &source_dir);
            let target_path_for_symlink = target_path.clone();
            run_blocking_fs_with(
                "platform import symlink creation",
                move || {
                    create_symlink(&relative_target, &target_path_for_symlink)
                        .map_err(ObsidianError::Installation)
                },
                ObsidianError::task_join,
            )
            .await?;
        }
        ObsidianInstallMethod::Copy => {
            copy_dir_all_blocking(&source_dir, &target_path).await?;
        }
    }

    let now = Utc::now().to_rfc3339();
    let skill_md_path = source_dir.join("SKILL.md");
    let stored_skill_md_path = match install_method {
        ObsidianInstallMethod::Symlink => skill_md_path.clone(),
        ObsidianInstallMethod::Copy => target_path.join("SKILL.md"),
    };
    let info = parse_skill_md(&skill_md_path);

    if let Some(skill_info) = info {
        let db_skill = db::Skill {
            id: skill_dir_name.clone(),
            name: skill_info.name,
            description: skill_info.description,
            file_path: stored_skill_md_path.to_string_lossy().into_owned(),
            canonical_path: None,
            is_central: false,
            source: Some(install_method.as_str().to_string()),
            content: None,
            scanned_at: now.clone(),
            fs_created_at: None,
            fs_updated_at: None,
        };
        db::upsert_skill(pool, &db_skill).await?;
    }

    let installation = db::SkillInstallation {
        skill_id: skill_dir_name.clone(),
        agent_id: agent_id.to_string(),
        installed_path: target_path.to_string_lossy().into_owned(),
        link_type: install_method.as_str().to_string(),
        symlink_target: match install_method {
            ObsidianInstallMethod::Symlink => Some(source_dir.to_string_lossy().into_owned()),
            ObsidianInstallMethod::Copy => None,
        },
        created_at: now,
    };
    db::upsert_skill_installation(pool, &installation).await?;

    Ok(ObsidianImportResult {
        skill_id: skill_dir_name,
        target: agent_id.to_string(),
    })
}
