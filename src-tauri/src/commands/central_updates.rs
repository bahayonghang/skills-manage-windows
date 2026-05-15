use chrono::{Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Semaphore;

use crate::{
    db::{self, DbPool, Skill, SkillRepositoryAssignment, SkillUpdateState},
    AppState,
};

use super::central_updates_fs::{
    collect_remote_skill_files, ensure_remote_skill_manifest, hash_remote_files,
    normalize_repo_path, CentralFs, RemoteSkillFile,
};
use super::github_import::{self, GitHubRepoRef, GitHubRepoSnapshot, RemoteSkillCandidate};

const UPDATE_PROGRESS_EVENT: &str = "central://skill-update-progress";
const STATUS_UP_TO_DATE: &str = "up_to_date";
const STATUS_UPDATE_AVAILABLE: &str = "update_available";
const STATUS_UNSUPPORTED: &str = "unsupported";
const STATUS_REMOTE_MISSING: &str = "remote_missing";
const STATUS_ERROR: &str = "error";
const STATUS_CANCELLED: &str = "cancelled";
fn snapshot_cache_ttl() -> ChronoDuration {
    ChronoDuration::minutes(10)
}
const SNAPSHOT_DOWNLOAD_CONCURRENCY: usize = 4;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillUpdateProgressPayload {
    pub phase: String,
    pub status: String,
    pub total: usize,
    pub completed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillUpdateFailure {
    pub skill_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillUpdateSkip {
    pub skill_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillUpdateResult {
    pub succeeded: Vec<String>,
    pub failed: Vec<CentralSkillUpdateFailure>,
    pub skipped: Vec<CentralSkillUpdateSkip>,
    pub states: Vec<SkillUpdateState>,
}

#[derive(Debug, Clone)]
struct GitHubUpdateSource {
    repo: GitHubRepoRef,
    source_path: String,
}

#[derive(Debug, Clone)]
struct PreparedSkillUpdate {
    skill: Skill,
    source: Option<GitHubUpdateSource>,
    assignment: SkillRepositoryAssignment,
    target_dir: Option<PathBuf>,
    previous_state: Option<SkillUpdateState>,
    reuse_previous_local_hash: bool,
    local_hash: Option<String>,
}

#[derive(Debug, Clone)]
struct RemoteSkillContent {
    source: GitHubUpdateSource,
    candidate: RemoteSkillCandidate,
    files: Vec<RemoteSkillFile>,
    remote_hash: String,
    local_hash: String,
    target_dir: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct UpdateCounters {
    completed: usize,
    succeeded: usize,
    failed: usize,
    skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RemoteSkillLoadError {
    RemoteMissing(String),
    Other(String),
}

impl RemoteSkillLoadError {
    fn remote_missing(message: impl Into<String>) -> Self {
        Self::RemoteMissing(message.into())
    }

    fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }

    #[cfg(test)]
    fn message(&self) -> &str {
        match self {
            Self::RemoteMissing(message) | Self::Other(message) => message,
        }
    }
}

#[tauri::command]
pub async fn get_central_skill_update_states(
    state: State<'_, AppState>,
) -> Result<Vec<SkillUpdateState>, String> {
    let pool = state.active_db().await?;
    db::get_skill_update_states(&pool).await
}

#[tauri::command]
pub async fn check_central_skill_updates(
    app: AppHandle,
    state: State<'_, AppState>,
    skill_ids: Option<Vec<String>>,
) -> Result<Vec<SkillUpdateState>, String> {
    let pool = state.active_db().await?;
    let fs = CentralFs::from_active_target(state.active_target().await?).await?;
    let skills = load_selected_central_skills(&pool, skill_ids.as_deref()).await?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await?;
    let client = github_import::github_client()?;
    let total = skills.len();
    let mut counters = UpdateCounters::default();
    let mut states = Vec::with_capacity(total);

    state.central_update_cancel.store(false, Ordering::SeqCst);
    emit_update_progress(&app, "checking", "started", total, &counters, None, None);

    let prepared = prepare_skill_updates(&pool, &fs, skills, auth.as_deref(), false).await?;
    let snapshots = prepare_snapshots(
        &client,
        auth.as_deref(),
        &prepared,
        &state.central_update_snapshots,
    )
    .await?;

    for prepared_skill in prepared {
        let skill = &prepared_skill.skill;
        if state.central_update_cancel.load(Ordering::SeqCst) {
            emit_update_progress(
                &app,
                "checking",
                STATUS_CANCELLED,
                total,
                &counters,
                None,
                None,
            );
            return Ok(states);
        }

        emit_update_progress(
            &app,
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

        db::upsert_skill_update_state(&pool, &state_result).await?;
        update_counters_for_state(&mut counters, &state_result);
        emit_update_progress(
            &app,
            "checking",
            &state_result.status,
            total,
            &counters,
            Some(skill),
            state_result.error.as_deref(),
        );
        states.push(state_result);
    }

    emit_update_progress(&app, "checking", "completed", total, &counters, None, None);

    Ok(states)
}

#[tauri::command]
pub async fn update_central_skills(
    app: AppHandle,
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
) -> Result<CentralSkillUpdateResult, String> {
    if skill_ids.is_empty() {
        return Err("Select at least one Central skill to update.".to_string());
    }
    let pool = state.active_db().await?;
    let fs = CentralFs::from_active_target(state.active_target().await?).await?;

    let skills = load_selected_central_skills(&pool, Some(&skill_ids)).await?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await?;
    let client = github_import::github_client()?;
    let total = skills.len();
    let mut counters = UpdateCounters::default();
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    let mut skipped = Vec::new();
    let mut states = Vec::new();

    state.central_update_cancel.store(false, Ordering::SeqCst);
    emit_update_progress(&app, "updating", "started", total, &counters, None, None);

    let prepared = prepare_skill_updates(&pool, &fs, skills, auth.as_deref(), true).await?;
    let snapshots = prepare_snapshots(
        &client,
        auth.as_deref(),
        &prepared,
        &state.central_update_snapshots,
    )
    .await?;

    for prepared_skill in prepared {
        let skill = &prepared_skill.skill;
        if state.central_update_cancel.load(Ordering::SeqCst) {
            emit_update_progress(
                &app,
                "updating",
                STATUS_CANCELLED,
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
            &app,
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
                db::upsert_skill_update_state(&pool, &state_result).await?;
                counters.completed += 1;
                counters.skipped += 1;
                skipped.push(CentralSkillUpdateSkip {
                    skill_id: skill.id.clone(),
                    reason: "Already up to date".to_string(),
                });
                emit_update_progress(
                    &app,
                    "updating",
                    STATUS_UP_TO_DATE,
                    total,
                    &counters,
                    Some(skill),
                    None,
                );
                states.push(state_result);
            }
            Ok(Some(remote)) => match update_one_skill(&pool, &fs, skill, remote).await {
                Ok(state_result) => {
                    db::upsert_skill_update_state(&pool, &state_result).await?;
                    counters.completed += 1;
                    counters.succeeded += 1;
                    succeeded.push(skill.id.clone());
                    emit_update_progress(
                        &app,
                        "updating",
                        STATUS_UP_TO_DATE,
                        total,
                        &counters,
                        Some(skill),
                        None,
                    );
                    states.push(state_result);
                }
                Err(error) => {
                    let state_result =
                        error_state_from_assignment(skill, &prepared_skill.assignment, &error);
                    db::upsert_skill_update_state(&pool, &state_result).await?;
                    counters.completed += 1;
                    counters.failed += 1;
                    failed.push(CentralSkillUpdateFailure {
                        skill_id: skill.id.clone(),
                        error: error.clone(),
                    });
                    emit_update_progress(
                        &app,
                        "updating",
                        STATUS_ERROR,
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
                db::upsert_skill_update_state(&pool, &state_result).await?;
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
                    &app,
                    "updating",
                    STATUS_UNSUPPORTED,
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
                db::upsert_skill_update_state(&pool, &state_result).await?;
                counters.completed += 1;
                counters.skipped += 1;
                skipped.push(CentralSkillUpdateSkip {
                    skill_id: skill.id.clone(),
                    reason: reason.clone(),
                });
                emit_update_progress(
                    &app,
                    "updating",
                    STATUS_REMOTE_MISSING,
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
                db::upsert_skill_update_state(&pool, &state_result).await?;
                counters.completed += 1;
                counters.failed += 1;
                failed.push(CentralSkillUpdateFailure {
                    skill_id: skill.id.clone(),
                    error: error.clone(),
                });
                emit_update_progress(
                    &app,
                    "updating",
                    STATUS_ERROR,
                    total,
                    &counters,
                    Some(skill),
                    Some(&error),
                );
                states.push(state_result);
            }
        }
    }

    emit_update_progress(&app, "updating", "completed", total, &counters, None, None);

    Ok(CentralSkillUpdateResult {
        succeeded,
        failed,
        skipped,
        states,
    })
}

#[tauri::command]
pub async fn cancel_central_skill_updates(state: State<'_, AppState>) -> Result<(), String> {
    state.central_update_cancel.store(true, Ordering::SeqCst);
    Ok(())
}

pub async fn keep_remote_missing_central_skills_impl(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<Vec<String>, String> {
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
            .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
        if !skill.is_central {
            return Err(format!("Skill '{}' is not a Central skill", skill_id));
        }

        let update_state = states_by_skill_id.get(skill_id).ok_or_else(|| {
            format!(
                "Skill '{}' does not have a remote-missing update state.",
                skill_id
            )
        })?;
        if update_state.status != STATUS_REMOTE_MISSING {
            return Err(format!(
                "Skill '{}' is not marked as removed remotely.",
                skill_id
            ));
        }
    }

    for skill_id in &unique_skill_ids {
        db::detach_skill_remote_source(pool, skill_id).await?;
    }

    Ok(unique_skill_ids)
}

#[tauri::command]
pub async fn keep_remote_missing_central_skills(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
) -> Result<Vec<String>, String> {
    let pool = state.active_db().await?;
    keep_remote_missing_central_skills_impl(&pool, &skill_ids).await
}

async fn load_selected_central_skills(
    pool: &DbPool,
    skill_ids: Option<&[String]>,
) -> Result<Vec<Skill>, String> {
    if let Some(skill_ids) = skill_ids {
        return db::get_central_skills_by_ids(pool, skill_ids).await;
    }
    db::get_central_skills(pool).await
}

async fn prepare_skill_updates(
    pool: &DbPool,
    fs: &CentralFs,
    skills: Vec<Skill>,
    auth_token: Option<&str>,
    allow_fresh_state_reuse: bool,
) -> Result<Vec<PreparedSkillUpdate>, String> {
    let skill_ids = skills
        .iter()
        .map(|skill| skill.id.clone())
        .collect::<Vec<_>>();
    let mut assignments = db::get_skill_repository_assignments_for_skills(pool, &skill_ids).await?;
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

async fn prepare_snapshots(
    client: &reqwest::Client,
    auth_token: Option<&str>,
    prepared: &[PreparedSkillUpdate],
    cache: &crate::CentralUpdateSnapshotCache,
) -> Result<HashMap<String, GitHubRepoSnapshot>, String> {
    let mut repos = HashMap::<String, GitHubRepoRef>::new();
    for prepared_skill in prepared {
        if let Some(source) = &prepared_skill.source {
            repos
                .entry(repo_cache_key(&source.repo))
                .or_insert_with(|| source.repo.clone());
        }
    }

    let mut snapshots = HashMap::new();
    let mut missing = Vec::new();
    for (key, repo) in repos {
        if let Some(snapshot) = cache.get_fresh(&key, snapshot_cache_ttl()) {
            snapshots.insert(key, snapshot);
        } else {
            missing.push(repo);
        }
    }

    let semaphore = Arc::new(Semaphore::new(SNAPSHOT_DOWNLOAD_CONCURRENCY));
    let auth = auth_token.map(str::to_string);
    let downloads = missing.into_iter().map(|repo| {
        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);
        let auth = auth.clone();
        async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .map_err(|_| "Central update snapshot downloader closed.".to_string())?;
            let snapshot =
                github_import::download_repo_snapshot(&client, &repo, auth.as_deref()).await?;
            Ok::<_, String>((repo_cache_key(&repo), snapshot))
        }
    });

    for result in futures_util::future::join_all(downloads).await {
        let (key, snapshot) = result?;
        cache.insert(key.clone(), snapshot.clone());
        snapshots.insert(key, snapshot);
    }

    Ok(snapshots)
}

fn load_remote_skill_content(
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
        .map_err(RemoteSkillLoadError::remote_missing)?;
    ensure_remote_skill_manifest(&files).map_err(RemoteSkillLoadError::remote_missing)?;

    let remote_hash = hash_remote_files(snapshot, &files).map_err(RemoteSkillLoadError::other)?;
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
) -> Result<RemoteSkillCandidate, RemoteSkillLoadError> {
    let candidates = github_import::build_repo_skill_candidates_from_snapshot_at_path(
        &source.repo,
        snapshot,
        Some(&source.source_path),
    )
    .map_err(RemoteSkillLoadError::other)?;

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
) -> Result<Option<GitHubUpdateSource>, String> {
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
        let url = repository_url(assignment).ok_or_else(|| {
            format!(
                "GitHub source for skill '{}' is missing repository URL.",
                skill.id
            )
        })?;
        github_import::resolve_repo_source(&url, auth_token)
            .await?
            .repo
    };

    Ok(Some(GitHubUpdateSource { repo, source_path }))
}

fn repository_url(assignment: &SkillRepositoryAssignment) -> Option<String> {
    assignment.repository.url.clone().or_else(|| {
        match (&assignment.repository.owner, &assignment.repository.repo) {
            (Some(owner), Some(repo)) => Some(format!("https://github.com/{owner}/{repo}")),
            _ => None,
        }
    })
}

fn repo_cache_key(repo: &GitHubRepoRef) -> String {
    format!("{}/{}/{}", repo.owner, repo.repo, repo.branch)
}

fn state_from_remote(
    skill: &Skill,
    remote: &RemoteSkillContent,
    updated: bool,
) -> SkillUpdateState {
    let now = Utc::now().to_rfc3339();
    let status = if remote.remote_hash == remote.local_hash {
        STATUS_UP_TO_DATE
    } else {
        STATUS_UPDATE_AVAILABLE
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
            STATUS_UP_TO_DATE.to_string()
        } else {
            status.to_string()
        },
        error: None,
    }
}

fn unsupported_state_from_assignment(
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
        status: STATUS_UNSUPPORTED.to_string(),
        error: Some(reason),
    }
}

fn is_fresh_update_available_state(state: &SkillUpdateState) -> bool {
    if state.status != STATUS_UPDATE_AVAILABLE {
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

fn remote_missing_state_from_assignment(
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
        status: STATUS_REMOTE_MISSING.to_string(),
        error: Some(reason.to_string()),
    }
}

fn error_state_from_assignment(
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
        status: STATUS_ERROR.to_string(),
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

async fn update_one_skill(
    pool: &DbPool,
    fs: &CentralFs,
    skill: &Skill,
    remote: RemoteSkillContent,
) -> Result<SkillUpdateState, String> {
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
    refresh_copy_installations(pool, fs, &skill.id, &remote.target_dir).await?;

    Ok(state_from_remote(skill, &remote, true))
}

async fn refresh_copy_installations(
    pool: &DbPool,
    fs: &CentralFs,
    skill_id: &str,
    source_dir: &Path,
) -> Result<(), String> {
    let installations = db::get_skill_installations(pool, skill_id).await?;
    for installation in installations {
        if installation.link_type != "copy" {
            continue;
        }
        fs.refresh_copy_install(skill_id, source_dir, &installation.installed_path)
            .await?;
    }
    Ok(())
}

fn skill_target_dir(skill: &Skill) -> Result<PathBuf, String> {
    skill
        .canonical_path
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| Path::new(&skill.file_path).parent().map(Path::to_path_buf))
        .ok_or_else(|| format!("Skill '{}' has no canonical directory.", skill.id))
}

fn update_counters_for_state(counters: &mut UpdateCounters, state: &SkillUpdateState) {
    counters.completed += 1;
    match state.status.as_str() {
        STATUS_UP_TO_DATE | STATUS_UPDATE_AVAILABLE => counters.succeeded += 1,
        STATUS_UNSUPPORTED | STATUS_REMOTE_MISSING => counters.skipped += 1,
        STATUS_ERROR => counters.failed += 1,
        _ => {}
    }
}

fn emit_update_progress(
    app: &AppHandle,
    phase: &str,
    status: &str,
    total: usize,
    counters: &UpdateCounters,
    skill: Option<&Skill>,
    error: Option<&str>,
) {
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

#[cfg(test)]
mod tests;
