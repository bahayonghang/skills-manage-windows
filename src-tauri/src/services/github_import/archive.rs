pub(crate) async fn download_repo_snapshot(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    auth_token: Option<&str>,
) -> Result<GitHubRepoSnapshot, String> {
    let archive = download_repository_archive(client, repo, auth_token).await?;
    snapshot_from_repository_archive(&archive)
}

async fn download_repository_archive(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    auth_token: Option<&str>,
) -> Result<Vec<u8>, String> {
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
        return Err("GitHub repository archive is unavailable.".to_string());
    }
    if !response.status().is_success() {
        let status = response.status();
        return Err(classify_github_denial_response(
            response,
            "downloading the repository archive",
        )
        .await
        .unwrap_or_else(|| {
            format!(
                "Failed to download GitHub repository archive: HTTP {}",
                status
            )
        }));
    }

    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|e| format!("Failed to read GitHub repository archive: {}", e))
}

fn snapshot_from_repository_archive(archive_bytes: &[u8]) -> Result<GitHubRepoSnapshot, String> {
    let cursor = Cursor::new(archive_bytes);
    let decoder = GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(decoder);
    let mut files = HashMap::new();

    for entry_result in archive
        .entries()
        .map_err(|e| format!("Failed to inspect GitHub repository archive: {}", e))?
    {
        let mut entry = entry_result
            .map_err(|e| format!("Failed to inspect GitHub repository archive: {}", e))?;

        if !entry.header().entry_type().is_file() {
            continue;
        }

        let relative_path = relative_archive_path(&entry)?;
        let mut content = Vec::new();
        entry.read_to_end(&mut content).map_err(|e| {
            format!(
                "Failed to read GitHub repository archive entry '{}': {}",
                relative_path, e
            )
        })?;
        files.insert(relative_path, content);
    }

    Ok(GitHubRepoSnapshot { files })
}

fn relative_archive_path<R: Read>(entry: &tar::Entry<'_, R>) -> Result<String, String> {
    let archive_path = entry
        .path()
        .map_err(|e| format!("Failed to inspect GitHub repository archive: {}", e))?;
    let relative = archive_path
        .components()
        .skip(1)
        .map(|component| match component {
            Component::Normal(value) => Ok(value.to_string_lossy().into_owned()),
            _ => Err("GitHub repository archive contains an unsupported path.".to_string()),
        })
        .collect::<Result<Vec<_>, _>>()?;

    if relative.is_empty() {
        return Err("GitHub repository archive contains an unsupported path.".to_string());
    }

    let joined = relative.join("/");
    if !is_safe_repo_relative_path(&joined) {
        return Err(format!(
            "GitHub repository archive contains an unsupported path '{}'.",
            joined
        ));
    }

    Ok(joined)
}

