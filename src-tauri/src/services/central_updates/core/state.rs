//! Source resolution, remote content loading, and update-state construction.

use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::db::{self, DbPool, Skill, SkillRepositoryAssignment, SkillUpdateState};
use crate::services::github_import::{self, GitHubRepoRef, GitHubRepoSnapshot};

use super::super::error::CentralUpdatesError;
use super::super::fs::{
    collect_remote_skill_files, ensure_remote_skill_manifest, hash_remote_files,
    normalize_repo_path, CentralFs,
};
use super::super::snapshots::{repo_cache_key, snapshot_cache_ttl, SharedGitHubSnapshots};
use super::super::types::{
    normalized_github_source_path, unsupported_reason_code, GitHubUpdateSource,
    PreparedSkillUpdate, RemoteSkillContent, RemoteSkillLoadError, SkillUpdateStatus,
    UnsupportedSkillReasonCode,
};

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

pub(crate) fn load_remote_skill_content(
    prepared: &PreparedSkillUpdate,
    snapshots: &SharedGitHubSnapshots,
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
        resolved_commit_sha: None,
        content_digest: None,
    }))
}

pub(super) fn reused_or_prepared_local_hash(prepared: &PreparedSkillUpdate) -> Option<String> {
    if prepared.reuse_previous_local_hash {
        return prepared
            .previous_state
            .as_ref()
            .and_then(|state| state.last_remote_hash.clone());
    }
    prepared.local_hash.clone()
}

pub(super) fn find_remote_skill_candidate(
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

    let Some(source_path) = normalized_github_source_path(assignment) else {
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
        github_import::resolve_repo_source(&url, auth_token)
            .await?
            .repo
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
            SkillUpdateStatus::UpToDate
        } else {
            status
        },
        error: None,
    }
}

pub(crate) fn state_from_relocated_source(
    prepared: &PreparedSkillUpdate,
    repo: &GitHubRepoRef,
    source_path: &str,
    snapshots: &SharedGitHubSnapshots,
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
        status: SkillUpdateStatus::Unsupported,
        error: Some(reason),
    }
}

pub(super) fn is_fresh_update_available_state(state: &SkillUpdateState) -> bool {
    if state.status != SkillUpdateStatus::UpdateAvailable {
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
        status: SkillUpdateStatus::RemoteMissing,
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
        status: SkillUpdateStatus::Error,
        error: Some(error.to_string()),
    }
}

fn unsupported_reason(assignment: &SkillRepositoryAssignment) -> String {
    match unsupported_reason_code(assignment) {
        UnsupportedSkillReasonCode::UnknownSource => {
            "Source is unknown or manually assigned.".to_string()
        }
        UnsupportedSkillReasonCode::UnsupportedSourceType => format!(
            "Source type '{}' is not supported for automatic updates.",
            assignment.repository.source_type
        ),
        UnsupportedSkillReasonCode::MissingSourcePath => {
            "GitHub source path is missing.".to_string()
        }
        UnsupportedSkillReasonCode::UnsupportedSource => {
            "Automatic update is not supported for this source.".to_string()
        }
    }
}

fn skill_target_dir(skill: &Skill) -> Result<PathBuf, CentralUpdatesError> {
    skill
        .canonical_path
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| Path::new(&skill.file_path).parent().map(Path::to_path_buf))
        .ok_or_else(|| CentralUpdatesError::NoCanonicalDirectory(skill.id.clone()))
}
