use std::collections::HashMap;

use crate::commands::central_updates;
use crate::commands::github_import::{self, GitHubRepoRef};
use crate::db::{self, DbPool, SkillRepository, SkillUpdateState};

use super::RemoteAddedSkill;

pub(crate) fn prepared_repo_ref(
    prepared: &central_updates::PreparedSkillUpdate,
) -> Option<GitHubRepoRef> {
    repo_ref_for_repository(&prepared.assignment.repository)
}

pub(crate) fn remote_added_from_item(
    item: central_updates::CentralRemoteAddedSkill,
) -> RemoteAddedSkill {
    let conflict_existing_skill_id = item
        .preview
        .conflict
        .as_ref()
        .map(|c| c.existing_skill_id.clone());
    RemoteAddedSkill {
        repository_id: item.repository_id,
        source_path: item.preview.source_path,
        skill_id: item.preview.skill_id,
        skill_name: item.preview.skill_name,
        conflict_existing_skill_id,
    }
}

pub(crate) fn repository_import_url(repository: &SkillRepository) -> Option<String> {
    if repository.source_type != "github" || repository.is_unknown {
        return None;
    }
    if let Some(url) = repository
        .url
        .as_deref()
        .filter(|url| !url.trim().is_empty())
    {
        if let Some(branch) = repository
            .branch
            .as_deref()
            .filter(|branch| !branch.is_empty())
        {
            return Some(format!(
                "{}/tree/{}",
                url.trim().trim_end_matches('/'),
                branch
            ));
        }
        return Some(url.to_string());
    }
    match (&repository.owner, &repository.repo, &repository.branch) {
        (Some(owner), Some(repo), Some(branch)) => {
            Some(format!("https://github.com/{owner}/{repo}/tree/{branch}"))
        }
        (Some(owner), Some(repo), None) => Some(format!("https://github.com/{owner}/{repo}")),
        _ => None,
    }
}

pub(crate) fn repository_id_for_state(
    repo_by_id: &HashMap<String, SkillRepository>,
    state_row: &SkillUpdateState,
) -> Option<String> {
    repo_by_id
        .iter()
        .find(|(_, repository)| {
            repository.source_type == state_row.source_type
                && repository.url == state_row.source_url
                && repository.branch == state_row.ref_name
        })
        .map(|(id, _)| id.clone())
}

pub(crate) async fn load_syncable_github_repositories(
    pool: &DbPool,
    repository_ids: &[String],
    auth_token: Option<&str>,
) -> Result<Vec<(SkillRepository, GitHubRepoRef)>, String> {
    let mut repositories = Vec::new();
    for repository_id in repository_ids {
        let Some(repository) = db::get_skill_repository_by_id(pool, repository_id).await? else {
            continue;
        };
        if repository.is_unknown || repository.source_type != "github" {
            continue;
        }
        let repo_ref = if let (Some(owner), Some(repo), Some(branch)) = (
            repository.owner.as_ref(),
            repository.repo.as_ref(),
            repository.branch.as_ref(),
        ) {
            GitHubRepoRef {
                owner: owner.clone(),
                repo: repo.clone(),
                branch: branch.clone(),
                normalized_url: repository
                    .url
                    .clone()
                    .unwrap_or_else(|| format!("https://github.com/{owner}/{repo}")),
            }
        } else {
            let Some(url) = repository_import_url(&repository) else {
                continue;
            };
            github_import::resolve_repo_source(&url, auth_token)
                .await
                .map_err(|e| e.to_string())?
                .repo
        };
        repositories.push((repository, repo_ref));
    }
    Ok(repositories)
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
