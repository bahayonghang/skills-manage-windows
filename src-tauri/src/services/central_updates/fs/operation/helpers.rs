use super::*;

pub(super) fn stage_local(
    manifest: &UpdateManifest,
    files: &[super::super::RemoteSkillFile],
) -> Result<(), CentralUpdatesError> {
    let target = Path::new(&manifest.target);
    let staging = Path::new(&manifest.staging);
    let backup = Path::new(&manifest.backup);
    let marker = Path::new(&manifest.marker);
    if std::fs::symlink_metadata(target).is_ok() != manifest.had_target
        || std::fs::symlink_metadata(staging).is_ok()
        || std::fs::symlink_metadata(backup).is_ok()
        || marker.exists()
    {
        return Err(CentralOperationError::RecoveryCollision {
            code: "update_stage_collision",
        }
        .into());
    }
    let parent = staging
        .parent()
        .ok_or_else(|| CentralUpdatesError::NoParentDirectory(manifest.staging.clone()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| CentralUpdatesError::io("Failed to create update parent", error))?;
    std::fs::write(marker, manifest.operation_id.as_bytes())
        .map_err(|error| CentralUpdatesError::io("Failed to write update marker", error))?;
    write_remote_skill_files(files, staging)?;
    if hash_local_directory(staging)? != manifest.new_fingerprint {
        return Err(CentralOperationError::RecoveryCollision {
            code: "update_staging_fingerprint",
        }
        .into());
    }
    Ok(())
}

pub(super) fn swap_local(manifest: &UpdateManifest) -> Result<(), CentralUpdatesError> {
    let target = Path::new(&manifest.target);
    let staging = Path::new(&manifest.staging);
    let backup = Path::new(&manifest.backup);
    verify_marker(Path::new(&manifest.marker), &manifest.operation_id)?;
    if hash_local_directory(staging)? != manifest.new_fingerprint
        || std::fs::symlink_metadata(backup).is_ok()
        || std::fs::symlink_metadata(target).is_ok() != manifest.had_target
    {
        return Err(CentralOperationError::RecoveryCollision {
            code: "update_swap_collision",
        }
        .into());
    }
    if manifest.had_target
        && hash_local_directory(target)?
            != manifest.old_fingerprint.clone().ok_or_else(|| {
                CentralOperationError::InvalidManifest(
                    "existing update target has no old fingerprint".to_string(),
                )
            })?
    {
        return Err(CentralOperationError::RecoveryCollision {
            code: "update_old_fingerprint",
        }
        .into());
    }
    if manifest.had_target {
        std::fs::rename(target, backup)
            .map_err(|error| CentralUpdatesError::io("Failed to stage update backup", error))?;
    }
    if let Err(error) = std::fs::rename(staging, target) {
        if manifest.had_target {
            std::fs::rename(backup, target).map_err(|restore_error| {
                CentralUpdatesError::io(
                    "Failed to restore update backup after swap failure",
                    restore_error,
                )
            })?;
        }
        return Err(CentralUpdatesError::io(
            "Failed to swap staged Central update",
            error,
        ));
    }
    Ok(())
}

pub(super) fn rollback_local(
    manifest: &UpdateManifest,
    phase: OperationPhase,
) -> Result<(), CentralUpdatesError> {
    let target = Path::new(&manifest.target);
    let staging = Path::new(&manifest.staging);
    let backup = Path::new(&manifest.backup);
    let marker = Path::new(&manifest.marker);
    if !marker.exists() {
        if std::fs::symlink_metadata(staging).is_ok()
            || std::fs::symlink_metadata(backup).is_ok()
            || std::fs::symlink_metadata(target).is_ok() != manifest.had_target
        {
            return Err(CentralOperationError::RecoveryCollision {
                code: "update_rollback_unowned_paths",
            }
            .into());
        }
        if manifest.had_target
            && hash_local_directory(target)?
                != manifest.old_fingerprint.clone().ok_or_else(|| {
                    CentralOperationError::InvalidManifest(
                        "existing update target has no old fingerprint".to_string(),
                    )
                })?
        {
            return Err(CentralOperationError::RecoveryCollision {
                code: "update_rollback_target_fingerprint",
            }
            .into());
        }
        return Ok(());
    }
    verify_marker(marker, &manifest.operation_id)?;
    if std::fs::symlink_metadata(backup).is_ok() {
        if std::fs::symlink_metadata(target).is_ok() {
            if hash_local_directory(target)? != manifest.new_fingerprint {
                return Err(CentralOperationError::RecoveryCollision {
                    code: "update_rollback_target_fingerprint",
                }
                .into());
            }
            remove_path(target)?;
        }
        if hash_local_directory(backup)?
            != manifest.old_fingerprint.clone().ok_or_else(|| {
                CentralOperationError::InvalidManifest(
                    "update backup has no old fingerprint".to_string(),
                )
            })?
        {
            return Err(CentralOperationError::RecoveryCollision {
                code: "update_backup_fingerprint",
            }
            .into());
        }
        std::fs::rename(backup, target)
            .map_err(|error| CentralUpdatesError::io("Failed to restore update backup", error))?;
    } else if manifest.had_target {
        if std::fs::symlink_metadata(target).is_err()
            || hash_local_directory(target)?
                != manifest.old_fingerprint.clone().ok_or_else(|| {
                    CentralOperationError::InvalidManifest(
                        "existing update target has no old fingerprint".to_string(),
                    )
                })?
        {
            return Err(CentralOperationError::RecoveryCollision {
                code: "update_rollback_target_fingerprint",
            }
            .into());
        }
    } else if std::fs::symlink_metadata(target).is_ok() {
        if phase == OperationPhase::Prepared
            || hash_local_directory(target)? != manifest.new_fingerprint
        {
            return Err(CentralOperationError::RecoveryCollision {
                code: "update_rollback_target_fingerprint",
            }
            .into());
        }
        remove_path(target)?;
    }
    if std::fs::symlink_metadata(staging).is_ok() {
        if phase != OperationPhase::Prepared
            && hash_local_directory(staging)? != manifest.new_fingerprint
        {
            return Err(CentralOperationError::RecoveryCollision {
                code: "update_rollback_staging_fingerprint",
            }
            .into());
        }
        remove_path(staging)?;
    }
    std::fs::remove_file(marker)
        .map_err(|error| CentralUpdatesError::io("Failed to remove update marker", error))?;
    Ok(())
}

pub(super) fn finalize_local(manifest: &UpdateManifest) -> Result<(), CentralUpdatesError> {
    let target = Path::new(&manifest.target);
    let marker = Path::new(&manifest.marker);
    if !marker.exists() {
        if std::fs::symlink_metadata(&manifest.backup).is_ok()
            || std::fs::symlink_metadata(&manifest.staging).is_ok()
            || hash_local_directory(target)? != manifest.new_fingerprint
        {
            return Err(CentralOperationError::RecoveryCollision {
                code: "update_finalize_unowned_paths",
            }
            .into());
        }
        return Ok(());
    }
    verify_marker(marker, &manifest.operation_id)?;
    if hash_local_directory(target)? != manifest.new_fingerprint {
        return Err(CentralOperationError::RecoveryCollision {
            code: "update_finalize_fingerprint",
        }
        .into());
    }
    if manifest.had_target
        && std::fs::symlink_metadata(&manifest.backup).is_ok()
        && hash_local_directory(Path::new(&manifest.backup))?
            != manifest.old_fingerprint.clone().ok_or_else(|| {
                CentralOperationError::InvalidManifest(
                    "update backup has no old fingerprint".to_string(),
                )
            })?
    {
        return Err(CentralOperationError::RecoveryCollision {
            code: "update_backup_fingerprint",
        }
        .into());
    }
    for path in [&manifest.backup, &manifest.staging] {
        let path = Path::new(path);
        if std::fs::symlink_metadata(path).is_ok() {
            remove_path(path)?;
        }
    }
    std::fs::remove_file(&manifest.marker)
        .map_err(|error| CentralUpdatesError::io("Failed to remove update marker", error))?;
    Ok(())
}

pub(super) fn verify_marker(path: &Path, operation_id: &str) -> Result<(), CentralUpdatesError> {
    let value =
        std::fs::read_to_string(path).map_err(|_| CentralOperationError::RecoveryCollision {
            code: "update_marker_missing",
        })?;
    if value != operation_id {
        return Err(CentralOperationError::RecoveryCollision {
            code: "update_marker_mismatch",
        }
        .into());
    }
    Ok(())
}

pub(super) async fn verify_remote_hash(
    fs: &CentralFs,
    path: &str,
    expected: &str,
) -> Result<(), CentralUpdatesError> {
    let path = PathBuf::from(path);
    let actual = fs
        .hash_directories(std::slice::from_ref(&path))
        .await?
        .remove(&path)
        .ok_or_else(|| CentralOperationError::RecoveryCollision {
            code: "update_remote_fingerprint_missing",
        })?;
    if actual != expected {
        return Err(CentralOperationError::RecoveryCollision {
            code: "update_remote_fingerprint",
        }
        .into());
    }
    Ok(())
}

pub(super) fn fingerprint_files(files: &[super::super::RemoteSkillFile]) -> String {
    let entries = files
        .iter()
        .map(|file| {
            let digest = Sha256::digest(&file.bytes);
            (file.relative_path.clone(), format!("{digest:x}"))
        })
        .collect();
    hash_entries(entries)
}

pub(super) fn short_digest(value: &str) -> String {
    let digest = format!("{:x}", Sha256::digest(value.as_bytes()));
    digest[..16].to_string()
}
