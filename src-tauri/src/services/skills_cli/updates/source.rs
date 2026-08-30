//! GitHub source/path normalization for Skills CLI update grouping.

use crate::services::github_import::{
    github_repository_key_from_source, normalize_github_source_url,
};

use super::super::SkillsCliError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubUpdateIdentity {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub skill_path: String,
    pub repository_key: String,
    pub normalized_source: String,
}

pub fn parse_github_update_identity(
    source: &str,
    lock_skill_path: Option<&str>,
) -> Result<GithubUpdateIdentity, SkillsCliError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(SkillsCliError::UpdateUnsupported);
    }
    let (without_skill, shorthand_skill) = split_cli_shorthand_skill(trimmed);
    let normalized = normalize_github_source_url(without_skill)
        .or_else(|_| normalize_github_source_url(&format!("https://github.com/{without_skill}")))
        .map_err(|_| SkillsCliError::UpdateUnsupported)?;
    let (owner, repo, branch) = parse_owner_repo_branch(&normalized)?;
    let skill_path = lock_skill_path
        .map(str::trim)
        .filter(|value| !value.is_empty() && !PathLooksLocal::is_local(value))
        .map(|value| value.replace('\\', "/").trim_matches('/').to_string())
        .or(shorthand_skill)
        .unwrap_or_default();
    if skill_path.contains("..") {
        return Err(SkillsCliError::UpdateUnsupported);
    }
    let _ = github_repository_key_from_source(&normalized)
        .map_err(|_| SkillsCliError::UpdateUnsupported)?;
    Ok(GithubUpdateIdentity {
        repository_key: format!("{owner}/{repo}@{branch}"),
        owner,
        repo,
        branch,
        skill_path,
        normalized_source: normalized,
    })
}

fn split_cli_shorthand_skill(source: &str) -> (&str, Option<String>) {
    if source.contains("://") || source.starts_with("git@") {
        return (source, None);
    }
    let Some((repo, skill)) = source.split_once('@') else {
        return (source, None);
    };
    if !repo.contains('/') || skill.is_empty() {
        return (source, None);
    }
    if skill.len() == 40 && skill.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return (source, None);
    }
    (repo, Some(skill.trim_matches('/').to_string()))
}

fn parse_owner_repo_branch(url: &str) -> Result<(String, String, String), SkillsCliError> {
    let trimmed = url
        .trim()
        .trim_start_matches("https://github.com/")
        .trim_start_matches("https://www.github.com/");
    let mut segments = trimmed.split('/').filter(|segment| !segment.is_empty());
    let owner = segments
        .next()
        .ok_or(SkillsCliError::UpdateUnsupported)?
        .to_ascii_lowercase();
    let repo = segments
        .next()
        .ok_or(SkillsCliError::UpdateUnsupported)?
        .trim_end_matches(".git")
        .to_ascii_lowercase();
    let branch = match (segments.next(), segments.next()) {
        (Some("tree"), Some(branch)) if !branch.is_empty() => branch.to_string(),
        _ => "main".to_string(),
    };
    if owner.is_empty() || repo.is_empty() {
        return Err(SkillsCliError::UpdateUnsupported);
    }
    Ok((owner, repo, branch))
}

struct PathLooksLocal;

impl PathLooksLocal {
    fn is_local(value: &str) -> bool {
        value.contains(":\\")
            || value.starts_with('/')
            || value.starts_with("~/")
            || value.contains("\\\\")
    }
}
