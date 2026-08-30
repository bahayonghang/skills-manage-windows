//! Remote Skills CLI remove via the transport seam.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use crate::db::DbPool;
use crate::services::central_mutation::{
    acquire_target_mutation_guard, DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};

use super::super::error::SkillsCliError;
use super::super::inventory::InventoryPlatform;
use super::super::placement::{classify_one_observed, ObservedSlot, PlacementPlatform};
use super::super::probe;
use super::super::{
    check_cancel, is_valid_skill_token, load_lock_from_transport, map_guard_error,
    mapped_inventory_platforms_via_transport, remove_recovery_dir_for_transport,
    SkillsCliManagedLinkKind, SkillsCliPlacement, SkillsCliPlacementState, SkillsCliRemovePlan,
    SkillsCliRemoveResult, SkillsCliTransport,
};
use super::{
    conflict_reason_is_symlink_slot, hex_digest, hit_fault, injected_fault, persist_manifest,
    plan_from_classified, recover_pending_via_transport, ManagedLinkRecord, RemoveFault,
    RemoveManifestV1, LOCK_READ_LIMIT, MANIFEST_VERSION, PHASE_METADATA_COMMITTED, PHASE_PREPARED,
    PHASE_STAGED, REMOVE_LOCK_OPERATION,
};

use std::fs;
use uuid::Uuid;

pub(super) async fn preview_remove_global_remote(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    skill_name: &str,
) -> Result<SkillsCliRemovePlan, SkillsCliError> {
    if !is_valid_skill_token(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    let (plan, _) = classify_remove_plan_remote(tx, pool, skill_name).await?;
    Ok(plan)
}

async fn classify_remove_plan_remote(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    skill_name: &str,
) -> Result<(SkillsCliRemovePlan, Vec<SkillsCliPlacement>), SkillsCliError> {
    let ownership = load_lock_from_transport(tx).await?;
    if !ownership.contains_name(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    let platforms = mapped_inventory_platforms_via_transport(tx, pool).await?;
    let placement_platforms: Vec<PlacementPlatform> = platforms
        .iter()
        .map(InventoryPlatform::as_placement_platform)
        .collect();
    let paths = tx.paths();
    let canonical = paths.join_child(paths.canonical_root(), skill_name);
    let mut probe_paths = vec![canonical.clone()];
    for platform in &placement_platforms {
        let slot = paths.join_child(&platform.global_skills_dir.to_string_lossy(), skill_name);
        probe_paths.push(slot);
    }
    let probes = tx.fs().probe_paths(&probe_paths).await?;
    let probe_map = probe::index_probes(&probes);
    let canonical_owned = probe::canonical_owned_from_probe(probe_map.get(&canonical));
    let link_kind = tx.managed_link_kind();
    let posix = paths.uses_posix();
    let mut placements = Vec::new();
    for platform in &placement_platforms {
        let slot = paths.join_child(&platform.global_skills_dir.to_string_lossy(), skill_name);
        let observed = probe_map
            .get(&slot)
            .map(|item| probe::observed_slot_from_probe(item, &canonical, link_kind, posix))
            .unwrap_or(ObservedSlot::Absent);
        placements.push(classify_one_observed(
            canonical_owned,
            observed,
            platform,
            slot,
        ));
    }
    Ok((
        plan_from_classified(skill_name, canonical_owned, &placements),
        placements,
    ))
}

pub(super) async fn remove_global_remote(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    skill_name: &str,
    force: bool,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliRemoveResult, SkillsCliError> {
    if !is_valid_skill_token(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    check_cancel(cancel)?;
    let _guard = acquire_target_mutation_guard(
        &tx.mutation_target(),
        REMOVE_LOCK_OPERATION,
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
    .map_err(map_guard_error)?;
    check_cancel(cancel)?;
    recover_pending_via_transport(tx, skill_name).await?;
    check_cancel(cancel)?;
    execute_remove_via_transport(tx, pool, skill_name, force).await
}

async fn execute_remove_via_transport(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    skill_name: &str,
    force: bool,
) -> Result<SkillsCliRemoveResult, SkillsCliError> {
    let (plan, placements) = classify_remove_plan_remote(tx, pool, skill_name).await?;
    if !plan.conflicts.is_empty() && !force {
        return Err(SkillsCliError::PlacementConflict);
    }
    let paths = tx.paths();
    let lock_path = paths.lock_path().to_string();
    let lock_bytes = match tx.fs().read_file_bounded(&lock_path, LOCK_READ_LIMIT).await {
        Ok(bytes) => bytes,
        Err(error) => return Err(error),
    };
    let fingerprint = hex_digest(&lock_bytes);
    let managed_links = managed_link_records(&placements);
    let force_slots = if force {
        force_unlink_records(&placements)
    } else {
        Vec::new()
    };
    let recovery_root = remove_recovery_dir_for_transport(tx);
    if !plan.owned_canonical && managed_links.is_empty() && force_slots.is_empty() {
        let next = lock_without_skill(&lock_bytes, skill_name)?;
        tx.fs().atomic_write(&lock_path, &next).await?;
        return Ok(result_from_plan(&plan, &force_slots));
    }

    let operation_id = Uuid::new_v4().to_string();
    let canonical = paths.join_child(paths.canonical_root(), skill_name);
    let backup = paths.join_child(
        paths.canonical_root(),
        &format!(".skillport-remove-{operation_id}"),
    );
    let manifest = RemoveManifestV1 {
        version: MANIFEST_VERSION,
        operation_id: operation_id.clone(),
        skill_name: skill_name.to_string(),
        phase: PHASE_PREPARED.to_string(),
        lock_fingerprint: fingerprint.clone(),
        lock_path: lock_path.clone(),
        canonical_path: canonical.clone(),
        canonical_backup_path: backup.clone(),
        managed_links: managed_links.clone(),
    };
    let manifest_file = persist_manifest(&recovery_root, &manifest)?;
    hit_fault(RemoveFault::AfterPrepared)?;

    if plan.owned_canonical {
        if let Err(error) = tx.fs().rename(&canonical, &backup).await {
            let _ = recover_pending_via_transport(tx, skill_name).await;
            return Err(error);
        }
    }
    if let Err(error) = hit_fault(RemoveFault::AfterCanonicalRename) {
        let _ = recover_pending_via_transport(tx, skill_name).await;
        return Err(error);
    }

    let link_paths: Vec<String> = managed_links.iter().map(|item| item.path.clone()).collect();
    if !link_paths.is_empty() {
        if let Err(error) = tx.fs().remove_verified_links(&link_paths).await {
            let _ = recover_pending_via_transport(tx, skill_name).await;
            return Err(error);
        }
    }
    let force_paths: Vec<String> = force_slots.iter().map(|item| item.path.clone()).collect();
    if !force_paths.is_empty() {
        // Verified-remove scripts only `rm -f` / `rmdir` when `[ -L ]`; dirs/files skip.
        if let Err(error) = tx.fs().remove_verified_links(&force_paths).await {
            let _ = recover_pending_via_transport(tx, skill_name).await;
            return Err(error);
        }
    }
    let mut staged = manifest.clone();
    staged.phase = PHASE_STAGED.to_string();
    persist_manifest(&recovery_root, &staged)?;
    if let Err(error) = hit_fault(RemoveFault::AfterLinkRemove) {
        let _ = recover_pending_via_transport(tx, skill_name).await;
        return Err(error);
    }

    if let Err(error) = hit_fault(RemoveFault::BeforeLockReplace) {
        let _ = recover_pending_via_transport(tx, skill_name).await;
        return Err(error);
    }
    let mut cas_bytes = tx
        .fs()
        .read_file_bounded(&lock_path, LOCK_READ_LIMIT)
        .await?;
    if injected_fault() == Some(RemoveFault::FingerprintDrift) {
        cas_bytes.extend_from_slice(b" ");
    }
    if hex_digest(&cas_bytes) != fingerprint {
        let _ = recover_pending_via_transport(tx, skill_name).await;
        return Err(SkillsCliError::RecoveryRequired);
    }
    let next = lock_without_skill(&cas_bytes, skill_name)?;
    if let Err(error) = tx.fs().atomic_write(&lock_path, &next).await {
        let _ = recover_pending_via_transport(tx, skill_name).await;
        return Err(error);
    }
    let mut committed = staged.clone();
    committed.phase = PHASE_METADATA_COMMITTED.to_string();
    persist_manifest(&recovery_root, &committed)?;
    hit_fault(RemoveFault::AfterLockReplace)?;

    if tx.fs().exists(&backup).await.unwrap_or(false) {
        // rm -rf is allowed ONLY on SkillPort-generated canonical backup paths.
        tx.fs().remove_tree(&backup).await?;
    }
    hit_fault(RemoveFault::AfterCleanupBackup)?;
    fs::remove_file(&manifest_file).map_err(|source| SkillsCliError::Io {
        context: "finalize Skills CLI remove manifest",
        source,
    })?;
    let _ = manifest_file;
    Ok(result_from_plan(&plan, &force_slots))
}

fn result_from_plan(
    plan: &SkillsCliRemovePlan,
    force_slots: &[ManagedLinkRecord],
) -> SkillsCliRemoveResult {
    let mut removed_managed: Vec<String> = plan
        .managed_placements
        .iter()
        .map(|item| item.agent_id.clone())
        .collect();
    for slot in force_slots {
        if !removed_managed.contains(&slot.agent_id) {
            removed_managed.push(slot.agent_id.clone());
        }
    }
    SkillsCliRemoveResult {
        removed_canonical: plan.owned_canonical,
        removed_managed_agent_ids: removed_managed,
        retained_direct_copy_agent_ids: plan
            .retained_direct_copies
            .iter()
            .map(|item| item.agent_id.clone())
            .collect(),
    }
}

fn managed_link_records(placements: &[SkillsCliPlacement]) -> Vec<ManagedLinkRecord> {
    placements
        .iter()
        .filter(|placement| placement.state == SkillsCliPlacementState::ManagedLink)
        .map(|placement| ManagedLinkRecord {
            kind: match placement.managed_link_kind {
                Some(SkillsCliManagedLinkKind::WindowsJunction) => "windows_junction".to_string(),
                Some(SkillsCliManagedLinkKind::Symlink) => "symlink".to_string(),
                None => "symlink".to_string(),
            },
            agent_id: placement.agent_id.clone(),
            path: placement.target_path.clone(),
        })
        .collect()
}

fn force_unlink_records(placements: &[SkillsCliPlacement]) -> Vec<ManagedLinkRecord> {
    placements
        .iter()
        .filter(|placement| {
            placement.state == SkillsCliPlacementState::Conflict
                && conflict_reason_is_symlink_slot(placement.reason_code.as_deref())
        })
        .map(|placement| ManagedLinkRecord {
            kind: "symlink".to_string(),
            agent_id: placement.agent_id.clone(),
            path: placement.target_path.clone(),
        })
        .collect()
}

fn lock_without_skill(bytes: &[u8], skill_name: &str) -> Result<Vec<u8>, SkillsCliError> {
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| SkillsCliError::RecoveryRequired)?;
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
        return Err(SkillsCliError::SkillNotOwned);
    }
    serde_json::to_vec(&value).map_err(|_| SkillsCliError::RecoveryRequired)
}

pub(super) async fn recover_manifest_via_transport(
    tx: &SkillsCliTransport,
    path: &Path,
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
    match manifest.phase.as_str() {
        PHASE_PREPARED => {
            restore_canonical_via_transport(tx, &manifest).await?;
            restore_managed_links_via_transport(tx, &manifest).await?;
            let _ = fs::remove_file(path);
            Ok(())
        }
        PHASE_STAGED => {
            restore_canonical_via_transport(tx, &manifest).await?;
            restore_managed_links_via_transport(tx, &manifest).await?;
            verify_lock_fingerprint_via_transport(
                tx,
                &manifest.lock_path,
                &manifest.lock_fingerprint,
            )
            .await?;
            let _ = fs::remove_file(path);
            Ok(())
        }
        PHASE_METADATA_COMMITTED => {
            if tx
                .fs()
                .exists(&manifest.canonical_backup_path)
                .await
                .unwrap_or(false)
            {
                tx.fs().remove_tree(&manifest.canonical_backup_path).await?;
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

async fn restore_canonical_via_transport(
    tx: &SkillsCliTransport,
    manifest: &RemoveManifestV1,
) -> Result<(), SkillsCliError> {
    let canonical_exists = tx.fs().exists(&manifest.canonical_path).await?;
    let backup_exists = tx.fs().exists(&manifest.canonical_backup_path).await?;
    match (canonical_exists, backup_exists) {
        (true, false) => Ok(()),
        (false, true) => {
            tx.fs()
                .rename(&manifest.canonical_backup_path, &manifest.canonical_path)
                .await
        }
        (true, true) => Err(SkillsCliError::RecoveryRequired),
        (false, false) if manifest.phase == PHASE_PREPARED => Ok(()),
        (false, false) => Err(SkillsCliError::RecoveryRequired),
    }
}

async fn restore_managed_links_via_transport(
    tx: &SkillsCliTransport,
    manifest: &RemoveManifestV1,
) -> Result<(), SkillsCliError> {
    if manifest.managed_links.is_empty() {
        return Ok(());
    }
    let mut paths: Vec<String> = manifest
        .managed_links
        .iter()
        .map(|item| item.path.clone())
        .collect();
    paths.push(manifest.canonical_path.clone());
    let probes = tx.fs().probe_paths(&paths).await?;
    let probe_map = probe::index_probes(&probes);
    let kind = tx.managed_link_kind();
    let posix = tx.paths().uses_posix();
    for link in &manifest.managed_links {
        let observed = probe_map
            .get(&link.path)
            .map(|item| {
                probe::observed_slot_from_probe(item, &manifest.canonical_path, kind, posix)
            })
            .unwrap_or(ObservedSlot::Absent);
        match observed {
            ObservedSlot::ManagedLink {
                resolves_to_canonical: true,
                ..
            } => {}
            ObservedSlot::Absent => {
                tx.fs()
                    .create_managed_link(&manifest.canonical_path, &link.path)
                    .await?;
            }
            ObservedSlot::PlainDirectory | ObservedSlot::Conflict { .. } => {
                return Err(SkillsCliError::RecoveryRequired);
            }
            ObservedSlot::ManagedLink {
                resolves_to_canonical: false,
                ..
            } => return Err(SkillsCliError::RecoveryRequired),
        }
    }
    Ok(())
}

async fn verify_lock_fingerprint_via_transport(
    tx: &SkillsCliTransport,
    lock_path: &str,
    expected: &str,
) -> Result<(), SkillsCliError> {
    let bytes = tx
        .fs()
        .read_file_bounded(lock_path, LOCK_READ_LIMIT)
        .await?;
    if hex_digest(&bytes) != expected {
        return Err(SkillsCliError::RecoveryRequired);
    }
    Ok(())
}
