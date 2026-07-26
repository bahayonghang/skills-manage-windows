//! Selected-subtree TreeRaw acquisition for local GitHub imports.

use futures_util::{stream, StreamExt};
use std::collections::BTreeMap;

use crate::services::resource_budget::ResourceBudget;

use super::*;

const TREE_RAW_DOWNLOAD_CONCURRENCY: usize = 8;
const TREE_RAW_MAX_REQUESTS: usize = 64;
const TREE_RAW_MAX_SELECTED_BYTES: u64 = 8 * 1024 * 1024;

pub(super) struct TreeSelectionPlan {
    pub(super) mode: tree_manifest::AcquisitionMode,
    pub(super) fallback_reason: Option<tree_manifest::FallbackReason>,
    pub(super) files: Vec<tree_manifest::RepositoryFileMeta>,
    pub(super) selected_bytes: u64,
}

pub(super) enum TreeImportOutcome {
    Ready {
        snapshot: GitHubRepoSnapshot,
        inspected: InspectedGitHubRepoSkills,
    },
    Fallback(tree_manifest::FallbackReason),
}

pub(super) async fn try_prepare_tree_import(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    source_path: Option<&str>,
    selections: &[GitHubSkillImportSelection],
    auth: Option<&str>,
    allow_invalid_candidates: bool,
) -> Result<TreeImportOutcome, GithubImportError> {
    let started = Instant::now();
    let manifest = match tree_manifest::try_fetch_tree_manifest(client, repo, auth).await {
        Ok(manifest) => manifest,
        Err(error) => return tree_import_fallback(error, "tree_manifest", started),
    };
    let inspection = match preview::inspect_tree_candidates_from_manifest(
        client,
        &manifest,
        repo,
        source_path,
        auth,
    )
    .await
    {
        Ok(inspection) => inspection,
        Err(error) => return tree_import_fallback(error, "candidate_metadata", started),
    };
    if !allow_invalid_candidates {
        if let Some(invalid) = inspection.inspected.invalid_candidates.first() {
            return Err(GithubImportError::InvalidCandidate(invalid.detail.clone()));
        }
    }
    if inspection.inspected.valid_candidates.is_empty()
        && inspection.inspected.invalid_candidates.is_empty()
    {
        return Err(GithubImportError::NoImportableSkills);
    }

    let planned_selections = if allow_invalid_candidates {
        let valid_paths = inspection
            .inspected
            .valid_candidates
            .iter()
            .map(|candidate| candidate.source_path.as_str())
            .collect::<HashSet<_>>();
        selections
            .iter()
            .filter(|selection| valid_paths.contains(selection.source_path.as_str()))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        selections.to_vec()
    };

    let plan = match plan_tree_selection(
        &manifest,
        &inspection.inspected.valid_candidates,
        &planned_selections,
    ) {
        Ok(plan) => plan,
        Err(error) => return tree_import_fallback(error, "selection_plan", started),
    };
    if plan.mode == tree_manifest::AcquisitionMode::Archive {
        let reason = plan
            .fallback_reason
            .unwrap_or(tree_manifest::FallbackReason::Threshold);
        tracing::debug!(
            acquisition_mode = "archive",
            fallback_reason = ?reason,
            request_count = plan.files.len(),
            transferred_bytes = plan.selected_bytes,
            elapsed_ms = started.elapsed().as_millis(),
            "GitHub import acquisition selected archive fallback"
        );
        return Ok(TreeImportOutcome::Fallback(reason));
    }

    let snapshot =
        match download_tree_selection(client, repo, &plan, auth, inspection.fetched_files).await {
            Ok(snapshot) => snapshot,
            Err(error) => return tree_import_fallback(error, "raw_download", started),
        };
    tracing::debug!(
        acquisition_mode = "tree_raw",
        request_count = plan.files.len(),
        transferred_bytes = plan.selected_bytes,
        elapsed_ms = started.elapsed().as_millis(),
        "GitHub import acquisition prepared selected repository files"
    );
    Ok(TreeImportOutcome::Ready {
        snapshot,
        inspected: inspection.inspected,
    })
}

fn tree_import_fallback(
    error: GithubImportError,
    stage: &'static str,
    started: Instant,
) -> Result<TreeImportOutcome, GithubImportError> {
    match tree_manifest::fallback_reason_for(&error) {
        Some(reason) => {
            tracing::debug!(
                acquisition_mode = "archive",
                fallback_reason = ?reason,
                fallback_stage = stage,
                elapsed_ms = started.elapsed().as_millis(),
                "GitHub import tree acquisition fell back to archive"
            );
            Ok(TreeImportOutcome::Fallback(reason))
        }
        None => Err(error),
    }
}

pub(super) fn plan_tree_selection(
    manifest: &tree_manifest::RepositoryManifest,
    candidates: &[RemoteSkillCandidate],
    selections: &[GitHubSkillImportSelection],
) -> Result<TreeSelectionPlan, GithubImportError> {
    let mut selected_paths = HashSet::new();
    let mut files_by_path = BTreeMap::new();
    let mut includes_root = false;

    for selection in selections {
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.source_path == selection.source_path)
            .ok_or_else(|| {
                GithubImportError::SelectionUnavailable(selection.source_path.clone())
            })?;
        if !selected_paths.insert(candidate.source_path.clone()) {
            return Err(GithubImportError::DuplicateSelection(
                candidate.source_path.clone(),
            ));
        }
        if selection.resolution == DuplicateResolution::Skip {
            continue;
        }
        includes_root |= candidate.source_path == ".";
        let mut candidate_file_count = 0;
        for file in &manifest.regular_files {
            if repo_file_relative_to_source(&file.repo_path, &candidate.source_path).is_some() {
                files_by_path
                    .entry(file.repo_path.clone())
                    .or_insert_with(|| file.clone());
                candidate_file_count += 1;
            }
        }
        if candidate_file_count == 0 {
            return Err(GithubImportError::RepoPathGone(
                candidate.source_path.clone(),
            ));
        }
    }

    let files = files_by_path.into_values().collect::<Vec<_>>();
    let selected_bytes = files.iter().try_fold(0_u64, |total, file| {
        total
            .checked_add(file.byte_len)
            .ok_or(GithubImportError::TreeManifestSizeOverflow)
    })?;
    ResourceBudget::default_skill()
        .reject_archive_expanded_size(selected_bytes)
        .map_err(GithubImportError::Budget)?;
    let threshold_exceeded = includes_root
        || files.len() > TREE_RAW_MAX_REQUESTS
        || selected_bytes > TREE_RAW_MAX_SELECTED_BYTES;

    Ok(TreeSelectionPlan {
        mode: if threshold_exceeded {
            tree_manifest::AcquisitionMode::Archive
        } else {
            tree_manifest::AcquisitionMode::TreeRaw
        },
        fallback_reason: threshold_exceeded.then_some(tree_manifest::FallbackReason::Threshold),
        files,
        selected_bytes,
    })
}

pub(super) async fn download_tree_selection(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    plan: &TreeSelectionPlan,
    auth: Option<&str>,
    fetched_files: HashMap<String, Vec<u8>>,
) -> Result<GitHubRepoSnapshot, GithubImportError> {
    let planned_paths = plan
        .files
        .iter()
        .map(|file| file.repo_path.as_str())
        .collect::<HashSet<_>>();
    let expected_by_path = plan
        .files
        .iter()
        .map(|file| (file.repo_path.as_str(), file.byte_len))
        .collect::<HashMap<_, _>>();
    let mut files = HashMap::new();
    for (path, bytes) in fetched_files {
        if !planned_paths.contains(path.as_str()) {
            continue;
        }
        let expected = expected_by_path[&path.as_str()];
        validate_tree_file_size(&path, expected, bytes.len() as u64)?;
        files.insert(path, bytes);
    }

    let missing = plan
        .files
        .iter()
        .filter(|file| !files.contains_key(&file.repo_path))
        .cloned()
        .collect::<Vec<_>>();
    let auth = auth.map(str::to_string);
    let downloads = stream::iter(missing.into_iter().map(|file| {
        let client = client.clone();
        let repo = repo.clone();
        let auth = auth.clone();
        async move {
            let bytes =
                fetch_raw_repo_file(&client, &repo, &file.repo_path, auth.as_deref()).await?;
            validate_tree_file_size(&file.repo_path, file.byte_len, bytes.len() as u64)?;
            Ok::<_, GithubImportError>((file.repo_path, bytes))
        }
    }))
    .buffer_unordered(TREE_RAW_DOWNLOAD_CONCURRENCY)
    .collect::<Vec<_>>()
    .await;
    for download in downloads {
        let (path, bytes) = download?;
        files.insert(path, bytes);
    }

    let actual_bytes = files.values().try_fold(0_u64, |total, bytes| {
        total
            .checked_add(bytes.len() as u64)
            .ok_or(GithubImportError::TreeManifestSizeOverflow)
    })?;
    if actual_bytes != plan.selected_bytes {
        return Err(GithubImportError::RepoFileSizeMismatch {
            path: "selected repository files".to_string(),
            expected: plan.selected_bytes,
            actual: actual_bytes,
        });
    }
    Ok(GitHubRepoSnapshot { files })
}

fn validate_tree_file_size(
    path: &str,
    expected: u64,
    actual: u64,
) -> Result<(), GithubImportError> {
    if expected != actual {
        return Err(GithubImportError::RepoFileSizeMismatch {
            path: path.to_string(),
            expected,
            actual,
        });
    }
    Ok(())
}
