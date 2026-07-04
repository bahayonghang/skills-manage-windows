//! Update orchestration core: prepare skills, compare local/remote hashes,
//! build `SkillUpdateState` rows, and apply updates atomically.
//!
//! All entry points take explicit dependencies (pool / fs façade / cancel
//! flag / snapshot cache / optional `AppHandle` for progress events) so the
//! Tauri command shells stay thin and unit tests can run fully offline with
//! a pre-filled snapshot cache.

use chrono::Utc;
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

use crate::db::{self, DbPool, Skill, SkillRepositoryAssignment, SkillUpdateState};
use crate::services::github_import::{self, GitHubRepoRef, GitHubRepoSnapshot};

use super::error::CentralUpdatesError;
use super::fs::{
    collect_remote_skill_files, ensure_remote_skill_manifest, hash_remote_files,
    normalize_repo_path, CentralFs,
};
use super::snapshots::{
    prepare_snapshots, repo_cache_key, snapshot_cache_ttl, CentralUpdateSnapshotCache,
};
use super::types::{
    CentralSkillUpdateFailure, CentralSkillUpdateProgressPayload, CentralSkillUpdateResult,
    CentralSkillUpdateSkip, GitHubUpdateSource, PreparedSkillUpdate, RemoteSkillContent,
    RemoteSkillLoadError, SkillUpdateStatus, UpdateCounters,
};

const UPDATE_PROGRESS_EVENT: &str = "central://skill-update-progress";
const COPY_INSTALL_REFRESH_CONCURRENCY: usize = 4;

#[cfg(test)]
mod tests;

pub(crate) async fn get_central_skill_update_states_impl(
    pool: &DbPool,
) -> Result<Vec<SkillUpdateState>, CentralUpdatesError> {
    Ok(db::get_skill_update_states(pool).await?)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn check_central_skill_updates_impl(
    app: Option<&AppHandle>,
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

    cancel.store(false, Ordering::SeqCst);
    emit_update_progress(app, "checking", "started", total, &counters, None, None);

    let prepared = prepare_skill_updates(pool, fs, skills, auth_token, false).await?;
    let snapshots = prepare_snapshots(client, auth_token, &prepared, snapshots_cache).await?;

    for prepared_skill in prepared {
        let skill = &prepared_skill.skill;
        if cancel.load(Ordering::SeqCst) {
            emit_update_progress(
                app,
                "checking",
                SkillUpdateStatus::Cancelled.as_str(),
                total,
                &counters,
                None,
                None,
            );
            return Ok(states);
        }

        emit_update_progress(app, "checking", "running", total, &counters, Some(skill), None);

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
            "checking",
            &state_result.status,
            total,
            &counters,
            Some(skill),
            state_result.error.as_deref(),
        );
        states.push(state_result);
    }

    emit_update_progress(app, "checking", "completed", total, &counters, None, None);

    Ok(states)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn update_central_skills_impl(
    app: Option<&AppHandle>,
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

    cancel.store(false, Ordering::SeqCst);
    emit_update_progress(app, "updating", "started", total, &counters, None, None);

    let prepared = prepare_skill_updates(pool, fs, skills, auth_token, true).await?;
    let snapshots = prepare_snapshots(client, auth_token, &prepared, snapshots_cache).await?;

    for prepared_skill in prepared {
        let skill = &prepared_skill.skill;
        if cancel.load(Ordering::SeqCst) {
            emit_update_progress(
                app,
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

        emit_update_progress(app, "updating", "running", total, &counters, Some(skill), None);

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
                    "updating",
                    SkillUpdateStatus::UpToDate.as_str(),
                    total,
                    &counters,
                    Some(skill),
                    None,
                );
                states.push(state_result);
            }
            Ok(Some(remote)) => match update_one_skill(pool, fs, skill, remote).await {
                Ok(state_result) => {
                    db::upsert_skill_update_state(pool, &state_result).await?;
                    counters.completed += 1;
                    counters.succeeded += 1;
                    succeeded.push(skill.id.clone());
                    emit_update_progress(
                        app,
                        "updating",
                        SkillUpdateStatus::UpToDate.as_str(),
                        total,
                        &counters,
                        Some(skill),
                        None,
                    );
                    states.push(state_result);
                }
                Err(error) => {
                    let error = error.to_string();
                    let state_result =
                        error_state_from_assignment(skill, &prepared_skill.assignment, &error);
                    db::upsert_skill_update_state(pool, &state_result).await?;
                    counters.completed += 1;
                    counters.failed += 1;
                    failed.push(CentralSkillUpdateFailure {
                        skill_id: skill.id.clone(),
                        error: error.clone(),
                    });
                    emit_update_progress(
                        app,
                        "updating",
                        SkillUpdateStatus::Error.as_str(),
                        total,
                        &counters,
                        Some(skill),
                        Some(&error),
                    );
                    states.push(state_result);
                }
            },
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
                let state_result =
                    remote_missing_state_from_assignment(skill, &prepared_skill.assignment, &reason);
                db::upsert_skill_update_state(pool, &state_result).await?;
                counters.completed += 1;
                counters.skipped += 1;
                skipped.push(CentralSkillUpdateSkip {
                    skill_id: skill.id.clone(),
                    reason: reason.clone(),
                });
                emit_update_progress(
                    app,
                    "updating",
                    SkillUpdateStatus::RemoteMissing.as_str(),
                    total,
                    &counters,
                    Some(skill),
                    Some(&reason),
                );
                states.push(state_result);
            }
            Err(RemoteSkillLoadError::Other(error)) => {
                let state_result =
                    error_state_from_assignment(skill, &prepared_skill.assignment, &error);
                db::upsert_skill_update_state(pool, &state_result).await?;
                counters.completed += 1;
                counters.failed += 1;
                failed.push(CentralSkillUpdateFailure {
                    skill_id: skill.id.clone(),
                    error: error.clone(),
                });
                emit_update_progress(
                    app,
                    "updating",
                    SkillUpdateStatus::Error.as_str(),
                    total,
                    &counters,
                    Some(skill),
                    Some(&error),
                );
                states.push(state_result);
            }
        }
    }

    emit_update_progress(app, "updating", "completed", total, &counters, None, None);

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
        if update_state.status != SkillUpdateStatus::RemoteMissing.as_str() {
            return Err(CentralUpdatesError::NotRemoteMissing(skill_id.clone()));
        }
    }

    for skill_id in &unique_skill_ids {
        db::detach_skill_remote_source(pool, skill_id).await?;
    }

    Ok(unique_skill_ids)
}

pub(crate) async fn load_selected_central_skills(
    pool: &DbPool,
    skill_ids: Option<&[String]>,
) -> Result<Vec<Skill>, CentralUpdatesError> {
    if let Some(skill_ids) = skill_ids {
        return Ok(db::get_central_skills_by_ids(pool, skill_ids).await?);
    }
    Ok(db::get_central_skills(pool).await?)
}

pub(crate) async fn prepare_skill_updates(
    pool: &DbPool,
    fs: &CentralFs,
    skills: Vec<Skill>,
    auth_token: Option<&str>,
    allow_fresh_state_reuse: bool,
) -> Result<Vec<PreparedSkillUpdate>, CentralUpdatesError> {
    let skill_ids = skills
        .iter()
        .map(|skill| skill.id.clone())
        .collect::<Vec<_>>();
    let mut assignments =
        db::get_skill_repository_assignments_for_skills(pool, &skill_ids).await?;
    let unknown_repository = db::get_local_unknown_repository(pool).await?;
    let previous_states = db::get_skill_update_states_for_skills(pool, &skill_ids)
        .await?
        .into_iter()
        .map(|state| (state.skill_id.clone(), state))
        .collect::<HashMap<_, _>>();

    let mut prepared = Vec::with_capacity(skills.len());
    let mut roots_to_hash = Vec::new();
    let mut skill_ids_to_hash = Vec::new();

    for skill in skills {
        let assignment =
            assignments
                .remove(&skill.id)
                .unwrap_or_else(|| SkillRepositoryAssignment {
                    repository: unknown_repository.clone(),
                    source_path: None,
                    is_source_unknown: true,
                });
        let source =
            resolve_github_update_source_from_assignment(&skill, &assignment, auth_token).await?;
        let previous_state = previous_states.get(&skill.id).cloned();
        let target_dir = if source.is_some() {
            Some(skill_target_dir(&skill)?)
        } else {
            None
        };
        let reuse_previous_local_hash = allow_fresh_state_reuse
            && previous_state
                .as_ref()
                .is_some_and(is_fresh_update_available_state);

        if source.is_some() && !reuse_previous_local_hash {
            if let Some(target_dir) = &target_dir {
                roots_to_hash.push(target_dir.clone());
                skill_ids_to_hash.push(skill.id.clone());
            }
        }

        prepared.push(PreparedSkillUpdate {
            skill,
            source,
            assignment,
            target_dir,
            previous_state,
            reuse_previous_local_hash,
            local_hash: None,
        });
    }

    let hashes = fs.hash_directories(&roots_to_hash).await?;
    let hash_by_skill_id = skill_ids_to_hash
        .into_iter()
        .zip(roots_to_hash)
        .filter_map(|(skill_id, root)| hashes.get(&root).cloned().map(|hash| (skill_id, hash)))
        .collect::<HashMap<_, _>>();

    for prepared_skill in &mut prepared {
        if let Some(hash) = hash_by_skill_id.get(&prepared_skill.skill.id) {
            prepared_skill.local_hash = Some(hash.clone());
        }
    }

    Ok(prepared)
}

pub(crate) fn load_remote_skill_content(
    prepared: &PreparedSkillUpdate,
    snapshots: &HashMap<String, GitHubRepoSnapshot>,
) -> Result<Option<RemoteSkillContent>, RemoteSkillLoadError> {
    let skill = &prepared.skill;
    let Some(source) = prepared.source.clone() else {
        return Ok(None);
    };

    let snapshot = snapshots
        .get(&repo_cache_key(&source.repo))
        .ok_or_else(|| RemoteSkillLoadError::other("GitHub repository snapshot is unavailable."))?;

    let candidate = find_remote_skill_candidate(&source, snapshot)?;

    let files = collect_remote_skill_files(snapshot, &source.source_path)
        .map_err(|e| RemoteSkillLoadError::remote_missing(e.to_string()))?;
    ensure_remote_skill_manifest(&files)
        .map_err(|e| RemoteSkillLoadError::remote_missing(e.to_string()))?;

    let remote_hash = hash_remote_files(snapshot, &files)
        .map_err(|e| RemoteSkillLoadError::other(e.to_string()))?;
    let target_dir = prepared.target_dir.clone().ok_or_else(|| {
        RemoteSkillLoadError::other(format!("Skill '{}' has no target directory.", skill.id))
    })?;
    let local_hash = reused_or_prepared_local_hash(prepared).ok_or_else(|| {
        RemoteSkillLoadError::other(format!("Skill '{}' local hash is unavailable.", skill.id))
    })?;

    Ok(Some(RemoteSkillContent {
        source,
        candidate,
        files,
        remote_hash,
        local_hash,
        target_dir,
    }))
}

fn reused_or_prepared_local_hash(prepared: &PreparedSkillUpdate) -> Option<String> {
    if prepared.reuse_previous_local_hash {
        return prepared
            .previous_state
            .as_ref()
            .and_then(|state| state.last_remote_hash.clone());
    }
    prepared.local_hash.clone()
}

fn find_remote_skill_candidate(
    source: &GitHubUpdateSource,
    snapshot: &GitHubRepoSnapshot,
) -> Result<crate::services::github_import::RemoteSkillCandidate, RemoteSkillLoadError> {
    let candidates = github_import::build_repo_skill_candidates_from_snapshot_at_path(
        &source.repo,
        snapshot,
        Some(&source.source_path),
    )
    .map_err(|e| RemoteSkillLoadError::other(e.to_string()))?;

    candidates
        .into_iter()
        .find(|candidate| {
            normalize_repo_path(&candidate.source_path)
                .map(|path| path == source.source_path)
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            RemoteSkillLoadError::remote_missing(format!(
                "Skill source path '{}' no longer contains an importable skill.",
                source.source_path
            ))
        })
}

async fn resolve_github_update_source_from_assignment(
    skill: &Skill,
    assignment: &SkillRepositoryAssignment,
    auth_token: Option<&str>,
) -> Result<Option<GitHubUpdateSource>, CentralUpdatesError> {
    if assignment.is_source_unknown || assignment.repository.is_unknown {
        return Ok(None);
    }
    if assignment.repository.source_type != "github" {
        return Ok(None);
    }

    let Some(source_path) = assignment
        .source_path
        .as_deref()
        .map(normalize_repo_path)
        .transpose()?
        .filter(|path| !path.is_empty())
    else {
        return Ok(None);
    };

    let repo = if let (Some(owner), Some(repo), Some(branch)) = (
        assignment.repository.owner.as_ref(),
        assignment.repository.repo.as_ref(),
        assignment.repository.branch.as_ref(),
    ) {
        GitHubRepoRef {
            owner: owner.clone(),
            repo: repo.clone(),
            branch: branch.clone(),
            normalized_url: assignment
                .repository
                .url
                .clone()
                .unwrap_or_else(|| format!("https://github.com/{owner}/{repo}")),
        }
    } else {
        let url = repository_url(assignment)
            .ok_or_else(|| CentralUpdatesError::MissingRepositoryUrl(skill.id.clone()))?;
        github_import::resolve_repo_source(&url, auth_token).await?.repo
    };

    Ok(Some(GitHubUpdateSource { repo, source_path }))
}

pub(crate) fn repository_url(assignment: &SkillRepositoryAssignment) -> Option<String> {
    assignment.repository.url.clone().or_else(|| {
        match (&assignment.repository.owner, &assignment.repository.repo) {
            (Some(owner), Some(repo)) => Some(format!("https://github.com/{owner}/{repo}")),
            _ => None,
        }
    })
}

pub(crate) fn state_from_remote(
    skill: &Skill,
    remote: &RemoteSkillContent,
    updated: bool,
) -> SkillUpdateState {
    let now = Utc::now().to_rfc3339();
    let status = if remote.remote_hash == remote.local_hash {
        SkillUpdateStatus::UpToDate
    } else {
        SkillUpdateStatus::UpdateAvailable
    };

    SkillUpdateState {
        skill_id: skill.id.clone(),
        source_type: "github".to_string(),
        source_url: Some(remote.source.repo.normalized_url.clone()),
        ref_name: Some(remote.source.repo.branch.clone()),
        source_path: Some(remote.source.source_path.clone()),
        last_remote_hash: Some(if updated {
            remote.remote_hash.clone()
        } else {
            remote.local_hash.clone()
        }),
        latest_remote_hash: Some(remote.remote_hash.clone()),
        last_checked_at: Some(now.clone()),
        last_updated_at: if updated { Some(now) } else { None },
        status: if updated {
            SkillUpdateStatus::UpToDate.to_string()
        } else {
            status.to_string()
        },
        error: None,
    }
}

pub(crate) fn state_from_relocated_source(
    prepared: &PreparedSkillUpdate,
    repo: &GitHubRepoRef,
    source_path: &str,
    snapshots: &HashMap<String, GitHubRepoSnapshot>,
) -> Result<SkillUpdateState, RemoteSkillLoadError> {
    let mut relocated = prepared.clone();
    relocated.source = Some(GitHubUpdateSource {
        repo: repo.clone(),
        source_path: source_path.to_string(),
    });
    relocated.assignment.source_path = Some(source_path.to_string());
    let remote = load_remote_skill_content(&relocated, snapshots)?
        .ok_or_else(|| RemoteSkillLoadError::other("Relocated GitHub source is unavailable."))?;
    Ok(state_from_remote(&relocated.skill, &remote, false))
}

pub(crate) fn unsupported_state_from_assignment(
    skill: &Skill,
    assignment: &SkillRepositoryAssignment,
    reason: Option<&str>,
) -> SkillUpdateState {
    let source_url = repository_url(assignment);
    let source_type = assignment.repository.source_type.clone();
    let ref_name = assignment.repository.branch.clone();
    let source_path = assignment.source_path.clone();
    let reason = reason
        .map(str::to_string)
        .unwrap_or_else(|| unsupported_reason(assignment));

    SkillUpdateState {
        skill_id: skill.id.clone(),
        source_type,
        source_url,
        ref_name,
        source_path,
        last_remote_hash: None,
        latest_remote_hash: None,
        last_checked_at: Some(Utc::now().to_rfc3339()),
        last_updated_at: None,
        status: SkillUpdateStatus::Unsupported.to_string(),
        error: Some(reason),
    }
}

fn is_fresh_update_available_state(state: &SkillUpdateState) -> bool {
    if state.status != SkillUpdateStatus::UpdateAvailable.as_str() {
        return false;
    }
    let Some(last_checked_at) = state.last_checked_at.as_deref() else {
        return false;
    };
    let Ok(last_checked_at) = chrono::DateTime::parse_from_rfc3339(last_checked_at) else {
        return false;
    };
    state.last_remote_hash.is_some()
        && Utc::now().signed_duration_since(last_checked_at.with_timezone(&Utc))
            <= snapshot_cache_ttl()
}

pub(crate) fn remote_missing_state_from_assignment(
    skill: &Skill,
    assignment: &SkillRepositoryAssignment,
    reason: &str,
) -> SkillUpdateState {
    let source_url = repository_url(assignment);
    SkillUpdateState {
        skill_id: skill.id.clone(),
        source_type: assignment.repository.source_type.clone(),
        source_url,
        ref_name: assignment.repository.branch.clone(),
        source_path: assignment.source_path.clone(),
        last_remote_hash: None,
        latest_remote_hash: None,
        last_checked_at: Some(Utc::now().to_rfc3339()),
        last_updated_at: None,
        status: SkillUpdateStatus::RemoteMissing.to_string(),
        error: Some(reason.to_string()),
    }
}

pub(crate) fn error_state_from_assignment(
    skill: &Skill,
    assignment: &SkillRepositoryAssignment,
    error: &str,
) -> SkillUpdateState {
    let source_url = repository_url(assignment);
    SkillUpdateState {
        skill_id: skill.id.clone(),
        source_type: assignment.repository.source_type.clone(),
        source_url,
        ref_name: assignment.repository.branch.clone(),
        source_path: assignment.source_path.clone(),
        last_remote_hash: None,
        latest_remote_hash: None,
        last_checked_at: Some(Utc::now().to_rfc3339()),
        last_updated_at: None,
        status: SkillUpdateStatus::Error.to_string(),
        error: Some(error.to_string()),
    }
}

fn unsupported_reason(assignment: &SkillRepositoryAssignment) -> String {
    if assignment.is_source_unknown || assignment.repository.is_unknown {
        return "Source is unknown or manually assigned.".to_string();
    }
    if assignment.repository.source_type != "github" {
        return format!(
            "Source type '{}' is not supported for automatic updates.",
            assignment.repository.source_type
        );
    }
    if assignment
        .source_path
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        return "GitHub source path is missing.".to_string();
    }
    "Automatic update is not supported for this source.".to_string()
}

pub(crate) async fn update_one_skill(
    pool: &DbPool,
    fs: &CentralFs,
    skill: &Skill,
    remote: RemoteSkillContent,
) -> Result<SkillUpdateState, CentralUpdatesError> {
    update_one_skill_with_options(pool, fs, skill, remote, true).await
}

pub(crate) async fn update_one_skill_with_options(
    pool: &DbPool,
    fs: &CentralFs,
    skill: &Skill,
    remote: RemoteSkillContent,
    refresh_copies: bool,
) -> Result<SkillUpdateState, CentralUpdatesError> {
    fs.write_skill_dir_atomic(&skill.id, &remote.target_dir, &remote.files)
        .await?;

    let skill_md_path = remote.target_dir.join("SKILL.md");
    let updated_skill = Skill {
        id: skill.id.clone(),
        name: remote.candidate.skill_name.clone(),
        description: remote.candidate.description.clone(),
        file_path: skill_md_path.to_string_lossy().into_owned(),
        canonical_path: Some(remote.target_dir.to_string_lossy().into_owned()),
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
    db::upsert_skill(pool, &updated_skill).await?;
    db::assign_github_repository_to_skill(
        pool,
        &remote.source.repo.owner,
        &remote.source.repo.repo,
        &remote.source.repo.branch,
        &remote.source.repo.normalized_url,
        &skill.id,
        &remote.source.source_path,
    )
    .await?;
    if refresh_copies {
        refresh_copy_installations(pool, fs, &skill.id, &remote.target_dir).await?;
    }

    Ok(state_from_remote(skill, &remote, true))
}

async fn refresh_copy_installations(
    pool: &DbPool,
    fs: &CentralFs,
    skill_id: &str,
    source_dir: &Path,
) -> Result<(), CentralUpdatesError> {
    let installations = db::get_skill_installations(pool, skill_id).await?;
    let mut seen_targets = HashSet::new();
    let copy_targets = installations
        .into_iter()
        .filter(|installation| installation.link_type == "copy")
        .filter_map(|installation| {
            if seen_targets.insert(installation.installed_path.clone()) {
                Some(installation.installed_path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut results = futures_util::stream::iter(copy_targets)
        .map(|target| async move { fs.refresh_copy_install(skill_id, source_dir, &target).await })
        .buffer_unordered(COPY_INSTALL_REFRESH_CONCURRENCY);

    while let Some(result) = futures_util::StreamExt::next(&mut results).await {
        result?;
    }
    Ok(())
}

fn skill_target_dir(skill: &Skill) -> Result<PathBuf, CentralUpdatesError> {
    skill
        .canonical_path
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| Path::new(&skill.file_path).parent().map(Path::to_path_buf))
        .ok_or_else(|| CentralUpdatesError::NoCanonicalDirectory(skill.id.clone()))
}

pub(crate) fn update_counters_for_state(counters: &mut UpdateCounters, state: &SkillUpdateState) {
    counters.completed += 1;
    let parsed = state.status.parse::<SkillUpdateStatus>().ok();
    match parsed {
        Some(SkillUpdateStatus::UpToDate) | Some(SkillUpdateStatus::UpdateAvailable) => {
            counters.succeeded += 1;
        }
        Some(SkillUpdateStatus::Unsupported) | Some(SkillUpdateStatus::RemoteMissing) => {
            counters.skipped += 1;
        }
        Some(SkillUpdateStatus::Error) => counters.failed += 1,
        Some(SkillUpdateStatus::Cancelled) | None => {}
    }
}

pub(crate) fn emit_update_progress(
    app: Option<&AppHandle>,
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
