use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::db::{self, DbPool};
use crate::paths;
use crate::services::installation::{copy_dir_all, create_symlink, symlink_target_path};
use crate::services::scanner::parse_skill_md;

use super::types::ImportResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiscoveredPlatformInstallMethod {
    Symlink,
    Copy,
}

impl DiscoveredPlatformInstallMethod {
    fn parse(method: Option<&str>) -> Result<Self, String> {
        match method.unwrap_or("symlink") {
            "symlink" | "auto" => Ok(Self::Symlink),
            "copy" => Ok(Self::Copy),
            other => Err(format!("Unsupported install method '{}'", other)),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Symlink => "symlink",
            Self::Copy => "copy",
        }
    }
}

/// Import a discovered skill to the default Central skills directory.
pub async fn import_discovered_skill_to_central_impl(
    pool: &DbPool,
    discovered_skill_id: &str,
) -> Result<ImportResult, String> {
    let central_dir = paths::central_skills_dir();
    import_discovered_skill_to_central_at(pool, discovered_skill_id, &central_dir).await
}

/// Import a discovered skill to a supplied Central skills directory.
///
/// Kept public so tests and future service callers can avoid depending on the
/// process home directory while exercising the same implementation path.
pub async fn import_discovered_skill_to_central_at(
    pool: &DbPool,
    discovered_skill_id: &str,
    central_dir: &Path,
) -> Result<ImportResult, String> {
    let skill = db::get_discovered_skill_by_id(pool, discovered_skill_id)
        .await?
        .ok_or_else(|| format!("Discovered skill '{}' not found", discovered_skill_id))?;

    let result =
        import_skill_dir_to_central_at(pool, Path::new(&skill.dir_path), central_dir).await?;

    db::delete_discovered_skill(pool, discovered_skill_id).await?;

    Ok(result)
}

/// Import a discovered skill to a specific platform's global skills directory.
pub async fn import_discovered_skill_to_platform_impl(
    pool: &DbPool,
    discovered_skill_id: &str,
    agent_id: &str,
) -> Result<ImportResult, String> {
    import_discovered_skill_to_platform_with_method_impl(pool, discovered_skill_id, agent_id, None)
        .await
}

pub async fn import_discovered_skill_to_platform_with_method_impl(
    pool: &DbPool,
    discovered_skill_id: &str,
    agent_id: &str,
    method: Option<&str>,
) -> Result<ImportResult, String> {
    let skill = db::get_discovered_skill_by_id(pool, discovered_skill_id)
        .await?
        .ok_or_else(|| format!("Discovered skill '{}' not found", discovered_skill_id))?;

    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

    let agent_dir = PathBuf::from(&agent.global_skills_dir);
    import_skill_dir_to_platform(
        pool,
        Path::new(&skill.dir_path),
        agent_id,
        &agent_dir,
        &agent.display_name,
        method,
    )
    .await
}

/// Import a discovered skill to a supplied platform skills directory.
pub async fn import_discovered_skill_to_platform_at(
    pool: &DbPool,
    discovered_skill_id: &str,
    agent_id: &str,
    agent_dir: &Path,
) -> Result<ImportResult, String> {
    import_discovered_skill_to_platform_with_method_at(
        pool,
        discovered_skill_id,
        agent_id,
        agent_dir,
        None,
    )
    .await
}

pub async fn import_discovered_skill_to_platform_with_method_at(
    pool: &DbPool,
    discovered_skill_id: &str,
    agent_id: &str,
    agent_dir: &Path,
    method: Option<&str>,
) -> Result<ImportResult, String> {
    let skill = db::get_discovered_skill_by_id(pool, discovered_skill_id)
        .await?
        .ok_or_else(|| format!("Discovered skill '{}' not found", discovered_skill_id))?;

    import_skill_dir_to_platform(
        pool,
        Path::new(&skill.dir_path),
        agent_id,
        agent_dir,
        agent_id,
        method,
    )
    .await
}

pub async fn import_source_skill_to_central_impl(
    pool: &DbPool,
    file_path: &str,
    dir_path: &str,
) -> Result<ImportResult, String> {
    let _ = file_path;
    let central_dir = paths::central_skills_dir();
    import_source_skill_to_central_at(pool, dir_path, &central_dir).await
}

pub async fn import_source_skill_to_central_at(
    pool: &DbPool,
    dir_path: &str,
    central_dir: &Path,
) -> Result<ImportResult, String> {
    import_skill_dir_to_central_at(pool, Path::new(dir_path), central_dir).await
}

pub async fn import_source_skill_to_platform_with_method_impl(
    pool: &DbPool,
    file_path: &str,
    dir_path: &str,
    agent_id: &str,
    method: Option<&str>,
) -> Result<ImportResult, String> {
    let _ = file_path;
    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;
    let agent_dir = PathBuf::from(&agent.global_skills_dir);
    import_skill_dir_to_platform(
        pool,
        Path::new(dir_path),
        agent_id,
        &agent_dir,
        &agent.display_name,
        method,
    )
    .await
}

async fn import_skill_dir_to_platform(
    pool: &DbPool,
    source_dir: &Path,
    agent_id: &str,
    agent_dir: &Path,
    target_label: &str,
    method: Option<&str>,
) -> Result<ImportResult, String> {
    let install_method = DiscoveredPlatformInstallMethod::parse(method)?;
    let skill_dir_name = skill_dir_name(source_dir)?;

    let target_path = agent_dir.join(&skill_dir_name);

    std::fs::create_dir_all(agent_dir)
        .map_err(|e| format!("Failed to create agent skills directory: {}", e))?;

    if target_path.exists() || std::fs::symlink_metadata(&target_path).is_ok() {
        return Err(format!(
            "Skill '{}' already exists in {}",
            skill_dir_name, target_label
        ));
    }

    match install_method {
        DiscoveredPlatformInstallMethod::Symlink => {
            let relative_target = symlink_target_path(agent_dir, source_dir);
            create_symlink(&relative_target, &target_path)?;
        }
        DiscoveredPlatformInstallMethod::Copy => {
            copy_dir_all(source_dir, &target_path)?;
        }
    }

    let now = Utc::now().to_rfc3339();
    let skill_md_path = source_dir.join("SKILL.md");
    let stored_skill_md_path = match install_method {
        DiscoveredPlatformInstallMethod::Symlink => skill_md_path.clone(),
        DiscoveredPlatformInstallMethod::Copy => target_path.join("SKILL.md"),
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
        };
        db::upsert_skill(pool, &db_skill).await?;
    }

    let installation = db::SkillInstallation {
        skill_id: skill_dir_name.clone(),
        agent_id: agent_id.to_string(),
        installed_path: target_path.to_string_lossy().into_owned(),
        link_type: install_method.as_str().to_string(),
        symlink_target: match install_method {
            DiscoveredPlatformInstallMethod::Symlink => {
                Some(source_dir.to_string_lossy().into_owned())
            }
            DiscoveredPlatformInstallMethod::Copy => None,
        },
        created_at: now,
    };
    db::upsert_skill_installation(pool, &installation).await?;

    // NOTE: Intentionally do NOT delete the discovered skill record. This allows
    // multi-platform install; cache reconciliation or Central import cleans it.
    Ok(ImportResult {
        skill_id: skill_dir_name,
        target: agent_id.to_string(),
    })
}

async fn import_skill_dir_to_central_at(
    pool: &DbPool,
    source_dir: &Path,
    central_dir: &Path,
) -> Result<ImportResult, String> {
    if !source_dir.is_dir() {
        return Err(format!(
            "Skill source directory '{}' does not exist.",
            source_dir.display()
        ));
    }

    let skill_dir_name = skill_dir_name(source_dir)?;
    let target_dir = central_dir.join(&skill_dir_name);

    if target_dir.exists() {
        return Err(format!(
            "A skill named '{}' already exists in central skills",
            skill_dir_name
        ));
    }

    copy_dir_all(source_dir, &target_dir)?;

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
        };
        db::upsert_skill(pool, &db_skill).await?;
    }

    Ok(ImportResult {
        skill_id: skill_dir_name,
        target: "central".to_string(),
    })
}

fn skill_dir_name(source_dir: &Path) -> Result<String, String> {
    source_dir
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| "Cannot extract skill directory name".to_string())
}
