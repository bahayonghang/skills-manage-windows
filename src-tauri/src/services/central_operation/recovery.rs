use std::str::FromStr;

use crate::db::{self, DbPool, FsDbOperationRow};
use crate::services::central_mutation::{
    acquire_target_mutation_guard, DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};
use crate::targets::{connect_remote_target, ActiveTarget};

use super::fs::{
    finalize_delete_local, finalize_delete_remote, restore_delete_local, restore_delete_remote,
};
use super::{
    CentralOperationError, OperationManifest, OperationPhase, PendingOperationSummary,
    MANIFEST_VERSION,
};

pub async fn list_pending_operations(
    pool: &DbPool,
    target: &ActiveTarget,
) -> Result<Vec<PendingOperationSummary>, CentralOperationError> {
    db::list_pending_fs_db_operations(pool, target.id())
        .await?
        .into_iter()
        .map(|row| {
            validate_row_identity(&row, target)?;
            decode_manifest(&row)?;
            Ok(summary(row))
        })
        .collect()
}

pub async fn recover_pending_operations(
    pool: &DbPool,
    target: &ActiveTarget,
) -> Result<Vec<PendingOperationSummary>, CentralOperationError> {
    let _guard = acquire_target_mutation_guard(
        target,
        "recover Central operations",
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
    .map_err(|error| CentralOperationError::InvalidManifest(error.to_string()))?;
    recover_pending_operations_under_guard(pool, target).await?;
    list_pending_operations(pool, target).await
}

pub(crate) async fn recover_pending_operations_under_guard(
    pool: &DbPool,
    target: &ActiveTarget,
) -> Result<(), CentralOperationError> {
    let rows = db::list_pending_fs_db_operations(pool, target.id()).await?;
    if rows.is_empty() {
        return Ok(());
    }
    if rows
        .iter()
        .any(|row| row.operation_kind == "central_update")
    {
        let fs = crate::services::central_updates::CentralFs::from_active_target(target.clone())
            .await
            .map_err(|_| CentralOperationError::Remote {
                code: "recovery_update_target",
            })?;
        return crate::services::central_updates::recover_pending_update_operations(pool, &fs)
            .await
            .map_err(|_| CentralOperationError::RecoveryCollision {
                code: "update_recovery_failed",
            });
    }
    let remote =
        if target.is_remote_like() {
            Some(connect_remote_target(target).await.map_err(|_| {
                CentralOperationError::Remote {
                    code: "recovery_target_offline",
                }
            })?)
        } else {
            None
        };

    for row in rows {
        if let Err(error) = recover_row(pool, target, remote.as_ref(), &row).await {
            db::record_fs_db_operation_error(
                pool,
                &row.id,
                error.code(),
                &error.redacted_message(),
            )
            .await?;
            return Err(error);
        }
    }
    Ok(())
}

pub(crate) async fn recover_pending_delete_operations_with_transport(
    pool: &DbPool,
    target_id: &str,
    target_kind: &str,
    remote: Option<&crate::targets::ConnectedRemoteTarget>,
) -> Result<(), CentralOperationError> {
    for row in db::list_pending_fs_db_operations(pool, target_id).await? {
        if row.target_kind != target_kind {
            return Err(CentralOperationError::InvalidManifest(
                "operation target identity mismatch".to_string(),
            ));
        }
        if row.operation_kind != "central_delete" {
            continue;
        }
        let phase =
            OperationPhase::from_str(&row.phase).map_err(CentralOperationError::InvalidManifest)?;
        let OperationManifest::Delete(manifest) = decode_manifest(&row)? else {
            return Err(CentralOperationError::InvalidManifest(
                "delete row contains an update manifest".to_string(),
            ));
        };
        let result = match phase {
            OperationPhase::Prepared | OperationPhase::FsStaged => if let Some(remote) = remote {
                restore_delete_remote(remote, &manifest).await
            } else {
                restore_delete_local(&manifest).await
            }
            .map(|()| OperationPhase::RolledBack),
            OperationPhase::DbCommitted => if let Some(remote) = remote {
                finalize_delete_remote(remote, &manifest).await
            } else {
                finalize_delete_local(&manifest).await
            }
            .map(|()| OperationPhase::Completed),
            OperationPhase::Completed | OperationPhase::RolledBack => continue,
            _ => Err(CentralOperationError::InvalidManifest(format!(
                "phase {} is invalid for delete recovery",
                row.phase
            ))),
        };
        match result {
            Ok(next) => {
                db::transition_fs_db_operation(pool, &row.id, &row.phase, next.as_str()).await?
            }
            Err(error) => {
                db::record_fs_db_operation_error(
                    pool,
                    &row.id,
                    error.code(),
                    &error.redacted_message(),
                )
                .await?;
                return Err(error);
            }
        }
    }
    Ok(())
}

pub async fn retry_operation(
    pool: &DbPool,
    target: &ActiveTarget,
    operation_id: &str,
) -> Result<Vec<PendingOperationSummary>, CentralOperationError> {
    let row = db::get_fs_db_operation(pool, operation_id)
        .await?
        .ok_or_else(|| CentralOperationError::InvalidManifest("operation not found".to_string()))?;
    validate_row_identity(&row, target)?;
    if OperationPhase::from_str(&row.phase)
        .map_err(CentralOperationError::InvalidManifest)?
        .is_terminal()
    {
        return list_pending_operations(pool, target).await;
    }
    let _guard = acquire_target_mutation_guard(
        target,
        "retry Central operation",
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
    .map_err(|error| CentralOperationError::InvalidManifest(error.to_string()))?;
    if row.operation_kind == "central_update" {
        let fs = crate::services::central_updates::CentralFs::from_active_target(target.clone())
            .await
            .map_err(|_| CentralOperationError::Remote {
                code: "recovery_update_target",
            })?;
        if let Err(_error) =
            crate::services::central_updates::recover_pending_update_operations(pool, &fs).await
        {
            let recovery_error = CentralOperationError::RecoveryCollision {
                code: "update_recovery_failed",
            };
            db::record_fs_db_operation_error(
                pool,
                &row.id,
                recovery_error.code(),
                &recovery_error.redacted_message(),
            )
            .await?;
            tracing::warn!(
                code = recovery_error.code(),
                operation_id = row.id,
                "Central update recovery failed"
            );
            return Err(recovery_error);
        }
        return list_pending_operations(pool, target).await;
    }
    let remote =
        if target.is_remote_like() {
            Some(connect_remote_target(target).await.map_err(|_| {
                CentralOperationError::Remote {
                    code: "recovery_target_offline",
                }
            })?)
        } else {
            None
        };
    if let Err(error) = recover_row(pool, target, remote.as_ref(), &row).await {
        db::record_fs_db_operation_error(pool, &row.id, error.code(), &error.redacted_message())
            .await?;
        return Err(error);
    }
    list_pending_operations(pool, target).await
}

async fn recover_row(
    pool: &DbPool,
    target: &ActiveTarget,
    remote: Option<&crate::targets::ConnectedRemoteTarget>,
    row: &FsDbOperationRow,
) -> Result<(), CentralOperationError> {
    validate_row_identity(row, target)?;
    let phase =
        OperationPhase::from_str(&row.phase).map_err(CentralOperationError::InvalidManifest)?;
    let manifest = decode_manifest(row)?;
    match (phase, manifest) {
        (OperationPhase::Prepared | OperationPhase::FsStaged, OperationManifest::Delete(value)) => {
            if let Some(remote) = remote {
                restore_delete_remote(remote, &value).await?;
            } else {
                restore_delete_local(&value).await?;
            }
            db::transition_fs_db_operation(
                pool,
                &row.id,
                phase.as_str(),
                OperationPhase::RolledBack.as_str(),
            )
            .await?;
        }
        (OperationPhase::DbCommitted, OperationManifest::Delete(value)) => {
            if let Some(remote) = remote {
                finalize_delete_remote(remote, &value).await?;
            } else {
                finalize_delete_local(&value).await?;
            }
            db::transition_fs_db_operation(
                pool,
                &row.id,
                phase.as_str(),
                OperationPhase::Completed.as_str(),
            )
            .await?;
        }
        (OperationPhase::Completed | OperationPhase::RolledBack, _) => {}
        (_, OperationManifest::Update(_)) => {
            return Err(CentralOperationError::InvalidManifest(
                "update recovery is not available".to_string(),
            ));
        }
        _ => {
            return Err(CentralOperationError::InvalidManifest(format!(
                "phase {} is invalid for delete recovery",
                row.phase
            )));
        }
    }
    Ok(())
}

fn decode_manifest(row: &FsDbOperationRow) -> Result<OperationManifest, CentralOperationError> {
    if row.manifest_version != MANIFEST_VERSION {
        return Err(CentralOperationError::InvalidManifest(format!(
            "unsupported manifest version {}",
            row.manifest_version
        )));
    }
    let manifest: OperationManifest = serde_json::from_str(&row.manifest_json)
        .map_err(|error| CentralOperationError::InvalidManifest(error.to_string()))?;
    manifest
        .validate(&row.id)
        .map_err(CentralOperationError::InvalidManifest)?;
    Ok(manifest)
}

fn validate_row_identity(
    row: &FsDbOperationRow,
    target: &ActiveTarget,
) -> Result<(), CentralOperationError> {
    let target_kind = match target {
        ActiveTarget::Local => "local",
        ActiveTarget::Ssh(_) => "ssh",
        ActiveTarget::Wsl(_) => "wsl",
    };
    if row.target_id != target.id() || row.target_kind != target_kind {
        return Err(CentralOperationError::InvalidManifest(
            "operation target identity mismatch".to_string(),
        ));
    }
    Ok(())
}

fn summary(row: FsDbOperationRow) -> PendingOperationSummary {
    PendingOperationSummary {
        operation_id: row.id,
        target_id: row.target_id,
        target_kind: row.target_kind,
        operation_kind: row.operation_kind,
        skill_id: row.skill_id,
        phase: row.phase,
        error_code: row.last_error_code,
        error_message: row.last_error_message,
        updated_at: row.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::central_operation::{DeleteManifest, ManagedPath, OperationKind};

    #[tokio::test]
    async fn pending_summary_never_exposes_manifest_paths() {
        let temp = tempfile::tempdir().unwrap();
        let pool = db::open_database(&temp.path().join("db.sqlite"))
            .await
            .unwrap();
        let operation_id = "op-redaction";
        let manifest = OperationManifest::Delete(DeleteManifest {
            version: MANIFEST_VERSION,
            operation_id: operation_id.to_string(),
            paths: vec![ManagedPath {
                original: "C:/Users/private/secret-skill".to_string(),
                backup: "C:/Users/private/.backup".to_string(),
                marker: "C:/Users/private/.marker".to_string(),
                expected_present: false,
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
                operation_kind: OperationKind::CentralDelete.as_str(),
                skill_id: "secret-skill",
                manifest_version: MANIFEST_VERSION,
                manifest_json: &manifest_json,
                old_fingerprint: None,
                new_fingerprint: None,
            },
        )
        .await
        .unwrap();

        let summaries = list_pending_operations(&pool, &ActiveTarget::Local)
            .await
            .unwrap();
        let json = serde_json::to_string(&summaries).unwrap();
        assert!(!json.contains("Users/private"));
        assert!(!json.contains("manifest"));
        assert!(json.contains(operation_id));
        let export = db::export_operation_logs_json(&pool, db::OperationLogFilter::default())
            .await
            .unwrap();
        assert!(!export.contains("Users/private"));
        assert!(!export.contains("op-redaction"));
    }

    #[tokio::test]
    async fn remote_pending_inventory_is_cache_only_and_does_not_connect() {
        let temp = tempfile::tempdir().unwrap();
        let pool = db::open_database(&temp.path().join("db.sqlite"))
            .await
            .unwrap();
        let operation_id = "op-remote-inventory";
        let fingerprint = "a".repeat(64);
        let manifest = OperationManifest::Delete(DeleteManifest {
            version: MANIFEST_VERSION,
            operation_id: operation_id.to_string(),
            paths: vec![ManagedPath {
                original: "/home/tester/secret-skill".to_string(),
                backup: "/home/tester/.backup".to_string(),
                marker: "/home/tester/.marker".to_string(),
                expected_present: true,
                fingerprint: Some(fingerprint.clone()),
            }],
        });
        let json = serde_json::to_string(&manifest).unwrap();
        db::insert_fs_db_operation(
            &pool,
            db::NewFsDbOperation {
                id: operation_id,
                batch_id: None,
                target_id: "ssh-offline",
                target_kind: "ssh",
                operation_kind: OperationKind::CentralDelete.as_str(),
                skill_id: "secret-skill",
                manifest_version: MANIFEST_VERSION,
                manifest_json: &json,
                old_fingerprint: Some(&fingerprint),
                new_fingerprint: None,
            },
        )
        .await
        .unwrap();
        let target = ActiveTarget::Ssh(Box::new(crate::targets::RemoteTargetConfig {
            id: "ssh-offline".to_string(),
            label: "Offline".to_string(),
            host: "does-not-resolve.invalid".to_string(),
            username: "tester".to_string(),
            port: 22,
            auth_method: crate::targets::SshAuthMethod::Key,
            key_path: "~/.ssh/id_ed25519".to_string(),
            credential_key: None,
            protected_password: None,
            password: None,
            remote_home: "/home/tester".to_string(),
            remote_os: "linux".to_string(),
            symlink_enabled: true,
        }));
        let summaries = list_pending_operations(&pool, &target).await.unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].operation_id, operation_id);
    }

    #[tokio::test]
    async fn startup_local_recovery_inventory_rolls_back_prepared_rows() {
        let temp = tempfile::tempdir().unwrap();
        let pool = db::open_database(&temp.path().join("db.sqlite"))
            .await
            .unwrap();
        let operation_id = "op-startup-local";
        let path = temp.path().join("already-missing");
        let manifest = OperationManifest::Delete(DeleteManifest {
            version: MANIFEST_VERSION,
            operation_id: operation_id.to_string(),
            paths: vec![ManagedPath {
                original: path.to_string_lossy().into_owned(),
                backup: temp.path().join("backup").to_string_lossy().into_owned(),
                marker: temp.path().join("marker").to_string_lossy().into_owned(),
                expected_present: false,
                fingerprint: None,
            }],
        });
        let json = serde_json::to_string(&manifest).unwrap();
        db::insert_fs_db_operation(
            &pool,
            db::NewFsDbOperation {
                id: operation_id,
                batch_id: None,
                target_id: "local",
                target_kind: "local",
                operation_kind: OperationKind::CentralDelete.as_str(),
                skill_id: "missing-skill",
                manifest_version: MANIFEST_VERSION,
                manifest_json: &json,
                old_fingerprint: None,
                new_fingerprint: None,
            },
        )
        .await
        .unwrap();
        let pending = recover_pending_operations(&pool, &ActiveTarget::Local)
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
    }
}
