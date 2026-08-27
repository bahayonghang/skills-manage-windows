//! Safe Skills CLI remove with a domain-local recovery manifest.
//!
//! This does not spawn `skills remove` and never uses unverified flags.
//! Ordinary directories / direct copies are not mutated.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::Builder;
use uuid::Uuid;

use crate::db::DbPool;
use crate::fs_util::run_blocking_fs_with;
use crate::services::central_mutation::{
    acquire_central_mutation_guard_at, acquire_target_mutation_guard,
    DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};
use crate::services::installation::fs_util::{
    create_skills_cli_directory_link, observe_directory_slot, remove_verified_directory_link,
    DirectorySlotObservation,
};
use crate::targets::ActiveTarget;

use super::error::SkillsCliError;
use super::inventory::InventoryPlatform;
use super::lock::load_cli_lock_ownership;
use super::placement::{canonical_is_owned_directory, classify_placements};
use super::{
    check_cancel, is_valid_skill_token, map_guard_error, mapped_inventory_platforms,
    SkillsCliManagedLinkKind, SkillsCliPlacement, SkillsCliPlacementConflict,
    SkillsCliPlacementState, SkillsCliRemovePlacementSummary, SkillsCliRemovePlan,
    SkillsCliRemoveResult, SkillsCliTransport,
};

const REMOVE_LOCK_OPERATION: &str = "Skills CLI global remove";
const MANIFEST_VERSION: u32 = 1;
const PHASE_PREPARED: &str = "prepared";
const PHASE_STAGED: &str = "staged";
const PHASE_METADATA_COMMITTED: &str = "metadata_committed";
const TEMP_PREFIX: &str = ".skillport-skills-cli-lock-";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoveFault {
    AfterPrepared,
    AfterCanonicalRename,
    AfterLinkRemove,
    BeforeLockReplace,
    AfterLockReplace,
    AfterCleanupBackup,
    FingerprintDrift,
}

#[cfg(test)]
thread_local! {
    static REMOVE_FAULT: std::cell::Cell<Option<RemoveFault>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn set_remove_fault(fault: Option<RemoveFault>) {
    REMOVE_FAULT.with(|cell| cell.set(fault));
}

fn injected_fault() -> Option<RemoveFault> {
    #[cfg(test)]
    {
        REMOVE_FAULT.with(|cell| cell.get())
    }
    #[cfg(not(test))]
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManagedLinkRecord {
    agent_id: String,
    path: String,
    kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoveManifestV1 {
    version: u32,
    operation_id: String,
    skill_name: String,
    phase: String,
    lock_fingerprint: String,
    lock_path: String,
    canonical_path: String,
    canonical_backup_path: String,
    managed_links: Vec<ManagedLinkRecord>,
}

pub(crate) async fn preview_remove_global(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    skill_name: &str,
) -> Result<SkillsCliRemovePlan, SkillsCliError> {
    let paths = tx.paths();
    preview_remove_global_at(
        pool,
        skill_name,
        &paths.canonical_root_path(),
        &paths.lock_path_buf(),
    )
    .await
}

pub(crate) async fn preview_remove_global_at(
    pool: &DbPool,
    skill_name: &str,
    canonical_root: &Path,
    lock_path: &Path,
) -> Result<SkillsCliRemovePlan, SkillsCliError> {
    if !is_valid_skill_token(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    let agents = crate::db::get_all_agents(pool)
        .await
        .map_err(|error| SkillsCliError::Io {
            context: "read platforms",
            source: std::io::Error::other(error.to_string()),
        })?;
    let platforms = mapped_inventory_platforms(&agents);
    build_remove_plan(skill_name, canonical_root, lock_path, &platforms)
}

pub(crate) async fn remove_global(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    skill_name: &str,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliRemoveResult, SkillsCliError> {
    let paths = tx.paths();
    remove_global_at(
        pool,
        skill_name,
        cancel,
        &paths.canonical_root_path(),
        &paths.lock_path_buf(),
        None,
        crate::paths::skills_cli_remove_recovery_dir(),
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn remove_global_at(
    pool: &DbPool,
    skill_name: &str,
    cancel: Option<&AtomicBool>,
    canonical_root: &Path,
    lock_path: &Path,
    mutation_lock_path: Option<PathBuf>,
    recovery_root: PathBuf,
    timeout: Duration,
) -> Result<SkillsCliRemoveResult, SkillsCliError> {
    if !is_valid_skill_token(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    check_cancel(cancel)?;
    let _guard = match mutation_lock_path {
        Some(path) => acquire_central_mutation_guard_at(path, REMOVE_LOCK_OPERATION, timeout)
            .await
            .map_err(map_guard_error)?,
        None => acquire_target_mutation_guard(&ActiveTarget::Local, REMOVE_LOCK_OPERATION, timeout)
            .await
            .map_err(map_guard_error)?,
    };
    check_cancel(cancel)?;

    let canonical_root = canonical_root.to_path_buf();
    let lock_path = lock_path.to_path_buf();
    let skill_name = skill_name.to_string();
    #[cfg(test)]
    let injected = injected_fault();
    let agents = crate::db::get_all_agents(pool)
        .await
        .map_err(|error| SkillsCliError::Io {
            context: "read platforms",
            source: std::io::Error::other(error.to_string()),
        })?;
    let platforms = mapped_inventory_platforms(&agents);
    run_blocking_fs_with(
        "Skills CLI safe remove",
        move || {
            #[cfg(test)]
            set_remove_fault(injected);
            let result = (|| {
                recover_pending_for_skill_at(
                    &recovery_root,
                    &canonical_root,
                    &lock_path,
                    &skill_name,
                )?;
                execute_remove(
                    &skill_name,
                    &canonical_root,
                    &lock_path,
                    &recovery_root,
                    &platforms,
                )
            })();
            #[cfg(test)]
            set_remove_fault(None);
            result
        },
        SkillsCliError::task_join,
    )
    .await
}

pub(crate) fn recover_pending_for_skill_at(
    recovery_root: &Path,
    canonical_root: &Path,
    lock_path: &Path,
    skill_name: &str,
) -> Result<(), SkillsCliError> {
    let path = manifest_path(recovery_root, skill_name);
    if !path.exists() {
        return Ok(());
    }
    recover_manifest(&path, canonical_root, lock_path)
}

fn build_remove_plan(
    skill_name: &str,
    canonical_root: &Path,
    lock_path: &Path,
    platforms: &[InventoryPlatform],
) -> Result<SkillsCliRemovePlan, SkillsCliError> {
    let ownership = load_cli_lock_ownership(lock_path)?;
    if !ownership.contains_name(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    let placement_platforms: Vec<_> = platforms
        .iter()
        .map(InventoryPlatform::as_placement_platform)
        .collect();
    let placements =
        classify_placements(&ownership, skill_name, canonical_root, &placement_platforms);
    Ok(plan_from_placements(
        skill_name,
        canonical_root,
        &placements,
    ))
}

fn plan_from_placements(
    skill_name: &str,
    canonical_root: &Path,
    placements: &[SkillsCliPlacement],
) -> SkillsCliRemovePlan {
    let owned_canonical = canonical_is_owned_directory(&canonical_root.join(skill_name));
    let mut managed = Vec::new();
    let mut retained = Vec::new();
    let mut conflicts = Vec::new();
    for placement in placements {
        match placement.state {
            SkillsCliPlacementState::ManagedLink => {
                managed.push(SkillsCliRemovePlacementSummary {
                    agent_id: placement.agent_id.clone(),
                    display_name: placement.display_name.clone(),
                });
            }
            SkillsCliPlacementState::DirectCopy => {
                retained.push(SkillsCliRemovePlacementSummary {
                    agent_id: placement.agent_id.clone(),
                    display_name: placement.display_name.clone(),
                });
            }
            SkillsCliPlacementState::Conflict => {
                conflicts.push(SkillsCliPlacementConflict {
                    agent_id: placement.agent_id.clone(),
                    display_name: placement.display_name.clone(),
                    reason_code: placement
                        .reason_code
                        .clone()
                        .unwrap_or_else(|| "unreadable_entry".to_string()),
                });
            }
            SkillsCliPlacementState::Missing | SkillsCliPlacementState::Unavailable => {}
        }
    }
    SkillsCliRemovePlan {
        confirmable: owned_canonical && conflicts.is_empty(),
        skill_name: skill_name.to_string(),
        owned_canonical,
        managed_placements: managed,
        retained_direct_copies: retained,
        conflicts,
    }
}

fn execute_remove(
    skill_name: &str,
    canonical_root: &Path,
    lock_path: &Path,
    recovery_root: &Path,
    platforms: &[InventoryPlatform],
) -> Result<SkillsCliRemoveResult, SkillsCliError> {
    let plan = build_remove_plan(skill_name, canonical_root, lock_path, platforms)?;
    if !plan.conflicts.is_empty() {
        return Err(SkillsCliError::PlacementConflict);
    }
    if !plan.owned_canonical && plan.managed_placements.is_empty() {
        remove_lock_row(lock_path, skill_name, None)?;
        return Ok(SkillsCliRemoveResult {
            removed_canonical: false,
            removed_managed_agent_ids: Vec::new(),
            retained_direct_copy_agent_ids: plan
                .retained_direct_copies
                .iter()
                .map(|item| item.agent_id.clone())
                .collect(),
        });
    }

    let lock_bytes = fs::read(lock_path).map_err(|source| SkillsCliError::Io {
        context: "read Skills CLI lock",
        source,
    })?;
    let fingerprint = hex_digest(&lock_bytes);
    let operation_id = Uuid::new_v4().to_string();
    let canonical = canonical_root.join(skill_name);
    let backup = canonical_root.join(format!(".skillport-remove-{operation_id}"));
    let managed_links = collect_managed_links(skill_name, canonical_root, lock_path, platforms)?;
    let manifest = RemoveManifestV1 {
        version: MANIFEST_VERSION,
        operation_id: operation_id.clone(),
        skill_name: skill_name.to_string(),
        phase: PHASE_PREPARED.to_string(),
        lock_fingerprint: fingerprint.clone(),
        lock_path: lock_path.to_string_lossy().into_owned(),
        canonical_path: canonical.to_string_lossy().into_owned(),
        canonical_backup_path: backup.to_string_lossy().into_owned(),
        managed_links,
    };
    let manifest_file = persist_manifest(recovery_root, &manifest)?;
    hit_fault(RemoveFault::AfterPrepared)?;

    if canonical_is_owned_directory(&canonical) {
        fs::rename(&canonical, &backup).map_err(|source| {
            let _ = recover_manifest(&manifest_file, canonical_root, lock_path);
            SkillsCliError::Io {
                context: "stage Skills CLI canonical",
                source,
            }
        })?;
    }
    hit_fault(RemoveFault::AfterCanonicalRename).inspect_err(|_| {
        let _ = recover_manifest(&manifest_file, canonical_root, lock_path);
    })?;

    for link in &manifest.managed_links {
        let path = PathBuf::from(&link.path);
        if let Err(error) = remove_verified_directory_link(&path, &backup)
            .or_else(|_| remove_verified_directory_link(&path, &canonical))
        {
            let _ = recover_manifest(&manifest_file, canonical_root, lock_path);
            return Err(map_remove_link_error(error));
        }
    }
    let mut staged = manifest.clone();
    staged.phase = PHASE_STAGED.to_string();
    persist_manifest(recovery_root, &staged)?;
    hit_fault(RemoveFault::AfterLinkRemove).inspect_err(|_| {
        let _ = recover_manifest(&manifest_file, canonical_root, lock_path);
    })?;

    hit_fault(RemoveFault::BeforeLockReplace).inspect_err(|_| {
        let _ = recover_manifest(&manifest_file, canonical_root, lock_path);
    })?;
    if injected_fault() == Some(RemoveFault::FingerprintDrift) {
        let mut drifted = lock_bytes.clone();
        drifted.extend_from_slice(b" ");
        let _ = fs::write(lock_path, drifted);
    }
    if let Err(error) = remove_lock_row(lock_path, skill_name, Some(&fingerprint)) {
        let _ = recover_manifest(&manifest_file, canonical_root, lock_path);
        return Err(error);
    }
    let mut committed = staged.clone();
    committed.phase = PHASE_METADATA_COMMITTED.to_string();
    persist_manifest(recovery_root, &committed)?;
    hit_fault(RemoveFault::AfterLockReplace)?;

    if backup.exists() {
        remove_backup_dir(&backup)?;
    }
    hit_fault(RemoveFault::AfterCleanupBackup)?;
    fs::remove_file(&manifest_file).map_err(|source| SkillsCliError::Io {
        context: "finalize Skills CLI remove manifest",
        source,
    })?;

    Ok(SkillsCliRemoveResult {
        removed_canonical: plan.owned_canonical,
        removed_managed_agent_ids: plan
            .managed_placements
            .iter()
            .map(|item| item.agent_id.clone())
            .collect(),
        retained_direct_copy_agent_ids: plan
            .retained_direct_copies
            .iter()
            .map(|item| item.agent_id.clone())
            .collect(),
    })
}

fn collect_managed_links(
    skill_name: &str,
    canonical_root: &Path,
    lock_path: &Path,
    platforms: &[InventoryPlatform],
) -> Result<Vec<ManagedLinkRecord>, SkillsCliError> {
    let ownership = load_cli_lock_ownership(lock_path)?;
    let placement_platforms: Vec<_> = platforms
        .iter()
        .map(InventoryPlatform::as_placement_platform)
        .collect();
    let placements =
        classify_placements(&ownership, skill_name, canonical_root, &placement_platforms);
    Ok(placements
        .into_iter()
        .filter(|placement| placement.state == SkillsCliPlacementState::ManagedLink)
        .map(|placement| ManagedLinkRecord {
            kind: match placement.managed_link_kind {
                Some(SkillsCliManagedLinkKind::WindowsJunction) => "windows_junction".to_string(),
                Some(SkillsCliManagedLinkKind::Symlink) => "symlink".to_string(),
                None => "symlink".to_string(),
            },
            agent_id: placement.agent_id,
            path: placement.target_path,
        })
        .collect())
}

fn persist_manifest(
    recovery_root: &Path,
    manifest: &RemoveManifestV1,
) -> Result<PathBuf, SkillsCliError> {
    fs::create_dir_all(recovery_root).map_err(|source| SkillsCliError::Io {
        context: "create Skills CLI recovery directory",
        source,
    })?;
    let path = manifest_path(recovery_root, &manifest.skill_name);
    let body = serde_json::to_vec(manifest).map_err(|_| SkillsCliError::RecoveryRequired)?;
    atomic_write(&path, &body)?;
    Ok(path)
}

fn manifest_path(recovery_root: &Path, skill_name: &str) -> PathBuf {
    recovery_root.join(format!("{skill_name}.json"))
}

fn recover_manifest(
    path: &Path,
    _canonical_root: &Path,
    lock_path: &Path,
) -> Result<(), SkillsCliError> {
    let bytes = fs::read(path).map_err(|source| SkillsCliError::Io {
        context: "read Skills CLI recovery manifest",
        source,
    })?;
    let manifest: RemoveManifestV1 =
        serde_json::from_slice(&bytes).map_err(|_| SkillsCliError::RecoveryRequired)?;
    if manifest.version != MANIFEST_VERSION {
        return Err(SkillsCliError::RecoveryRequired);
    }
    let canonical = PathBuf::from(&manifest.canonical_path);
    let backup = PathBuf::from(&manifest.canonical_backup_path);
    match manifest.phase.as_str() {
        PHASE_PREPARED => {
            if backup.exists() && !canonical.exists() {
                restore_canonical(&canonical, &backup)?;
            } else if backup.exists() && canonical.exists() {
                return Err(SkillsCliError::RecoveryRequired);
            }
            restore_managed_links(&manifest, &canonical)?;
            let _ = fs::remove_file(path);
            Ok(())
        }
        PHASE_STAGED => {
            restore_canonical(&canonical, &backup)?;
            restore_managed_links(&manifest, &canonical)?;
            verify_lock_fingerprint(lock_path, &manifest.lock_fingerprint)?;
            let _ = fs::remove_file(path);
            Ok(())
        }
        PHASE_METADATA_COMMITTED => {
            if backup.exists() {
                remove_backup_dir(&backup)?;
            }
            fs::remove_file(path).map_err(|source| SkillsCliError::Io {
                context: "finalize Skills CLI recovery",
                source,
            })?;
            Ok(())
        }
        _ => Err(SkillsCliError::RecoveryRequired),
    }
}

fn restore_canonical(canonical: &Path, backup: &Path) -> Result<(), SkillsCliError> {
    match (canonical.exists(), backup.exists()) {
        (true, false) => Ok(()),
        (false, true) => fs::rename(backup, canonical).map_err(|source| SkillsCliError::Io {
            context: "restore Skills CLI canonical",
            source,
        }),
        (false, false) | (true, true) => Err(SkillsCliError::RecoveryRequired),
    }
}

fn restore_managed_links(
    manifest: &RemoveManifestV1,
    canonical: &Path,
) -> Result<(), SkillsCliError> {
    for link in &manifest.managed_links {
        let path = PathBuf::from(&link.path);
        match observe_directory_slot(&path, canonical) {
            DirectorySlotObservation::Managed { .. } => {}
            DirectorySlotObservation::Absent => {
                create_skills_cli_directory_link(canonical, &path)
                    .map_err(map_remove_link_error)?;
            }
            DirectorySlotObservation::OrdinaryDirectory
            | DirectorySlotObservation::Conflict { .. } => {
                return Err(SkillsCliError::RecoveryRequired);
            }
        }
    }
    Ok(())
}

fn verify_lock_fingerprint(lock_path: &Path, expected: &str) -> Result<(), SkillsCliError> {
    let bytes = fs::read(lock_path).map_err(|source| SkillsCliError::Io {
        context: "read Skills CLI lock",
        source,
    })?;
    if hex_digest(&bytes) != expected {
        return Err(SkillsCliError::RecoveryRequired);
    }
    Ok(())
}

fn remove_lock_row(
    lock_path: &Path,
    skill_name: &str,
    expected_fingerprint: Option<&str>,
) -> Result<(), SkillsCliError> {
    let lease_path = lock_path.with_extension("json.skillport-lease");
    let lease = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&lease_path)
        .map_err(|source| SkillsCliError::Io {
            context: "open Skills CLI lock lease",
            source,
        })?;
    lease
        .try_lock_exclusive()
        .map_err(|_| SkillsCliError::Busy)?;
    let bytes = fs::read(lock_path).map_err(|source| SkillsCliError::Io {
        context: "read Skills CLI lock",
        source,
    })?;
    if let Some(expected) = expected_fingerprint {
        if hex_digest(&bytes) != expected {
            let _ = FileExt::unlock(&lease);
            return Err(SkillsCliError::RecoveryRequired);
        }
    }
    let mut value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| SkillsCliError::RecoveryRequired)?;
    let removed = if let Some(skills) = value
        .get_mut("skills")
        .and_then(|item| item.as_object_mut())
    {
        skills.remove(skill_name).is_some()
    } else if let Some(object) = value.as_object_mut() {
        object.remove(skill_name).is_some()
    } else {
        false
    };
    if !removed {
        let _ = FileExt::unlock(&lease);
        return Err(SkillsCliError::SkillNotOwned);
    }
    let next = serde_json::to_vec(&value).map_err(|_| SkillsCliError::RecoveryRequired)?;
    atomic_write(lock_path, &next)?;
    let _ = FileExt::unlock(&lease);
    let _ = fs::remove_file(lease_path);
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), SkillsCliError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp = Builder::new()
        .prefix(TEMP_PREFIX)
        .tempfile_in(parent)
        .map_err(|source| SkillsCliError::Io {
            context: "create Skills CLI lock temp",
            source,
        })?;
    temp.write_all(bytes).map_err(|source| SkillsCliError::Io {
        context: "write Skills CLI lock temp",
        source,
    })?;
    temp.flush().map_err(|source| SkillsCliError::Io {
        context: "flush Skills CLI lock temp",
        source,
    })?;
    temp.as_file()
        .sync_all()
        .map_err(|source| SkillsCliError::Io {
            context: "sync Skills CLI lock temp",
            source,
        })?;
    temp.persist(path).map_err(|error| SkillsCliError::Io {
        context: "replace Skills CLI lock",
        source: error.error,
    })?;
    Ok(())
}

fn remove_backup_dir(path: &Path) -> Result<(), SkillsCliError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| SkillsCliError::Io {
        context: "inspect Skills CLI backup",
        source,
    })?;
    if metadata.file_type().is_symlink() {
        return fs::remove_dir(path)
            .or_else(|_| fs::remove_file(path))
            .map_err(|source| SkillsCliError::Io {
                context: "remove Skills CLI backup link",
                source,
            });
    }
    if !metadata.is_dir() {
        return Err(SkillsCliError::RecoveryRequired);
    }
    fs::remove_dir_all(path).map_err(|source| SkillsCliError::Io {
        context: "remove Skills CLI backup",
        source,
    })
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn hit_fault(fault: RemoveFault) -> Result<(), SkillsCliError> {
    if injected_fault() == Some(fault) {
        Err(SkillsCliError::RecoveryRequired)
    } else {
        Ok(())
    }
}

fn map_remove_link_error(
    error: crate::services::installation::InstallationError,
) -> SkillsCliError {
    SkillsCliError::Io {
        context: "managed directory link",
        source: std::io::Error::other(error.to_string()),
    }
}

#[cfg(test)]
mod tests;
