//! Per-repository sync summary and remote-missing skill builders.

use std::collections::HashMap;

use super::{CentralRemoteAddedSkill, CentralRemoteMissingSkill, CentralRepositorySyncSummary};
use crate::db::{SkillRepository, SkillUpdateState};
use crate::services::central_updates::SkillUpdateStatus;
use crate::services::github_import::GitHubRepoRef;

pub(super) fn build_repository_sync_summaries(
    repo_by_id: &HashMap<String, SkillRepository>,
    states: &[SkillUpdateState],
    remote_added: &[CentralRemoteAddedSkill],
    skipped_remote_added: &[CentralRemoteAddedSkill],
    remote_missing: &[CentralRemoteMissingSkill],
) -> Vec<CentralRepositorySyncSummary> {
    let mut checked_by_repo = HashMap::<String, usize>::new();
    let mut update_available_by_repo = HashMap::<String, usize>::new();
    let mut unsupported_by_repo = HashMap::<String, usize>::new();
    let mut failed_by_repo = HashMap::<String, usize>::new();
    for state in states {
        let Some(repo_id) = repository_id_for_update_state(repo_by_id, state) else {
            continue;
        };
        *checked_by_repo.entry(repo_id.clone()).or_default() += 1;
        match state.status {
            SkillUpdateStatus::UpdateAvailable => {
                *update_available_by_repo.entry(repo_id).or_default() += 1;
            }
            SkillUpdateStatus::Unsupported => {
                *unsupported_by_repo.entry(repo_id).or_default() += 1;
            }
            SkillUpdateStatus::Error => {
                *failed_by_repo.entry(repo_id).or_default() += 1;
            }
            _ => {}
        }
    }

    let mut remote_missing_by_repo = HashMap::<String, usize>::new();
    for item in remote_missing {
        if let Some(repo_id) = &item.repository_id {
            *remote_missing_by_repo.entry(repo_id.clone()).or_default() += 1;
        }
    }
    let mut remote_added_by_repo = HashMap::<String, usize>::new();
    for item in remote_added {
        *remote_added_by_repo
            .entry(item.repository_id.clone())
            .or_default() += 1;
    }
    let mut skipped_remote_added_by_repo = HashMap::<String, usize>::new();
    for item in skipped_remote_added {
        *skipped_remote_added_by_repo
            .entry(item.repository_id.clone())
            .or_default() += 1;
    }

    let mut summaries = repo_by_id
        .iter()
        .map(|(repository_id, repository)| CentralRepositorySyncSummary {
            repository_id: repository_id.clone(),
            name: repository.name.clone(),
            checked: checked_by_repo.get(repository_id).copied().unwrap_or(0),
            update_available: update_available_by_repo
                .get(repository_id)
                .copied()
                .unwrap_or(0),
            remote_missing: remote_missing_by_repo
                .get(repository_id)
                .copied()
                .unwrap_or(0),
            unsupported: unsupported_by_repo.get(repository_id).copied().unwrap_or(0),
            failed: failed_by_repo.get(repository_id).copied().unwrap_or(0),
            remote_added: remote_added_by_repo
                .get(repository_id)
                .copied()
                .unwrap_or(0),
            skipped_remote_added: skipped_remote_added_by_repo
                .get(repository_id)
                .copied()
                .unwrap_or(0),
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| left.name.cmp(&right.name));
    summaries
}

pub(crate) fn build_remote_missing_skills(
    repo_by_id: &HashMap<String, SkillRepository>,
    states: Vec<SkillUpdateState>,
) -> Vec<CentralRemoteMissingSkill> {
    states
        .into_iter()
        .map(|state| {
            let repository = repository_for_update_state(repo_by_id, &state);
            CentralRemoteMissingSkill {
                repository_id: repository.map(|(id, _)| id.clone()),
                repository_name: repository
                    .map(|(_, repository)| repository_display_name(repository))
                    .unwrap_or_else(|| "Unknown source".to_string()),
                repo: repository.and_then(|(_, repository)| repo_ref_for_repository(repository)),
                state,
            }
        })
        .collect()
}

fn repository_display_name(repository: &SkillRepository) -> String {
    match (&repository.owner, &repository.repo) {
        (Some(owner), Some(repo)) if !owner.is_empty() && !repo.is_empty() => {
            format!("{owner}/{repo}")
        }
        _ => repository.name.clone(),
    }
}

fn repo_ref_for_repository(repository: &SkillRepository) -> Option<GitHubRepoRef> {
    if repository.source_type != "github" || repository.is_unknown {
        return None;
    }
    let (Some(owner), Some(repo), Some(branch)) = (
        repository.owner.as_ref(),
        repository.repo.as_ref(),
        repository.branch.as_ref(),
    ) else {
        return None;
    };
    Some(GitHubRepoRef {
        owner: owner.clone(),
        repo: repo.clone(),
        branch: branch.clone(),
        normalized_url: repository
            .url
            .clone()
            .unwrap_or_else(|| format!("https://github.com/{owner}/{repo}")),
    })
}

fn repository_id_for_update_state(
    repo_by_id: &HashMap<String, SkillRepository>,
    state: &SkillUpdateState,
) -> Option<String> {
    repository_for_update_state(repo_by_id, state).map(|(id, _)| id.clone())
}

fn repository_for_update_state<'a>(
    repo_by_id: &'a HashMap<String, SkillRepository>,
    state: &SkillUpdateState,
) -> Option<(&'a String, &'a SkillRepository)> {
    repo_by_id.iter().find(|(_, repository)| {
        repository.source_type == state.source_type
            && repository.url == state.source_url
            && repository.branch == state.ref_name
    })
}
