//! Safe "already installed" detection for local Central installs.
//!
//! These helpers only skip when the target has positive same-origin evidence:
//! an existing DB row with a live path, a same-source symlink, a shared-root
//! native directory, or a real directory whose contents exactly match the
//! Central canonical skill directory.

use std::path::{Path, PathBuf};

use crate::db::{self, DbPool, SkillInstallation};

use super::error::InstallationError;
use super::fs_util::{dirs_have_same_contents, run_blocking_fs, symlink_points_to};
use super::types::SkippedInstall;

pub(crate) const SKIP_REASON_EXISTING_RECORD: &str = "already_installed";
pub(crate) const SKIP_REASON_SHARED_TARGET_RECORD: &str = "shared_target_record";
pub(crate) const SKIP_REASON_CENTRAL_SYMLINK: &str = "central_symlink";
pub(crate) const SKIP_REASON_MATCHING_COPY: &str = "matching_copy";

pub(crate) async fn record_installation(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
    installed_path: &Path,
    link_type: &str,
    symlink_target: Option<String>,
) -> Result<(), InstallationError> {
    let installation = SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        installed_path: installed_path.to_string_lossy().into_owned(),
        link_type: link_type.to_string(),
        symlink_target,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db::upsert_skill_installation(pool, &installation)
        .await
        .map_err(InstallationError::Other) // TODO(C3): typed repos passthrough
}

pub(crate) async fn detect_existing_agent_install(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
    target_path: &Path,
    canonical_dir: &Path,
) -> Result<Option<SkippedInstall>, InstallationError> {
    let installations = db::get_skill_installations(pool, skill_id)
        .await
        .map_err(InstallationError::Other)?; // TODO(C3): typed repos passthrough

    if let Some(record) = installations
        .iter()
        .find(|record| record.agent_id == agent_id)
    {
        let installed_path = PathBuf::from(&record.installed_path);
        if target_exists(&installed_path).await? {
            return Ok(Some(skipped(
                agent_id,
                &installed_path,
                SKIP_REASON_EXISTING_RECORD,
            )));
        }
    }

    if installations.iter().any(|record| {
        record.agent_id != agent_id
            && crate::paths::paths_equivalent(Path::new(&record.installed_path), target_path)
    }) && target_exists(target_path).await?
    {
        let link_type = infer_existing_target_link_type(target_path, canonical_dir).await?;
        record_installation(
            pool,
            skill_id,
            agent_id,
            target_path,
            &link_type,
            symlink_target_for_link_type(&link_type, canonical_dir),
        )
        .await?;
        return Ok(Some(skipped(
            agent_id,
            target_path,
            SKIP_REASON_SHARED_TARGET_RECORD,
        )));
    }

    if target_is_symlink_to(target_path, canonical_dir).await? {
        record_installation(
            pool,
            skill_id,
            agent_id,
            target_path,
            "symlink",
            Some(canonical_dir.to_string_lossy().into_owned()),
        )
        .await?;
        return Ok(Some(skipped(
            agent_id,
            target_path,
            SKIP_REASON_CENTRAL_SYMLINK,
        )));
    }

    if target_is_matching_copy(target_path, canonical_dir).await? {
        record_installation(pool, skill_id, agent_id, target_path, "copy", None).await?;
        return Ok(Some(skipped(
            agent_id,
            target_path,
            SKIP_REASON_MATCHING_COPY,
        )));
    }

    Ok(None)
}

pub(crate) async fn detect_existing_project_install(
    skill_id: &str,
    agent_id: &str,
    target_path: &Path,
    canonical_dir: &Path,
) -> Result<Option<SkippedInstall>, InstallationError> {
    if target_is_symlink_to(target_path, canonical_dir).await? {
        return Ok(Some(skipped(
            agent_id,
            target_path,
            SKIP_REASON_CENTRAL_SYMLINK,
        )));
    }

    if target_is_matching_copy(target_path, canonical_dir).await? {
        return Ok(Some(skipped(
            agent_id,
            target_path,
            SKIP_REASON_MATCHING_COPY,
        )));
    }

    let _ = skill_id;
    Ok(None)
}

fn skipped(agent_id: &str, target_path: &Path, reason: &str) -> SkippedInstall {
    SkippedInstall {
        agent_id: agent_id.to_string(),
        target_path: target_path.to_string_lossy().into_owned(),
        reason: reason.to_string(),
    }
}

async fn target_exists(path: &Path) -> Result<bool, InstallationError> {
    let path = path.to_path_buf();
    run_blocking_fs("install target existence inspection", move || {
        Ok(std::fs::symlink_metadata(&path).is_ok())
    })
    .await
}

async fn target_is_symlink_to(path: &Path, canonical_dir: &Path) -> Result<bool, InstallationError> {
    let path = path.to_path_buf();
    let canonical_dir = canonical_dir.to_path_buf();
    run_blocking_fs("install target symlink inspection", move || {
        symlink_points_to(&path, &canonical_dir)
    })
    .await
}

async fn target_is_matching_copy(
    path: &Path,
    canonical_dir: &Path,
) -> Result<bool, InstallationError> {
    let path = path.to_path_buf();
    let canonical_dir = canonical_dir.to_path_buf();
    run_blocking_fs("install target copy comparison", move || {
        dirs_have_same_contents(&path, &canonical_dir)
    })
    .await
}

async fn infer_existing_target_link_type(
    target_path: &Path,
    canonical_dir: &Path,
) -> Result<String, InstallationError> {
    if target_is_symlink_to(target_path, canonical_dir).await? {
        return Ok("symlink".to_string());
    }
    if crate::paths::paths_equivalent(target_path, canonical_dir) {
        return Ok("native".to_string());
    }
    if target_is_matching_copy(target_path, canonical_dir).await? {
        return Ok("copy".to_string());
    }
    Ok("native".to_string())
}

fn symlink_target_for_link_type(link_type: &str, canonical_dir: &Path) -> Option<String> {
    if link_type == "symlink" {
        Some(canonical_dir.to_string_lossy().into_owned())
    } else {
        None
    }
}
