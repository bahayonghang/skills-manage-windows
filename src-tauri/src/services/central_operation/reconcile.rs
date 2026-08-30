use std::path::Path;

use crate::db::{self, DbPool, FsDbOperationRow};
use crate::services::central_mutation::{
    acquire_target_mutation_guard, DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};
use crate::targets::{connect_remote_target, ActiveTarget, ConnectedRemoteTarget};

use super::fs::fingerprint_local_path;
use super::path::{normalize_remote_delete_path, remote_fingerprint};
use super::recovery::list_pending_operations;
use super::{
    CentralOperationError, DeleteManifest, ManagedPath, OperationManifest,
    PreparedDeleteReconciliationPreview,
};

pub(super) const BLOCK_TARGET_MISMATCH: &str = "recovery.reconcile_target_mismatch";
pub(super) const BLOCK_UNSUPPORTED_KIND: &str = "recovery.reconcile_unsupported_kind";
pub(super) const BLOCK_UNSUPPORTED_PHASE: &str = "recovery.reconcile_unsupported_phase";
pub(super) const BLOCK_INVALID_MANIFEST: &str = "recovery.reconcile_invalid_manifest";
pub(super) const BLOCK_INCONSISTENT_DUPLICATE: &str = "recovery.reconcile_inconsistent_duplicate";
const BLOCK_SKILL_MISSING: &str = "recovery.reconcile_skill_missing";
const BLOCK_OWNED_PATH_MISSING: &str = "recovery.reconcile_owned_path_missing";
const BLOCK_FINGERPRINT_DRIFT: &str = "recovery.reconcile_fingerprint_drift";
pub(super) const BLOCK_ARTIFACT_REMAINING: &str = "recovery.reconcile_artifact_remaining";
const BLOCK_REMOTE_INSPECTION: &str = "recovery.reconcile_remote_inspection_failed";

pub async fn preview_prepared_delete_reconciliation(
    pool: &DbPool,
    target: &ActiveTarget,
    operation_id: &str,
) -> Result<PreparedDeleteReconciliationPreview, CentralOperationError> {
    let _guard = acquire_target_mutation_guard(
        target,
        "preview Central operation reconciliation",
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
    .map_err(|_| CentralOperationError::ReconciliationBlocked {
        code: "reconcile_guard_unavailable",
    })?;
    preview_under_guard(pool, target, operation_id).await
}

pub async fn reconcile_prepared_delete(
    pool: &DbPool,
    target: &ActiveTarget,
    operation_id: &str,
) -> Result<Vec<super::PendingOperationSummary>, CentralOperationError> {
    let _guard = acquire_target_mutation_guard(
        target,
        "reconcile Central operation",
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
    .map_err(|_| CentralOperationError::ReconciliationBlocked {
        code: "reconcile_guard_unavailable",
    })?;
    let preview = preview_under_guard(pool, target, operation_id).await?;
    if !preview.eligible {
        return Err(CentralOperationError::ReconciliationBlocked {
            code: "reconcile_preflight_blocked",
        });
    }
    db::transition_fs_db_operation(pool, operation_id, "prepared", "rolled_back").await?;
    list_pending_operations(pool, target).await
}

async fn preview_under_guard(
    pool: &DbPool,
    target: &ActiveTarget,
    operation_id: &str,
) -> Result<PreparedDeleteReconciliationPreview, CentralOperationError> {
    let row = db::get_fs_db_operation(pool, operation_id)
        .await?
        .ok_or_else(|| CentralOperationError::InvalidManifest("operation not found".to_string()))?;
    let mut preview = PreparedDeleteReconciliationPreview {
        operation_id: row.id.clone(),
        skill_id: row.skill_id.clone(),
        eligible: false,
        duplicate_path_count: 0,
        missing_unowned_path_count: 0,
        blocker_codes: Vec::new(),
    };

    if row.target_id != target.id() || row.target_kind != target_kind(target) {
        push_blocker(&mut preview, BLOCK_TARGET_MISMATCH);
        return Ok(preview);
    }
    if row.operation_kind != "central_delete" {
        push_blocker(&mut preview, BLOCK_UNSUPPORTED_KIND);
    }
    if row.phase != "prepared" {
        push_blocker(&mut preview, BLOCK_UNSUPPORTED_PHASE);
    }

    let manifest = match decode_delete_manifest(&row) {
        Ok(manifest) => manifest,
        Err(()) => {
            push_blocker(&mut preview, BLOCK_INVALID_MANIFEST);
            return Ok(preview);
        }
    };
    if manifest
        .paths
        .iter()
        .any(|path| !reconciliation_path_is_valid(target, path))
    {
        push_blocker(&mut preview, BLOCK_INVALID_MANIFEST);
        return Ok(preview);
    }
    let collapsed = collapse_managed_paths(target, manifest.paths);
    preview.duplicate_path_count = collapsed.duplicate_path_count;
    if collapsed.inconsistent {
        push_blocker(&mut preview, BLOCK_INCONSISTENT_DUPLICATE);
    }
    let unique_paths = collapsed.unique;

    let Some(skill) = db::get_skill_by_id(pool, &row.skill_id).await? else {
        push_blocker(&mut preview, BLOCK_SKILL_MISSING);
        return Ok(preview);
    };
    let installations = db::get_skill_installations(pool, &row.skill_id).await?;
    let mut owned_paths = installations
        .into_iter()
        .map(|installation| installation.installed_path)
        .collect::<Vec<_>>();
    if let Some(canonical_path) = skill.canonical_path {
        owned_paths.push(canonical_path);
    } else if let Some(parent) = Path::new(&skill.file_path).parent() {
        owned_paths.push(parent.to_string_lossy().into_owned());
    }

    let remote = if target.is_remote_like() {
        match connect_remote_target(target).await {
            Ok(remote) => Some(remote),
            Err(_) => {
                push_blocker(&mut preview, BLOCK_REMOTE_INSPECTION);
                return Ok(preview);
            }
        }
    } else {
        None
    };

    for path in &unique_paths {
        if inspect_path(
            &row.id,
            path,
            &owned_paths,
            target,
            remote.as_ref(),
            &mut preview,
        )
        .await
        .is_err()
        {
            push_blocker(&mut preview, BLOCK_REMOTE_INSPECTION);
            break;
        }
    }
    preview.eligible = preview.blocker_codes.is_empty();
    Ok(preview)
}

async fn inspect_path(
    _operation_id: &str,
    path: &ManagedPath,
    owned_paths: &[String],
    target: &ActiveTarget,
    remote: Option<&ConnectedRemoteTarget>,
    preview: &mut PreparedDeleteReconciliationPreview,
) -> Result<(), ()> {
    let (original_exists, backup_exists, marker_exists, fingerprint) = if let Some(remote) = remote
    {
        let original_exists = remote.exists(&path.original).await.map_err(|_| ())?;
        let backup_exists = remote.exists(&path.backup).await.map_err(|_| ())?;
        let marker_exists = remote.exists(&path.marker).await.map_err(|_| ())?;
        let fingerprint = if original_exists {
            remote_fingerprint(remote, &path.original)
                .await
                .map_err(|_| ())?
        } else {
            None
        };
        (original_exists, backup_exists, marker_exists, fingerprint)
    } else {
        let original_exists = std::fs::symlink_metadata(&path.original).is_ok();
        let backup_exists = std::fs::symlink_metadata(&path.backup).is_ok();
        let marker_exists = std::fs::symlink_metadata(&path.marker).is_ok();
        let fingerprint = if original_exists {
            fingerprint_local_path(Path::new(&path.original))
                .await
                .map_err(|_| ())?
        } else {
            None
        };
        (original_exists, backup_exists, marker_exists, fingerprint)
    };

    if backup_exists || marker_exists {
        push_blocker(preview, BLOCK_ARTIFACT_REMAINING);
    }
    if original_exists {
        if !path.expected_present || fingerprint.as_deref() != path.fingerprint.as_deref() {
            push_blocker(preview, BLOCK_FINGERPRINT_DRIFT);
        }
    } else if path.expected_present {
        if owned_paths
            .iter()
            .any(|owned| paths_match(target, owned, &path.original))
        {
            push_blocker(preview, BLOCK_OWNED_PATH_MISSING);
        } else {
            preview.missing_unowned_path_count += 1;
        }
    }
    Ok(())
}

pub(super) struct CollapsedManagedPaths {
    pub unique: Vec<ManagedPath>,
    pub duplicate_path_count: usize,
    pub inconsistent: bool,
}

pub(super) fn collapse_managed_paths(
    target: &ActiveTarget,
    paths: Vec<ManagedPath>,
) -> CollapsedManagedPaths {
    let mut unique: Vec<ManagedPath> = Vec::new();
    let mut duplicate_path_count = 0;
    let mut inconsistent = false;
    for path in paths {
        if let Some(existing) = unique
            .iter()
            .find(|existing| paths_match(target, &existing.original, &path.original))
        {
            duplicate_path_count += 1;
            if existing.expected_present != path.expected_present
                || existing.fingerprint != path.fingerprint
                || !paths_match(target, &existing.backup, &path.backup)
                || !paths_match(target, &existing.marker, &path.marker)
            {
                inconsistent = true;
            }
        } else {
            unique.push(path);
        }
    }
    CollapsedManagedPaths {
        unique,
        duplicate_path_count,
        inconsistent,
    }
}

pub(super) fn decode_delete_manifest(row: &FsDbOperationRow) -> Result<DeleteManifest, ()> {
    if row.manifest_version != super::MANIFEST_VERSION {
        return Err(());
    }
    let manifest: OperationManifest = serde_json::from_str(&row.manifest_json).map_err(|_| ())?;
    manifest.validate(&row.id).map_err(|_| ())?;
    match manifest {
        OperationManifest::Delete(manifest) => Ok(manifest),
        OperationManifest::Update(_) => Err(()),
    }
}

pub(super) fn paths_match(target: &ActiveTarget, left: &str, right: &str) -> bool {
    match target {
        ActiveTarget::Local => crate::paths::paths_equivalent(Path::new(left), Path::new(right)),
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            match (
                normalize_remote_delete_path(left),
                normalize_remote_delete_path(right),
            ) {
                (Ok(left), Ok(right)) => left == right,
                _ => false,
            }
        }
    }
}

pub(super) fn reconciliation_path_is_valid(target: &ActiveTarget, path: &ManagedPath) -> bool {
    match target {
        ActiveTarget::Local => true,
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => [&path.original, &path.backup, &path.marker]
            .into_iter()
            .all(|value| normalize_remote_delete_path(value).is_ok()),
    }
}

pub(super) fn target_kind(target: &ActiveTarget) -> &'static str {
    match target {
        ActiveTarget::Local => "local",
        ActiveTarget::Ssh(_) => "ssh",
        ActiveTarget::Wsl(_) => "wsl",
    }
}

pub(super) fn push_unique_code(codes: &mut Vec<String>, code: &str) {
    if !codes.iter().any(|existing| existing == code) {
        codes.push(code.to_string());
    }
}

fn push_blocker(preview: &mut PreparedDeleteReconciliationPreview, code: &str) {
    push_unique_code(&mut preview.blocker_codes, code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn eligible_legacy_duplicate_preview_and_apply_only_roll_back_journal() {
        let temp = tempfile::tempdir().unwrap();
        let pool = crate::test_support::mem_pool().await;
        let canonical = temp.path().join("central").join("demo");
        crate::test_support::seed_central_skill(&pool, &canonical, "demo", "Demo").await;
        let original_bytes = std::fs::read(canonical.join("SKILL.md")).unwrap();
        let fingerprint = fingerprint_local_path(&canonical).await.unwrap();
        let missing = temp.path().join("removed-copy");
        let operation_id = "legacy-duplicate-reconcile";
        let managed = ManagedPath {
            original: canonical.to_string_lossy().into_owned(),
            backup: temp
                .path()
                .join("canonical-backup")
                .to_string_lossy()
                .into_owned(),
            marker: temp
                .path()
                .join("canonical-marker")
                .to_string_lossy()
                .into_owned(),
            expected_present: true,
            fingerprint: fingerprint.clone(),
        };
        let manifest = OperationManifest::Delete(DeleteManifest {
            version: super::super::MANIFEST_VERSION,
            operation_id: operation_id.to_string(),
            paths: vec![
                managed.clone(),
                managed,
                ManagedPath {
                    original: missing.to_string_lossy().into_owned(),
                    backup: temp
                        .path()
                        .join("missing-backup")
                        .to_string_lossy()
                        .into_owned(),
                    marker: temp
                        .path()
                        .join("missing-marker")
                        .to_string_lossy()
                        .into_owned(),
                    expected_present: true,
                    fingerprint: Some("historical".to_string()),
                },
            ],
        });
        let manifest_json = serde_json::to_string(&manifest).unwrap();
        db::insert_fs_db_operation(
            &pool,
            db::NewFsDbOperation {
                id: operation_id,
                batch_id: None,
                target_id: "local",
                target_kind: "local",
                operation_kind: "central_delete",
                skill_id: "demo",
                manifest_version: super::super::MANIFEST_VERSION,
                manifest_json: &manifest_json,
                old_fingerprint: fingerprint.as_deref(),
                new_fingerprint: None,
            },
        )
        .await
        .unwrap();

        let preview =
            preview_prepared_delete_reconciliation(&pool, &ActiveTarget::Local, operation_id)
                .await
                .unwrap();
        assert!(preview.eligible);
        assert_eq!(preview.duplicate_path_count, 1);
        assert_eq!(preview.missing_unowned_path_count, 1);

        let pending = reconcile_prepared_delete(&pool, &ActiveTarget::Local, operation_id)
            .await
            .unwrap();
        assert!(pending.is_empty());
        assert_eq!(
            db::get_fs_db_operation(&pool, operation_id)
                .await
                .unwrap()
                .unwrap()
                .phase,
            "rolled_back"
        );
        assert_eq!(
            std::fs::read(canonical.join("SKILL.md")).unwrap(),
            original_bytes
        );
        assert!(db::get_skill_by_id(&pool, "demo").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn inconsistent_duplicate_evidence_blocks_reconciliation() {
        let temp = tempfile::tempdir().unwrap();
        let pool = crate::test_support::mem_pool().await;
        let canonical = temp.path().join("central").join("demo");
        crate::test_support::seed_central_skill(&pool, &canonical, "demo", "Demo").await;
        let fingerprint = fingerprint_local_path(&canonical).await.unwrap();
        let operation_id = "legacy-inconsistent-duplicate";
        let mut duplicate = ManagedPath {
            original: canonical.to_string_lossy().into_owned(),
            backup: temp.path().join("backup").to_string_lossy().into_owned(),
            marker: temp.path().join("marker").to_string_lossy().into_owned(),
            expected_present: true,
            fingerprint: fingerprint.clone(),
        };
        let original = duplicate.clone();
        duplicate.marker = temp
            .path()
            .join("other-marker")
            .to_string_lossy()
            .into_owned();
        let manifest = OperationManifest::Delete(DeleteManifest {
            version: super::super::MANIFEST_VERSION,
            operation_id: operation_id.to_string(),
            paths: vec![original, duplicate],
        });
        let manifest_json = serde_json::to_string(&manifest).unwrap();
        db::insert_fs_db_operation(
            &pool,
            db::NewFsDbOperation {
                id: operation_id,
                batch_id: None,
                target_id: "local",
                target_kind: "local",
                operation_kind: "central_delete",
                skill_id: "demo",
                manifest_version: super::super::MANIFEST_VERSION,
                manifest_json: &manifest_json,
                old_fingerprint: fingerprint.as_deref(),
                new_fingerprint: None,
            },
        )
        .await
        .unwrap();

        let preview =
            preview_prepared_delete_reconciliation(&pool, &ActiveTarget::Local, operation_id)
                .await
                .unwrap();
        assert!(!preview.eligible);
        assert!(preview
            .blocker_codes
            .contains(&BLOCK_INCONSISTENT_DUPLICATE.to_string()));
        assert_eq!(
            db::get_fs_db_operation(&pool, operation_id)
                .await
                .unwrap()
                .unwrap()
                .phase,
            "prepared"
        );
    }
}
