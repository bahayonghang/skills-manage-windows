//! Local execution half of skill install / uninstall: symlink, copy, auto
//! fallback, native records, and observation-row uninstall. The
//! business orchestration lives in `install.rs`; this module only implements
//! the Local arms of the [`super::transport::InstallTransport`] hooks.

use std::path::{Path, PathBuf};

use crate::db::{self, AgentSkillObservation, DbPool};

use super::centralize::{ensure_centralized, ensure_replaceable_target};
use super::error::InstallationError;
use super::fs_util::{
    copy_dir_all_blocking, create_symlink, remove_symlink_path, run_blocking_fs,
    symlink_target_path,
};
use super::transport::{Placement, ResolvedMethod};

/// Shared-root Local arm: centralize, then verify the canonical skill is
/// really present before the caller records a native installation.
pub(crate) async fn centralize_shared_root_local(
    pool: &DbPool,
    skill_id: &str,
    central: &db::Agent,
) -> Result<String, InstallationError> {
    let canonical_dir = PathBuf::from(&central.global_skills_dir).join(skill_id);
    ensure_centralized(pool, skill_id, &canonical_dir).await?;

    let skill_md = canonical_dir.join("SKILL.md");
    if !skill_md.exists() {
        return Err(InstallationError::CanonicalSkillMissing(
            skill_md.display().to_string(),
        ));
    }

    Ok(canonical_dir.to_string_lossy().into_owned())
}

/// Non-shared-root Local arm: centralize eagerly, then make sure the agent
/// skills directory exists (both must precede skip detection, which compares
/// target contents against the canonical directory).
pub(crate) async fn prepare_target_local(
    pool: &DbPool,
    skill_id: &str,
    agent: &db::Agent,
    central: &db::Agent,
) -> Result<(), InstallationError> {
    let canonical_dir = PathBuf::from(&central.global_skills_dir).join(skill_id);
    ensure_centralized(pool, skill_id, &canonical_dir).await?;

    let agent_dir = PathBuf::from(&agent.global_skills_dir);
    run_blocking_fs("agent skills directory creation", move || {
        std::fs::create_dir_all(&agent_dir)
            .map_err(|e| InstallationError::io("Failed to create agent skills directory", e))
    })
    .await
}

/// Placement Local arm: clear the install slot, then lay down a relative
/// symlink or a recursive copy. `Auto` retries the placement step as a copy
/// when symlink creation fails on Windows.
pub(crate) async fn place_install_local(
    agent: &db::Agent,
    central: &db::Agent,
    skill_id: &str,
    method: ResolvedMethod,
) -> Result<Placement, InstallationError> {
    let agent_dir = PathBuf::from(&agent.global_skills_dir);
    let canonical_dir = PathBuf::from(&central.global_skills_dir).join(skill_id);
    let target_path = agent_dir.join(skill_id);

    ensure_replaceable_target(&target_path).await?;

    match method {
        ResolvedMethod::Copy => place_copy_local(&canonical_dir, &target_path).await,
        ResolvedMethod::Symlink => {
            place_symlink_local(&agent_dir, &canonical_dir, &target_path).await
        }
        ResolvedMethod::Auto => {
            match place_symlink_local(&agent_dir, &canonical_dir, &target_path).await {
                Ok(placement) => Ok(placement),
                Err(error) if should_fallback_to_copy(&error) => {
                    ensure_replaceable_target(&target_path).await?;
                    place_copy_local(&canonical_dir, &target_path).await
                }
                Err(error) => Err(error),
            }
        }
    }
}

async fn place_symlink_local(
    agent_dir: &Path,
    canonical_dir: &Path,
    target_path: &Path,
) -> Result<Placement, InstallationError> {
    let relative_target = symlink_target_path(agent_dir, canonical_dir);
    let target_path_for_create = target_path.to_path_buf();
    run_blocking_fs("skill symlink creation", move || {
        create_symlink(&relative_target, &target_path_for_create)
    })
    .await?;

    Ok(Placement {
        installed_path: target_path.to_string_lossy().into_owned(),
        link_type: "symlink",
        symlink_target: Some(canonical_dir.to_string_lossy().into_owned()),
    })
}

async fn place_copy_local(
    canonical_dir: &Path,
    target_path: &Path,
) -> Result<Placement, InstallationError> {
    copy_dir_all_blocking(canonical_dir, target_path).await?;

    Ok(Placement {
        installed_path: target_path.to_string_lossy().into_owned(),
        link_type: "copy",
        symlink_target: None,
    })
}

/// Try the symlink path; on Windows fall back to copy when the symlink call
/// fails (typically due to missing privileges or non-NTFS targets).
#[cfg(windows)]
pub(crate) fn should_fallback_to_copy(error: &InstallationError) -> bool {
    matches!(error, InstallationError::SymlinkCreate(_))
}

#[cfg(not(windows))]
pub(crate) fn should_fallback_to_copy(_error: &InstallationError) -> bool {
    false
}

/// Removal Local arm: classify the entry by its recorded link type.
///
/// For symlinked skills: removes the symlink.
/// For copied skills: removes the copied directory (tracked as link_type='copy').
/// Refuses to delete real directories not tracked as copies in the DB.
pub(crate) async fn remove_install_local(
    pool: &DbPool,
    agent: &db::Agent,
    skill_id: &str,
) -> Result<(), InstallationError> {
    let install_path = PathBuf::from(&agent.global_skills_dir).join(skill_id);

    let installations = db::get_skill_installations(pool, skill_id).await?;
    let record = installations
        .iter()
        .find(|record| record.agent_id == agent.id);
    let link_type = record
        .map(|record| record.link_type.clone())
        .unwrap_or_else(|| "symlink".to_string());

    run_blocking_fs("skill uninstall", move || {
        remove_install_path(&install_path, &link_type, false)
    })
    .await
}

fn remove_install_path(
    path: &Path,
    link_type: &str,
    allow_native_dir: bool,
) -> Result<(), InstallationError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            remove_symlink_path(path)
                .map_err(|e| InstallationError::io("Failed to remove symlink", e))?;
        }
        Ok(meta) if meta.is_dir() => {
            if link_type == "copy" || (allow_native_dir && link_type == "native") {
                std::fs::remove_dir_all(path).map_err(|e| {
                    InstallationError::io(
                        format!(
                            "Failed to remove copied skill directory '{}'",
                            path.display()
                        ),
                        e,
                    )
                })?;
            } else {
                return Err(InstallationError::NotASymlink(path.display().to_string()));
            }
        }
        Ok(_) => {
            return Err(InstallationError::NotASymlink(path.display().to_string()));
        }
        Err(_) => {
            // Path doesn't exist — still clean up DB state.
        }
    }

    Ok(())
}

fn ensure_user_observation(
    observation: &AgentSkillObservation,
    skill_id: &str,
    agent_id: &str,
) -> Result<(), InstallationError> {
    if observation.agent_id != agent_id {
        return Err(InstallationError::ObservationRowAgentMismatch {
            row_id: observation.row_id.clone(),
            actual: observation.agent_id.clone(),
            expected: agent_id.to_string(),
        });
    }

    if observation.skill_id != skill_id {
        return Err(InstallationError::ObservationRowSkillMismatch {
            row_id: observation.row_id.clone(),
            actual: observation.skill_id.clone(),
            expected: skill_id.to_string(),
        });
    }

    // Only user-managed sources are unlinkable; plugin copies are read-only.
    // The scanner writes exactly "user" / "plugin" (see scanner SourceKind).
    if observation.source_kind != "user" || observation.is_read_only {
        return Err(InstallationError::ObservationRowReadOnly(
            observation.row_id.clone(),
        ));
    }

    Ok(())
}

fn ensure_child_path(root: &Path, child: &Path) -> Result<(), InstallationError> {
    if crate::paths::paths_equivalent(root, child) {
        return Err(InstallationError::ObservationRootDeletion(
            root.display().to_string(),
        ));
    }

    let child_parent = child
        .parent()
        .ok_or_else(|| InstallationError::PathHasNoParent(child.display().to_string()))?;

    // Scanner observations always represent immediate children of the
    // configured skills root. Accepting deeper descendants would let a stale
    // or corrupted observation widen the deletion target beyond that contract.
    if !crate::paths::paths_equivalent(root, child_parent) {
        return Err(InstallationError::OutsideObservationRoot {
            child: child.display().to_string(),
            root: root.display().to_string(),
        });
    }

    Ok(())
}

fn ensure_observation_path_identity(
    observation: &AgentSkillObservation,
) -> Result<(), InstallationError> {
    let expected_row_id = format!("{}::{}", observation.agent_id, observation.dir_path);
    let observed_skill_id = Path::new(&observation.dir_path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_lowercase().replace(' ', "-"));
    // Claude observations historically use row_id, not the directory name, to
    // distinguish multiple paths that resolve to the same logical skill.
    let skill_path_matches = observation.agent_id == "claude-code"
        || observed_skill_id.as_deref() == Some(observation.skill_id.as_str());

    if observation.row_id != expected_row_id || !skill_path_matches {
        return Err(InstallationError::ObservationRowPathMismatch(
            observation.row_id.clone(),
        ));
    }

    Ok(())
}

/// Observation-row uninstall, generalized to every local agent (D1): verify
/// the `(agent_id, row_id)` observation row, refuse Central storage and
/// read-only / non-user sources, check the recorded `dir_path` stays inside
/// the agent skills root *before* any on-disk delete, then remove the entry
/// (native real directories are allowed here, unlike the generic path) and
/// delete both the observation row and the matching installation record.
pub(crate) async fn uninstall_observation_from_agent_impl(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
    row_id: &str,
) -> Result<(), InstallationError> {
    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| InstallationError::AgentNotFound(agent_id.to_string()))?;
    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or(InstallationError::CentralAgentMissing)?;

    // Observation unlink never touches Central storage: `central` itself and
    // agents sharing the Central skills directory must go through the
    // journaled Central delete paths instead (skill-deletion-integrity.md).
    if agent_id == "central" || super::centralize::agents_share_skills_dir(&agent, &central) {
        return Err(InstallationError::SharedCentralUninstall {
            display_name: agent.display_name,
        });
    }

    let observation = db::get_agent_skill_observation_by_row_id(pool, row_id)
        .await?
        .ok_or_else(|| InstallationError::ObservationRowNotFound(row_id.to_string()))?;
    ensure_user_observation(&observation, skill_id, agent_id)?;
    ensure_observation_path_identity(&observation)?;

    let user_root = PathBuf::from(&agent.global_skills_dir);
    let install_path = PathBuf::from(&observation.dir_path);
    ensure_child_path(&user_root, &install_path)?;

    let install_path_for_remove = install_path.clone();
    let link_type = observation.link_type.clone();
    run_blocking_fs("observation uninstall", move || {
        remove_install_path(&install_path_for_remove, &link_type, true)
    })
    .await?;
    db::delete_skill_installation_with_observations(
        pool,
        skill_id,
        agent_id,
        &[row_id.to_string()],
    )
    .await?;

    Ok(())
}
