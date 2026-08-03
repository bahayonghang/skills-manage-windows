use crate::services::{
    bounded_ingestion::{read_response_bytes_bounded, BoundedReadError, ReadLimit},
    resource_budget::{BudgetExceeded, ResourceBudget},
};

use super::*;

pub(crate) async fn download_repo_snapshot(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    auth_token: Option<&str>,
) -> Result<GitHubRepoSnapshot, GithubImportError> {
    let archive = download_repository_archive(client, repo, auth_token).await?;
    snapshot_from_repository_archive(&archive)
}

pub(super) async fn download_repository_archive(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    auth_token: Option<&str>,
) -> Result<Vec<u8>, GithubImportError> {
    download_repository_archive_with_budget(
        client,
        repo,
        auth_token,
        ResourceBudget::default_skill(),
    )
    .await
}

async fn download_repository_archive_with_budget(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    auth_token: Option<&str>,
    budget: ResourceBudget,
) -> Result<Vec<u8>, GithubImportError> {
    let response = send_github_request_with_fallback(
        client,
        GitHubFetchSurface::Api,
        |endpoint| {
            github_endpoint_url(
                endpoint,
                GitHubFetchSurface::Api,
                &format!(
                    "/repos/{}/{}/tarball/{}",
                    repo.owner, repo.repo, repo.branch
                ),
            )
        },
        "Failed to download GitHub repository archive",
        auth_token,
    )
    .await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(GithubImportError::ArchiveUnavailable);
    }
    if !response.status().is_success() {
        let status = response.status();
        return Err(classify_github_denial_response(
            response,
            "downloading the repository archive",
        )
        .await
        .unwrap_or_else(|| {
            GithubImportError::Http(format!(
                "Failed to download GitHub repository archive: HTTP {}",
                status
            ))
        }));
    }

    read_response_bytes_bounded(
        response,
        ReadLimit::new("GitHub repository archive", budget.archive_bytes),
    )
    .await
    .map_err(|error| match error {
        BoundedReadError::LimitExceeded { actual, limit, .. } => GithubImportError::Budget(
            BudgetExceeded::new("GitHub repository archive", actual, limit),
        ),
        _ => GithubImportError::Http("Failed to read GitHub repository archive.".to_string()),
    })
}

pub(super) fn snapshot_from_repository_archive(
    archive_bytes: &[u8],
) -> Result<GitHubRepoSnapshot, GithubImportError> {
    snapshot_from_repository_archive_with_budget(archive_bytes, ResourceBudget::default_skill())
}

pub(super) fn snapshot_from_repository_archive_with_budget(
    archive_bytes: &[u8],
    budget: ResourceBudget,
) -> Result<GitHubRepoSnapshot, GithubImportError> {
    budget
        .reject_archive_size(archive_bytes.len() as u64)
        .map_err(GithubImportError::Budget)?;

    let cursor = Cursor::new(archive_bytes);
    let decoder = GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(decoder);
    let mut files = HashMap::new();
    let mut expanded_bytes = 0_u64;

    for entry_result in archive
        .entries()
        .map_err(|e| GithubImportError::io("Failed to inspect GitHub repository archive", e))?
    {
        let mut entry = entry_result
            .map_err(|e| GithubImportError::io("Failed to inspect GitHub repository archive", e))?;

        if !entry.header().entry_type().is_file() {
            continue;
        }

        if files.len() >= budget.archive_files {
            return Err(GithubImportError::ArchiveFileBudgetExceeded(
                budget.archive_files,
            ));
        }

        let relative_path = relative_archive_path(&entry)?;
        let entry_size = entry.header().size().map_err(|e| {
            GithubImportError::io("Failed to inspect GitHub repository archive entry size", e)
        })?;
        budget
            .reject_archive_entry_size(&relative_path, entry_size)
            .map_err(GithubImportError::Budget)?;
        expanded_bytes = expanded_bytes
            .checked_add(entry_size)
            .ok_or(GithubImportError::ArchiveSizeOverflow)?;
        budget
            .reject_archive_expanded_size(expanded_bytes)
            .map_err(GithubImportError::Budget)?;

        let mut content = Vec::new();
        entry.read_to_end(&mut content).map_err(|e| {
            GithubImportError::io(
                format!(
                    "Failed to read GitHub repository archive entry '{}'",
                    relative_path
                ),
                e,
            )
        })?;
        let actual_entry_size = content.len() as u64;
        budget
            .reject_archive_entry_size(&relative_path, actual_entry_size)
            .map_err(GithubImportError::Budget)?;
        if actual_entry_size != entry_size {
            let corrected_expanded_bytes = expanded_bytes
                .checked_sub(entry_size)
                .and_then(|size| size.checked_add(actual_entry_size))
                .ok_or(GithubImportError::ArchiveSizeOverflow)?;
            budget
                .reject_archive_expanded_size(corrected_expanded_bytes)
                .map_err(GithubImportError::Budget)?;
            expanded_bytes = corrected_expanded_bytes;
        }
        files.insert(relative_path, content);
    }

    Ok(GitHubRepoSnapshot { files })
}

pub(super) fn relative_archive_path<R: Read>(
    entry: &tar::Entry<'_, R>,
) -> Result<String, GithubImportError> {
    let archive_path = entry
        .path()
        .map_err(|e| GithubImportError::io("Failed to inspect GitHub repository archive", e))?;
    let relative = archive_path
        .components()
        .skip(1)
        .map(|component| match component {
            Component::Normal(value) => Ok(value.to_string_lossy().into_owned()),
            _ => Err(GithubImportError::ArchiveUnsupportedPath),
        })
        .collect::<Result<Vec<_>, _>>()?;

    if relative.is_empty() {
        return Err(GithubImportError::ArchiveUnsupportedPath);
    }

    let joined = relative.join("/");
    if !is_safe_repo_relative_path(&joined) {
        return Err(GithubImportError::ArchiveUnsupportedPathNamed(joined));
    }

    Ok(joined)
}
