use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::db::{self, DbPool};
use crate::paths;
use crate::services::installation::{copy_dir_all, create_symlink, symlink_target_path};
use crate::services::scanner::parse_skill_md;

use super::types::ImportResult;

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

    let skill_dir_name = Path::new(&skill.dir_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Cannot extract skill directory name".to_string())?
        .to_string();

    let target_dir = central_dir.join(&skill_dir_name);

    if target_dir.exists() {
        return Err(format!(
            "A skill named '{}' already exists in central skills",
            skill_dir_name
        ));
    }

    copy_dir_all(Path::new(&skill.dir_path), &target_dir)?;

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

    db::delete_discovered_skill(pool, discovered_skill_id).await?;

    Ok(ImportResult {
        skill_id: skill_dir_name,
        target: "central".to_string(),
    })
}

/// Import a discovered skill to a specific platform's global skills directory.
pub async fn import_discovered_skill_to_platform_impl(
    pool: &DbPool,
    discovered_skill_id: &str,
    agent_id: &str,
) -> Result<ImportResult, String> {
    let skill = db::get_discovered_skill_by_id(pool, discovered_skill_id)
        .await?
        .ok_or_else(|| format!("Discovered skill '{}' not found", discovered_skill_id))?;

    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

    let agent_dir = PathBuf::from(&agent.global_skills_dir);
    import_discovered_skill_to_platform_row(pool, &skill, agent_id, &agent_dir, &agent.display_name)
        .await
}

/// Import a discovered skill to a supplied platform skills directory.
pub async fn import_discovered_skill_to_platform_at(
    pool: &DbPool,
    discovered_skill_id: &str,
    agent_id: &str,
    agent_dir: &Path,
) -> Result<ImportResult, String> {
    let skill = db::get_discovered_skill_by_id(pool, discovered_skill_id)
        .await?
        .ok_or_else(|| format!("Discovered skill '{}' not found", discovered_skill_id))?;

    import_discovered_skill_to_platform_row(pool, &skill, agent_id, agent_dir, agent_id).await
}

async fn import_discovered_skill_to_platform_row(
    pool: &DbPool,
    skill: &db::DiscoveredSkillRow,
    agent_id: &str,
    agent_dir: &Path,
    target_label: &str,
) -> Result<ImportResult, String> {
    let skill_dir_name = Path::new(&skill.dir_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Cannot extract skill directory name".to_string())?
        .to_string();

    let target_path = agent_dir.join(&skill_dir_name);

    std::fs::create_dir_all(agent_dir)
        .map_err(|e| format!("Failed to create agent skills directory: {}", e))?;

    if target_path.exists() || std::fs::symlink_metadata(&target_path).is_ok() {
        return Err(format!(
            "Skill '{}' already exists in {}",
            skill_dir_name, target_label
        ));
    }

    let src_path = Path::new(&skill.dir_path);
    let relative_target = symlink_target_path(agent_dir, src_path);
    create_symlink(&relative_target, &target_path)?;

    let now = Utc::now().to_rfc3339();
    let skill_md_path = src_path.join("SKILL.md");
    let info = parse_skill_md(&skill_md_path);

    if let Some(skill_info) = info {
        let db_skill = db::Skill {
            id: skill_dir_name.clone(),
            name: skill_info.name,
            description: skill_info.description,
            file_path: skill_md_path.to_string_lossy().into_owned(),
            canonical_path: None,
            is_central: false,
            source: Some("symlink".to_string()),
            content: None,
            scanned_at: now.clone(),
        };
        db::upsert_skill(pool, &db_skill).await?;
    }

    let installation = db::SkillInstallation {
        skill_id: skill_dir_name.clone(),
        agent_id: agent_id.to_string(),
        installed_path: target_path.to_string_lossy().into_owned(),
        link_type: "symlink".to_string(),
        symlink_target: Some(skill.dir_path.clone()),
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
