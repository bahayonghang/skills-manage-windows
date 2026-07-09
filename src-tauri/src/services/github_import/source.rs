use crate::services::resource_budget::ResourceBudget;

use super::*;
pub(crate) async fn resolve_repo_source(
    repo_url: &str,
    auth_token: Option<&str>,
) -> Result<ResolvedGitHubRepoSource, GithubImportError> {
    let parsed = parse_github_source(repo_url)?;
    let owner = parsed.owner;
    let repo = parsed.repo;
    let client = github_client()?;
    let response = send_github_request_with_fallback(
        &client,
        GitHubFetchSurface::Api,
        |endpoint| {
            github_endpoint_url(
                endpoint,
                GitHubFetchSurface::Api,
                &format!("/repos/{owner}/{repo}"),
            )
        },
        "Failed to inspect GitHub repository",
        auth_token,
    )
    .await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(GithubImportError::RepoNotFound);
    }
    if !response.status().is_success() {
        let status = response.status();
        return Err(
            classify_github_denial_response(response, "inspecting the repository")
                .await
                .unwrap_or_else(|| {
                    GithubImportError::Http(format!(
                        "Failed to inspect GitHub repository: HTTP {}",
                        status
                    ))
                }),
        );
    }

    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|e| GithubImportError::Parse(e.to_string()))?;
    let branch = payload
        .get("default_branch")
        .and_then(|v| v.as_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("main")
        .to_string();
    let branch = parsed.branch.unwrap_or(branch);

    Ok(ResolvedGitHubRepoSource {
        repo: GitHubRepoRef {
            owner: owner.clone(),
            repo: repo.clone(),
            branch,
            normalized_url: format!("https://github.com/{owner}/{repo}"),
        },
        source_path: parsed.source_path,
    })
}

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

    Ok(ParsedGitHubSource {
        owner: owner.to_lowercase(),
        repo: repo.to_lowercase(),
        branch,
        source_path,
    })
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

pub(crate) async fn fetch_repo_skill_candidates_from_source(
    repo: &GitHubRepoRef,
    source_path: Option<&str>,
    auth_token: Option<&str>,
) -> Result<Vec<RemoteSkillCandidate>, GithubImportError> {
    let client = github_client()?;
    let snapshot = download_repo_snapshot(&client, repo, auth_token).await?;
    build_repo_skill_candidates_from_snapshot_at_path(repo, &snapshot, source_path)
}

pub(crate) async fn inspect_github_repo_skills_with_auth(
    repo_url: &str,
    auth_token: Option<&str>,
) -> Result<InspectedGitHubRepoSkills, GithubImportError> {
    let resolved = resolve_repo_source(repo_url, auth_token).await?;
    inspect_repo_skill_candidates_from_source(
        &resolved.repo,
        resolved.source_path.as_deref(),
        auth_token,
    )
    .await
}

pub(crate) async fn inspect_repo_skill_candidates_from_source(
    repo: &GitHubRepoRef,
    source_path: Option<&str>,
    auth_token: Option<&str>,
) -> Result<InspectedGitHubRepoSkills, GithubImportError> {
    let client = github_client()?;
    let snapshot = download_repo_snapshot(&client, repo, auth_token).await?;
    inspect_repo_skill_candidates_from_snapshot_at_path(repo, &snapshot, source_path)
}

#[cfg(test)]
pub(super) fn build_repo_skill_candidates_from_snapshot(
    repo: &GitHubRepoRef,
    snapshot: &GitHubRepoSnapshot,
) -> Result<Vec<RemoteSkillCandidate>, GithubImportError> {
    build_repo_skill_candidates_from_snapshot_at_path(repo, snapshot, None)
}

pub(crate) fn build_repo_skill_candidates_from_snapshot_at_path(
    repo: &GitHubRepoRef,
    snapshot: &GitHubRepoSnapshot,
    source_path: Option<&str>,
) -> Result<Vec<RemoteSkillCandidate>, GithubImportError> {
    let inspected =
        inspect_repo_skill_candidates_from_snapshot_at_path(repo, snapshot, source_path)?;
    if let Some(invalid) = inspected.invalid_candidates.into_iter().next() {
        return Err(GithubImportError::InvalidCandidate(invalid.detail));
    }
    Ok(inspected.valid_candidates)
}

pub(crate) fn inspect_repo_skill_candidates_from_snapshot_at_path(
    repo: &GitHubRepoRef,
    snapshot: &GitHubRepoSnapshot,
    source_path: Option<&str>,
) -> Result<InspectedGitHubRepoSkills, GithubImportError> {
    let direct_endpoint = GITHUB_MIRROR_ENDPOINTS.first().expect("github endpoint");
    let manifests = discover_skill_manifests(snapshot, source_path)?;

    let mut valid_candidates = Vec::with_capacity(manifests.len());
    let mut invalid_candidates = Vec::new();
    let mut seen_names = HashSet::new();
    for manifest in manifests {
        let outcome = snapshot
            .files
            .get(&manifest.skill_md_path)
            .ok_or_else(|| format!("Missing snapshot file '{}'.", manifest.skill_md_path))
            .and_then(|raw| {
                build_remote_skill_candidate(repo, &manifest, raw.clone(), direct_endpoint)
                    .map_err(|invalid| invalid.detail.clone())
            });

        match outcome {
            Ok(candidate) => {
                if is_generic_remote_skill_candidate(&candidate) {
                    continue;
                }
                if seen_names.insert(candidate.skill_name.clone()) {
                    valid_candidates.push(candidate);
                }
            }
            Err(error) => {
                if !manifest.from_manifest_hint {
                    invalid_candidates.push(invalid_candidate_from_manifest(&manifest, &error));
                }
            }
        }
    }

    Ok(InspectedGitHubRepoSkills {
        repo: repo.clone(),
        valid_candidates,
        invalid_candidates,
    })
}

pub(super) async fn build_remote_repo_skill_candidates_from_workspace(
    connection: &ConnectedRemoteTarget,
    repo: &GitHubRepoRef,
    remote_repo_dir: &str,
    source_path: Option<&str>,
) -> Result<Vec<RemoteSkillCandidate>, GithubImportError> {
    let direct_endpoint = GITHUB_MIRROR_ENDPOINTS.first().expect("github endpoint");
    let manifest_paths = remote_skill_manifest_paths(connection, remote_repo_dir).await?;
    let plugin_discovery =
        plugin_manifest_discovery_from_remote_workspace(connection, remote_repo_dir, source_path)
            .await?;
    let manifests = discover_skill_manifests_from_paths_with_plugin_discovery(
        manifest_paths.iter().map(String::as_str),
        source_path,
        &plugin_discovery,
    )?;

    let mut candidates = Vec::with_capacity(manifests.len());
    let mut seen_names = HashSet::new();
    for manifest in manifests {
        let skill_md_remote_path = remote_join(remote_repo_dir, &manifest.skill_md_path);
        let raw = match connection.read_file(&skill_md_remote_path).await {
            Ok(raw) => raw,
            Err(error) if manifest.from_manifest_hint => {
                let _ = error;
                continue;
            }
            Err(error) => return Err(GithubImportError::Remote(error.to_string())),
        };
        if let Err(error) = ResourceBudget::default_skill()
            .reject_file_read_size(&skill_md_remote_path, raw.len() as u64)
        {
            if manifest.from_manifest_hint {
                continue;
            }
            return Err(GithubImportError::Budget(error));
        }
        let candidate = match build_remote_skill_candidate(repo, &manifest, raw, direct_endpoint) {
            Ok(candidate) => candidate,
            Err(invalid) if manifest.from_manifest_hint => {
                let _ = invalid;
                continue;
            }
            Err(invalid) => return Err(GithubImportError::InvalidCandidate(invalid.detail)),
        };
        if is_generic_remote_skill_candidate(&candidate) {
            continue;
        }
        if !seen_names.insert(candidate.skill_name.clone()) {
            continue;
        }

        candidates.push(candidate);
    }

    Ok(candidates)
}

pub(super) fn build_remote_skill_candidate(
    repo: &GitHubRepoRef,
    manifest: &SnapshotSkillManifest,
    raw: Vec<u8>,
    direct_endpoint: &GitHubMirrorEndpoint,
) -> Result<RemoteSkillCandidate, InvalidRemoteSkillCandidate> {
    let content = String::from_utf8(raw).map_err(|_| InvalidRemoteSkillCandidate {
        source_path: manifest.source_path.clone(),
        reason: "invalid_utf8".to_string(),
        detail: invalid_utf8_message(manifest),
    })?;
    let frontmatter = parse_frontmatter(&content).ok_or_else(|| InvalidRemoteSkillCandidate {
        source_path: manifest.source_path.clone(),
        reason: "invalid_frontmatter".to_string(),
        detail: invalid_frontmatter_message(manifest),
    })?;

    let skill_id = if manifest.source_path == "." {
        let repo_skill_id =
            sanitize_skill_id(&repo.repo).map_err(|error| InvalidRemoteSkillCandidate {
                source_path: manifest.source_path.clone(),
                reason: "invalid_skill_id".to_string(),
                detail: error.to_string(),
            })?;
        repo_skill_id
            .strip_suffix("-skill")
            .unwrap_or(&repo_skill_id)
            .to_string()
    } else {
        sanitize_skill_id(&manifest.skill_directory_name).map_err(|error| {
            InvalidRemoteSkillCandidate {
                source_path: manifest.source_path.clone(),
                reason: "invalid_skill_id".to_string(),
                detail: error.to_string(),
            }
        })?
    };

    Ok(RemoteSkillCandidate {
        source_path: manifest.source_path.clone(),
        skill_id,
        skill_name: frontmatter.name,
        description: frontmatter.description,
        plugin_name: manifest.plugin_name.clone(),
        root_directory: manifest.root_directory.clone(),
        skill_directory_name: if manifest.source_path == "." {
            repo.repo.clone()
        } else {
            manifest.skill_directory_name.clone()
        },
        download_url: raw_file_url(direct_endpoint, repo, &manifest.skill_md_path),
    })
}

pub(super) fn invalid_candidate_from_manifest(
    manifest: &SnapshotSkillManifest,
    detail: &str,
) -> InvalidRemoteSkillCandidate {
    let reason = if detail.contains("valid frontmatter") {
        "invalid_frontmatter"
    } else if detail.contains("valid UTF-8") {
        "invalid_utf8"
    } else {
        "invalid_skill_id"
    };
    InvalidRemoteSkillCandidate {
        source_path: manifest.source_path.clone(),
        reason: reason.to_string(),
        detail: detail.to_string(),
    }
}

fn is_generic_remote_skill_candidate(candidate: &RemoteSkillCandidate) -> bool {
    candidate.source_path != "." && candidate.skill_id == "skill"
}

pub(super) fn invalid_utf8_message(manifest: &SnapshotSkillManifest) -> String {
    format!("Skill '{}' is not valid UTF-8.", manifest.source_path)
}

pub(super) fn invalid_frontmatter_message(manifest: &SnapshotSkillManifest) -> String {
    if manifest.source_path == "." {
        "Repository root SKILL.md is missing valid frontmatter.".to_string()
    } else {
        format!(
            "Skill '{}' is missing valid frontmatter.",
            manifest.source_path
        )
    }
}

pub(super) async fn remote_skill_manifest_paths(
    connection: &ConnectedRemoteTarget,
    remote_repo_dir: &str,
) -> Result<Vec<String>, GithubImportError> {
    let output = connection
        .run_script(
            r#"set -eu
repo_dir=$1
cd "$repo_dir"
find . -type f -iname 'SKILL.md' -print | sed 's#^\./##'
"#,
            &[remote_repo_dir],
        )
        .await
        .map_err(|e| GithubImportError::Remote(e.to_string()))?;

    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(normalize_repo_path)
        .filter_map(|result| match result {
            Ok(path) if is_skill_md_repo_path(&path) => Some(Ok(path)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

#[derive(Debug, Clone)]
pub(super) struct SnapshotSkillManifest {
    pub(super) source_path: String,
    pub(super) root_directory: String,
    pub(super) skill_directory_name: String,
    pub(super) skill_md_path: String,
    pub(super) plugin_name: Option<String>,
    pub(super) from_manifest_hint: bool,
}

pub(super) fn discover_skill_manifests(
    snapshot: &GitHubRepoSnapshot,
    source_path: Option<&str>,
) -> Result<Vec<SnapshotSkillManifest>, GithubImportError> {
    let plugin_discovery = plugin_manifest_discovery_from_snapshot(snapshot, source_path)?;
    discover_skill_manifests_from_paths_with_plugin_discovery(
        snapshot.files.keys().map(String::as_str),
        source_path,
        &plugin_discovery,
    )
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn discover_skill_manifests_from_paths<'a, I>(
    paths: I,
    source_path: Option<&str>,
) -> Result<Vec<SnapshotSkillManifest>, GithubImportError>
where
    I: IntoIterator<Item = &'a str>,
{
    discover_skill_manifests_from_paths_with_plugin_discovery(
        paths,
        source_path,
        &PluginManifestDiscovery::default(),
    )
}

pub(super) fn discover_skill_manifests_from_paths_with_plugin_discovery<'a, I>(
    paths: I,
    source_path: Option<&str>,
    plugin_discovery: &PluginManifestDiscovery,
) -> Result<Vec<SnapshotSkillManifest>, GithubImportError>
where
    I: IntoIterator<Item = &'a str>,
{
    let path_set = paths
        .into_iter()
        .map(normalize_repo_path)
        .collect::<Result<HashSet<_>, _>>()?;
    let base_path = source_path
        .map(normalize_repo_path)
        .transpose()?
        .unwrap_or_default();
    let mut manifests = Vec::new();
    let mut seen_source_paths = HashSet::new();

    if let Some(manifest) = direct_skill_manifest(&path_set, &base_path) {
        insert_manifest(&mut manifests, &mut seen_source_paths, manifest);
    }

    for root in PRIORITY_SKILL_ROOTS {
        let search_root = join_repo_path(&base_path, root)?;
        for manifest in immediate_skill_manifests(&path_set, &search_root)? {
            insert_manifest(&mut manifests, &mut seen_source_paths, manifest);
        }
    }

    if manifests.is_empty() {
        for manifest in recursive_skill_manifests(&path_set, &base_path)? {
            insert_manifest(&mut manifests, &mut seen_source_paths, manifest);
        }
    }

    for source_path in &plugin_discovery.explicit_skill_paths {
        let skill_md_path = skill_md_path_from_source_path(source_path)?;
        if !path_set.contains(&skill_md_path) {
            continue;
        }
        if let Some(mut manifest) = manifest_from_skill_md_path(&skill_md_path) {
            manifest.from_manifest_hint = true;
            insert_manifest(&mut manifests, &mut seen_source_paths, manifest);
        }
    }

    for manifest in &mut manifests {
        manifest.plugin_name = plugin_discovery
            .plugin_by_source_path
            .get(&manifest.source_path)
            .cloned();
    }

    Ok(manifests)
}

pub(super) fn direct_skill_manifest(
    paths: &HashSet<String>,
    base_path: &str,
) -> Option<SnapshotSkillManifest> {
    let skill_md_path = join_repo_path(base_path, "SKILL.md").ok()?;
    paths
        .contains(&skill_md_path)
        .then(|| manifest_from_skill_md_path(&skill_md_path))
        .flatten()
}

pub(super) fn immediate_skill_manifests(
    paths: &HashSet<String>,
    search_root: &str,
) -> Result<Vec<SnapshotSkillManifest>, GithubImportError> {
    let mut manifests = paths
        .iter()
        .filter(|path| is_immediate_skill_manifest(path, search_root))
        .filter_map(|path| manifest_from_skill_md_path(path))
        .collect::<Vec<_>>();
    manifests.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(manifests)
}

pub(super) fn recursive_skill_manifests(
    paths: &HashSet<String>,
    base_path: &str,
) -> Result<Vec<SnapshotSkillManifest>, GithubImportError> {
    let mut manifests = paths
        .iter()
        .filter(|path| is_recursive_skill_manifest(path, base_path))
        .filter_map(|path| manifest_from_skill_md_path(path))
        .collect::<Vec<_>>();
    manifests.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(manifests)
}

pub(super) fn insert_manifest(
    manifests: &mut Vec<SnapshotSkillManifest>,
    seen_source_paths: &mut HashSet<String>,
    manifest: SnapshotSkillManifest,
) {
    if seen_source_paths.insert(manifest.source_path.clone()) {
        manifests.push(manifest);
    }
}

pub(super) fn is_immediate_skill_manifest(path: &str, search_root: &str) -> bool {
    if !is_skill_md_repo_path(path) {
        return false;
    }
    if has_skipped_discovery_segment(path) {
        return false;
    }

    let Some(source_path) = source_path_from_skill_md(path) else {
        return false;
    };
    let source_segments = repo_path_segments(&source_path);
    let root_segments = repo_path_segments(search_root);
    source_segments.len() == root_segments.len() + 1 && source_segments.starts_with(&root_segments)
}

pub(super) fn is_recursive_skill_manifest(path: &str, base_path: &str) -> bool {
    if !is_skill_md_repo_path(path) {
        return false;
    }
    if has_skipped_discovery_segment(path) {
        return false;
    }

    let Some(source_path) = source_path_from_skill_md(path) else {
        return false;
    };
    let source_segments = repo_path_segments(&source_path);
    let base_segments = repo_path_segments(base_path);
    source_segments.starts_with(&base_segments)
        && source_segments.len().saturating_sub(base_segments.len())
            <= RECURSIVE_DISCOVERY_MAX_DEPTH
}

pub(super) fn manifest_from_skill_md_path(path: &str) -> Option<SnapshotSkillManifest> {
    let normalized = normalize_repo_path(path).ok()?;
    let source_path = source_path_from_skill_md(&normalized)?;
    let skill_directory_name = if source_path == "." {
        String::new()
    } else {
        source_path.rsplit('/').next()?.to_string()
    };
    let root_directory = if source_path == "." {
        "/".to_string()
    } else {
        source_path
            .rsplit_once('/')
            .map(|(root, _)| {
                if root.is_empty() {
                    "/".to_string()
                } else {
                    root.to_string()
                }
            })
            .unwrap_or_else(|| "/".to_string())
    };

    Some(SnapshotSkillManifest {
        source_path,
        root_directory,
        skill_directory_name,
        skill_md_path: normalized,
        plugin_name: None,
        from_manifest_hint: false,
    })
}

pub(super) fn source_path_from_skill_md(path: &str) -> Option<String> {
    let normalized = normalize_repo_path(path).ok()?;
    if normalized.eq_ignore_ascii_case("SKILL.md") {
        return Some(".".to_string());
    }

    let lower = normalized.to_ascii_lowercase();
    lower
        .strip_suffix("/skill.md")
        .map(|_| normalized[..normalized.len() - "/SKILL.md".len()].to_string())
}

pub(super) fn join_repo_path(base_path: &str, child: &str) -> Result<String, GithubImportError> {
    let mut parts = Vec::new();
    for part in base_path.split('/').chain(child.split('/')) {
        let trimmed = part.trim();
        if trimmed.is_empty() || trimmed == "." {
            continue;
        }
        parts.push(trimmed);
    }
    normalize_repo_path(&parts.join("/"))
}

pub(super) fn normalize_repo_path(path: &str) -> Result<String, GithubImportError> {
    let normalized = path.trim().trim_matches('/').replace('\\', "/");
    if normalized.is_empty() || normalized == "." {
        return Ok(String::new());
    }
    if !is_safe_repo_relative_path(&normalized) {
        return Err(GithubImportError::UnsupportedRepoPath(path.to_string()));
    }
    Ok(normalized)
}

pub(super) fn repo_path_segments(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

pub(super) fn has_skipped_discovery_segment(path: &str) -> bool {
    repo_path_segments(path).iter().any(|segment| {
        SKIP_DISCOVERY_DIRS
            .iter()
            .any(|skip| segment.eq_ignore_ascii_case(skip))
    })
}

pub(super) fn is_skill_md_repo_path(path: &str) -> bool {
    path.eq_ignore_ascii_case("SKILL.md") || path.to_ascii_lowercase().ends_with("/skill.md")
}
pub(super) fn is_safe_repo_relative_path(path: &str) -> bool {
    let relative = Path::new(path);
    !relative.is_absolute()
        && relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}
pub(crate) fn parse_frontmatter(content: &str) -> Option<SkillFrontmatter> {
    let block = crate::services::scanner::extract_frontmatter_block(content)?;
    serde_norway::from_str::<SkillFrontmatter>(block).ok()
}

pub(super) fn sanitize_skill_id(raw: &str) -> Result<String, GithubImportError> {
    let lowered = raw.trim().to_lowercase();
    let mut sanitized = String::new();
    let mut last_was_dash = false;
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            sanitized.push('-');
            last_was_dash = true;
        }
    }
    let sanitized = sanitized.trim_matches('-').to_string();
    if sanitized.is_empty() {
        return Err(GithubImportError::InvalidSkillIdentifier(raw.to_string()));
    }
    Ok(sanitized)
}
