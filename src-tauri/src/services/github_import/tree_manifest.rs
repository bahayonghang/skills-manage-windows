//! Git tree manifest acquisition for the GitHub import fast-path.
//!
//! This module owns the typed structures and the pure parser for the recursive
//! Git tree API response (`/repos/{owner}/{repo}/git/trees/{branch}?recursive=1`).
//! It does **not** perform HTTP, touch the DB, or write to Central — it is the
//! acquisition-layer primitive the preview/import dispatcher (Commit 2/3) will
//! call to obtain a `RepositoryManifest`, then feed into the existing
//! path-based discovery (`discover_skill_manifests_from_paths_with_plugin_discovery`).
//!
//! Parity contract with the archive parser (`archive.rs`):
//!
//! | Git mode | `RepositoryFileKind` | Archive behaviour |
//! | --- | --- | --- |
//! | `100644` / `100755` blob | `RegularBlob` | regular file → snapshot |
//! | `120000` blob | `SymlinkBlob` | tar symlink entry → skipped |
//! | `160000` commit | `Gitlink` | tar gitlink/dir absence → skipped |
//! | `040000` tree | (excluded, no size) | tar directory entry → skipped |
//! | unknown | `Other` | n/a — typed fallback to archive |
//!
//! Unknown mode/type combinations return a typed `GithubImportError` so the
//! dispatcher can switch to archive acquisition (where the equivalent entries
//! are simply absent from the tar regular-file set), keeping the candidate/
//! preview/file-manifest output equivalent.

use crate::services::{
    bounded_ingestion::{read_response_text_bounded, BoundedReadError, ReadLimit},
    resource_budget::{BudgetExceeded, ResourceBudget},
};

use super::*;

/// Classification of a Git tree entry by its `mode` + `type` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RepositoryFileKind {
    /// `100644` or `100755` blob — a regular file. Only these enter the
    /// candidate path set and the raw-download plan.
    RegularBlob,
    /// `120000` blob — a symlink. Skipped from candidates and raw downloads,
    /// matching the archive parser's `is_file()` filter.
    SymlinkBlob,
    /// `160000` commit — a submodule gitlink. Skipped, matching the archive
    /// parser (no tar regular-file entry exists for submodules).
    Gitlink,
    /// Any other mode/type combination. The dispatcher treats `Other` as a
    /// typed fallback signal so acquisition switches to archive.
    Other,
}

/// A single regular blob discovered in the Git tree response, ready for
/// candidate discovery and bounded raw download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RepositoryFileMeta {
    pub(super) repo_path: String,
    pub(super) byte_len: u64,
    pub(super) kind: RepositoryFileKind,
}

/// The parsed repository tree manifest. `regular_files` is the candidate set
/// fed to `discover_skill_manifests_from_paths_with_plugin_discovery`;
/// `skipped` records symlink/gitlink/other entries for internal diagnostics
/// only (never serialized to the frontend or Central).
#[derive(Debug, Clone)]
pub(super) struct RepositoryManifest {
    /// Commit 3 (import cost model + diagnostics) reads the repo ref to
    /// attribute acquisition decisions; preview only needs `regular_files`.
    #[allow(dead_code)]
    pub(super) repo: GitHubRepoRef,
    pub(super) regular_files: Vec<RepositoryFileMeta>,
    /// Symlink/gitlink diagnostics. Commit 3 records these in acquisition
    /// telemetry; preview does not read them.
    #[allow(dead_code)]
    pub(super) skipped: Vec<RepositoryFileMeta>,
}

impl RepositoryManifest {
    /// Regular-blob paths, suitable for feeding into
    /// `discover_skill_manifests_from_paths_with_plugin_discovery`.
    pub(super) fn regular_paths(&self) -> impl Iterator<Item = &str> {
        self.regular_files
            .iter()
            .map(|file| file.repo_path.as_str())
    }

    /// Sum of regular-blob byte lengths. Used by the dispatcher's cost model
    /// (`tree_raw_cost = request_overhead * file_count + selected_bytes`).
    /// Commit 3 (import cost decision) reads this; preview does not.
    #[allow(dead_code)]
    pub(super) fn regular_total_bytes(&self) -> u64 {
        self.regular_files.iter().map(|file| file.byte_len).sum()
    }
}

/// Acquisition strategy chosen by the dispatcher. `TreeRaw` downloads only the
/// selected subtree's raw blobs; `Archive` falls back to the existing tarball
/// path. Kept in the acquisition module so the Commit 3 import dispatcher can
/// return it from its cost decision without re-deriving the variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum AcquisitionMode {
    TreeRaw,
    Archive,
}

/// Reason the dispatcher switched from `TreeRaw` to `Archive`. The dispatcher
/// maps `GithubImportError` variants from the tree parser into this enum for
/// internal diagnostics/telemetry; it is not serialized to the frontend or
/// persisted into Central/source metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FallbackReason {
    /// `GithubImportError::TreeManifestTruncated` — recursive tree truncated.
    Truncated,
    /// `GithubImportError::TreeManifestUnsupportedMode` — unknown mode/type.
    Unsupported,
    /// `GithubImportError::RateLimited` / `AccessDenied` — GitHub denial.
    Denied,
    /// `GithubImportError::Http` — transport/mirror failure.
    Transport,
    /// `GithubImportError::Budget` / `TreeManifestEntryBudgetExceeded` /
    /// `TreeManifestSizeOverflow` — resource budget exceeded.
    Budget,
    /// `GithubImportError::TreeManifestEntryMissingSize` — integrity gap.
    Integrity,
    /// `GithubImportError::TreeManifestEntryMissingSize` or cost model picked
    /// archive as cheaper for the selection (e.g. root scope, many files).
    /// Constructed by the Commit 3 import cost decision; preview never
    /// selects `Threshold` (it always tries TreeRaw then falls back).
    #[allow(dead_code)]
    Threshold,
}

// ── Serde model for the Git tree API response ─────────────────────────────

#[derive(Debug, Deserialize)]
pub(super) struct GitTreeResponse {
    #[serde(default)]
    tree: Vec<GitTreeEntry>,
    #[serde(default)]
    truncated: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct GitTreeEntry {
    path: String,
    mode: String,
    #[serde(rename = "type")]
    entry_type: String,
    #[serde(default)]
    size: Option<u64>,
}

/// Classify a tree entry by its mode/type pair. Returns `None` for `tree`
/// directory nodes (no size, no candidate relevance); returns `Some(kind)` for
/// blobs/commits/unknowns so the parser can route them.
pub(super) fn classify_tree_entry(mode: &str, entry_type: &str) -> Option<RepositoryFileKind> {
    match (mode, entry_type) {
        ("100644", "blob") | ("100755", "blob") => Some(RepositoryFileKind::RegularBlob),
        ("120000", "blob") => Some(RepositoryFileKind::SymlinkBlob),
        ("160000", "commit") => Some(RepositoryFileKind::Gitlink),
        ("040000", "tree") => None,
        _ => Some(RepositoryFileKind::Other),
    }
}

/// Parse the raw Git tree API response body into a `RepositoryManifest`.
///
/// The parser is pure: it takes the response body (already bounded by the
/// caller via `ResourceBudget::reject_tree_response_size`), classifies entries,
/// enforces entry-count and summed-byte budgets, detects `truncated`, and
/// rejects unsafe repository-relative paths through the shared
/// `normalize_repo_path` helper (same path policy as the archive parser).
///
/// Returns typed `GithubImportError` variants for every fast-path fallback
/// condition so the dispatcher can classify them into `FallbackReason` without
/// string sniffing.
pub(super) fn parse_tree_response(
    body: &str,
    repo: &GitHubRepoRef,
    budget: ResourceBudget,
) -> Result<RepositoryManifest, GithubImportError> {
    // Bound the response body before serde allocates the full structure.
    budget
        .reject_tree_response_size(body.len() as u64)
        .map_err(GithubImportError::Budget)?;

    let response: GitTreeResponse =
        serde_json::from_str(body).map_err(|e| GithubImportError::Parse(e.to_string()))?;

    if response.truncated {
        return Err(GithubImportError::TreeManifestTruncated);
    }

    let mut regular_files = Vec::new();
    let mut skipped = Vec::new();
    let mut seen_paths = HashSet::new();
    let mut total_bytes = 0u64;

    for entry in response.tree {
        // Reject entries with unsafe paths through the shared path policy so
        // the tree parser cannot introduce traversal/absolute-path entries
        // that the archive parser would have rejected via
        // `is_safe_repo_relative_path`.
        let repo_path = normalize_repo_path(&entry.path)?;

        let kind = match classify_tree_entry(&entry.mode, &entry.entry_type) {
            None => continue, // tree directory node — skip, like archive's is_file() filter
            Some(kind) => kind,
        };

        if kind == RepositoryFileKind::RegularBlob {
            let byte_len = entry.size.ok_or_else(|| {
                GithubImportError::TreeManifestEntryMissingSize(repo_path.clone())
            })?;
            budget
                .reject_archive_entry_size(&repo_path, byte_len)
                .map_err(GithubImportError::Budget)?;

            if regular_files.len() >= budget.tree_entries {
                return Err(GithubImportError::TreeManifestEntryBudgetExceeded(
                    budget.tree_entries,
                ));
            }
            if !seen_paths.insert(repo_path.clone()) {
                // Duplicate path in tree response — treat as integrity failure
                // (archive parser would have overwritten the HashMap entry; we
                // fail closed to surface the upstream anomaly).
                return Err(GithubImportError::TreeManifestEntryMissingSize(repo_path));
            }
            total_bytes = total_bytes
                .checked_add(byte_len)
                .ok_or(GithubImportError::TreeManifestSizeOverflow)?;
            budget
                .reject_archive_expanded_size(total_bytes)
                .map_err(GithubImportError::Budget)?;
            regular_files.push(RepositoryFileMeta {
                repo_path,
                byte_len,
                kind,
            });
        } else if kind == RepositoryFileKind::SymlinkBlob || kind == RepositoryFileKind::Gitlink {
            // Symlink/gitlink: safely ignored, matching the archive parser's
            // `is_file()` filter. Recorded for diagnostics only; never enters
            // the candidate set or raw-download plan.
            skipped.push(RepositoryFileMeta {
                repo_path,
                byte_len: entry.size.unwrap_or(0),
                kind,
            });
        } else {
            // `Other` — unknown mode/type combination. The archive parser has
            // no equivalent entry (such modes never produce tar regular
            // files), so we cannot guarantee candidate/preview parity. Return
            // a typed fallback error so the dispatcher switches to archive.
            return Err(GithubImportError::TreeManifestUnsupportedMode {
                path: repo_path,
                mode: entry.mode,
            });
        }
    }

    // Stable ordering by repo_path, matching `parse_remote_preview_repository_files`
    // and `attach_preview_file_manifests` sort semantics so downstream discovery
    // and preview file manifests are deterministic.
    regular_files.sort_by(|left, right| left.repo_path.cmp(&right.repo_path));
    skipped.sort_by(|left, right| left.repo_path.cmp(&right.repo_path));

    Ok(RepositoryManifest {
        repo: repo.clone(),
        regular_files,
        skipped,
    })
}

/// Fetch the recursive Git tree manifest through the shared GitHub/mirror
/// fallback boundary and parse it with [`parse_tree_response`].
///
/// This is the acquisition-layer entry point the preview dispatcher calls
/// before deciding between TreeRaw and Archive. It performs no DB, no
/// Central write, and no candidate discovery — it only obtains the typed
/// `RepositoryManifest` (or a typed fallback error).
pub(super) async fn try_fetch_tree_manifest(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    auth_token: Option<&str>,
) -> Result<RepositoryManifest, GithubImportError> {
    let response = send_github_request_with_fallback(
        client,
        GitHubFetchSurface::Api,
        |endpoint| {
            github_endpoint_url(
                endpoint,
                GitHubFetchSurface::Api,
                &format!(
                    "/repos/{}/{}/git/trees/{}?recursive=1",
                    repo.owner, repo.repo, repo.branch
                ),
            )
        },
        "Failed to fetch GitHub repository tree",
        auth_token,
    )
    .await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        // resolve_repo_source already confirmed the repo exists; a tree 404
        // means the branch/ref is unreadable. The archive fallback will also
        // 404 (→ `ArchiveUnavailable`), so surface a typed transport fallback.
        return Err(GithubImportError::RepoNotFound);
    }
    if !response.status().is_success() {
        let status = response.status();
        return Err(
            classify_github_denial_response(response, "fetching the repository tree")
                .await
                .unwrap_or_else(|| {
                    GithubImportError::Http(format!(
                        "Failed to fetch GitHub repository tree: HTTP {}",
                        status
                    ))
                }),
        );
    }

    read_tree_response_with_budget(response, repo, ResourceBudget::default_skill()).await
}

pub(super) async fn read_tree_response_with_budget(
    response: reqwest::Response,
    repo: &GitHubRepoRef,
    budget: ResourceBudget,
) -> Result<RepositoryManifest, GithubImportError> {
    let body = read_response_text_bounded(
        response,
        ReadLimit::new(
            "GitHub repository tree API response",
            budget.tree_response_bytes,
        ),
    )
    .await
    .map_err(|error| match error {
        BoundedReadError::LimitExceeded { actual, limit, .. } => GithubImportError::Budget(
            BudgetExceeded::new("GitHub repository tree API response", actual, limit),
        ),
        BoundedReadError::InvalidUtf8 { .. } => GithubImportError::Parse(
            "Failed to parse repository tree response: response is not valid UTF-8".to_string(),
        ),
        _ => GithubImportError::Http("Failed to read repository tree.".to_string()),
    })?;
    parse_tree_response(&body, repo, budget)
}

/// Classify an acquisition-layer error into a [`FallbackReason`] so the
/// dispatcher can record why it switched from TreeRaw to Archive. Returns
/// `None` for domain errors (e.g. `InvalidCandidate`) that the archive path
/// would surface identically — those do **not** trigger fallback.
pub(super) fn fallback_reason_for(error: &GithubImportError) -> Option<FallbackReason> {
    match error {
        GithubImportError::TreeManifestTruncated => Some(FallbackReason::Truncated),
        GithubImportError::TreeManifestUnsupportedMode { .. } => Some(FallbackReason::Unsupported),
        GithubImportError::RateLimited(_) | GithubImportError::AccessDenied(_) => {
            Some(FallbackReason::Denied)
        }
        GithubImportError::Http(_) | GithubImportError::RepoNotFound => {
            Some(FallbackReason::Transport)
        }
        GithubImportError::Budget(_)
        | GithubImportError::TreeManifestEntryBudgetExceeded(_)
        | GithubImportError::TreeManifestSizeOverflow => Some(FallbackReason::Budget),
        GithubImportError::TreeManifestEntryMissingSize(_)
        | GithubImportError::RepoFileGone(_)
        | GithubImportError::Parse(_)
        | GithubImportError::RepoFileSizeMismatch { .. } => Some(FallbackReason::Integrity),
        // Domain errors (invalid candidate, no importable skills, path policy
        // violations on candidate discovery) are not acquisition fallbacks —
        // archive would produce the same domain failure.
        _ => None,
    }
}

#[cfg(test)]
impl RepositoryFileMeta {
    pub(super) fn new(repo_path: &str, byte_len: u64, kind: RepositoryFileKind) -> Self {
        Self {
            repo_path: repo_path.to_string(),
            byte_len,
            kind,
        }
    }
}
