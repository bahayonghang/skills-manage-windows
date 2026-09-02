use std::collections::HashMap;

use crate::db::repos::repositories_repo;
use crate::db::{DbPool, SkillRepository, SkillUpdateState};
use crate::services::central_updates::{
    CentralRemoteAddedSkill, CentralUpdatesError, PreparedSkillUpdate,
};
use crate::services::github_import::{self, GitHubRepoRef};

use super::RemoteAddedSkill;

pub(crate) fn prepared_repo_ref(prepared: &PreparedSkillUpdate) -> Option<GitHubRepoRef> {
    prepared.source.as_ref().map(|source| source.repo.clone())
}

pub(crate) fn remote_added_from_item(item: CentralRemoteAddedSkill) -> RemoteAddedSkill {
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

/// Reconstruct the persisted repository reference without resolving a URL or
/// branch. Decision Apply must stay bound to the immutable commit recorded by
/// Refresh, so incomplete legacy repository metadata is not repaired through
/// a second GitHub lookup here.
pub(crate) fn repository_repo_ref(repository: &SkillRepository) -> Option<GitHubRepoRef> {
    if repository.source_type != "github" || repository.is_unknown {
        return None;
    }
    let owner = repository.owner.as_ref()?.trim();
    let repo = repository.repo.as_ref()?.trim();
    let branch = repository.branch.as_ref()?.trim();
    if owner.is_empty() || repo.is_empty() || branch.is_empty() {
        return None;
    }
    Some(GitHubRepoRef {
        owner: owner.to_string(),
        repo: repo.to_string(),
        branch: branch.to_string(),
        normalized_url: repository
            .url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
            .map(|url| url.trim().trim_end_matches('/').to_string())
            .unwrap_or_else(|| format!("https://github.com/{owner}/{repo}")),
    })
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
) -> Result<Vec<(SkillRepository, GitHubRepoRef)>, CentralUpdatesError> {
    let mut repositories = Vec::new();
    for repository_id in repository_ids {
        let Some(repository) =
            repositories_repo::get_skill_repository_by_id(pool, repository_id).await?
        else {
            continue;
        };
        if repository.is_unknown || repository.source_type != "github" {
            continue;
        }
        let repo_ref = if let Some(repo_ref) = repository_repo_ref(&repository) {
            repo_ref
        } else {
            let Some(url) = repository_import_url(&repository) else {
                continue;
            };
            github_import::resolve_repo_source(&url, auth_token)
                .await?
                .repo
        };
        repositories.push((repository, repo_ref));
    }
    Ok(repositories)
}
