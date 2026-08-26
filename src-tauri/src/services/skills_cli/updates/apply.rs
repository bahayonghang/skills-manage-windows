//! Journaled Skills CLI canonical refresh from a pinned GitHub snapshot.
//!
//! Product argv never includes `--force`, `--keep-links`, or an unverified
//! full-SHA `skills add` source. Direct copies are blocked before journal.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::{
    self, insert_update_operation, list_pending_update_operations, transition_update_operation,
    transition_update_operation_in_transaction, upsert_update_state_in_transaction,
    NewSkillsCliUpdateOperation, SkillsCliUpdateStateRow as PersistedSkill,
};
use crate::db::DbPool;
use crate::fs_util::run_blocking_fs_with;
use crate::services::central_mutation::{
    acquire_central_mutation_guard_at, acquire_target_mutation_guard,
    DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};
use crate::services::github_import::{
    candidate_content_digest_from_snapshot, GitHubRepoSnapshot,
};
use crate::targets::ActiveTarget;

use super::super::{
    check_cancel, list_global_at, map_guard_error, SkillsCliError, SkillsCliPlacementState,
};
use super::capability::argv_contains_forbidden_flags;
use super::detect::topology_blockers;
use super::digest::{copy_dir_recursive, digest_skill_directory, write_skill_files};
use super::github::SkillsCliUpdateGithub;
use super::status::SkillsCliPersistedUpdateStatus;
use super::{
    map_db_error, SkillsCliApplyRecoveryResult, SkillsCliApplyResult, SkillsCliApplyUpdateRequest,
    SkillsCliUpdateProgress, UpdateProgressEmitter, UPDATE_LOCK_OPERATION,
};

const MANIFEST_VERSION: i64 = 1;
const PHASE_PREPARED: &str = "prepared";
const PHASE_BACKUPS: &str = "backups_staged";
const PHASE_CLI_STARTED: &str = "cli_started";
const PHASE_CLI_SUCCEEDED: &str = "cli_succeeded";
const PHASE_DB_COMMITTED: &str = "db_committed";
const PHASE_CLEANUP: &str = "cleanup_pending";
const PHASE_COMPLETED: &str = "completed";
const PHASE_ROLLED_BACK: &str = "rolled_back";
const PHASE_RECOVERY: &str = "recovery_required";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyFault {
    AfterPrepared,
    AfterBackups,
    AfterCliStarted,
    AfterCliSucceeded,
    AfterDbCommitted,
}

#[cfg(test)]
thread_local! {
    static APPLY_FAULT: std::cell::Cell<Option<ApplyFault>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub fn set_apply_fault(fault: Option<ApplyFault>) {
    APPLY_FAULT.with(|cell| cell.set(fault));
}

fn injected_fault() -> Option<ApplyFault> {
    #[cfg(test)]
    {
        APPLY_FAULT.with(|cell| cell.get())
    }
    #[cfg(not(test))]
    {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApplyManifest {
    operation_id: String,
    repository_key: String,
    skill_names: Vec<String>,
    expected_pending_revision: String,
    lock_fingerprint: String,
    selections: Vec<ApplyManifestSelection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApplyManifestSelection {
    skill_name: String,
    skill_path: String,
    expected_pending_digest: String,
    expected_installed_local_digest: Option<String>,
    backup_relative: String,
}

pub struct ApplyContext<'a> {
    pub pool: &'a DbPool,
    pub canonical_root: &'a Path,
    pub lock_path: &'a Path,
    pub recovery_root: &'a Path,
    pub github: &'a dyn SkillsCliUpdateGithub,
    pub progress: &'a dyn UpdateProgressEmitter,
    pub request: &'a SkillsCliApplyUpdateRequest,
    pub cancel: Option<&'a AtomicBool>,
    pub mutation_lock_path: Option<PathBuf>,
}

pub(crate) async fn apply_updates(
    pool: &DbPool,
    github: &dyn SkillsCliUpdateGithub,
    progress: &dyn UpdateProgressEmitter,
    request: &SkillsCliApplyUpdateRequest,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliApplyResult, SkillsCliError> {
    let home = crate::paths::resolve_home_dir();
    apply_updates_at(ApplyContext {
        pool,
        canonical_root: &crate::paths::universal_skills_dir(),
        lock_path: &crate::services::skills_cli::skills_cli_lock_path(&home),
        recovery_root: &crate::paths::skills_cli_update_recovery_dir(),
        github,
        progress,
        request,
        cancel,
        mutation_lock_path: None,
    })
    .await
}

pub(crate) async fn apply_updates_at(
    context: ApplyContext<'_>,
) -> Result<SkillsCliApplyResult, SkillsCliError> {
    let request = context.request;
    if request.selections.is_empty() {
        return Err(SkillsCliError::SelectionEmpty);
    }
    let preview = super::capability::apply_argv_preview(
        &request
            .selections
            .iter()
            .map(|item| item.skill_name.clone())
            .collect::<Vec<_>>(),
    );
    if argv_contains_forbidden_flags(&preview) {
        return Err(SkillsCliError::UpdateUnsupported);
    }
    check_cancel(context.cancel)?;

    let snapshot = list_global_at(context.pool, context.canonical_root, context.lock_path).await?;
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
        let state = db::get_update_state(context.pool, &selection.skill_name)
            .await
            .map_err(map_db_error)?
            .ok_or(SkillsCliError::UpdateStale)?;
        if state.repository_key.as_deref() != Some(request.repository_key.as_str()) {
            return Err(SkillsCliError::UpdateStale);
        }
        if state.is_stale != 0 {
            return Err(SkillsCliError::UpdateStale);
        }
        if state.pending_revision_sha.as_deref() != Some(selection.expected_pending_revision.as_str())
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
    let (owner_repo, branch_unused) = identity_parts.split_first().ok_or(SkillsCliError::UpdateStale)?;
    let _ = branch_unused;
    let mut owner_repo_parts = owner_repo.split('/');
    let owner = owner_repo_parts
        .next()
        .ok_or(SkillsCliError::UpdateStale)?;
    let repo = owner_repo_parts
        .next()
        .ok_or(SkillsCliError::UpdateStale)?;
    let sha = &request.selections[0].expected_pending_revision;
    if sha.len() != 40 || !sha.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(SkillsCliError::UpdateStale);
    }
    let pinned = context
        .github
        .snapshot_at_sha(owner, repo, sha)
        .await?;
    for selection in &request.selections {
        let digest = candidate_content_digest_from_snapshot(&pinned, &selection.skill_path)
            .map_err(|_| SkillsCliError::UpdateStale)?;
        if digest != selection.expected_pending_digest {
            return Err(SkillsCliError::UpdateStale);
        }
    }

    context.progress.emit_update_progress(&SkillsCliUpdateProgress {
        job_id: request.job_id.clone(),
        phase: "prepare".to_string(),
        repository_total: 1,
        repository_completed: 0,
        current_repository_key: Some(request.repository_key.clone()),
        selected_total: request.selections.len() as u32,
        selected_completed: 0,
        terminal_status: None,
    });

    let _guard = if let Some(lock_path) = context.mutation_lock_path.clone() {
        acquire_central_mutation_guard_at(
            lock_path,
            UPDATE_LOCK_OPERATION,
            DEFAULT_CENTRAL_MUTATION_TIMEOUT,
        )
        .await
        .map_err(map_guard_error)?
    } else {
        acquire_target_mutation_guard(
            &ActiveTarget::Local,
            UPDATE_LOCK_OPERATION,
            DEFAULT_CENTRAL_MUTATION_TIMEOUT,
        )
        .await
        .map_err(map_guard_error)?
    };

    recover_pending_at(
        context.pool,
        context.canonical_root,
        context.lock_path,
        context.recovery_root,
        context.cancel,
    )
    .await?;

    let snapshot = list_global_at(context.pool, context.canonical_root, context.lock_path).await?;
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
        let canonical = context.canonical_root.join(&selection.skill_name);
        let current = if canonical.is_dir() {
            let path = canonical.clone();
            Some(
                run_blocking_fs_with(
                    "skills-cli-apply-recheck-digest",
                    move || digest_skill_directory(&path),
                    SkillsCliError::task_join,
                )
                .await?,
            )
        } else {
            None
        };
        if selection.expected_installed_local_digest.as_deref() != current.as_deref()
            && selection.expected_installed_local_digest.is_some()
        {
            return Err(SkillsCliError::UpdateStale);
        }
    }

    let operation_id = Uuid::new_v4().to_string();
    let lock_fingerprint = lock_fingerprint(context.lock_path)?;
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
        context.pool,
        NewSkillsCliUpdateOperation {
            id: &operation_id,
            phase: PHASE_PREPARED,
            manifest_version: MANIFEST_VERSION,
            manifest_json: &manifest_json,
        },
    )
    .await
    .map_err(map_db_error)?;
    fail_if(ApplyFault::AfterPrepared)?;

    let recovery_root = context.recovery_root.to_path_buf();
    let canonical_root = context.canonical_root.to_path_buf();
    let backup_plan = manifest.selections.clone();
    let snapshot_files = pinned.clone();
    run_blocking_fs_with(
        "skills-cli-update-backup",
        move || {
            for item in &backup_plan {
                let source = canonical_root.join(&item.skill_name);
                let dest = recovery_root.join(&item.backup_relative);
                if source.is_dir() {
                    copy_dir_recursive(&source, &dest)?;
                }
            }
            Ok(())
        },
        SkillsCliError::task_join,
    )
    .await?;
    transition_update_operation(
        context.pool,
        &operation_id,
        PHASE_PREPARED,
        PHASE_BACKUPS,
        None,
    )
    .await
    .map_err(map_db_error)?;
    fail_if(ApplyFault::AfterBackups)?;
    transition_update_operation(
        context.pool,
        &operation_id,
        PHASE_BACKUPS,
        PHASE_CLI_STARTED,
        None,
    )
    .await
    .map_err(map_db_error)?;
    fail_if(ApplyFault::AfterCliStarted)?;

    check_cancel(context.cancel)?;
    let refresh_plan = manifest.selections.clone();
    let canonical_root = context.canonical_root.to_path_buf();
    run_blocking_fs_with(
        "skills-cli-update-refresh",
        move || refresh_canonicals(&canonical_root, &refresh_plan, &snapshot_files),
        SkillsCliError::task_join,
    )
    .await?;
    transition_update_operation(
        context.pool,
        &operation_id,
        PHASE_CLI_STARTED,
        PHASE_CLI_SUCCEEDED,
        None,
    )
    .await
    .map_err(map_db_error)?;
    fail_if(ApplyFault::AfterCliSucceeded)?;

    for selection in &request.selections {
        let canonical = context.canonical_root.join(&selection.skill_name);
        let path = canonical.clone();
        let digest = run_blocking_fs_with(
            "skills-cli-update-verify",
            move || digest_skill_directory(&path),
            SkillsCliError::task_join,
        )
        .await?;
        if digest != selection.expected_pending_digest {
            return Err(SkillsCliError::UpdateIntegrity);
        }
    }

    let now = Utc::now().to_rfc3339();
    let mut transaction = context.pool.begin().await.map_err(map_db_error)?;
    for selection in &request.selections {
        let mut row = db::get_update_state(context.pool, &selection.skill_name)
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
    fail_if(ApplyFault::AfterDbCommitted)?;

    let recovery_root = context.recovery_root.to_path_buf();
    let cleanup_id = operation_id.clone();
    let cleanup = run_blocking_fs_with(
        "skills-cli-update-cleanup",
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
    if cleanup.is_err() {
        let _ = transition_update_operation(
            context.pool,
            &operation_id,
            PHASE_DB_COMMITTED,
            PHASE_CLEANUP,
            Some("skills_cli.update_recovery_required"),
        )
        .await;
        return Err(SkillsCliError::UpdateRecoveryRequired);
    }
    transition_update_operation(
        context.pool,
        &operation_id,
        PHASE_DB_COMMITTED,
        PHASE_COMPLETED,
        None,
    )
    .await
    .map_err(map_db_error)?;

    context.progress.emit_update_progress(&SkillsCliUpdateProgress {
        job_id: request.job_id.clone(),
        phase: "completed".to_string(),
        repository_total: 1,
        repository_completed: 1,
        current_repository_key: Some(request.repository_key.clone()),
        selected_total: request.selections.len() as u32,
        selected_completed: request.selections.len() as u32,
        terminal_status: Some("completed".to_string()),
    });

    Ok(SkillsCliApplyResult {
        applied_skill_names: request
            .selections
            .iter()
            .map(|item| item.skill_name.clone())
            .collect(),
        installed_revision_sha: sha.clone(),
    })
}

pub(crate) async fn retry_update_recovery(
    pool: &DbPool,
    operation_id: &str,
    canonical_root: &Path,
    lock_path: &Path,
    recovery_root: &Path,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliApplyRecoveryResult, SkillsCliError> {
    let _guard = acquire_target_mutation_guard(
        &ActiveTarget::Local,
        UPDATE_LOCK_OPERATION,
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
    .map_err(map_guard_error)?;
    recover_one(
        pool,
        operation_id,
        canonical_root,
        lock_path,
        recovery_root,
        cancel,
    )
    .await
}

#[cfg(test)]
pub(crate) async fn retry_update_recovery_at(
    pool: &DbPool,
    operation_id: &str,
    canonical_root: &Path,
    lock_path: &Path,
    recovery_root: &Path,
    mutation_lock_path: PathBuf,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliApplyRecoveryResult, SkillsCliError> {
    let _guard = acquire_central_mutation_guard_at(
        mutation_lock_path,
        UPDATE_LOCK_OPERATION,
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
    .map_err(map_guard_error)?;
    recover_one(
        pool,
        operation_id,
        canonical_root,
        lock_path,
        recovery_root,
        cancel,
    )
    .await
}

async fn recover_pending_at(
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

async fn recover_one(
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
            transition_update_operation(pool, operation_id, PHASE_PREPARED, PHASE_ROLLED_BACK, None)
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
            transition_update_operation(pool, &manifest.operation_id, phase, PHASE_ROLLED_BACK, None)
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

fn refresh_canonicals(
    canonical_root: &Path,
    selections: &[ApplyManifestSelection],
    snapshot: &GitHubRepoSnapshot,
) -> Result<(), SkillsCliError> {
    for item in selections {
        let files = skill_files_from_snapshot(snapshot, &item.skill_path)?;
        write_skill_files(&canonical_root.join(&item.skill_name), &files)?;
    }
    Ok(())
}

fn skill_files_from_snapshot(
    snapshot: &GitHubRepoSnapshot,
    skill_path: &str,
) -> Result<Vec<(String, Vec<u8>)>, SkillsCliError> {
    let mut files = Vec::new();
    for (repo_path, bytes) in &snapshot.files {
        if let Some(relative) =
            crate::services::github_import::repo_file_relative_to_source(repo_path, skill_path)
        {
            files.push((relative, bytes.clone()));
        }
    }
    if !files.iter().any(|(path, _)| path == "SKILL.md") {
        return Err(SkillsCliError::UpdateIntegrity);
    }
    Ok(files)
}

fn lock_fingerprint(lock_path: &Path) -> Result<String, SkillsCliError> {
    let bytes = std::fs::read(lock_path).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn fail_if(fault: ApplyFault) -> Result<(), SkillsCliError> {
    if injected_fault() == Some(fault) {
        Err(SkillsCliError::UpdateRecoveryRequired)
    } else {
        Ok(())
    }
}
