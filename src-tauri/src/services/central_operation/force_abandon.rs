use crate::db::{self, DbPool, FsDbOperationRow};
use crate::targets::{ActiveTarget, ConnectedRemoteTarget};

use super::reconcile::{
    collapse_managed_paths, decode_delete_manifest, push_unique_code, reconciliation_path_is_valid,
    target_kind, BLOCK_ARTIFACT_REMAINING, BLOCK_INCONSISTENT_DUPLICATE, BLOCK_INVALID_MANIFEST,
    BLOCK_TARGET_MISMATCH, BLOCK_UNSUPPORTED_KIND, BLOCK_UNSUPPORTED_PHASE,
};
use super::{CentralOperationError, PendingDeleteRecoveryPreview};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceAbandonDecision {
    NoPending,
    Abandoned,
    Blocked,
}

pub async fn preview_pending_delete_recovery(
    pool: &DbPool,
    target: &ActiveTarget,
    skill_id: &str,
    remote: Option<&ConnectedRemoteTarget>,
) -> Result<Option<PendingDeleteRecoveryPreview>, CentralOperationError> {
    let Some(row) = pending_row_for_skill(pool, target, skill_id).await? else {
        return Ok(None);
    };
    Ok(Some(inspect_force_abandon(target, &row, remote).await))
}

pub async fn force_abandon_prepared_delete_under_guard(
    pool: &DbPool,
    target: &ActiveTarget,
    skill_id: &str,
    remote: Option<&ConnectedRemoteTarget>,
) -> Result<ForceAbandonDecision, CentralOperationError> {
    let Some(preview) = preview_pending_delete_recovery(pool, target, skill_id, remote).await?
    else {
        return Ok(ForceAbandonDecision::NoPending);
    };
    if !preview.force_delete_eligible {
        return Ok(ForceAbandonDecision::Blocked);
    }
    db::transition_fs_db_operation(pool, &preview.operation_id, "prepared", "rolled_back").await?;
    Ok(ForceAbandonDecision::Abandoned)
}

async fn pending_row_for_skill(
    pool: &DbPool,
    target: &ActiveTarget,
    skill_id: &str,
) -> Result<Option<FsDbOperationRow>, CentralOperationError> {
    let rows = db::list_pending_fs_db_operations(pool, target.id()).await?;
    Ok(rows.into_iter().find(|row| row.skill_id == skill_id))
}

async fn inspect_force_abandon(
    target: &ActiveTarget,
    row: &FsDbOperationRow,
    remote: Option<&ConnectedRemoteTarget>,
) -> PendingDeleteRecoveryPreview {
    let mut blocker_codes = Vec::new();
    if row.target_id != target.id() || row.target_kind != target_kind(target) {
        push_unique_code(&mut blocker_codes, BLOCK_TARGET_MISMATCH);
        return finish_preview(row, blocker_codes);
    }
    if row.operation_kind != "central_delete" {
        push_unique_code(&mut blocker_codes, BLOCK_UNSUPPORTED_KIND);
    }
    if row.phase != "prepared" {
        push_unique_code(&mut blocker_codes, BLOCK_UNSUPPORTED_PHASE);
    }

    let manifest = match decode_delete_manifest(row) {
        Ok(manifest) => manifest,
        Err(()) => {
            push_unique_code(&mut blocker_codes, BLOCK_INVALID_MANIFEST);
            return finish_preview(row, blocker_codes);
        }
    };
    if manifest
        .paths
        .iter()
        .any(|path| !reconciliation_path_is_valid(target, path))
    {
        push_unique_code(&mut blocker_codes, BLOCK_INVALID_MANIFEST);
        return finish_preview(row, blocker_codes);
    }

    let collapsed = collapse_managed_paths(target, manifest.paths);
    if collapsed.inconsistent {
        push_unique_code(&mut blocker_codes, BLOCK_INCONSISTENT_DUPLICATE);
    }

    match inspect_artifacts(target, remote, &collapsed.unique).await {
        Ok(true) => push_unique_code(&mut blocker_codes, BLOCK_ARTIFACT_REMAINING),
        Ok(false) => {}
        Err(()) => push_unique_code(&mut blocker_codes, BLOCK_ARTIFACT_REMAINING),
    }

    finish_preview(row, blocker_codes)
}

async fn inspect_artifacts(
    target: &ActiveTarget,
    remote: Option<&ConnectedRemoteTarget>,
    paths: &[super::ManagedPath],
) -> Result<bool, ()> {
    if target.is_remote_like() && remote.is_none() {
        return Err(());
    }
    for path in paths {
        let (backup_exists, marker_exists) = if let Some(remote) = remote {
            (
                remote.exists(&path.backup).await.map_err(|_| ())?,
                remote.exists(&path.marker).await.map_err(|_| ())?,
            )
        } else {
            (
                std::fs::symlink_metadata(&path.backup).is_ok(),
                std::fs::symlink_metadata(&path.marker).is_ok(),
            )
        };
        if backup_exists || marker_exists {
            return Ok(true);
        }
    }
    Ok(false)
}

fn finish_preview(
    row: &FsDbOperationRow,
    blocker_codes: Vec<String>,
) -> PendingDeleteRecoveryPreview {
    PendingDeleteRecoveryPreview {
        operation_id: row.id.clone(),
        operation_kind: row.operation_kind.clone(),
        phase: row.phase.clone(),
        error_code: qualify_error_code(row.last_error_code.as_deref()),
        force_delete_eligible: blocker_codes.is_empty(),
        blocker_codes,
    }
}

fn qualify_error_code(code: Option<&str>) -> Option<String> {
    code.map(|code| {
        if code.contains('.') {
            code.to_string()
        } else {
            format!("central_operation.{code}")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::super::fs::fingerprint_local_path;
    use super::*;
    use crate::services::central_operation::{
        DeleteManifest, ManagedPath, OperationManifest, MANIFEST_VERSION,
    };
    use std::path::Path;

    async fn insert_yao_meta_shaped_pending(
        pool: &DbPool,
        central_dir: &Path,
        agents_dir: &Path,
        skill_id: &str,
        fingerprint: Option<String>,
        operation_id: &str,
    ) {
        let agents_original = agents_dir.join(skill_id);
        let gemini_original = central_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(".gemini")
            .join("antigravity-cli")
            .join("skills")
            .join(skill_id);
        let claude_original = central_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(".claude")
            .join("skills")
            .join(skill_id);
        let zed_original = central_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(".config")
            .join("zed")
            .join("skills")
            .join(skill_id);
        let sibling = |original: &Path, suffix: &str| {
            original
                .parent()
                .unwrap()
                .join(format!(
                    "{}.{}",
                    original.file_name().unwrap().to_string_lossy(),
                    suffix
                ))
                .to_string_lossy()
                .into_owned()
        };
        let managed = |original: &Path| ManagedPath {
            original: original.to_string_lossy().into_owned(),
            backup: sibling(original, "backup"),
            marker: sibling(original, "marker"),
            expected_present: true,
            fingerprint: fingerprint.clone(),
        };
        let agents_path = managed(&agents_original);
        let mut paths = vec![
            agents_path.clone(),
            managed(&gemini_original),
            managed(&claude_original),
            managed(&zed_original),
            managed(central_dir),
        ];
        paths.extend(std::iter::repeat_n(agents_path, 9));
        let manifest = OperationManifest::Delete(DeleteManifest {
            version: MANIFEST_VERSION,
            operation_id: operation_id.to_string(),
            paths,
        });
        let manifest_json = serde_json::to_string(&manifest).unwrap();
        db::insert_fs_db_operation(
            pool,
            db::NewFsDbOperation {
                id: operation_id,
                batch_id: None,
                target_id: "local",
                target_kind: "local",
                operation_kind: "central_delete",
                skill_id,
                manifest_version: MANIFEST_VERSION,
                manifest_json: &manifest_json,
                old_fingerprint: fingerprint.as_deref(),
                new_fingerprint: None,
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn yao_meta_shape_is_force_eligible_with_fingerprint_drift() {
        let temp = tempfile::tempdir().unwrap();
        let pool = crate::test_support::mem_pool().await;
        let central = temp
            .path()
            .join(".skillsmanage")
            .join("skills")
            .join("yao-meta");
        crate::test_support::seed_central_skill(&pool, &central, "yao-meta", "Yao Meta").await;
        let agents = temp.path().join(".agents").join("skills");
        std::fs::create_dir_all(&agents).unwrap();
        let recorded = fingerprint_local_path(&central).await.unwrap();
        std::fs::write(central.join("SKILL.md"), "changed after journal").unwrap();
        insert_yao_meta_shaped_pending(
            &pool,
            &central,
            &agents,
            "yao-meta",
            recorded,
            "yao-meta-pending",
        )
        .await;
        sqlx::query(
            "UPDATE fs_db_operations SET last_error_code = 'delete_restore_collision' WHERE id = ?",
        )
        .bind("yao-meta-pending")
        .execute(&pool)
        .await
        .unwrap();

        let preview =
            preview_pending_delete_recovery(&pool, &ActiveTarget::Local, "yao-meta", None)
                .await
                .unwrap()
                .expect("pending recovery");
        assert!(preview.force_delete_eligible);
        assert!(preview.blocker_codes.is_empty());
        assert_eq!(
            preview.error_code.as_deref(),
            Some("central_operation.delete_restore_collision")
        );
        assert!(!preview
            .blocker_codes
            .iter()
            .any(|code| { code.contains("fingerprint") || code.contains("owned_path") }));
    }

    #[tokio::test]
    async fn remaining_backup_blocks_force_abandon() {
        let temp = tempfile::tempdir().unwrap();
        let pool = crate::test_support::mem_pool().await;
        let central = temp
            .path()
            .join(".skillsmanage")
            .join("skills")
            .join("demo");
        crate::test_support::seed_central_skill(&pool, &central, "demo", "Demo").await;
        let backup = central.parent().unwrap().join("demo.backup");
        std::fs::create_dir_all(&backup).unwrap();
        let operation_id = "backup-remaining";
        let manifest = OperationManifest::Delete(DeleteManifest {
            version: MANIFEST_VERSION,
            operation_id: operation_id.to_string(),
            paths: vec![ManagedPath {
                original: central.to_string_lossy().into_owned(),
                backup: backup.to_string_lossy().into_owned(),
                marker: central
                    .parent()
                    .unwrap()
                    .join("demo.marker")
                    .to_string_lossy()
                    .into_owned(),
                expected_present: true,
                fingerprint: None,
            }],
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
                manifest_version: MANIFEST_VERSION,
                manifest_json: &manifest_json,
                old_fingerprint: None,
                new_fingerprint: None,
            },
        )
        .await
        .unwrap();

        let preview = preview_pending_delete_recovery(&pool, &ActiveTarget::Local, "demo", None)
            .await
            .unwrap()
            .expect("pending recovery");
        assert!(!preview.force_delete_eligible);
        assert!(preview
            .blocker_codes
            .contains(&BLOCK_ARTIFACT_REMAINING.to_string()));
        assert!(backup.exists());
        assert_eq!(
            force_abandon_prepared_delete_under_guard(&pool, &ActiveTarget::Local, "demo", None)
                .await
                .unwrap(),
            ForceAbandonDecision::Blocked
        );
        assert_eq!(
            db::get_fs_db_operation(&pool, operation_id)
                .await
                .unwrap()
                .unwrap()
                .phase,
            "prepared"
        );
    }

    #[tokio::test]
    async fn non_prepared_phase_blocks_force_abandon() {
        let temp = tempfile::tempdir().unwrap();
        let pool = crate::test_support::mem_pool().await;
        let central = temp
            .path()
            .join(".skillsmanage")
            .join("skills")
            .join("demo");
        crate::test_support::seed_central_skill(&pool, &central, "demo", "Demo").await;
        let operation_id = "fs-staged-pending";
        let manifest = OperationManifest::Delete(DeleteManifest {
            version: MANIFEST_VERSION,
            operation_id: operation_id.to_string(),
            paths: vec![ManagedPath {
                original: central.to_string_lossy().into_owned(),
                backup: central
                    .parent()
                    .unwrap()
                    .join("demo.backup")
                    .to_string_lossy()
                    .into_owned(),
                marker: central
                    .parent()
                    .unwrap()
                    .join("demo.marker")
                    .to_string_lossy()
                    .into_owned(),
                expected_present: true,
                fingerprint: None,
            }],
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
                manifest_version: MANIFEST_VERSION,
                manifest_json: &manifest_json,
                old_fingerprint: None,
                new_fingerprint: None,
            },
        )
        .await
        .unwrap();
        sqlx::query("UPDATE fs_db_operations SET phase = 'fs_staged' WHERE id = ?")
            .bind(operation_id)
            .execute(&pool)
            .await
            .unwrap();

        let preview = preview_pending_delete_recovery(&pool, &ActiveTarget::Local, "demo", None)
            .await
            .unwrap()
            .expect("pending recovery");
        assert!(!preview.force_delete_eligible);
        assert!(preview
            .blocker_codes
            .contains(&BLOCK_UNSUPPORTED_PHASE.to_string()));
        assert_eq!(
            force_abandon_prepared_delete_under_guard(&pool, &ActiveTarget::Local, "demo", None)
                .await
                .unwrap(),
            ForceAbandonDecision::Blocked
        );
    }

    #[tokio::test]
    async fn no_pending_row_is_not_blocked() {
        let pool = crate::test_support::mem_pool().await;
        assert!(
            preview_pending_delete_recovery(&pool, &ActiveTarget::Local, "missing", None)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            force_abandon_prepared_delete_under_guard(&pool, &ActiveTarget::Local, "missing", None)
                .await
                .unwrap(),
            ForceAbandonDecision::NoPending
        );
    }
}
