//! Skills CLI link/unlink mutations.
//!
//! Lock order: exclusive job lease (command) → Local mutation guard →
//! under-guard ownership/placement recheck → FS mutation.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crate::db::DbPool;
use crate::fs_util::run_blocking_fs_with;
use crate::services::central_mutation::{
    acquire_central_mutation_guard_at, acquire_target_mutation_guard, CentralMutationGuard,
    DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};
use crate::services::installation::fs_util::{
    create_skills_cli_directory_link, inspect_managed_directory_link, remove_directory_link_slot,
    remove_verified_directory_link,
};
use crate::targets::ActiveTarget;

use super::error::SkillsCliError;
use super::files::resolve_owned_canonical;
use super::lock::load_cli_lock_ownership;
use super::placement::{classify_one, classify_one_observed, ObservedSlot, PlacementPlatform};
use super::probe;
use super::remote_scripts::VerifiedLinkRemoveStatus;
use super::remove::recover_pending_for_skill_at;
use super::{
    check_cancel, is_valid_skill_token, load_lock_from_transport, map_guard_error,
    mapped_inventory_platforms, mapped_inventory_platforms_via_transport,
    remove_recovery_dir_for_transport, SkillsCliPlacement, SkillsCliPlacementState,
    SkillsCliTransport,
};

pub(super) const LINK_LOCK_OPERATION: &str = "Skills CLI platform link";
pub(super) const UNLINK_LOCK_OPERATION: &str = "Skills CLI platform unlink";

pub(crate) async fn link_platform(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    skill_name: &str,
    skillport_agent_id: &str,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliPlacement, SkillsCliError> {
    if tx.is_remote() {
        return mutate_placement_remote(
            tx,
            pool,
            skill_name,
            skillport_agent_id,
            cancel,
            DEFAULT_CENTRAL_MUTATION_TIMEOUT,
            PlacementAction::Link,
        )
        .await;
    }
    let paths = tx.paths();
    link_platform_at(
        pool,
        &paths.canonical_root_path(),
        &paths.lock_path_buf(),
        None,
        remove_recovery_dir_for_transport(tx),
        skill_name,
        skillport_agent_id,
        cancel,
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
}

pub(crate) async fn unlink_platform(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    skill_name: &str,
    skillport_agent_id: &str,
    force: bool,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliPlacement, SkillsCliError> {
    if tx.is_remote() {
        return mutate_placement_remote(
            tx,
            pool,
            skill_name,
            skillport_agent_id,
            cancel,
            DEFAULT_CENTRAL_MUTATION_TIMEOUT,
            PlacementAction::Unlink { force },
        )
        .await;
    }
    let paths = tx.paths();
    unlink_platform_at(
        pool,
        &paths.canonical_root_path(),
        &paths.lock_path_buf(),
        None,
        remove_recovery_dir_for_transport(tx),
        skill_name,
        skillport_agent_id,
        cancel,
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
        force,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn link_platform_at(
    pool: &DbPool,
    canonical_root: &Path,
    lock_path: &Path,
    mutation_lock_path: Option<PathBuf>,
    recovery_root: PathBuf,
    skill_name: &str,
    skillport_agent_id: &str,
    cancel: Option<&AtomicBool>,
    timeout: Duration,
) -> Result<SkillsCliPlacement, SkillsCliError> {
    mutate_placement(
        pool,
        canonical_root,
        lock_path,
        mutation_lock_path,
        recovery_root,
        skill_name,
        skillport_agent_id,
        cancel,
        timeout,
        LINK_LOCK_OPERATION,
        PlacementAction::Link,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn unlink_platform_at(
    pool: &DbPool,
    canonical_root: &Path,
    lock_path: &Path,
    mutation_lock_path: Option<PathBuf>,
    recovery_root: PathBuf,
    skill_name: &str,
    skillport_agent_id: &str,
    cancel: Option<&AtomicBool>,
    timeout: Duration,
    force: bool,
) -> Result<SkillsCliPlacement, SkillsCliError> {
    mutate_placement(
        pool,
        canonical_root,
        lock_path,
        mutation_lock_path,
        recovery_root,
        skill_name,
        skillport_agent_id,
        cancel,
        timeout,
        UNLINK_LOCK_OPERATION,
        PlacementAction::Unlink { force },
    )
    .await
}

#[derive(Clone, Copy)]
pub(super) enum PlacementAction {
    Link,
    Unlink { force: bool },
}

#[allow(clippy::too_many_arguments)]
async fn mutate_placement(
    pool: &DbPool,
    canonical_root: &Path,
    lock_path: &Path,
    mutation_lock_path: Option<PathBuf>,
    recovery_root: PathBuf,
    skill_name: &str,
    skillport_agent_id: &str,
    cancel: Option<&AtomicBool>,
    timeout: Duration,
    operation: &'static str,
    action: PlacementAction,
) -> Result<SkillsCliPlacement, SkillsCliError> {
    if !is_valid_skill_token(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    check_cancel(cancel)?;
    let _guard = acquire_mutation_guard(mutation_lock_path, operation, timeout).await?;
    check_cancel(cancel)?;

    let platform = load_platform(pool, skillport_agent_id).await?;
    check_cancel(cancel)?;
    let canonical_root = canonical_root.to_path_buf();
    let lock_path = lock_path.to_path_buf();
    let skill_name = skill_name.to_string();
    #[cfg(test)]
    let create_fault = crate::services::installation::directory_link::create_fault_after_dir();
    run_blocking_fs_with(
        operation,
        move || {
            #[cfg(test)]
            crate::services::installation::directory_link::set_create_fault_after_dir(create_fault);
            let result = (|| {
                recover_pending_for_skill_at(
                    &recovery_root,
                    &canonical_root,
                    &lock_path,
                    &skill_name,
                )?;
                let _owned = resolve_owned_canonical(&skill_name, &canonical_root, &lock_path)?;
                let ownership = load_cli_lock_ownership(&lock_path)?;
                if !ownership.contains_name(&skill_name) {
                    return Err(SkillsCliError::SkillNotOwned);
                }
                let canonical = ownership.canonical_dir(&canonical_root, &skill_name);
                let current = classify_one(&skill_name, &canonical, &platform);
                match action {
                    PlacementAction::Link => {
                        apply_link(&canonical, &current, &platform, &skill_name)?
                    }
                    PlacementAction::Unlink { force } => apply_unlink(&canonical, &current, force)?,
                }
                Ok(classify_one(&skill_name, &canonical, &platform))
            })();
            #[cfg(test)]
            crate::services::installation::directory_link::set_create_fault_after_dir(false);
            result
        },
        SkillsCliError::task_join,
    )
    .await
}

async fn acquire_mutation_guard(
    mutation_lock_path: Option<PathBuf>,
    operation: &'static str,
    timeout: Duration,
) -> Result<CentralMutationGuard, SkillsCliError> {
    match mutation_lock_path {
        Some(path) => acquire_central_mutation_guard_at(path, operation, timeout)
            .await
            .map_err(map_guard_error),
        None => acquire_target_mutation_guard(&ActiveTarget::Local, operation, timeout)
            .await
            .map_err(map_guard_error),
    }
}

async fn load_platform(
    pool: &DbPool,
    skillport_agent_id: &str,
) -> Result<PlacementPlatform, SkillsCliError> {
    let agents = crate::db::get_all_agents(pool)
        .await
        .map_err(|error| SkillsCliError::Io {
            context: "read platforms",
            source: std::io::Error::other(error.to_string()),
        })?;
    mapped_inventory_platforms(&agents)
        .into_iter()
        .find(|platform| platform.agent_id == skillport_agent_id)
        .map(|platform| platform.as_placement_platform())
        .ok_or_else(|| SkillsCliError::AgentUnmapped(skillport_agent_id.to_string()))
}

fn apply_link(
    canonical: &Path,
    current: &SkillsCliPlacement,
    platform: &PlacementPlatform,
    skill_name: &str,
) -> Result<(), SkillsCliError> {
    match current.state {
        SkillsCliPlacementState::ManagedLink => Ok(()),
        SkillsCliPlacementState::Missing => {
            let link = platform.global_skills_dir.join(skill_name);
            create_skills_cli_directory_link(canonical, &link).map_err(map_link_error)?;
            if inspect_managed_directory_link(&link, canonical)
                .ok()
                .flatten()
                .is_none()
            {
                let _ = remove_verified_directory_link(&link, canonical);
                return Err(SkillsCliError::PlacementConflict);
            }
            Ok(())
        }
        SkillsCliPlacementState::DirectCopy => Err(SkillsCliError::DirectCopyNotToggleable),
        SkillsCliPlacementState::Conflict => Err(SkillsCliError::PlacementConflict),
        SkillsCliPlacementState::Unavailable => Err(SkillsCliError::PlacementUnavailable),
    }
}

fn apply_unlink(
    canonical: &Path,
    current: &SkillsCliPlacement,
    force: bool,
) -> Result<(), SkillsCliError> {
    match current.state {
        SkillsCliPlacementState::Missing => Ok(()),
        SkillsCliPlacementState::ManagedLink => {
            let link = PathBuf::from(&current.target_path);
            remove_verified_directory_link(&link, canonical).map_err(map_link_error)?;
            Ok(())
        }
        SkillsCliPlacementState::DirectCopy => Err(SkillsCliError::DirectCopyNotToggleable),
        SkillsCliPlacementState::Conflict => {
            if !force {
                return Err(SkillsCliError::PlacementConflict);
            }
            let link = PathBuf::from(&current.target_path);
            let removed = remove_directory_link_slot(&link).map_err(map_link_error)?;
            if removed {
                return Ok(());
            }
            // Absent is idempotent (same as managed-link remove). Ordinary
            // directories and files are never deleted.
            if std::fs::symlink_metadata(&link).is_err() {
                Ok(())
            } else {
                Err(SkillsCliError::DirectCopyNotToggleable)
            }
        }
        SkillsCliPlacementState::Unavailable => Err(SkillsCliError::PlacementUnavailable),
    }
}

fn map_link_error(error: crate::services::installation::InstallationError) -> SkillsCliError {
    match error {
        crate::services::installation::InstallationError::ManagedDirectoryLinkUnsupported => {
            SkillsCliError::PlacementUnavailable
        }
        crate::services::installation::InstallationError::ManagedDirectoryLinkTargetMismatch => {
            SkillsCliError::PlacementConflict
        }
        other => SkillsCliError::Io {
            context: "managed directory link",
            source: std::io::Error::other(other.to_string()),
        },
    }
}

#[derive(Clone, Copy)]
pub(super) enum LinkOp {
    Noop,
    Create,
}

#[derive(Clone, Copy)]
pub(super) enum UnlinkOp {
    Noop,
    Remove,
}

pub(super) fn decide_link(state: SkillsCliPlacementState) -> Result<LinkOp, SkillsCliError> {
    match state {
        SkillsCliPlacementState::ManagedLink => Ok(LinkOp::Noop),
        SkillsCliPlacementState::Missing => Ok(LinkOp::Create),
        SkillsCliPlacementState::DirectCopy => Err(SkillsCliError::DirectCopyNotToggleable),
        SkillsCliPlacementState::Conflict => Err(SkillsCliError::PlacementConflict),
        SkillsCliPlacementState::Unavailable => Err(SkillsCliError::PlacementUnavailable),
    }
}

pub(super) fn decide_unlink(
    state: SkillsCliPlacementState,
    force: bool,
) -> Result<UnlinkOp, SkillsCliError> {
    match state {
        SkillsCliPlacementState::Missing => Ok(UnlinkOp::Noop),
        SkillsCliPlacementState::ManagedLink => Ok(UnlinkOp::Remove),
        SkillsCliPlacementState::DirectCopy => Err(SkillsCliError::DirectCopyNotToggleable),
        SkillsCliPlacementState::Conflict => {
            if force {
                Ok(UnlinkOp::Remove)
            } else {
                Err(SkillsCliError::PlacementConflict)
            }
        }
        SkillsCliPlacementState::Unavailable => Err(SkillsCliError::PlacementUnavailable),
    }
}

fn placement_after_link(
    current: SkillsCliPlacement,
    kind: super::SkillsCliManagedLinkKind,
) -> SkillsCliPlacement {
    SkillsCliPlacement {
        state: SkillsCliPlacementState::ManagedLink,
        managed_link_kind: Some(kind),
        reason_code: None,
        ..current
    }
}

fn placement_after_unlink(current: SkillsCliPlacement) -> SkillsCliPlacement {
    SkillsCliPlacement {
        state: SkillsCliPlacementState::Missing,
        managed_link_kind: None,
        reason_code: None,
        ..current
    }
}

async fn mutate_placement_remote(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    skill_name: &str,
    skillport_agent_id: &str,
    cancel: Option<&AtomicBool>,
    timeout: Duration,
    action: PlacementAction,
) -> Result<SkillsCliPlacement, SkillsCliError> {
    if !is_valid_skill_token(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    check_cancel(cancel)?;
    let operation = match action {
        PlacementAction::Link => LINK_LOCK_OPERATION,
        PlacementAction::Unlink { .. } => UNLINK_LOCK_OPERATION,
    };
    let _guard = acquire_target_mutation_guard(&tx.mutation_target(), operation, timeout)
        .await
        .map_err(map_guard_error)?;
    check_cancel(cancel)?;
    super::remove::recover_pending_via_transport(tx, skill_name).await?;
    check_cancel(cancel)?;

    let ownership = load_lock_from_transport(tx).await?;
    if !ownership.contains_name(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    let platforms = mapped_inventory_platforms_via_transport(tx, pool).await?;
    let platform = platforms
        .into_iter()
        .find(|item| item.agent_id == skillport_agent_id)
        .map(|item| item.as_placement_platform())
        .ok_or_else(|| SkillsCliError::AgentUnmapped(skillport_agent_id.to_string()))?;
    let paths = tx.paths();
    let canonical = paths.join_child(paths.canonical_root(), skill_name);
    let slot = paths.join_child(&platform.global_skills_dir.to_string_lossy(), skill_name);
    let probes = tx
        .fs()
        .probe_paths(&[canonical.clone(), slot.clone()])
        .await?;
    let probe_map = probe::index_probes(&probes);
    let canonical_owned = probe::canonical_owned_from_probe(probe_map.get(&canonical));
    let observed = probe_map
        .get(&slot)
        .map(|item| {
            probe::observed_slot_from_probe(
                item,
                &canonical,
                tx.managed_link_kind(),
                paths.uses_posix(),
            )
        })
        .unwrap_or(ObservedSlot::Absent);
    let current = classify_one_observed(canonical_owned, observed, &platform, slot.clone());
    match action {
        PlacementAction::Link => match decide_link(current.state)? {
            LinkOp::Noop => Ok(current),
            LinkOp::Create => {
                let kind = tx.fs().create_managed_link(&canonical, &slot).await?;
                Ok(placement_after_link(current, kind))
            }
        },
        PlacementAction::Unlink { force } => match decide_unlink(current.state, force)? {
            UnlinkOp::Noop => Ok(current),
            UnlinkOp::Remove => {
                let status = tx.fs().remove_verified_link(&slot).await?;
                match status {
                    VerifiedLinkRemoveStatus::SkippedNotLink => {
                        Err(SkillsCliError::DirectCopyNotToggleable)
                    }
                    VerifiedLinkRemoveStatus::Removed | VerifiedLinkRemoveStatus::Absent => {
                        Ok(placement_after_unlink(current))
                    }
                }
            }
        },
    }
}

mod batch;
pub(crate) use batch::{link_platforms_batch, unlink_platforms_batch};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::central_mutation::acquire_central_mutation_guard_at;
    use crate::test_support::{mem_pool, set_agent_dir};
    use tempfile::TempDir;

    async fn harness() -> (DbPool, TempDir, PathBuf, PathBuf, PathBuf, PathBuf) {
        let pool = mem_pool().await;
        let temp = TempDir::new().unwrap();
        let canonical_root = temp.path().join("universal");
        let cursor = temp.path().join("cursor");
        std::fs::create_dir_all(canonical_root.join("demo")).unwrap();
        std::fs::create_dir_all(&cursor).unwrap();
        set_agent_dir(&pool, "cursor", &cursor).await;
        sqlx::query("UPDATE agents SET is_enabled = 1 WHERE id = ?")
            .bind("cursor")
            .execute(&pool)
            .await
            .unwrap();
        let lock_path = temp.path().join(".skill-lock.json");
        std::fs::write(
            &lock_path,
            r#"{"version":3,"skills":{"demo":{"source":"owner/repo"}}}"#,
        )
        .unwrap();
        let recovery = temp.path().join("recovery");
        (pool, temp, canonical_root, lock_path, recovery, cursor)
    }

    #[tokio::test]
    async fn cancel_before_guard_does_not_wait_on_busy_lock() {
        let (pool, temp, canonical_root, lock_path, recovery, _) = harness().await;
        let mutation_lock = temp.path().join("mutation.lock");
        let _guard = acquire_central_mutation_guard_at(
            mutation_lock.clone(),
            "held",
            Duration::from_secs(2),
        )
        .await
        .unwrap();
        let cancel = AtomicBool::new(true);
        let started = std::time::Instant::now();
        let err = link_platform_at(
            &pool,
            &canonical_root,
            &lock_path,
            Some(mutation_lock),
            recovery,
            "demo",
            "cursor",
            Some(&cancel),
            Duration::from_secs(30),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, SkillsCliError::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn busy_guard_is_typed_busy() {
        let (pool, temp, canonical_root, lock_path, recovery, _) = harness().await;
        let mutation_lock = temp.path().join("mutation.lock");
        let _guard = acquire_central_mutation_guard_at(
            mutation_lock.clone(),
            "held",
            Duration::from_secs(2),
        )
        .await
        .unwrap();
        let err = link_platform_at(
            &pool,
            &canonical_root,
            &lock_path,
            Some(mutation_lock),
            recovery,
            "demo",
            "cursor",
            None,
            Duration::from_millis(1),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, SkillsCliError::Busy));
    }

    #[tokio::test]
    async fn missing_to_managed_link_and_idempotent_unlink() {
        let (pool, temp, canonical_root, lock_path, recovery, cursor) = harness().await;
        let mutation_lock = temp.path().join("mutation.lock");
        let linked = link_platform_at(
            &pool,
            &canonical_root,
            &lock_path,
            Some(mutation_lock.clone()),
            recovery.clone(),
            "demo",
            "cursor",
            None,
            Duration::from_secs(2),
        )
        .await
        .unwrap();
        assert_eq!(linked.state, SkillsCliPlacementState::ManagedLink);
        assert!(cursor.join("demo").exists());

        let again = link_platform_at(
            &pool,
            &canonical_root,
            &lock_path,
            Some(mutation_lock.clone()),
            recovery.clone(),
            "demo",
            "cursor",
            None,
            Duration::from_secs(2),
        )
        .await
        .unwrap();
        assert_eq!(again.state, SkillsCliPlacementState::ManagedLink);

        let missing = unlink_platform_at(
            &pool,
            &canonical_root,
            &lock_path,
            Some(mutation_lock),
            recovery,
            "demo",
            "cursor",
            None,
            Duration::from_secs(2),
            false,
        )
        .await
        .unwrap();
        assert_eq!(missing.state, SkillsCliPlacementState::Missing);
        assert!(!cursor.join("demo").exists());
    }

    #[tokio::test]
    async fn ordinary_directory_is_zero_write() {
        let (pool, temp, canonical_root, lock_path, recovery, cursor) = harness().await;
        std::fs::create_dir_all(cursor.join("demo")).unwrap();
        std::fs::write(cursor.join("demo/keep.bin"), b"copy").unwrap();
        let mutation_lock = temp.path().join("mutation.lock");
        let err = link_platform_at(
            &pool,
            &canonical_root,
            &lock_path,
            Some(mutation_lock.clone()),
            recovery.clone(),
            "demo",
            "cursor",
            None,
            Duration::from_secs(2),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, SkillsCliError::DirectCopyNotToggleable));
        assert_eq!(
            std::fs::read(cursor.join("demo/keep.bin")).unwrap(),
            b"copy"
        );

        let err = unlink_platform_at(
            &pool,
            &canonical_root,
            &lock_path,
            Some(mutation_lock),
            recovery,
            "demo",
            "cursor",
            None,
            Duration::from_secs(2),
            false,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, SkillsCliError::DirectCopyNotToggleable));
        assert_eq!(
            std::fs::read(cursor.join("demo/keep.bin")).unwrap(),
            b"copy"
        );
    }

    #[tokio::test]
    async fn partial_create_cleanup_leaves_no_empty_entry() {
        let (pool, temp, canonical_root, lock_path, recovery, cursor) = harness().await;
        crate::services::installation::directory_link::set_create_fault_after_dir(true);
        let mutation_lock = temp.path().join("mutation.lock");
        let err = link_platform_at(
            &pool,
            &canonical_root,
            &lock_path,
            Some(mutation_lock),
            recovery,
            "demo",
            "cursor",
            None,
            Duration::from_secs(2),
        )
        .await
        .unwrap_err();
        crate::services::installation::directory_link::set_create_fault_after_dir(false);
        assert!(matches!(err, SkillsCliError::Io { .. }));
        assert!(!cursor.join("demo").exists());
    }

    #[tokio::test]
    async fn force_unlinks_wrong_target_symlink_and_leaves_direct_copy() {
        let (pool, temp, canonical_root, lock_path, recovery, cursor) = harness().await;
        let foreign = temp.path().join("central/demo");
        std::fs::create_dir_all(&foreign).unwrap();
        std::fs::write(foreign.join("keep.bin"), b"central").unwrap();
        let slot = cursor.join("demo");
        create_skills_cli_directory_link(&foreign, &slot).unwrap();
        let mutation_lock = temp.path().join("mutation.lock");
        let missing = unlink_platform_at(
            &pool,
            &canonical_root,
            &lock_path,
            Some(mutation_lock),
            recovery,
            "demo",
            "cursor",
            None,
            Duration::from_secs(2),
            true,
        )
        .await
        .unwrap();
        assert_eq!(missing.state, SkillsCliPlacementState::Missing);
        assert!(!slot.exists());
        assert_eq!(std::fs::read(foreign.join("keep.bin")).unwrap(), b"central");
    }

    #[tokio::test]
    async fn force_unlink_skips_ordinary_directory() {
        let (pool, temp, canonical_root, lock_path, recovery, cursor) = harness().await;
        std::fs::create_dir_all(cursor.join("demo")).unwrap();
        std::fs::write(cursor.join("demo/keep.bin"), b"copy").unwrap();
        let mutation_lock = temp.path().join("mutation.lock");
        let err = unlink_platform_at(
            &pool,
            &canonical_root,
            &lock_path,
            Some(mutation_lock),
            recovery,
            "demo",
            "cursor",
            None,
            Duration::from_secs(2),
            true,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, SkillsCliError::DirectCopyNotToggleable));
        assert_eq!(
            std::fs::read(cursor.join("demo/keep.bin")).unwrap(),
            b"copy"
        );
    }
}
