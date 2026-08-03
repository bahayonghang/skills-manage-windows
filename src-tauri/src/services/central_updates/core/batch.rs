use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use tracing::Instrument;

use crate::db::{self, DbPool, Skill, SkillUpdateState};
use crate::services::central_operation::{CentralOperationError, OperationPhase, MANIFEST_VERSION};

use super::state_from_remote;
use crate::services::central_updates::error::CentralUpdatesError;
use crate::services::central_updates::fs::{
    CentralFs, CentralSkillWrite, CopyRefreshRequest, OperationUpdateStage,
};
use crate::services::central_updates::types::RemoteSkillContent;

#[derive(Debug, Clone)]
pub(crate) struct SkillUpdatePlan {
    pub(crate) skill: Skill,
    pub(crate) remote: RemoteSkillContent,
    pub(crate) refresh_copies: bool,
}

#[derive(Debug)]
pub(crate) struct SkillUpdateBatchOutcome {
    pub(crate) skill_id: String,
    pub(crate) result: Result<SkillUpdateState, CentralUpdatesError>,
}

pub(crate) async fn update_skills_batch(
    pool: &DbPool,
    fs: &CentralFs,
    plans: Vec<SkillUpdatePlan>,
    cancel: Option<&AtomicBool>,
) -> Vec<SkillUpdateBatchOutcome> {
    let _mutation_guard =
        match crate::services::central_mutation::acquire_target_mutation_guard_by_id(
            fs.target_id(),
            fs.target_kind_value(),
            "update Central skills",
            crate::services::central_mutation::DEFAULT_CENTRAL_MUTATION_TIMEOUT,
        )
        .await
        {
            Ok(guard) => guard,
            Err(error) => {
                let message = error.to_string();
                return plans
                    .into_iter()
                    .map(|plan| SkillUpdateBatchOutcome {
                        skill_id: plan.skill.id,
                        result: Err(CentralUpdatesError::CentralMutation(message.clone())),
                    })
                    .collect();
            }
        };

    if let Err(error) = recover_pending_update_operations(pool, fs).await {
        let message = error.to_string();
        return plans
            .into_iter()
            .map(|plan| SkillUpdateBatchOutcome {
                skill_id: plan.skill.id,
                result: Err(CentralUpdatesError::CentralMutation(message.clone())),
            })
            .collect();
    }

    let persist_span = tracing::info_span!(
        "central_update_phase",
        phase = "db_persist",
        skills = plans.len()
    );
    let batch_id = uuid::Uuid::new_v4().to_string();
    let mut results = std::iter::repeat_with(|| None)
        .take(plans.len())
        .collect::<Vec<Option<Result<SkillUpdateState, CentralUpdatesError>>>>();
    let skill_ids = plans
        .iter()
        .map(|plan| plan.skill.id.clone())
        .collect::<Vec<_>>();
    let mut prepared = Vec::with_capacity(plans.len());
    async {
        for (index, plan) in plans.into_iter().enumerate() {
            if cancel.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
                results[index] = Some(Err(CentralUpdatesError::BatchCancelled));
                continue;
            }
            match prepare_update(pool, fs, plan, &batch_id, index).await {
                Ok(update) => prepared.push(update),
                Err(error) => results[index] = Some(Err(error)),
            }
        }
    }
    .instrument(persist_span)
    .await;

    let stage_outcomes = fs
        .stage_operation_updates(
            prepared
                .iter()
                .map(|update| OperationUpdateStage {
                    manifest: update.manifest.clone(),
                    write: update.write.clone(),
                })
                .collect(),
            cancel,
        )
        .await;
    let mut stage_results = stage_outcomes
        .into_iter()
        .map(|outcome| (outcome.operation_id, outcome.result))
        .collect::<HashMap<_, _>>();
    let mut copies_pending = Vec::new();
    for update in prepared {
        let stage_result = stage_results
            .remove(&update.operation_id)
            .unwrap_or_else(|| {
                Err(CentralUpdatesError::Batch(format!(
                    "Central durable stage returned no outcome for skill '{}'.",
                    update.plan.skill.id
                )))
            });
        if let Err(error) = stage_result {
            let index = update.index;
            results[index] = Some(Err(settle_failed_stage(
                pool,
                fs,
                &update.operation_id,
                &update.manifest,
                error,
            )
            .await));
            continue;
        }
        if let Err(error) =
            db::transition_fs_db_operation(pool, &update.operation_id, "prepared", "fs_staged")
                .await
        {
            results[update.index] = Some(Err(error.into()));
            continue;
        }
        match commit_staged_update(pool, fs, update).await {
            Ok(CommittedUpdateResult::Completed { index, state }) => {
                results[index] = Some(Ok(*state));
            }
            Ok(CommittedUpdateResult::CopiesPending(update)) => copies_pending.push(*update),
            Err((index, error)) => results[index] = Some(Err(error)),
        }
    }

    refresh_and_finalize_copies(pool, fs, copies_pending, &mut results).await;

    skill_ids
        .into_iter()
        .zip(results)
        .map(|(skill_id, result)| SkillUpdateBatchOutcome {
            skill_id: skill_id.clone(),
            result: result.unwrap_or_else(|| {
                Err(CentralUpdatesError::Batch(format!(
                    "Central update returned no outcome for skill '{skill_id}'."
                )))
            }),
        })
        .collect()
}

struct PreparedUpdate {
    index: usize,
    plan: SkillUpdatePlan,
    write: CentralSkillWrite,
    operation_id: String,
    manifest: crate::services::central_operation::UpdateManifest,
    copy_requests: Vec<CopyRefreshRequest>,
}

enum CommittedUpdateResult {
    Completed {
        index: usize,
        state: Box<SkillUpdateState>,
    },
    CopiesPending(Box<PreparedUpdate>),
}

async fn prepare_update(
    pool: &DbPool,
    fs: &CentralFs,
    plan: SkillUpdatePlan,
    batch_id: &str,
    index: usize,
) -> Result<PreparedUpdate, CentralUpdatesError> {
    let copy_requests = if plan.refresh_copies {
        copy_refresh_requests(pool, &plan.skill.id, &plan.remote.target_dir).await?
    } else {
        Vec::new()
    };
    let write = CentralSkillWrite {
        skill_id: plan.skill.id.clone(),
        target_dir: plan.remote.target_dir.clone(),
        files: plan.remote.files.clone(),
    };
    let operation_id = uuid::Uuid::new_v4().to_string();
    let manifest = fs
        .build_operation_update_manifest(
            &operation_id,
            &write,
            copy_requests
                .iter()
                .map(|request| request.target.clone())
                .collect(),
        )
        .await?;
    insert_update_operation(pool, fs, &plan, batch_id, &operation_id, &manifest).await?;
    Ok(PreparedUpdate {
        index,
        plan,
        write,
        operation_id,
        manifest,
        copy_requests,
    })
}

async fn commit_staged_update(
    pool: &DbPool,
    fs: &CentralFs,
    update: PreparedUpdate,
) -> Result<CommittedUpdateResult, (usize, CentralUpdatesError)> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| (update.index, error.into()))?;
    if let Err(error) = fs.swap_operation_update(&update.manifest).await {
        transaction
            .rollback()
            .await
            .map_err(|rollback_error| (update.index, rollback_error.into()))?;
        record_update_error(pool, &update.operation_id, &error)
            .await
            .map_err(|record_error| (update.index, record_error))?;
        return Err((update.index, error));
    }
    let apply_result = async {
        db::transition_fs_db_operation_in_transaction(
            &mut transaction,
            &update.operation_id,
            "fs_staged",
            "fs_swapped",
        )
        .await?;
        persist_updated_skill_in_transaction(
            &mut transaction,
            fs.target_kind_value(),
            &update.plan.skill,
            &update.plan.remote,
        )
        .await?;
        db::transition_fs_db_operation_in_transaction(
            &mut transaction,
            &update.operation_id,
            "fs_swapped",
            "db_committed",
        )
        .await?;
        Ok::<(), sqlx::Error>(())
    }
    .await;
    if let Err(error) = apply_result {
        if let Err(rollback_error) = transaction.rollback().await {
            return Err((update.index, rollback_error.into()));
        }
        let error = rollback_staged_after_db_failure(
            pool,
            fs,
            &update.operation_id,
            &update.manifest,
            error.into(),
        )
        .await;
        return Err((update.index, error));
    }

    if let Err(error) = transaction.commit().await {
        let commit_is_visible = db::get_fs_db_operation(pool, &update.operation_id)
            .await
            .map_err(|error| (update.index, error.into()))?
            .is_some_and(|row| row.phase == "db_committed");
        if !commit_is_visible {
            let error = rollback_staged_after_db_failure(
                pool,
                fs,
                &update.operation_id,
                &update.manifest,
                error.into(),
            )
            .await;
            return Err((update.index, error));
        }
    }

    if update.copy_requests.is_empty() {
        if let Err(error) = fs.finalize_operation_update(&update.manifest).await {
            record_update_error(pool, &update.operation_id, &error)
                .await
                .map_err(|record_error| (update.index, record_error))?;
            return Err((update.index, error));
        }
        if let Err(error) =
            db::transition_fs_db_operation(pool, &update.operation_id, "db_committed", "completed")
                .await
        {
            return Err((update.index, error.into()));
        }
        Ok(CommittedUpdateResult::Completed {
            index: update.index,
            state: Box::new(state_from_remote(
                &update.plan.skill,
                &update.plan.remote,
                true,
            )),
        })
    } else {
        if let Err(error) = db::transition_fs_db_operation(
            pool,
            &update.operation_id,
            "db_committed",
            "copies_pending",
        )
        .await
        {
            return Err((update.index, error.into()));
        }
        Ok(CommittedUpdateResult::CopiesPending(Box::new(update)))
    }
}

async fn rollback_staged_after_db_failure(
    pool: &DbPool,
    fs: &CentralFs,
    operation_id: &str,
    manifest: &crate::services::central_operation::UpdateManifest,
    error: CentralUpdatesError,
) -> CentralUpdatesError {
    if let Err(record_error) = record_update_error(pool, operation_id, &error).await {
        return record_error;
    }
    if let Err(rollback_error) = fs
        .rollback_operation_update(manifest, OperationPhase::FsStaged)
        .await
    {
        if let Err(record_error) = record_update_error(pool, operation_id, &rollback_error).await {
            return record_error;
        }
        return rollback_error;
    }
    if let Err(transition_error) =
        db::transition_fs_db_operation(pool, operation_id, "fs_staged", "rolled_back").await
    {
        return transition_error.into();
    }
    error
}

async fn settle_failed_stage(
    pool: &DbPool,
    fs: &CentralFs,
    operation_id: &str,
    manifest: &crate::services::central_operation::UpdateManifest,
    error: CentralUpdatesError,
) -> CentralUpdatesError {
    if let Err(record_error) = record_update_error(pool, operation_id, &error).await {
        return record_error;
    }
    if let Err(rollback_error) = fs
        .rollback_operation_update(manifest, OperationPhase::Prepared)
        .await
    {
        if let Err(record_error) = record_update_error(pool, operation_id, &rollback_error).await {
            return record_error;
        }
        return rollback_error;
    }
    if let Err(transition_error) =
        db::transition_fs_db_operation(pool, operation_id, "prepared", "rolled_back").await
    {
        return transition_error.into();
    }
    error
}

async fn refresh_and_finalize_copies(
    pool: &DbPool,
    fs: &CentralFs,
    mut updates: Vec<PreparedUpdate>,
    results: &mut [Option<Result<SkillUpdateState, CentralUpdatesError>>],
) {
    let requests = updates
        .iter()
        .flat_map(|update| update.copy_requests.iter().cloned())
        .collect::<Vec<_>>();
    let mut copy_results = fs
        .refresh_copy_installs_cancellable(requests, None)
        .await
        .into_iter()
        .map(|outcome| (outcome.target, outcome.result))
        .collect::<HashMap<_, _>>();

    for mut update in updates.drain(..) {
        let mut first_error = None;
        for copy in &mut update.manifest.copies {
            match copy_results.remove(&copy.target) {
                Some(Ok(())) => copy.completed = true,
                Some(Err(error)) if first_error.is_none() => first_error = Some(error),
                Some(Err(_)) => {}
                None if first_error.is_none() => {
                    first_error = Some(CentralUpdatesError::Batch(
                        "Copy refresh returned no outcome for a planned target.".to_string(),
                    ));
                }
                None => {}
            }
        }
        let result = async {
            persist_update_manifest(
                pool,
                &update.operation_id,
                "copies_pending",
                &update.manifest,
            )
            .await?;
            if let Some(error) = first_error {
                record_update_error(pool, &update.operation_id, &error).await?;
                return Err(error);
            }
            fs.finalize_operation_update(&update.manifest).await?;
            db::transition_fs_db_operation(
                pool,
                &update.operation_id,
                "copies_pending",
                "completed",
            )
            .await?;
            Ok(state_from_remote(
                &update.plan.skill,
                &update.plan.remote,
                true,
            ))
        }
        .await;
        results[update.index] = Some(result);
    }
}

async fn persist_updated_skill_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    target_kind: crate::targets::TargetKind,
    skill: &Skill,
    remote: &RemoteSkillContent,
) -> Result<(), sqlx::Error> {
    let (skill_md_path, canonical_path) =
        central_skill_persistence_paths(target_kind, &remote.target_dir);
    let updated_skill = Skill {
        id: skill.id.clone(),
        uid: skill.uid.clone(),
        name: remote.candidate.skill_name.clone(),
        description: remote.candidate.description.clone(),
        file_path: skill_md_path,
        canonical_path: Some(canonical_path),
        is_central: true,
        source: Some(format!(
            "github:{}/{}",
            remote.source.repo.owner, remote.source.repo.repo
        )),
        content: skill.content.clone(),
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    db::upsert_skill_with_github_repository_in_transaction(
        transaction,
        &updated_skill,
        &remote.source.repo.owner,
        &remote.source.repo.repo,
        &remote.source.repo.branch,
        &remote.source.repo.normalized_url,
        &remote.source.source_path,
        remote.resolved_commit_sha.as_deref(),
        remote.content_digest.as_deref(),
    )
    .await?;
    Ok(())
}

fn central_skill_persistence_paths(
    target_kind: crate::targets::TargetKind,
    target_dir: &Path,
) -> (String, String) {
    match target_kind {
        crate::targets::TargetKind::Local => (
            target_dir.join("SKILL.md").to_string_lossy().into_owned(),
            target_dir.to_string_lossy().into_owned(),
        ),
        crate::targets::TargetKind::Ssh | crate::targets::TargetKind::Wsl => {
            let canonical_path = target_dir.to_string_lossy().replace('\\', "/");
            let file_path = crate::targets::remote_join(&canonical_path, "SKILL.md");
            (file_path, canonical_path)
        }
    }
}

async fn insert_update_operation(
    pool: &DbPool,
    fs: &CentralFs,
    plan: &SkillUpdatePlan,
    batch_id: &str,
    operation_id: &str,
    manifest: &crate::services::central_operation::UpdateManifest,
) -> Result<(), CentralUpdatesError> {
    let manifest_json = serde_json::to_string(
        &crate::services::central_operation::OperationManifest::Update(manifest.clone()),
    )
    .map_err(|error| CentralUpdatesError::Json(error.to_string()))?;
    db::insert_fs_db_operation(
        pool,
        db::NewFsDbOperation {
            id: operation_id,
            batch_id: Some(batch_id),
            target_id: fs.target_id(),
            target_kind: fs.target_kind(),
            operation_kind: crate::services::central_operation::OperationKind::CentralUpdate
                .as_str(),
            skill_id: &plan.skill.id,
            manifest_version: crate::services::central_operation::MANIFEST_VERSION,
            manifest_json: &manifest_json,
            old_fingerprint: manifest.old_fingerprint.as_deref(),
            new_fingerprint: Some(&manifest.new_fingerprint),
        },
    )
    .await?;
    Ok(())
}

async fn persist_update_manifest(
    pool: &DbPool,
    operation_id: &str,
    phase: &str,
    manifest: &crate::services::central_operation::UpdateManifest,
) -> Result<(), CentralUpdatesError> {
    let manifest_json = serde_json::to_string(
        &crate::services::central_operation::OperationManifest::Update(manifest.clone()),
    )
    .map_err(|error| CentralUpdatesError::Json(error.to_string()))?;
    db::update_fs_db_operation_manifest(pool, operation_id, phase, &manifest_json).await?;
    Ok(())
}

async fn record_update_error(
    pool: &DbPool,
    operation_id: &str,
    error: &CentralUpdatesError,
) -> Result<(), CentralUpdatesError> {
    let (code, message) = match error {
        CentralUpdatesError::CentralOperation(error) => (error.code(), error.redacted_message()),
        CentralUpdatesError::Db(_) => ("update_db", "Database update failed".to_string()),
        CentralUpdatesError::Remote(_) => ("update_remote", "Remote update failed".to_string()),
        _ => ("update_failed", "Central update failed".to_string()),
    };
    db::record_fs_db_operation_error(pool, operation_id, code, &message).await?;
    Ok(())
}

pub(crate) async fn recover_pending_update_operations(
    pool: &DbPool,
    fs: &CentralFs,
) -> Result<(), CentralUpdatesError> {
    crate::services::central_operation::recover_pending_delete_operations_with_transport(
        pool,
        fs.target_id(),
        fs.target_kind(),
        fs.connected_remote(),
    )
    .await?;

    for row in db::list_pending_fs_db_operations(pool, fs.target_id()).await? {
        if row.operation_kind != "central_update" {
            continue;
        }
        if row.target_kind != fs.target_kind() {
            return Err(
                crate::services::central_operation::CentralOperationError::InvalidManifest(
                    "operation target identity mismatch".to_string(),
                )
                .into(),
            );
        }
        if row.manifest_version != MANIFEST_VERSION {
            return Err(CentralOperationError::InvalidManifest(format!(
                "unsupported manifest version {}",
                row.manifest_version
            ))
            .into());
        }
        let manifest_value: crate::services::central_operation::OperationManifest =
            serde_json::from_str(&row.manifest_json)
                .map_err(|error| CentralUpdatesError::Json(error.to_string()))?;
        manifest_value.validate(&row.id).map_err(|error| {
            crate::services::central_operation::CentralOperationError::InvalidManifest(error)
        })?;
        let crate::services::central_operation::OperationManifest::Update(mut manifest) =
            manifest_value
        else {
            return Err(
                crate::services::central_operation::CentralOperationError::InvalidManifest(
                    "update row contains a delete manifest".to_string(),
                )
                .into(),
            );
        };

        let phase = row
            .phase
            .parse::<OperationPhase>()
            .map_err(CentralOperationError::InvalidManifest)?;
        match phase {
            OperationPhase::Prepared | OperationPhase::FsStaged | OperationPhase::FsSwapped => {
                if let Err(error) = fs.rollback_operation_update(&manifest, phase).await {
                    record_update_error(pool, &row.id, &error).await?;
                    return Err(error);
                }
                db::transition_fs_db_operation(pool, &row.id, phase.as_str(), "rolled_back")
                    .await?;
            }
            OperationPhase::DbCommitted => {
                if manifest.copies.iter().all(|copy| copy.completed) {
                    fs.finalize_operation_update(&manifest).await?;
                    db::transition_fs_db_operation(pool, &row.id, "db_committed", "completed")
                        .await?;
                    continue;
                }
                db::transition_fs_db_operation(pool, &row.id, "db_committed", "copies_pending")
                    .await?;
                recover_copy_projections(pool, fs, &row.id, &mut manifest).await?;
            }
            OperationPhase::CopiesPending => {
                recover_copy_projections(pool, fs, &row.id, &mut manifest).await?;
            }
            OperationPhase::Completed | OperationPhase::RolledBack => {}
        }
    }
    Ok(())
}

async fn recover_copy_projections(
    pool: &DbPool,
    fs: &CentralFs,
    operation_id: &str,
    manifest: &mut crate::services::central_operation::UpdateManifest,
) -> Result<(), CentralUpdatesError> {
    let skill_id = db::get_fs_db_operation(pool, operation_id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?
        .skill_id;
    let requests = manifest
        .copies
        .iter()
        .filter(|copy| !copy.completed)
        .map(|copy| CopyRefreshRequest {
            skill_id: skill_id.clone(),
            source_dir: Path::new(&manifest.target).to_path_buf(),
            target: copy.target.clone(),
        })
        .collect();
    let mut first_error = None;
    for outcome in fs.refresh_copy_installs_cancellable(requests, None).await {
        if outcome.result.is_ok() {
            if let Some(copy) = manifest
                .copies
                .iter_mut()
                .find(|copy| copy.target == outcome.target)
            {
                copy.completed = true;
            }
        } else if first_error.is_none() {
            first_error = outcome.result.err();
        }
    }
    persist_update_manifest(pool, operation_id, "copies_pending", manifest).await?;
    if let Some(error) = first_error {
        record_update_error(pool, operation_id, &error).await?;
        return Err(error);
    }
    fs.finalize_operation_update(manifest).await?;
    db::transition_fs_db_operation(pool, operation_id, "copies_pending", "completed").await?;
    Ok(())
}

async fn copy_refresh_requests(
    pool: &DbPool,
    skill_id: &str,
    source_dir: &Path,
) -> Result<Vec<CopyRefreshRequest>, CentralUpdatesError> {
    let installations = db::get_skill_installations(pool, skill_id).await?;
    let mut seen_targets = HashSet::new();
    Ok(installations
        .into_iter()
        .filter(|installation| installation.link_type == "copy")
        .filter_map(|installation| {
            if seen_targets.insert(installation.installed_path.clone()) {
                Some(CopyRefreshRequest {
                    skill_id: skill_id.to_string(),
                    source_dir: source_dir.to_path_buf(),
                    target: installation.installed_path,
                })
            } else {
                None
            }
        })
        .collect())
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
