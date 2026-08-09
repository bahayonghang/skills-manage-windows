//! Update orchestration core: prepare skills, compare local/remote hashes,
//! build `SkillUpdateState` rows, and apply updates atomically.
//!
//! All entry points take explicit dependencies (pool / fs façade / cancel
//! flag / snapshot cache / optional `AppHandle` for progress events) so the
//! Tauri command shells stay thin and unit tests can run fully offline with
//! a pre-filled snapshot cache.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

use crate::db::{self, DbPool, Skill, SkillUpdateState};

use super::error::CentralUpdatesError;
use super::fs::CentralFs;
use super::snapshots::{prepare_snapshots, CentralUpdateSnapshotCache};
use super::types::{
    CentralSkillUpdateFailure, CentralSkillUpdateProgressPayload, CentralSkillUpdateResult,
    CentralSkillUpdateSkip, RemoteSkillLoadError, SkillUpdateStatus, UpdateCounters,
};

const UPDATE_PROGRESS_EVENT: &str = "central://skill-update-progress";

mod batch;
mod content_upsert;
mod state;
#[cfg(test)]
mod tests;

pub(crate) use batch::{
    recover_pending_update_operation, recover_pending_update_operations, update_skills_batch,
    SkillUpdatePlan,
};
pub(crate) use content_upsert::{journaled_central_content_upsert, JournaledCentralContentUpsert};
#[allow(unused_imports)]
pub(crate) use state::repository_url;
pub(crate) use state::{
    error_state_from_assignment, load_remote_skill_content, load_selected_central_skills,
    prepare_skill_updates, remote_missing_state_from_assignment, state_from_relocated_source,
    state_from_remote, unsupported_state_from_assignment,
};

pub(crate) async fn get_central_skill_update_states_impl(
    pool: &DbPool,
) -> Result<Vec<SkillUpdateState>, CentralUpdatesError> {
    Ok(db::get_skill_update_states(pool).await?)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn check_central_skill_updates_impl(
    app: Option<&AppHandle>,
    job_id: &str,
    pool: &DbPool,
    fs: &CentralFs,
    cancel: &AtomicBool,
    auth_token: Option<&str>,
    client: &reqwest::Client,
    snapshots_cache: &CentralUpdateSnapshotCache,
    skill_ids: Option<Vec<String>>,
) -> Result<Vec<SkillUpdateState>, CentralUpdatesError> {
    let skills = load_selected_central_skills(pool, skill_ids.as_deref()).await?;
    let total = skills.len();
    let mut counters = UpdateCounters::default();
    let mut states = Vec::with_capacity(total);
    emit_update_progress(
        app, job_id, "checking", "started", total, &counters, None, None,
    );
    let prepared = prepare_skill_updates(pool, fs, skills, auth_token, false).await?;
    let snapshots = prepare_snapshots(client, auth_token, &prepared, snapshots_cache).await?;
    for prepared_skill in prepared {
        let skill = &prepared_skill.skill;
        if cancel.load(Ordering::SeqCst) {
            emit_update_progress(
                app,
                job_id,
                "checking",
                SkillUpdateStatus::Cancelled.as_str(),
                total,
                &counters,
                None,
                None,
            );
            return Ok(states);
        }
        emit_update_progress(
            app,
            job_id,
            "checking",
            "running",
            total,
            &counters,
            Some(skill),
            None,
        );
        let state_result = match load_remote_skill_content(&prepared_skill, &snapshots) {
            Ok(Some(remote)) => state_from_remote(skill, &remote, false),
            Ok(None) => unsupported_state_from_assignment(skill, &prepared_skill.assignment, None),
            Err(RemoteSkillLoadError::RemoteMissing(reason)) => {
                remote_missing_state_from_assignment(skill, &prepared_skill.assignment, &reason)
            }
            Err(RemoteSkillLoadError::Other(error)) => {
                error_state_from_assignment(skill, &prepared_skill.assignment, &error)
            }
        };
        db::upsert_skill_update_state(pool, &state_result).await?;
        update_counters_for_state(&mut counters, &state_result);
        emit_update_progress(
            app,
            job_id,
            "checking",
            state_result.status.as_str(),
            total,
            &counters,
            Some(skill),
            state_result.error.as_deref(),
        );
        states.push(state_result);
    }

    emit_update_progress(
        app,
        job_id,
        "checking",
        "completed",
        total,
        &counters,
        None,
        None,
    );

    Ok(states)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn update_central_skills_impl(
    app: Option<&AppHandle>,
    job_id: &str,
    pool: &DbPool,
    fs: &CentralFs,
    cancel: &AtomicBool,
    auth_token: Option<&str>,
    client: &reqwest::Client,
    snapshots_cache: &CentralUpdateSnapshotCache,
    skill_ids: Vec<String>,
) -> Result<CentralSkillUpdateResult, CentralUpdatesError> {
    if skill_ids.is_empty() {
        return Err(CentralUpdatesError::NoUpdateSelection);
    }

    let skills = load_selected_central_skills(pool, Some(&skill_ids)).await?;
    let total = skills.len();
    let mut counters = UpdateCounters::default();
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    let mut skipped = Vec::new();
    let mut states = Vec::new();

    emit_update_progress(
        app, job_id, "updating", "started", total, &counters, None, None,
    );

    let prepared = prepare_skill_updates(pool, fs, skills, auth_token, true).await?;
    let snapshots = prepare_snapshots(client, auth_token, &prepared, snapshots_cache).await?;
    let mut pending_updates = Vec::new();

    for prepared_skill in prepared {
        let skill = &prepared_skill.skill;
        if cancel.load(Ordering::SeqCst) {
            emit_update_progress(
                app,
                job_id,
                "updating",
                SkillUpdateStatus::Cancelled.as_str(),
                total,
                &counters,
                None,
                None,
            );
            return Ok(CentralSkillUpdateResult {
                succeeded,
                failed,
                skipped,
                states,
            });
        }

        emit_update_progress(
            app,
            job_id,
            "updating",
            "running",
            total,
            &counters,
            Some(skill),
            None,
        );

        match load_remote_skill_content(&prepared_skill, &snapshots) {
            Ok(Some(remote)) if remote.remote_hash == remote.local_hash => {
                let state_result = state_from_remote(skill, &remote, false);
                db::upsert_skill_update_state(pool, &state_result).await?;
                counters.completed += 1;
                counters.skipped += 1;
                skipped.push(CentralSkillUpdateSkip {
                    skill_id: skill.id.clone(),
                    reason: "Already up to date".to_string(),
                });
                emit_update_progress(
                    app,
                    job_id,
                    "updating",
                    SkillUpdateStatus::UpToDate.as_str(),
                    total,
                    &counters,
                    Some(skill),
                    None,
                );
                states.push(state_result);
            }
            Ok(Some(remote)) => pending_updates.push((prepared_skill.clone(), remote)),
            Ok(None) => {
                let state_result =
                    unsupported_state_from_assignment(skill, &prepared_skill.assignment, None);
                db::upsert_skill_update_state(pool, &state_result).await?;
                counters.completed += 1;
                counters.skipped += 1;
                skipped.push(CentralSkillUpdateSkip {
                    skill_id: skill.id.clone(),
                    reason: state_result
                        .error
                        .clone()
                        .unwrap_or_else(|| "Unsupported source".to_string()),
                });
                emit_update_progress(
                    app,
                    job_id,
                    "updating",
                    SkillUpdateStatus::Unsupported.as_str(),
                    total,
                    &counters,
                    Some(skill),
                    state_result.error.as_deref(),
                );
                states.push(state_result);
            }
            Err(RemoteSkillLoadError::RemoteMissing(reason)) => {
                let state_result = remote_missing_state_from_assignment(
                    skill,
                    &prepared_skill.assignment,
                    &reason,
                );
                db::upsert_skill_update_state(pool, &state_result).await?;
                counters.completed += 1;
                counters.skipped += 1;
                skipped.push(CentralSkillUpdateSkip {
                    skill_id: skill.id.clone(),
                    reason: reason.clone(),
                });
                emit_update_progress(
                    app,
                    job_id,
                    "updating",
                    SkillUpdateStatus::RemoteMissing.as_str(),
                    total,
                    &counters,
                    Some(skill),
                    Some(&reason),
                );
                states.push(state_result);
            }
            Err(RemoteSkillLoadError::Other(_error)) => {
                let public_error = "This update item could not be applied.";
                let state_result =
                    error_state_from_assignment(skill, &prepared_skill.assignment, public_error);
                db::upsert_skill_update_state(pool, &state_result).await?;
                counters.completed += 1;
                counters.failed += 1;
                failed.push(CentralSkillUpdateFailure::decision_apply_fallback(
                    skill.id.clone(),
                ));
                emit_update_progress(
                    app,
                    job_id,
                    "updating",
                    SkillUpdateStatus::Error.as_str(),
                    total,
                    &counters,
                    Some(skill),
                    Some(public_error),
                );
                states.push(state_result);
            }
        }
    }

    let plans = pending_updates
        .iter()
        .map(|(prepared, remote)| SkillUpdatePlan {
            skill: prepared.skill.clone(),
            remote: remote.clone(),
            refresh_copies: true,
        })
        .collect();
    let update_outcomes = update_skills_batch(pool, fs, plans, Some(cancel)).await;
    for ((prepared_skill, _), outcome) in pending_updates.into_iter().zip(update_outcomes) {
        let skill = &prepared_skill.skill;
        match outcome.result {
            Ok(state_result) => {
                db::upsert_skill_update_state(pool, &state_result).await?;
                counters.completed += 1;
                counters.succeeded += 1;
                succeeded.push(skill.id.clone());
                emit_update_progress(
                    app,
                    job_id,
                    "updating",
                    SkillUpdateStatus::UpToDate.as_str(),
                    total,
                    &counters,
                    Some(skill),
                    None,
                );
                states.push(state_result);
            }
            Err(error) if matches!(error.error(), CentralUpdatesError::BatchCancelled) => {
                emit_update_progress(
                    app,
                    job_id,
                    "updating",
                    SkillUpdateStatus::Cancelled.as_str(),
                    total,
                    &counters,
                    None,
                    None,
                );
                return Ok(CentralSkillUpdateResult {
                    succeeded,
                    failed,
                    skipped,
                    states,
                });
            }
            Err(error) => {
                let public_error = error.error().public_update_message();
                let state_result =
                    error_state_from_assignment(skill, &prepared_skill.assignment, public_error);
                db::upsert_skill_update_state(pool, &state_result).await?;
                counters.completed += 1;
                counters.failed += 1;
                failed.push(CentralSkillUpdateFailure::from_item_error(
                    skill.id.clone(),
                    &error,
                ));
                emit_update_progress(
                    app,
                    job_id,
                    "updating",
                    SkillUpdateStatus::Error.as_str(),
                    total,
                    &counters,
                    Some(skill),
                    Some(public_error),
                );
                states.push(state_result);
            }
        }
    }

    emit_update_progress(
        app,
        job_id,
        "updating",
        "completed",
        total,
        &counters,
        None,
        None,
    );

    Ok(CentralSkillUpdateResult {
        succeeded,
        failed,
        skipped,
        states,
    })
}

pub(crate) async fn keep_remote_missing_central_skills_impl(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<Vec<String>, CentralUpdatesError> {
    if skill_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut seen = HashSet::new();
    let unique_skill_ids = skill_ids
        .iter()
        .filter(|skill_id| seen.insert((*skill_id).clone()))
        .cloned()
        .collect::<Vec<_>>();

    let states = db::get_skill_update_states_for_skills(pool, &unique_skill_ids).await?;
    let states_by_skill_id = states
        .into_iter()
        .map(|state| (state.skill_id.clone(), state))
        .collect::<HashMap<_, _>>();

    for skill_id in &unique_skill_ids {
        let skill = db::get_skill_by_id(pool, skill_id)
            .await?
            .ok_or_else(|| CentralUpdatesError::SkillNotFound(skill_id.clone()))?;
        if !skill.is_central {
            return Err(CentralUpdatesError::NotCentralSkill(skill_id.clone()));
        }

        let update_state = states_by_skill_id
            .get(skill_id)
            .ok_or_else(|| CentralUpdatesError::NoRemoteMissingState(skill_id.clone()))?;
        if update_state.status != SkillUpdateStatus::RemoteMissing {
            return Err(CentralUpdatesError::NotRemoteMissing(skill_id.clone()));
        }
    }

    for skill_id in &unique_skill_ids {
        db::detach_skill_remote_source(pool, skill_id).await?;
    }

    Ok(unique_skill_ids)
}

pub(crate) fn update_counters_for_state(counters: &mut UpdateCounters, state: &SkillUpdateState) {
    counters.completed += 1;
    match state.status {
        SkillUpdateStatus::UpToDate | SkillUpdateStatus::UpdateAvailable => {
            counters.succeeded += 1;
        }
        SkillUpdateStatus::Unsupported | SkillUpdateStatus::RemoteMissing => {
            counters.skipped += 1;
        }
        SkillUpdateStatus::Error => counters.failed += 1,
        SkillUpdateStatus::Cancelled => {}
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_update_progress(
    app: Option<&AppHandle>,
    job_id: &str,
    phase: &str,
    status: &str,
    total: usize,
    counters: &UpdateCounters,
    skill: Option<&Skill>,
    error: Option<&str>,
) {
    let Some(app) = app else {
        return;
    };
    let payload = CentralSkillUpdateProgressPayload {
        job_id: job_id.to_string(),
        phase: phase.to_string(),
        status: status.to_string(),
        total,
        completed: counters.completed,
        succeeded: counters.succeeded,
        failed: counters.failed,
        skipped: counters.skipped,
        skill_id: skill.map(|skill| skill.id.clone()),
        skill_name: skill.map(|skill| skill.name.clone()),
        error: error.map(str::to_string),
    };
    let _ = app.emit(UPDATE_PROGRESS_EVENT, payload);
}
