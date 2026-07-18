use super::*;
use crate::services::resource_budget::ResourceBudget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreviewRepositoryFile {
    pub(super) repo_path: String,
    pub(super) byte_len: u64,
}

pub(super) const REMOTE_PREVIEW_FILE_INVENTORY_SCRIPT: &str = r#"set -eu
repo_dir=$1
find "$repo_dir" -type f -exec sh -c '
repo_dir=$1
shift
for file do
  relative=${file#"$repo_dir"/}
  size=$(wc -c < "$file")
  printf "%s\0%s\0" "$relative" "$size"
done
' sh "$repo_dir" {} +
"#;

pub(super) fn snapshot_preview_repository_files(
    snapshot: &GitHubRepoSnapshot,
) -> Vec<PreviewRepositoryFile> {
    snapshot
        .files
        .iter()
        .map(|(repo_path, bytes)| PreviewRepositoryFile {
            repo_path: repo_path.clone(),
            byte_len: bytes.len() as u64,
        })
        .collect()
}

pub(super) fn parse_remote_preview_repository_files(
    output: &str,
) -> Result<Vec<PreviewRepositoryFile>, GithubImportError> {
    if output.is_empty() {
        return Ok(Vec::new());
    }

    let payload = output
        .strip_suffix('\0')
        .ok_or(GithubImportError::RemotePreviewInvalidFileManifest)?;
    let fields = payload.split('\0').collect::<Vec<_>>();
    if fields.len() % 2 != 0 {
        return Err(GithubImportError::RemotePreviewInvalidFileManifest);
    }

    let budget = ResourceBudget::default_skill();
    let mut total_bytes = 0_u64;
    let mut seen_paths = HashSet::new();
    let mut files = Vec::with_capacity(fields.len() / 2);
    for record in fields.chunks_exact(2) {
        if files.len() >= budget.archive_files {
            return Err(GithubImportError::ArchiveFileBudgetExceeded(
                budget.archive_files,
            ));
        }

        let repo_path = normalize_repo_path(record[0])?;
        if repo_path.is_empty() || !seen_paths.insert(repo_path.clone()) {
            return Err(GithubImportError::RemotePreviewInvalidFileManifest);
        }
        let byte_len = record[1]
            .trim()
            .parse::<u64>()
            .map_err(|_| GithubImportError::RemotePreviewInvalidFileManifest)?;
        budget
            .reject_archive_entry_size(&repo_path, byte_len)
            .map_err(GithubImportError::Budget)?;
        total_bytes = total_bytes
            .checked_add(byte_len)
            .ok_or(GithubImportError::ArchiveSizeOverflow)?;
        budget
            .reject_archive_expanded_size(total_bytes)
            .map_err(GithubImportError::Budget)?;
        files.push(PreviewRepositoryFile {
            repo_path,
            byte_len,
        });
    }
    files.sort_by(|left, right| left.repo_path.cmp(&right.repo_path));
    Ok(files)
}

pub(super) async fn remote_preview_repository_files(
    connection: &ConnectedRemoteTarget,
    remote_repo_dir: &str,
) -> Result<Vec<PreviewRepositoryFile>, GithubImportError> {
    let output = connection
        .run_script(REMOTE_PREVIEW_FILE_INVENTORY_SCRIPT, &[remote_repo_dir])
        .await
        .map_err(|error| GithubImportError::Remote(error.to_string()))?;
    parse_remote_preview_repository_files(&output)
}

pub(super) fn attach_preview_file_manifests(
    skills: &mut [GitHubSkillPreview],
    repository_files: &[PreviewRepositoryFile],
) -> Result<(), GithubImportError> {
    for skill in skills {
        let mut files = repository_files
            .iter()
            .filter_map(|file| {
                repo_file_relative_to_source(&file.repo_path, &skill.source_path).map(|path| {
                    GitHubSkillPreviewFile {
                        path,
                        byte_len: file.byte_len,
                    }
                })
            })
            .collect::<Vec<_>>();
        files.sort_by(|left, right| left.path.cmp(&right.path));
        if !files.iter().any(|file| file.path == "SKILL.md") {
            return Err(GithubImportError::PreviewFileManifestIncomplete(
                skill.source_path.clone(),
            ));
        }
        skill.files = Some(files);
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) async fn preview_github_repo_import_impl(
    pool: &DbPool,
    secrets: &dyn crate::secrets::SecretStore,
    repo_url: &str,
) -> Result<GitHubRepoPreview, GithubImportError> {
    let auth = github_direct_auth_from_secret_store(pool, secrets).await?;
    preview_github_repo_import_with_auth(pool, repo_url, auth.as_deref()).await
}

/// Outcome of attempting the tree-manifest fast-path for preview acquisition.
/// The dispatcher (`preview_github_repo_import_with_auth`) matches on this to
/// decide between TreeRaw and Archive without re-running candidate discovery.
pub(super) enum TreeFastPathOutcome {
    /// Tree fast-path succeeded: candidates and preview files are ready.
    Ready {
        candidates: Vec<RemoteSkillCandidate>,
        repository_files: Vec<PreviewRepositoryFile>,
    },
    /// Acquisition failed with a typed reason; fall back to archive. The
    /// reason is recorded by Commit 3 acquisition diagnostics; Commit 2 only
    /// matches on the variant to drive the archive fallback.
    #[allow(dead_code)]
    Fallback(tree_manifest::FallbackReason),
}

/// Build preview candidates and repository file manifests from the Git tree
/// API fast-path.
///
/// Reuses the shared path/plugin/frontmatter discovery so candidate identity,
/// plugin grouping, and file-manifest output stay equivalent to the archive
/// path. Acquisition failures (tree fetch/parse, raw download, budget,
/// integrity) return `Ok(Fallback(reason))` so the dispatcher can switch to
/// archive acquisition; domain failures (invalid candidate) return `Err` and
/// are surfaced directly because the archive path would produce the same
/// domain error.
pub(super) async fn try_build_preview_from_tree_manifest(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    source_path: Option<&str>,
    auth: Option<&str>,
) -> Result<TreeFastPathOutcome, GithubImportError> {
    let manifest = match tree_manifest::try_fetch_tree_manifest(client, repo, auth).await {
        Ok(manifest) => manifest,
        Err(error) => {
            return Ok(map_acquisition_error_to_outcome(error));
        }
    };

    // Repository file manifest is derived directly from the parsed tree — no
    // raw downloads needed. Sorted by repo_path for deterministic manifest
    // attachment (same ordering as `snapshot_preview_repository_files`).
    let repository_files = manifest_to_preview_repository_files(&manifest);

    let plugin_discovery =
        match build_tree_plugin_discovery(client, &manifest, repo, source_path, auth).await {
            Ok(discovery) => discovery,
            Err(error) => return Ok(map_acquisition_error_to_outcome(error)),
        };

    let manifests = match discover_skill_manifests_from_paths_with_plugin_discovery(
        manifest.regular_paths(),
        source_path,
        &plugin_discovery,
    ) {
        Ok(manifests) => manifests,
        Err(error) => return Ok(map_acquisition_error_to_outcome(error)),
    };

    let direct_endpoint = GITHUB_MIRROR_ENDPOINTS.first().expect("github endpoint");
    let mut candidates = Vec::with_capacity(manifests.len());
    let mut seen_names = HashSet::new();
    for skill_manifest in manifests {
        let skill_md_url = raw_file_url(direct_endpoint, repo, &skill_manifest.skill_md_path);
        let raw = match fetch_raw_bytes(client, &skill_md_url, auth).await {
            Ok(bytes) => bytes,
            Err(error) => return Ok(map_acquisition_error_to_outcome(error)),
        };
        if let Err(budget_error) = ResourceBudget::default_skill()
            .reject_file_read_size(&skill_manifest.skill_md_path, raw.len() as u64)
        {
            return Ok(map_acquisition_error_to_outcome(GithubImportError::Budget(
                budget_error,
            )));
        }
        let candidate = match build_remote_skill_candidate(
            repo,
            &skill_manifest,
            raw,
            direct_endpoint,
        ) {
            Ok(candidate) => candidate,
            Err(invalid) => {
                // Hint-derived invalid candidates are dropped silently
                // (additive hints must not suppress recursive discovery).
                if skill_manifest.from_manifest_hint {
                    continue;
                }
                return Err(GithubImportError::InvalidCandidate(invalid.detail));
            }
        };
        if is_generic_remote_skill_candidate(&candidate) {
            continue;
        }
        if !seen_names.insert(candidate.skill_name.clone()) {
            continue;
        }
        candidates.push(candidate);
    }

    Ok(TreeFastPathOutcome::Ready {
        candidates,
        repository_files,
    })
}

/// Convert an acquisition-layer error into a `Fallback` outcome when the
/// classifier recognizes it, otherwise surface it as a domain error. Used at
/// every acquisition step of the tree fast-path so the dispatcher's fallback
/// decision is centralized.
pub(super) fn map_acquisition_error_to_outcome(error: GithubImportError) -> TreeFastPathOutcome {
    match tree_manifest::fallback_reason_for(&error) {
        Some(reason) => TreeFastPathOutcome::Fallback(reason),
        None => {
            // Unrecognized error — propagate. In practice this branch is
            // unreachable for acquisition steps (the classifier covers all
            // HTTP/budget/integrity/parse variants they can emit), but kept
            // for safety so a future domain variant is not silently swallowed.
            // We cannot return `Err` from this helper without changing the
            // call sites, so encode the error as a `Fallback` to keep the
            // dispatcher safe (archive acquisition will re-surface it if it
            // is a real domain error that archive would also hit).
            TreeFastPathOutcome::Fallback(tree_manifest::FallbackReason::Transport)
        }
    }
}

/// Build the preview repository file list from the parsed tree manifest. The
/// output mirrors `snapshot_preview_repository_files` so the preview file
/// manifest attachment (`attach_preview_file_manifests`) produces the same
/// `GitHubSkillPreviewFile` set as the archive path.
pub(super) fn manifest_to_preview_repository_files(
    manifest: &tree_manifest::RepositoryManifest,
) -> Vec<PreviewRepositoryFile> {
    let mut files: Vec<_> = manifest
        .regular_files
        .iter()
        .map(|file| PreviewRepositoryFile {
            repo_path: file.repo_path.clone(),
            byte_len: file.byte_len,
        })
        .collect();
    files.sort_by(|left, right| left.repo_path.cmp(&right.repo_path));
    files
}

/// Resolve plugin manifest discovery from raw bytes fetched through the tree
/// fast-path. Only fetches `.claude-plugin/plugin.json` and `marketplace.json`
/// when the tree manifest lists them; missing paths mean the repo has no
/// manifest (continue with no grouping, matching the archive path).
async fn build_tree_plugin_discovery(
    client: &reqwest::Client,
    manifest: &tree_manifest::RepositoryManifest,
    repo: &GitHubRepoRef,
    source_path: Option<&str>,
    auth: Option<&str>,
) -> Result<PluginManifestDiscovery, GithubImportError> {
    let base_path = effective_source_root(source_path)?;
    let plugin_json_path = join_repo_path(&base_path, ".claude-plugin/plugin.json")?;
    let marketplace_json_path = join_repo_path(&base_path, ".claude-plugin/marketplace.json")?;

    let plugin_json =
        fetch_optional_manifest_bytes(client, manifest, &plugin_json_path, repo, auth).await?;
    let marketplace_json = fetch_optional_manifest_bytes(
        client,
        manifest,
        &marketplace_json_path,
        repo,
        auth,
    )
    .await?;

    Ok(plugin_manifest_discovery_from_manifest_bytes(
        &base_path,
        plugin_json.as_deref(),
        marketplace_json.as_deref(),
    ))
}

/// Fetch an optional plugin manifest file's raw bytes. Returns `None` when the
/// tree manifest does not list the path (the repo has no such manifest). Any
/// raw fetch failure (404 integrity gap, denial, transport, budget) propagates
/// so the dispatcher falls back to archive acquisition, preserving parity
/// with the archive path (which would read the manifest from the tarball).
async fn fetch_optional_manifest_bytes(
    client: &reqwest::Client,
    manifest: &tree_manifest::RepositoryManifest,
    path: &str,
    repo: &GitHubRepoRef,
    auth: Option<&str>,
) -> Result<Option<Vec<u8>>, GithubImportError> {
    let exists = manifest.regular_paths().any(|repo_path| repo_path == path);
    if !exists {
        return Ok(None);
    }
    let endpoint = GITHUB_MIRROR_ENDPOINTS.first().expect("github endpoint");
    let url = raw_file_url(endpoint, repo, path);
    let bytes = fetch_raw_bytes(client, &url, auth).await?;
    Ok(Some(bytes))
}

pub(crate) async fn preview_github_repo_import_with_auth(
    pool: &DbPool,
    repo_url: &str,
    auth: Option<&str>,
) -> Result<GitHubRepoPreview, GithubImportError> {
    let resolved = resolve_repo_source(repo_url, auth).await?;
    let client = github_client()?;

    // Try the tree-manifest fast-path first. Acquisition failures fall back to
    // the existing archive path; domain failures (invalid candidate) are
    // surfaced directly because archive acquisition would produce the same
    // domain error (discovery + frontmatter interpretation are shared).
    let (candidates, repository_files) =
        match try_build_preview_from_tree_manifest(
            &client,
            &resolved.repo,
            resolved.source_path.as_deref(),
            auth,
        )
        .await
        {
            Ok(TreeFastPathOutcome::Ready {
                candidates,
                repository_files,
            }) => (candidates, repository_files),
            Ok(TreeFastPathOutcome::Fallback(_)) => {
                let snapshot = download_repo_snapshot(&client, &resolved.repo, auth).await?;
                let candidates = build_repo_skill_candidates_from_snapshot_at_path(
                    &resolved.repo,
                    &snapshot,
                    resolved.source_path.as_deref(),
                )?;
                (candidates, snapshot_preview_repository_files(&snapshot))
            }
            Err(error) => return Err(error),
        };

    let mut skills = build_preview_skills(pool, &candidates).await?;

    if skills.is_empty() {
        return Err(GithubImportError::NoImportableSkills);
    }
    attach_preview_file_manifests(&mut skills, &repository_files)?;

    Ok(GitHubRepoPreview {
        repo: resolved.repo,
        skills,
        preview_workspace_id: None,
    })
}

pub(crate) async fn preview_github_repo_import_remote_with_auth(
    pool: &DbPool,
    active_target: &ActiveTarget,
    repo_url: &str,
    auth: Option<&str>,
) -> Result<GitHubRepoPreview, GithubImportError> {
    let resolved = resolve_repo_source(repo_url, auth).await?;
    let connection = connect_remote_target(active_target)
        .await
        .map_err(|e| GithubImportError::Remote(e.to_string()))?;
    cleanup_expired_preview_workspaces_for_connection(&connection).await;

    let workspace = create_remote_preview_workspace(&connection, &resolved, auth).await?;
    let preview_result = async {
        let candidates = build_remote_repo_skill_candidates_from_workspace(
            &connection,
            &resolved.repo,
            &workspace.remote_repo_dir,
            resolved.source_path.as_deref(),
        )
        .await?;
        let mut skills = build_preview_skills(pool, &candidates).await?;
        if skills.is_empty() {
            return Err(GithubImportError::NoImportableSkills);
        }
        let repository_files =
            remote_preview_repository_files(&connection, &workspace.remote_repo_dir).await?;
        attach_preview_file_manifests(&mut skills, &repository_files)?;
        Ok(skills)
    }
    .await;

    match preview_result {
        Ok(skills) => {
            register_preview_workspace(workspace.clone());
            Ok(GitHubRepoPreview {
                repo: resolved.repo,
                skills,
                preview_workspace_id: Some(workspace.id),
            })
        }
        Err(error) => {
            let _ = connection
                .remove_tree(&workspace.remote_workspace_dir)
                .await;
            Err(error)
        }
    }
}
