//! Immutable preview snapshot acquisition, verification, and reads.
//!
//! Preview resolves the branch tip to a commit SHA once and performs every
//! acquisition request against that pinned commit, so a force-push between
//! resolution and acquisition cannot change the bytes the user reviews.
//! Import and Markdown reads then resolve content only through the registered
//! snapshot; they never re-resolve a branch.

use super::*;

/// Resolve a branch (or tag) tip to an immutable commit SHA.
pub(crate) async fn resolve_commit_sha(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    auth_token: Option<&str>,
) -> Result<String, GithubImportError> {
    validate_repo_ref(repo)?;
    let response = send_github_request_with_fallback(
        client,
        GitHubFetchSurface::Api,
        |endpoint| {
            github_endpoint_url(
                endpoint,
                GitHubFetchSurface::Api,
                &format!(
                    "/repos/{}/{}/commits/{}",
                    repo.owner, repo.repo, repo.branch
                ),
            )
        },
        "Failed to resolve GitHub repository commit",
        auth_token,
    )
    .await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(GithubImportError::RepoNotFound);
    }
    if !response.status().is_success() {
        let status = response.status();
        return Err(
            classify_github_denial_response(response, "resolving the repository commit")
                .await
                .unwrap_or_else(|| {
                    GithubImportError::Http(format!(
                        "Failed to resolve GitHub repository commit: HTTP {}",
                        status
                    ))
                }),
        );
    }

    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|e| GithubImportError::Parse(e.to_string()))?;
    let sha = payload
        .get("sha")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    validate_commit_sha(&sha)?;
    Ok(sha)
}

/// Accept only a full 40-character lowercase-hex commit SHA. The value becomes
/// an acquisition ref, so it must never carry path or query characters.
pub(crate) fn validate_commit_sha(value: &str) -> Result<(), GithubImportError> {
    if value.len() == 40 && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(GithubImportError::PreviewCommitUnresolved)
}

/// A repository ref pinned to the resolved commit.
///
/// Used only to construct acquisition URLs (tree / tarball / raw). Display
/// metadata, candidate download URLs, and persisted repository rows keep the
/// user-facing branch from the display ref.
pub(crate) fn pinned_repo_ref(repo: &GitHubRepoRef, commit_sha: &str) -> GitHubRepoRef {
    GitHubRepoRef {
        owner: repo.owner.clone(),
        repo: repo.repo.clone(),
        branch: commit_sha.to_string(),
        normalized_url: repo.normalized_url.clone(),
    }
}

/// Build the verified file inventory of a retained local snapshot.
pub(super) fn snapshot_files_from_local(snapshot: &GitHubRepoSnapshot) -> Vec<PreviewSnapshotFile> {
    let mut files = snapshot
        .files
        .iter()
        .map(|(repo_path, bytes)| PreviewSnapshotFile {
            repo_path: repo_path.clone(),
            byte_len: bytes.len() as u64,
            sha256: file_sha256(bytes),
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.repo_path.as_bytes().cmp(right.repo_path.as_bytes()));
    files
}

/// Domain-separated digest of the complete retained repository snapshot.
pub(super) fn repository_snapshot_digest(files: &[PreviewSnapshotFile]) -> String {
    aggregate_digest(
        REPOSITORY_SNAPSHOT_DIGEST_DOMAIN,
        &files
            .iter()
            .map(|file| DigestFileEntry {
                path: file.repo_path.clone(),
                byte_len: file.byte_len,
                sha256: file.sha256,
            })
            .collect::<Vec<_>>(),
    )
}

/// Compute the immutable repository digest directly from a retained Local
/// snapshot. Central Update uses the same framing as the GitHub preview
/// registry so cache hits and pinned reacquisition have one identity format.
pub(crate) fn repository_snapshot_digest_from_local(snapshot: &GitHubRepoSnapshot) -> String {
    repository_snapshot_digest(&snapshot_files_from_local(snapshot))
}

/// Domain-separated digest of one candidate skill's complete file set, using
/// paths relative to the final skill directory.
pub(super) fn candidate_content_digest(
    files: &[GitHubSkillPreviewFile],
) -> Result<String, GithubImportError> {
    let entries = files
        .iter()
        .map(|file| {
            let raw = decode_file_digest(&file.sha256)?;
            Ok(DigestFileEntry {
                path: file.path.clone(),
                byte_len: file.byte_len,
                sha256: raw,
            })
        })
        .collect::<Result<Vec<_>, GithubImportError>>()?;
    Ok(aggregate_digest(SKILL_CONTENT_DIGEST_DOMAIN, &entries))
}

pub(crate) fn candidate_content_digest_from_snapshot(
    snapshot: &GitHubRepoSnapshot,
    source_path: &str,
) -> Result<String, GithubImportError> {
    candidate_content_digest_from_repository_files(
        &snapshot_files_from_local(snapshot),
        source_path,
    )
}

pub(super) fn candidate_content_digest_from_repository_files(
    repository_files: &[PreviewSnapshotFile],
    source_path: &str,
) -> Result<String, GithubImportError> {
    let mut files = repository_files
        .iter()
        .filter_map(|file| {
            repo_file_relative_to_source(&file.repo_path, source_path).map(|path| {
                GitHubSkillPreviewFile {
                    path,
                    byte_len: file.byte_len,
                    sha256: encode_file_digest(&file.sha256),
                }
            })
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    if !files.iter().any(|file| file.path == "SKILL.md") {
        return Err(GithubImportError::PreviewFileManifestIncomplete(
            source_path.to_string(),
        ));
    }
    candidate_content_digest(&files)
}

pub(super) fn encode_file_digest(sha256: &[u8; 32]) -> String {
    format!("sha256-v1:{}", hex_encode(sha256))
}

fn decode_file_digest(value: &str) -> Result<[u8; 32], GithubImportError> {
    let hex = value
        .strip_prefix("sha256-v1:")
        .ok_or(GithubImportError::PreviewSnapshotIntegrity)?;
    if hex.len() != 64 {
        return Err(GithubImportError::PreviewSnapshotIntegrity);
    }
    let mut raw = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
        let text =
            std::str::from_utf8(chunk).map_err(|_| GithubImportError::PreviewSnapshotIntegrity)?;
        raw[index] = u8::from_str_radix(text, 16)
            .map_err(|_| GithubImportError::PreviewSnapshotIntegrity)?;
    }
    Ok(raw)
}

/// Recompute the repository snapshot digest from the retained storage and
/// compare it with the value registered at preview time.
///
/// Runs before any Central filesystem or database mutation. Failure is closed:
/// there is no branch re-fetch fallback.
pub(super) async fn verify_snapshot_integrity(
    active_target: &ActiveTarget,
    snapshot: &PreviewSnapshot,
) -> Result<(), GithubImportError> {
    let files = match &snapshot.storage {
        PreviewSnapshotStorage::Local(local) => snapshot_files_from_local(local),
        PreviewSnapshotStorage::Remote(workspace) => {
            let connection = connect_remote_target(active_target)
                .await
                .map_err(|e| GithubImportError::Remote(e.to_string()))?;
            remote_preview_repository_files(&connection, &workspace.remote_repo_dir).await?
        }
    };
    if repository_snapshot_digest(&files) != snapshot.snapshot_digest {
        return Err(GithubImportError::PreviewSnapshotIntegrity);
    }
    Ok(())
}

/// Read one repository file from a registered snapshot and re-verify it against
/// the registered manifest entry before returning bytes.
pub(super) async fn read_snapshot_repo_file(
    active_target: &ActiveTarget,
    snapshot: &PreviewSnapshot,
    repo_path: &str,
) -> Result<Vec<u8>, GithubImportError> {
    let expected = snapshot
        .file(repo_path)
        .ok_or_else(|| GithubImportError::RepoFileGone(repo_path.to_string()))?;
    let bytes = match &snapshot.storage {
        PreviewSnapshotStorage::Local(local) => local
            .files
            .get(repo_path)
            .cloned()
            .ok_or_else(|| GithubImportError::RepoFileGone(repo_path.to_string()))?,
        PreviewSnapshotStorage::Remote(workspace) => {
            let connection = connect_remote_target(active_target)
                .await
                .map_err(|e| GithubImportError::Remote(e.to_string()))?;
            connection
                .read_file_bounded(
                    &remote_join(&workspace.remote_repo_dir, repo_path),
                    expected.byte_len,
                )
                .await
                .map_err(|error| match error {
                    crate::targets::TargetsError::RemoteFileTooLarge { .. } => {
                        GithubImportError::PreviewSnapshotIntegrity
                    }
                    error => GithubImportError::Remote(error.to_string()),
                })?
        }
    };
    crate::services::resource_budget::ResourceBudget::default_skill()
        .reject_file_read_size(repo_path, bytes.len() as u64)
        .map_err(GithubImportError::Budget)?;
    if bytes.len() as u64 != expected.byte_len || file_sha256(&bytes) != expected.sha256 {
        return Err(GithubImportError::PreviewSnapshotIntegrity);
    }
    Ok(bytes)
}

/// Provenance persisted with an imported skill: the resolved commit SHA and the
/// candidate content digest confirmed in the preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ImportProvenance {
    pub(super) resolved_commit_sha: String,
    pub(super) content_digest_by_source_path: HashMap<String, String>,
}

impl ImportProvenance {
    pub(super) fn from_snapshot(snapshot: &PreviewSnapshot) -> Self {
        Self {
            resolved_commit_sha: snapshot.resolved_commit_sha.clone(),
            content_digest_by_source_path: snapshot
                .candidates
                .iter()
                .map(|candidate| {
                    (
                        candidate.source_path.clone(),
                        candidate.content_digest.clone(),
                    )
                })
                .collect(),
        }
    }

    pub(super) fn for_source_path(&self, source_path: &str) -> (Option<&str>, Option<&str>) {
        (
            Some(self.resolved_commit_sha.as_str()),
            self.content_digest_by_source_path
                .get(source_path)
                .map(String::as_str),
        )
    }
}

/// Resolve the provenance pair for a source path, tolerating a missing
/// provenance context (non-preview import paths stay `provenance unknown`).
pub(super) fn provenance_for(
    provenance: Option<&ImportProvenance>,
    source_path: &str,
) -> (Option<String>, Option<String>) {
    match provenance {
        Some(provenance) => {
            let (commit, digest) = provenance.for_source_path(source_path);
            (commit.map(str::to_string), digest.map(str::to_string))
        }
        None => (None, None),
    }
}

/// Validate that a registered snapshot still belongs to the active target and
/// to the repository/source the caller submitted.
///
/// The repository URL is parsed offline: it is binding evidence, never a
/// content locator.
#[cfg(test)]
pub(super) fn validate_snapshot_binding(
    snapshot: &PreviewSnapshot,
    active_target: &ActiveTarget,
    repo_url: &str,
) -> Result<(), GithubImportError> {
    validate_snapshot_binding_with_branch(snapshot, active_target, repo_url, None)
}

pub(super) fn validate_snapshot_binding_with_branch(
    snapshot: &PreviewSnapshot,
    active_target: &ActiveTarget,
    repo_url: &str,
    branch: Option<&str>,
) -> Result<(), GithubImportError> {
    if snapshot.target_id != active_target.id() || snapshot.target_kind != active_target.kind() {
        return Err(GithubImportError::PreviewTargetChanged);
    }
    let parsed = parse_github_source(repo_url)?;
    if parsed.owner != snapshot.repo.owner || parsed.repo != snapshot.repo.repo {
        return Err(GithubImportError::PreviewWorkspaceMismatch);
    }
    if let Some(branch) = reconcile_selected_branch(parsed.branch.as_deref(), branch)? {
        if branch != snapshot.repo.branch {
            return Err(GithubImportError::PreviewWorkspaceMismatch);
        }
    }
    if parsed.source_path.as_deref() != snapshot.source_path.as_deref() {
        return Err(GithubImportError::PreviewWorkspaceMismatch);
    }
    Ok(())
}

/// Read a previewed skill's `SKILL.md` from its registered snapshot.
///
/// Repeated reads never consume the token; they only succeed while the snapshot
/// is registered, unexpired, and bound to the active target.
pub(crate) async fn fetch_github_skill_markdown_from_snapshot(
    active_target: &ActiveTarget,
    preview_id: &str,
    repo: &GitHubRepoRef,
    source_path: &str,
) -> Result<String, GithubImportError> {
    validate_repo_ref(repo)?;
    let snapshot = lookup_preview_snapshot(preview_id, Utc::now())?;
    if snapshot.target_id != active_target.id() || snapshot.target_kind != active_target.kind() {
        return Err(GithubImportError::PreviewTargetChanged);
    }
    if &snapshot.repo != repo {
        return Err(GithubImportError::PreviewWorkspaceMismatch);
    }
    if snapshot.candidate(source_path).is_none() {
        return Err(GithubImportError::SelectionUnavailable(
            source_path.to_string(),
        ));
    }

    let skill_md_path = if source_path == "." {
        "SKILL.md".to_string()
    } else {
        join_repo_path(source_path, "SKILL.md")?
    };
    let bytes = read_snapshot_repo_file(active_target, &snapshot, &skill_md_path).await?;
    String::from_utf8(bytes).map_err(|e| {
        GithubImportError::Parse(format!("Preview SKILL.md is not valid UTF-8: {}", e))
    })
}
