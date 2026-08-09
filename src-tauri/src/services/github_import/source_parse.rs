use super::*;

pub(super) fn parse_github_source(url: &str) -> Result<ParsedGitHubSource, GithubImportError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(GithubImportError::InvalidRepoUrl);
    }
    if has_raw_path_traversal(trimmed) {
        return Err(GithubImportError::SubpathTraversal);
    }

    let parse_target =
        if trimmed.starts_with("github.com/") || trimmed.starts_with("www.github.com/") {
            format!("https://{trimmed}")
        } else if is_github_shorthand_source(trimmed) {
            format!("https://github.com/{trimmed}")
        } else {
            trimmed.to_string()
        };

    let parsed =
        reqwest::Url::parse(&parse_target).map_err(|_| GithubImportError::InvalidRepoUrl)?;

    if parsed.scheme() != "https" {
        return Err(GithubImportError::RepoUrlNotHttps);
    }
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
        || parsed.query().is_some()
        || parsed.port_or_known_default() != Some(443)
    {
        return Err(GithubImportError::InvalidRepoUrl);
    }
    let host = parsed.host_str().unwrap_or_default();
    if host != "github.com" && host != "www.github.com" {
        return Err(GithubImportError::RepoUrlNotGithub);
    }

    let segments = parsed
        .path_segments()
        .ok_or(GithubImportError::InvalidRepoUrl)?
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let owner = segments
        .first()
        .filter(|segment| !segment.is_empty())
        .ok_or(GithubImportError::RepoUrlMissingOwner)?;
    let repo = segments
        .get(1)
        .filter(|segment| !segment.is_empty())
        .ok_or(GithubImportError::RepoUrlMissingRepo)?;

    let repo = repo.strip_suffix(".git").unwrap_or(repo);
    if owner.is_empty() || repo.is_empty() {
        return Err(GithubImportError::RepoUrlMissingOwnerRepo);
    }

    let (branch, source_segments) = match segments.get(2).copied() {
        Some("tree") => {
            let branch = segments
                .get(3)
                .filter(|segment| !segment.is_empty())
                .ok_or(GithubImportError::TreeUrlMissingBranch)?;
            (Some((*branch).to_string()), &segments[4..])
        }
        Some("blob") => {
            return Err(GithubImportError::BlobUrlUnsupported);
        }
        Some(_) => (None, &segments[2..]),
        None => (None, &segments[2..]),
    };
    let source_path = normalize_repo_subpath(source_segments)?;

    let validation_ref = GitHubRepoRef {
        owner: (*owner).to_string(),
        repo: repo.to_string(),
        branch: branch.clone().unwrap_or_else(|| "main".to_string()),
        normalized_url: String::new(),
    };
    validate_repo_ref(&validation_ref)?;

    Ok(ParsedGitHubSource {
        owner: owner.to_lowercase(),
        repo: repo.to_lowercase(),
        branch,
        source_path,
    })
}

/// Normalize a source using the same owner/repository/branch/subpath parser as
/// GitHub preview and import, without performing network access.
pub(crate) fn normalize_github_source_url(url: &str) -> Result<String, GithubImportError> {
    let parsed = parse_github_source(url)?;
    let mut normalized = format!("https://github.com/{}/{}", parsed.owner, parsed.repo);

    if let Some(branch) = parsed.branch {
        normalized.push_str("/tree/");
        normalized.push_str(&branch);
    }
    if let Some(source_path) = parsed.source_path {
        normalized.push('/');
        normalized.push_str(&source_path);
    }

    Ok(normalized)
}

pub(crate) fn github_repository_key_from_source(url: &str) -> Result<String, GithubImportError> {
    let parsed = parse_github_source(url)?;
    Ok(format!(
        "{}/{}",
        parsed.owner.to_ascii_lowercase(),
        parsed.repo.to_ascii_lowercase()
    ))
}

pub(super) fn is_github_shorthand_source(value: &str) -> bool {
    let mut segments = value.split('/').filter(|segment| !segment.is_empty());
    let Some(owner) = segments.next() else {
        return false;
    };
    let Some(repo) = segments.next() else {
        return false;
    };

    !owner.contains(':')
        && !owner.contains('\\')
        && !repo.contains(':')
        && !repo.contains('\\')
        && !owner.starts_with('.')
        && !repo.starts_with('.')
}

pub(super) fn has_raw_path_traversal(value: &str) -> bool {
    let path_only = value
        .split(['?', '#'])
        .next()
        .unwrap_or(value)
        .replace('\\', "/")
        .to_ascii_lowercase();
    path_only
        .split('/')
        .any(|segment| segment == ".." || segment == "%2e%2e")
}

pub(super) fn normalize_repo_subpath(
    segments: &[&str],
) -> Result<Option<String>, GithubImportError> {
    if segments.is_empty() {
        return Ok(None);
    }

    let path = segments.join("/");
    if !is_safe_repo_relative_path(&path) {
        return Err(GithubImportError::UnsupportedSubpath(path));
    }

    Ok(Some(path))
}
