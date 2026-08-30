//! Remote apply: local GitHub snapshot, tar subset over SSH stdin, journaled swap.
//!
//! Tokens stay in this process's HTTP headers. Do not write GitHub auth
//! material into remote helper files.

use std::sync::atomic::AtomicBool;

use chrono::Utc;
use uuid::Uuid;

use crate::db::DbPool;
use crate::db::{
    self, insert_update_operation, list_pending_update_operations, transition_update_operation,
    transition_update_operation_in_transaction, upsert_update_state_in_transaction,
    NewSkillsCliUpdateOperation, SkillsCliUpdateStateRow as PersistedSkill,
};
use crate::paths::remote_join;
use crate::services::central_mutation::{
    acquire_target_mutation_guard, DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};
use crate::services::github_import::{candidate_content_digest_from_snapshot, GitHubRepoSnapshot};
use crate::services::skills_cli::remote_scripts::{
    build_copy_trees_if_exist_script, build_swap_canonicals_script, remote_update_backup_dir,
    remote_update_staging_dir,
};
use crate::services::skills_cli::{
    check_cancel, list_global, map_guard_error, SkillsCliError, SkillsCliPlacementState,
    SkillsCliTransport,
};

use super::super::capability::argv_contains_forbidden_flags;
use super::super::detect::topology_blockers;
use super::super::github::SkillsCliUpdateGithub;
use super::super::status::SkillsCliPersistedUpdateStatus;
use super::super::{
    map_db_error, SkillsCliApplyRecoveryResult, SkillsCliApplyResult, SkillsCliApplyUpdateRequest,
    SkillsCliUpdateProgress, UpdateProgressEmitter, UPDATE_LOCK_OPERATION,
};
use super::{
    fail_if, fingerprint_lock_bytes, skill_files_from_snapshot, ApplyFault, ApplyManifest,
    ApplyManifestSelection, MANIFEST_VERSION, PHASE_BACKUPS, PHASE_CLEANUP, PHASE_CLI_STARTED,
    PHASE_CLI_SUCCEEDED, PHASE_COMPLETED, PHASE_DB_COMMITTED, PHASE_PREPARED, PHASE_RECOVERY,
    PHASE_ROLLED_BACK,
};

const LOCK_READ_LIMIT: u64 = 1_048_576;

pub(super) async fn apply_updates_remote(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    github: &dyn SkillsCliUpdateGithub,
    progress: &dyn UpdateProgressEmitter,
    request: &SkillsCliApplyUpdateRequest,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliApplyResult, SkillsCliError> {
    if request.selections.is_empty() {
        return Err(SkillsCliError::SelectionEmpty);
    }
    let preview = super::super::capability::apply_argv_preview(
        &request
            .selections
            .iter()
            .map(|item| item.skill_name.clone())
            .collect::<Vec<_>>(),
    );
    if argv_contains_forbidden_flags(&preview) {
        return Err(SkillsCliError::UpdateUnsupported);
    }
    check_cancel(cancel)?;

    let snapshot = list_global(tx, pool).await?;
    for selection in &request.selections {
        let Some(skill) = snapshot
            .skills
            .iter()
            .find(|skill| skill.name == selection.skill_name)
        else {
            return Err(SkillsCliError::SkillNotOwned);
        };
        if !topology_blockers(skill).is_empty() {
            return Err(SkillsCliError::UpdateTopologyConflict);
        }
        let state = db::get_update_state(pool, &selection.skill_name)
            .await
            .map_err(map_db_error)?
            .ok_or(SkillsCliError::UpdateStale)?;
        if state.repository_key.as_deref() != Some(request.repository_key.as_str()) {
            return Err(SkillsCliError::UpdateStale);
        }
        if state.is_stale != 0 {
            return Err(SkillsCliError::UpdateStale);
        }
        if state.pending_revision_sha.as_deref()
            != Some(selection.expected_pending_revision.as_str())
            || state.pending_upstream_digest.as_deref()
                != Some(selection.expected_pending_digest.as_str())
        {
            return Err(SkillsCliError::UpdateStale);
        }
        if selection.expected_installed_revision.as_deref()
            != state.installed_revision_sha.as_deref()
            || selection.expected_installed_local_digest.as_deref()
                != state.installed_local_digest.as_deref()
        {
            return Err(SkillsCliError::UpdateStale);
        }
    }

    let identity_parts: Vec<&str> = request.repository_key.split('@').collect();
    let (owner_repo, branch_unused) = identity_parts
        .split_first()
        .ok_or(SkillsCliError::UpdateStale)?;
    let _ = branch_unused;
    let mut owner_repo_parts = owner_repo.split('/');
    let owner = owner_repo_parts.next().ok_or(SkillsCliError::UpdateStale)?;
    let repo = owner_repo_parts.next().ok_or(SkillsCliError::UpdateStale)?;
    let sha = &request.selections[0].expected_pending_revision;
    if sha.len() != 40 || !sha.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(SkillsCliError::UpdateStale);
    }
    let pinned = github.snapshot_at_sha(owner, repo, sha).await?;
    for selection in &request.selections {
        let digest = candidate_content_digest_from_snapshot(&pinned, &selection.skill_path)
            .map_err(|_| SkillsCliError::UpdateStale)?;
        if digest != selection.expected_pending_digest {
            return Err(SkillsCliError::UpdateStale);
        }
    }

    let archive = pack_skill_subset_tar(&pinned, request)?;

    progress.emit_update_progress(&SkillsCliUpdateProgress {
        job_id: request.job_id.clone(),
        phase: "prepare".to_string(),
        repository_total: 1,
        repository_completed: 0,
        current_repository_key: Some(request.repository_key.clone()),
        selected_total: request.selections.len() as u32,
        selected_completed: 0,
        terminal_status: None,
    });

    let _guard = acquire_target_mutation_guard(
        &tx.mutation_target(),
        UPDATE_LOCK_OPERATION,
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
    .map_err(map_guard_error)?;

    recover_pending_remote(tx, pool, cancel).await?;

    let snapshot = list_global(tx, pool).await?;
    let mut digest_roots = Vec::new();
    for selection in &request.selections {
        let Some(skill) = snapshot
            .skills
            .iter()
            .find(|skill| skill.name == selection.skill_name)
        else {
            return Err(SkillsCliError::UpdateStale);
        };
        if skill.placements.iter().any(|placement| {
            matches!(
                placement.state,
                SkillsCliPlacementState::DirectCopy | SkillsCliPlacementState::Conflict
            )
        }) {
            return Err(SkillsCliError::UpdateTopologyConflict);
        }
        digest_roots.push(
            tx.paths()
                .join_child(tx.paths().canonical_root(), &selection.skill_name),
        );
    }
    let current_digests = tx.digest_remote_skill_dirs(&digest_roots).await?;
    for selection in &request.selections {
        let current = current_digests.get(
            &tx.paths()
                .join_child(tx.paths().canonical_root(), &selection.skill_name),
        );
        if selection.expected_installed_local_digest.as_deref() != current.map(String::as_str)
            && selection.expected_installed_local_digest.is_some()
        {
            return Err(SkillsCliError::UpdateStale);
        }
    }

    let operation_id = Uuid::new_v4().to_string();
    let lock_bytes = match tx
        .fs()
        .read_file_bounded(tx.paths().lock_path(), LOCK_READ_LIMIT)
        .await
    {
        Ok(bytes) => bytes,
        Err(error) => match &error {
            SkillsCliError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
                Vec::new()
            }
            _ => return Err(error),
        },
    };
    let lock_fingerprint = fingerprint_lock_bytes(&lock_bytes);
    let manifest = ApplyManifest {
        operation_id: operation_id.clone(),
        repository_key: request.repository_key.clone(),
        skill_names: request
            .selections
            .iter()
            .map(|item| item.skill_name.clone())
            .collect(),
        expected_pending_revision: sha.clone(),
        lock_fingerprint,
        selections: request
            .selections
            .iter()
            .map(|item| ApplyManifestSelection {
                skill_name: item.skill_name.clone(),
                skill_path: item.skill_path.clone(),
                expected_pending_digest: item.expected_pending_digest.clone(),
                expected_installed_local_digest: item.expected_installed_local_digest.clone(),
                backup_relative: format!("{operation_id}/{}", item.skill_name),
            })
            .collect(),
    };
    let manifest_json =
        serde_json::to_string(&manifest).map_err(|_| SkillsCliError::UpdateIntegrity)?;
    insert_update_operation(
        pool,
        NewSkillsCliUpdateOperation {
            id: &operation_id,
            phase: PHASE_PREPARED,
            manifest_version: MANIFEST_VERSION,
            manifest_json: &manifest_json,
        },
    )
    .await
    .map_err(map_db_error)?;
    fail_if(ApplyFault::Prepared)?;

    let canonical_root = tx.paths().canonical_root().to_string();
    let backup_root = remote_update_backup_dir(&canonical_root, &operation_id);
    let staging_root = remote_update_staging_dir(&canonical_root, &operation_id);
    let backup_pairs: Vec<(String, String)> = request
        .selections
        .iter()
        .map(|item| {
            (
                remote_join(&canonical_root, &item.skill_name),
                remote_join(&backup_root, &item.skill_name),
            )
        })
        .collect();
    tx.run_remote_script(&build_copy_trees_if_exist_script(&backup_pairs), true)
        .await?;
    transition_update_operation(pool, &operation_id, PHASE_PREPARED, PHASE_BACKUPS, None)
        .await
        .map_err(map_db_error)?;
    fail_if(ApplyFault::Backups)?;

    check_cancel(cancel)?;
    tx.extract_tar_stdin_cancellable(&staging_root, &archive, cancel)
        .await?;
    transition_update_operation(pool, &operation_id, PHASE_BACKUPS, PHASE_CLI_STARTED, None)
        .await
        .map_err(map_db_error)?;
    fail_if(ApplyFault::CliStarted)?;

    let swap_pairs: Vec<(String, String)> = request
        .selections
        .iter()
        .map(|item| {
            (
                remote_join(&staging_root, &item.skill_name),
                remote_join(&canonical_root, &item.skill_name),
            )
        })
        .collect();
    tx.run_remote_script(&build_swap_canonicals_script(&swap_pairs), true)
        .await?;
    transition_update_operation(
        pool,
        &operation_id,
        PHASE_CLI_STARTED,
        PHASE_CLI_SUCCEEDED,
        None,
    )
    .await
    .map_err(map_db_error)?;
    fail_if(ApplyFault::CliSucceeded)?;

    let verify_roots: Vec<String> = request
        .selections
        .iter()
        .map(|item| remote_join(&canonical_root, &item.skill_name))
        .collect();
    let verified = tx.digest_remote_skill_dirs(&verify_roots).await?;
    for selection in &request.selections {
        let path = remote_join(&canonical_root, &selection.skill_name);
        let digest = verified.get(&path).ok_or(SkillsCliError::UpdateIntegrity)?;
        if digest != &selection.expected_pending_digest {
            return Err(SkillsCliError::UpdateIntegrity);
        }
    }

    let now = Utc::now().to_rfc3339();
    let mut transaction = pool.begin().await.map_err(map_db_error)?;
    for selection in &request.selections {
        let mut row = db::get_update_state(pool, &selection.skill_name)
            .await
            .map_err(map_db_error)?
            .ok_or(SkillsCliError::UpdateMigration)?;
        row.installed_revision_sha = Some(selection.expected_pending_revision.clone());
        row.installed_upstream_digest = Some(selection.expected_pending_digest.clone());
        row.installed_local_digest = Some(selection.expected_pending_digest.clone());
        row.installed_at = Some(now.clone());
        row.status = SkillsCliPersistedUpdateStatus::Current.as_str().to_string();
        row.is_stale = 0;
        row.last_error_code = None;
        row.updated_at = now.clone();
        upsert_update_state_in_transaction(&mut transaction, &row, true, true)
            .await
            .map_err(map_db_error)?;
    }
    transition_update_operation_in_transaction(
        &mut transaction,
        &operation_id,
        PHASE_CLI_SUCCEEDED,
        PHASE_DB_COMMITTED,
        None,
    )
    .await
    .map_err(map_db_error)?;
    transaction.commit().await.map_err(map_db_error)?;
    progress.emit_update_progress(&SkillsCliUpdateProgress {
        job_id: request.job_id.clone(),
        phase: "completed".to_string(),
        repository_total: 1,
        repository_completed: 1,
        current_repository_key: Some(request.repository_key.clone()),
        selected_total: request.selections.len() as u32,
        selected_completed: request.selections.len() as u32,
        terminal_status: Some("completed".to_string()),
    });
    fail_if(ApplyFault::DbCommitted)?;

    if tx
        .remove_update_scratch(&[&backup_root, &staging_root])
        .await
        .is_err()
    {
        let _ = transition_update_operation(
            pool,
            &operation_id,
            PHASE_DB_COMMITTED,
            PHASE_CLEANUP,
            Some("skills_cli.update_recovery_required"),
        )
        .await;
        return Err(SkillsCliError::UpdateRecoveryRequired);
    }
    transition_update_operation(
        pool,
        &operation_id,
        PHASE_DB_COMMITTED,
        PHASE_COMPLETED,
        None,
    )
    .await
    .map_err(map_db_error)?;

    Ok(SkillsCliApplyResult {
        applied_skill_names: request
            .selections
            .iter()
            .map(|item| item.skill_name.clone())
            .collect(),
        installed_revision_sha: sha.clone(),
    })
}

pub(super) async fn recover_one_remote(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    operation_id: &str,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliApplyRecoveryResult, SkillsCliError> {
    check_cancel(cancel)?;
    let row = db::get_update_operation(pool, operation_id)
        .await
        .map_err(map_db_error)?
        .ok_or(SkillsCliError::UpdateRecoveryRequired)?;
    let manifest: ApplyManifest =
        serde_json::from_str(&row.manifest_json).map_err(|_| SkillsCliError::UpdateIntegrity)?;
    let canonical_root = tx.paths().canonical_root().to_string();
    let backup_root = remote_update_backup_dir(&canonical_root, operation_id);
    let staging_root = remote_update_staging_dir(&canonical_root, operation_id);
    match row.phase.as_str() {
        PHASE_PREPARED => {
            let _ = tx
                .remove_update_scratch(&[&backup_root, &staging_root])
                .await;
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
            settle_remote(tx, pool, &row.phase, &manifest, &canonical_root).await
        }
        PHASE_CLI_SUCCEEDED => {
            settle_remote(tx, pool, PHASE_CLI_SUCCEEDED, &manifest, &canonical_root).await
        }
        PHASE_DB_COMMITTED | PHASE_CLEANUP => {
            let _ = tx
                .remove_update_scratch(&[&backup_root, &staging_root])
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

async fn recover_pending_remote(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    cancel: Option<&AtomicBool>,
) -> Result<(), SkillsCliError> {
    let pending = list_pending_update_operations(pool)
        .await
        .map_err(map_db_error)?;
    for row in pending {
        recover_one_remote(tx, pool, &row.id, cancel).await?;
    }
    Ok(())
}

async fn settle_remote(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    phase: &str,
    manifest: &ApplyManifest,
    canonical_root: &str,
) -> Result<SkillsCliApplyRecoveryResult, SkillsCliError> {
    let backup_root = remote_update_backup_dir(canonical_root, &manifest.operation_id);
    let staging_root = remote_update_staging_dir(canonical_root, &manifest.operation_id);
    let mut roots = Vec::new();
    for item in &manifest.selections {
        roots.push(remote_join(canonical_root, &item.skill_name));
        roots.push(remote_join(&backup_root, &item.skill_name));
    }
    let hashes = tx.digest_remote_skill_dirs(&roots).await?;
    let mut all_new = true;
    let mut all_old = true;
    for item in &manifest.selections {
        let current = hashes.get(&remote_join(canonical_root, &item.skill_name));
        let backup = hashes.get(&remote_join(&backup_root, &item.skill_name));
        if current != backup {
            all_old = false;
        }
        if current.map(String::as_str) != Some(item.expected_pending_digest.as_str()) {
            all_new = false;
        }
    }
    if all_new && !all_old {
        return finalize_new_baseline_remote(tx, pool, phase, manifest).await;
    }
    if all_old || (phase == PHASE_CLI_SUCCEEDED && !all_new) {
        let restore_pairs: Vec<(String, String)> = manifest
            .selections
            .iter()
            .map(|item| {
                (
                    remote_join(&backup_root, &item.skill_name),
                    remote_join(canonical_root, &item.skill_name),
                )
            })
            .collect();
        tx.run_remote_script(&build_copy_trees_if_exist_script(&restore_pairs), true)
            .await?;
        let _ = tx
            .remove_update_scratch(&[&backup_root, &staging_root])
            .await;
        transition_update_operation(pool, &manifest.operation_id, phase, PHASE_ROLLED_BACK, None)
            .await
            .map_err(map_db_error)?;
        return Ok(SkillsCliApplyRecoveryResult {
            operation_id: manifest.operation_id.clone(),
            phase: PHASE_ROLLED_BACK.to_string(),
        });
    }
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

async fn finalize_new_baseline_remote(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    phase: &str,
    manifest: &ApplyManifest,
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
    let canonical_root = tx.paths().canonical_root().to_string();
    let backup_root = remote_update_backup_dir(&canonical_root, &manifest.operation_id);
    let staging_root = remote_update_staging_dir(&canonical_root, &manifest.operation_id);
    let _ = tx
        .remove_update_scratch(&[&backup_root, &staging_root])
        .await;
    Ok(SkillsCliApplyRecoveryResult {
        operation_id: manifest.operation_id.clone(),
        phase: PHASE_COMPLETED.to_string(),
    })
}

fn pack_skill_subset_tar(
    snapshot: &GitHubRepoSnapshot,
    request: &SkillsCliApplyUpdateRequest,
) -> Result<Vec<u8>, SkillsCliError> {
    let mut builder = tar::Builder::new(Vec::new());
    for selection in &request.selections {
        let files = skill_files_from_snapshot(snapshot, &selection.skill_path)?;
        for (relative, bytes) in files {
            let archive_path = format!("{}/{}", selection.skill_name, relative);
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, archive_path, bytes.as_slice())
                .map_err(|source| SkillsCliError::Io {
                    context: "pack update archive",
                    source: std::io::Error::other(source.to_string()),
                })?;
        }
    }
    builder.into_inner().map_err(|source| SkillsCliError::Io {
        context: "finalize update archive",
        source: std::io::Error::other(source.to_string()),
    })
}
