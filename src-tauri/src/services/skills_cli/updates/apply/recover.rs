//! Recovery settlement for journaled Skills CLI canonical refresh.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use chrono::Utc;

use crate::db::DbPool;
use crate::db::{
    self, list_pending_update_operations, transition_update_operation,
    transition_update_operation_in_transaction, upsert_update_state_in_transaction,
    SkillsCliUpdateStateRow as PersistedSkill,
};
use crate::fs_util::run_blocking_fs_with;

use super::super::super::{check_cancel, SkillsCliError};
use super::super::digest::{copy_dir_recursive, digest_skill_directory};
use super::super::status::SkillsCliPersistedUpdateStatus;
use super::super::{map_db_error, SkillsCliApplyRecoveryResult};
use super::{
    ApplyManifest, PHASE_BACKUPS, PHASE_CLEANUP, PHASE_CLI_STARTED, PHASE_CLI_SUCCEEDED,
    PHASE_COMPLETED, PHASE_DB_COMMITTED, PHASE_PREPARED, PHASE_RECOVERY, PHASE_ROLLED_BACK,
};

pub(super) async fn recover_pending_at(
    pool: &DbPool,
    canonical_root: &Path,
    lock_path: &Path,
    recovery_root: &Path,
    cancel: Option<&AtomicBool>,
) -> Result<(), SkillsCliError> {
    let pending = list_pending_update_operations(pool)
        .await
        .map_err(map_db_error)?;
    for row in pending {
        recover_one(
            pool,
            &row.id,
            canonical_root,
            lock_path,
            recovery_root,
            cancel,
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn recover_one(
    pool: &DbPool,
    operation_id: &str,
    canonical_root: &Path,
    _lock_path: &Path,
    recovery_root: &Path,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliApplyRecoveryResult, SkillsCliError> {
    check_cancel(cancel)?;
    let row = db::get_update_operation(pool, operation_id)
        .await
        .map_err(map_db_error)?
        .ok_or(SkillsCliError::UpdateRecoveryRequired)?;
    let manifest: ApplyManifest =
        serde_json::from_str(&row.manifest_json).map_err(|_| SkillsCliError::UpdateIntegrity)?;
    match row.phase.as_str() {
        PHASE_PREPARED => {
            transition_update_operation(
                pool,
                operation_id,
                PHASE_PREPARED,
                PHASE_ROLLED_BACK,
                None,
            )
            .await
            .map_err(map_db_error)?;
            Ok(SkillsCliApplyRecoveryResult {
                operation_id: operation_id.to_string(),
                phase: PHASE_ROLLED_BACK.to_string(),
            })
        }
        PHASE_BACKUPS | PHASE_CLI_STARTED => {
            settle_old_or_new(pool, &row.phase, &manifest, canonical_root, recovery_root).await
        }
        PHASE_CLI_SUCCEEDED => {
            settle_new_or_restore(pool, &manifest, canonical_root, recovery_root).await
        }
        PHASE_DB_COMMITTED | PHASE_CLEANUP => {
            let recovery_root = recovery_root.to_path_buf();
            let cleanup_id = operation_id.to_string();
            let _ = run_blocking_fs_with(
                "skills-cli-update-recovery-cleanup",
                move || {
                    let dir = recovery_root.join(&cleanup_id);
                    if dir.exists() {
                        std::fs::remove_dir_all(&dir).map_err(|source| SkillsCliError::Io {
                            context: "cleanup update backup",
                            source,
                        })?;
                    }
                    Ok(())
                },
                SkillsCliError::task_join,
            )
            .await;
            transition_update_operation(
                pool,
                operation_id,
                row.phase.as_str(),
                PHASE_COMPLETED,
                None,
            )
            .await
            .map_err(map_db_error)?;
            Ok(SkillsCliApplyRecoveryResult {
                operation_id: operation_id.to_string(),
                phase: PHASE_COMPLETED.to_string(),
            })
        }
        PHASE_RECOVERY => Err(SkillsCliError::UpdateRecoveryRequired),
        _ => Ok(SkillsCliApplyRecoveryResult {
            operation_id: operation_id.to_string(),
            phase: row.phase,
        }),
    }
}

async fn settle_old_or_new(
    pool: &DbPool,
    phase: &str,
    manifest: &ApplyManifest,
    canonical_root: &Path,
    recovery_root: &Path,
) -> Result<SkillsCliApplyRecoveryResult, SkillsCliError> {
    let canonical_root = canonical_root.to_path_buf();
    let recovery_root_buf = recovery_root.to_path_buf();
    let recovery_for_finalize = recovery_root_buf.clone();
    let manifest_clone = manifest.clone();
    let outcome = run_blocking_fs_with(
        "skills-cli-update-recover-observe",
        move || observe_or_restore_old(&canonical_root, &recovery_root_buf, &manifest_clone),
        SkillsCliError::task_join,
    )
    .await?;
    match outcome {
        SettleFsOutcome::AllNew => {
            finalize_new_baseline(pool, phase, manifest, &recovery_for_finalize).await
        }
        SettleFsOutcome::RestoredOld => {
            transition_update_operation(
                pool,
                &manifest.operation_id,
                phase,
                PHASE_ROLLED_BACK,
                None,
            )
            .await
            .map_err(map_db_error)?;
            Ok(SkillsCliApplyRecoveryResult {
                operation_id: manifest.operation_id.clone(),
                phase: PHASE_ROLLED_BACK.to_string(),
            })
        }
        SettleFsOutcome::Mixed => {
            transition_update_operation(
                pool,
                &manifest.operation_id,
                phase,
                PHASE_RECOVERY,
                Some("skills_cli.update_recovery_required"),
            )
            .await
            .map_err(map_db_error)?;
            Err(SkillsCliError::UpdateRecoveryRequired)
        }
    }
}

async fn settle_new_or_restore(
    pool: &DbPool,
    manifest: &ApplyManifest,
    canonical_root: &Path,
    recovery_root: &Path,
) -> Result<SkillsCliApplyRecoveryResult, SkillsCliError> {
    let canonical_root = canonical_root.to_path_buf();
    let recovery_root_buf = recovery_root.to_path_buf();
    let manifest_clone = manifest.clone();
    let outcome = run_blocking_fs_with(
        "skills-cli-update-recover-new",
        move || observe_or_restore_new(&canonical_root, &recovery_root_buf, &manifest_clone),
        SkillsCliError::task_join,
    )
    .await?;
    match outcome {
        SettleFsOutcome::AllNew => {
            finalize_new_baseline(pool, PHASE_CLI_SUCCEEDED, manifest, recovery_root).await
        }
        SettleFsOutcome::RestoredOld => {
            transition_update_operation(
                pool,
                &manifest.operation_id,
                PHASE_CLI_SUCCEEDED,
                PHASE_ROLLED_BACK,
                None,
            )
            .await
            .map_err(map_db_error)?;
            Ok(SkillsCliApplyRecoveryResult {
                operation_id: manifest.operation_id.clone(),
                phase: PHASE_ROLLED_BACK.to_string(),
            })
        }
        SettleFsOutcome::Mixed => {
            transition_update_operation(
                pool,
                &manifest.operation_id,
                PHASE_CLI_SUCCEEDED,
                PHASE_RECOVERY,
                Some("skills_cli.update_recovery_required"),
            )
            .await
            .map_err(map_db_error)?;
            Err(SkillsCliError::UpdateRecoveryRequired)
        }
    }
}

async fn finalize_new_baseline(
    pool: &DbPool,
    phase: &str,
    manifest: &ApplyManifest,
    recovery_root: &Path,
) -> Result<SkillsCliApplyRecoveryResult, SkillsCliError> {
    let now = Utc::now().to_rfc3339();
    let mut transaction = pool.begin().await.map_err(map_db_error)?;
    for item in &manifest.selections {
        let mut row = db::get_update_state(pool, &item.skill_name)
            .await
            .map_err(map_db_error)?
            .unwrap_or(PersistedSkill {
                skill_name: item.skill_name.clone(),
                repository_key: Some(manifest.repository_key.clone()),
                normalized_source: None,
                skill_path: Some(item.skill_path.clone()),
                installed_revision_sha: None,
                installed_upstream_digest: None,
                installed_local_digest: None,
                installed_at: None,
                observed_revision_sha: None,
                observed_upstream_digest: None,
                observed_at: None,
                pending_revision_sha: None,
                pending_upstream_digest: None,
                pending_detected_at: None,
                status: SkillsCliPersistedUpdateStatus::Current.as_str().to_string(),
                last_error_code: None,
                is_stale: 0,
                updated_at: now.clone(),
            });
        row.installed_revision_sha = Some(manifest.expected_pending_revision.clone());
        row.installed_upstream_digest = Some(item.expected_pending_digest.clone());
        row.installed_local_digest = Some(item.expected_pending_digest.clone());
        row.installed_at = Some(now.clone());
        row.status = SkillsCliPersistedUpdateStatus::Current.as_str().to_string();
        row.updated_at = now.clone();
        upsert_update_state_in_transaction(&mut transaction, &row, true, true)
            .await
            .map_err(map_db_error)?;
    }
    transition_update_operation_in_transaction(
        &mut transaction,
        &manifest.operation_id,
        phase,
        PHASE_COMPLETED,
        None,
    )
    .await
    .map_err(map_db_error)?;
    transaction.commit().await.map_err(map_db_error)?;
    let recovery_root = recovery_root.to_path_buf();
    let cleanup_id = manifest.operation_id.clone();
    let _ = run_blocking_fs_with(
        "skills-cli-update-recover-cleanup",
        move || {
            let dir = recovery_root.join(&cleanup_id);
            if dir.exists() {
                std::fs::remove_dir_all(&dir).map_err(|source| SkillsCliError::Io {
                    context: "cleanup update backup",
                    source,
                })?;
            }
            Ok(())
        },
        SkillsCliError::task_join,
    )
    .await;
    Ok(SkillsCliApplyRecoveryResult {
        operation_id: manifest.operation_id.clone(),
        phase: PHASE_COMPLETED.to_string(),
    })
}

fn restore_backups(
    canonical_root: &Path,
    recovery_root: &Path,
    manifest: &ApplyManifest,
) -> Result<(), SkillsCliError> {
    for item in &manifest.selections {
        let dest = canonical_root.join(&item.skill_name);
        let source = recovery_root.join(&item.backup_relative);
        if source.is_dir() {
            copy_dir_recursive(&source, &dest)?;
        }
    }
    Ok(())
}

enum SettleFsOutcome {
    AllNew,
    RestoredOld,
    Mixed,
}

fn observe_or_restore_old(
    canonical_root: &Path,
    recovery_root: &Path,
    manifest: &ApplyManifest,
) -> Result<SettleFsOutcome, SkillsCliError> {
    let mut all_old = true;
    let mut all_new = true;
    for item in &manifest.selections {
        let canonical = canonical_root.join(&item.skill_name);
        let current = if canonical.is_dir() {
            Some(digest_skill_directory(&canonical)?)
        } else {
            None
        };
        let backup = recovery_root.join(&item.backup_relative);
        let backup_digest = if backup.is_dir() {
            Some(digest_skill_directory(&backup)?)
        } else {
            None
        };
        if current.as_deref() != backup_digest.as_deref() {
            all_old = false;
        }
        if current.as_deref() != Some(item.expected_pending_digest.as_str()) {
            all_new = false;
        }
    }
    if all_new && !all_old {
        return Ok(SettleFsOutcome::AllNew);
    }
    if all_old {
        restore_backups(canonical_root, recovery_root, manifest)?;
        return Ok(SettleFsOutcome::RestoredOld);
    }
    Ok(SettleFsOutcome::Mixed)
}

fn observe_or_restore_new(
    canonical_root: &Path,
    recovery_root: &Path,
    manifest: &ApplyManifest,
) -> Result<SettleFsOutcome, SkillsCliError> {
    let mut all_new = true;
    for item in &manifest.selections {
        let canonical = canonical_root.join(&item.skill_name);
        let current = if canonical.is_dir() {
            digest_skill_directory(&canonical)?
        } else {
            all_new = false;
            continue;
        };
        if current != item.expected_pending_digest {
            all_new = false;
        }
    }
    if all_new {
        return Ok(SettleFsOutcome::AllNew);
    }
    restore_backups(canonical_root, recovery_root, manifest)?;
    Ok(SettleFsOutcome::RestoredOld)
}
