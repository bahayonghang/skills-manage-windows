use crate::services::{
    bounded_ingestion::{read_response_bytes_bounded, BoundedReadError, ReadLimit},
    resource_budget::{BudgetExceeded, ResourceBudget},
};
use reqwest::{header::LOCATION, StatusCode, Url};

use super::*;

const CODELOAD_HOST: &str = "codeload.github.com";

#[derive(Clone)]
struct ArchiveAuthorityPolicy {
    scheme: String,
    host: String,
    port: u16,
}

impl ArchiveAuthorityPolicy {
    fn production(host: &str) -> Self {
        Self {
            scheme: "https".to_string(),
            host: host.to_string(),
            port: 443,
        }
    }

    #[cfg(test)]
    fn from_test_base(base_url: &str) -> Result<Self, GithubImportError> {
        let url = Url::parse(base_url).map_err(|_| GithubImportError::ArchiveRedirectRejected)?;
        let host = url
            .host_str()
            .ok_or(GithubImportError::ArchiveRedirectRejected)?;
        let port = url
            .port_or_known_default()
            .ok_or(GithubImportError::ArchiveRedirectRejected)?;
        Ok(Self {
            scheme: url.scheme().to_string(),
            host: host.to_string(),
            port,
        })
    }

    fn matches(&self, url: &Url) -> bool {
        url.scheme() == self.scheme
            && url.username().is_empty()
            && url.password().is_none()
            && url.host_str() == Some(self.host.as_str())
            && url.port_or_known_default() == Some(self.port)
            && url.query().is_none()
            && url.fragment().is_none()
    }
}

#[derive(Clone)]
struct ArchiveRedirectPolicy {
    api: ArchiveAuthorityPolicy,
    codeload: ArchiveAuthorityPolicy,
}

impl ArchiveRedirectPolicy {
    fn production() -> Self {
        Self {
            api: ArchiveAuthorityPolicy::production("api.github.com"),
            codeload: ArchiveAuthorityPolicy::production(CODELOAD_HOST),
        }
    }

    #[cfg(test)]
    fn from_test_base(base_url: &str) -> Result<Self, GithubImportError> {
        let authority = ArchiveAuthorityPolicy::from_test_base(base_url)?;
        Ok(Self {
            api: authority.clone(),
            codeload: authority,
        })
    }
}

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
    validate_repo_ref(repo)?;
    let response = send_github_archive_request_with_fallback(
        client,
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

    finish_repository_archive_response(
        client,
        repo,
        response,
        auth_token,
        budget,
        &ArchiveRedirectPolicy::production(),
    )
    .await
}

#[cfg(test)]
pub(super) async fn download_repository_archive_with_test_endpoints(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    auth_token: Option<&str>,
    budget: ResourceBudget,
    endpoints: &[GitHubMirrorEndpoint],
    redirect_base_url: &str,
) -> Result<Vec<u8>, GithubImportError> {
    validate_repo_ref(repo)?;
    let response = send_github_archive_request_with_test_endpoints(
        client,
        endpoints,
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

    finish_repository_archive_response(
        client,
        repo,
        response,
        auth_token,
        budget,
        &ArchiveRedirectPolicy::from_test_base(redirect_base_url)?,
    )
    .await
}

#[cfg(test)]
pub(crate) async fn download_repo_snapshot_with_test_endpoint(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    auth_token: Option<&str>,
    direct_base_url: String,
    redirect_base_url: &str,
) -> Result<GitHubRepoSnapshot, GithubImportError> {
    let direct_base_url: &'static str = Box::leak(direct_base_url.into_boxed_str());
    let endpoints = [GitHubMirrorEndpoint {
        label: "github",
        api_base: direct_base_url,
        raw_base: direct_base_url,
    }];
    let archive = download_repository_archive_with_test_endpoints(
        client,
        repo,
        auth_token,
        ResourceBudget::default_skill(),
        &endpoints,
        redirect_base_url,
    )
    .await?;
    snapshot_from_repository_archive(&archive)
}

async fn finish_repository_archive_response(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    initial: GitHubArchiveInitialResponse,
    auth_token: Option<&str>,
    budget: ResourceBudget,
    redirect_policy: &ArchiveRedirectPolicy,
) -> Result<Vec<u8>, GithubImportError> {
    let response = match initial.response.status() {
        StatusCode::MOVED_PERMANENTLY => {
            if initial.provenance != GitHubEndpointProvenance::TrustedDirect {
                return Err(GithubImportError::ArchiveRedirectRejected);
            }
            let api_url = archive_api_redirect_url_from_response(
                initial.response.headers(),
                repo,
                redirect_policy,
            )?;
            let api_response = send_archive_redirect_request(client, api_url, auth_token).await?;
            if api_response.status() != StatusCode::FOUND {
                return Err(GithubImportError::ArchiveRedirectRejected);
            }
            let codeload_url = archive_codeload_redirect_url_from_response(
                api_response.headers(),
                repo,
                redirect_policy,
                CodeloadIdentity::Canonicalized,
            )?;
            send_codeload_request(client, codeload_url).await?
        }
        StatusCode::FOUND => {
            let codeload_url = archive_codeload_redirect_url_from_response(
                initial.response.headers(),
                repo,
                redirect_policy,
                CodeloadIdentity::SameRepository,
            )?;
            send_codeload_request(client, codeload_url).await?
        }
        _ => initial.response,
    };

    if response.status() == StatusCode::NOT_FOUND {
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
            if status.is_server_error() {
                GithubImportError::ArchiveStatusExhausted
            } else {
                GithubImportError::Http("Failed to download GitHub repository archive.".to_string())
            }
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
        _ => GithubImportError::ArchiveResponseBody,
    })
}

async fn send_archive_redirect_request(
    client: &reqwest::Client,
    url: Url,
    auth_token: Option<&str>,
) -> Result<reqwest::Response, GithubImportError> {
    let mut request = client.get(url);
    if let Some(token) = auth_token {
        request = request.bearer_auth(token);
    }
    request.send().await.map_err(archive_transport_error)
}

async fn send_codeload_request(
    client: &reqwest::Client,
    url: Url,
) -> Result<reqwest::Response, GithubImportError> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(archive_transport_error)?;
    if response.status().is_redirection() {
        return Err(GithubImportError::ArchiveRedirectRejected);
    }
    Ok(response)
}

fn archive_transport_error(error: reqwest::Error) -> GithubImportError {
    GithubImportError::from_archive_transport(&error)
}

fn single_archive_location(
    headers: &reqwest::header::HeaderMap,
) -> Result<&str, GithubImportError> {
    let mut locations = headers.get_all(LOCATION).iter();
    let location = locations
        .next()
        .ok_or(GithubImportError::ArchiveRedirectRejected)?;
    if locations.next().is_some() {
        return Err(GithubImportError::ArchiveRedirectRejected);
    }
    let location = location
        .to_str()
        .map_err(|_| GithubImportError::ArchiveRedirectRejected)?;
    Ok(location)
}

#[derive(Clone, Copy)]
enum CodeloadIdentity {
    SameRepository,
    Canonicalized,
}

fn archive_codeload_redirect_url_from_response(
    headers: &reqwest::header::HeaderMap,
    repo: &GitHubRepoRef,
    policy: &ArchiveRedirectPolicy,
    identity: CodeloadIdentity,
) -> Result<Url, GithubImportError> {
    validate_archive_redirect_url_with_policy(
        single_archive_location(headers)?,
        repo,
        policy,
        identity,
    )
}

fn archive_api_redirect_url_from_response(
    headers: &reqwest::header::HeaderMap,
    repo: &GitHubRepoRef,
    policy: &ArchiveRedirectPolicy,
) -> Result<Url, GithubImportError> {
    validate_archive_api_redirect_url_with_policy(single_archive_location(headers)?, repo, policy)
}

#[cfg(test)]
pub(super) fn validate_archive_redirect_headers(
    headers: &reqwest::header::HeaderMap,
    repo: &GitHubRepoRef,
) -> Result<Url, GithubImportError> {
    archive_codeload_redirect_url_from_response(
        headers,
        repo,
        &ArchiveRedirectPolicy::production(),
        CodeloadIdentity::SameRepository,
    )
}

#[cfg(test)]
pub(super) fn validate_archive_redirect_url(
    location: &str,
    repo: &GitHubRepoRef,
) -> Result<Url, GithubImportError> {
    validate_archive_redirect_url_with_policy(
        location,
        repo,
        &ArchiveRedirectPolicy::production(),
        CodeloadIdentity::SameRepository,
    )
}

#[cfg(test)]
pub(super) fn validate_archive_api_redirect_url(
    location: &str,
    repo: &GitHubRepoRef,
) -> Result<Url, GithubImportError> {
    validate_archive_api_redirect_url_with_policy(
        location,
        repo,
        &ArchiveRedirectPolicy::production(),
    )
}

fn validate_archive_redirect_url_with_policy(
    location: &str,
    repo: &GitHubRepoRef,
    policy: &ArchiveRedirectPolicy,
    identity: CodeloadIdentity,
) -> Result<Url, GithubImportError> {
    if archive_location_has_forbidden_raw_syntax(location) {
        return Err(GithubImportError::ArchiveRedirectRejected);
    }
    let url = Url::parse(location).map_err(|_| GithubImportError::ArchiveRedirectRejected)?;
    let actual_segments = url
        .path_segments()
        .ok_or(GithubImportError::ArchiveRedirectRejected)?
        .collect::<Vec<_>>();
    let path_matches = if is_full_commit_sha(&repo.branch) {
        actual_segments.len() == 4
            && actual_segments[2] == "legacy.tar.gz"
            && actual_segments[3] == repo.branch
    } else {
        actual_segments.len() == 6
            && actual_segments[2] == "legacy.tar.gz"
            && actual_segments[3] == "refs"
            && actual_segments[4] == "heads"
            && actual_segments[5] == repo.branch
    };
    if !policy.codeload.matches(&url) || !path_matches {
        return Err(GithubImportError::ArchiveRedirectRejected);
    }

    match identity {
        CodeloadIdentity::SameRepository => {
            if !actual_segments[0].eq_ignore_ascii_case(&repo.owner)
                || !actual_segments[1].eq_ignore_ascii_case(&repo.repo)
            {
                return Err(GithubImportError::ArchiveRedirectRejected);
            }
        }
        CodeloadIdentity::Canonicalized => {
            validate_repo_owner(actual_segments[0])
                .map_err(|_| GithubImportError::ArchiveRedirectRejected)?;
            validate_repo_name(actual_segments[1])
                .map_err(|_| GithubImportError::ArchiveRedirectRejected)?;
        }
    }

    Ok(url)
}

fn validate_archive_api_redirect_url_with_policy(
    location: &str,
    repo: &GitHubRepoRef,
    policy: &ArchiveRedirectPolicy,
) -> Result<Url, GithubImportError> {
    if archive_location_has_forbidden_raw_syntax(location) {
        return Err(GithubImportError::ArchiveRedirectRejected);
    }
    let url = Url::parse(location).map_err(|_| GithubImportError::ArchiveRedirectRejected)?;
    let segments = url
        .path_segments()
        .ok_or(GithubImportError::ArchiveRedirectRejected)?
        .collect::<Vec<_>>();
    let valid_id = segments.get(1).is_some_and(|id| {
        !id.is_empty()
            && id.bytes().all(|byte| byte.is_ascii_digit())
            && id.parse::<u64>().is_ok_and(|value| value > 0)
    });
    let valid = policy.api.matches(&url)
        && segments.len() == 4
        && segments[0] == "repositories"
        && valid_id
        && segments[2] == "tarball"
        && segments[3] == repo.branch;
    if !valid {
        return Err(GithubImportError::ArchiveRedirectRejected);
    }

    Ok(url)
}

fn is_full_commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn archive_location_has_forbidden_raw_syntax(location: &str) -> bool {
    let has_encoded_separator = location
        .as_bytes()
        .windows(3)
        .any(|window| window.eq_ignore_ascii_case(b"%2f") || window.eq_ignore_ascii_case(b"%5c"));
    let has_userinfo = location
        .split_once("://")
        .map(|(_, remainder)| {
            remainder
                .split(['/', '\\', '?', '#'])
                .next()
                .is_some_and(|authority| authority.contains('@'))
        })
        .unwrap_or(false);

    location.contains('\\')
        || has_encoded_separator
        || has_userinfo
        || location.split('/').any(raw_segment_is_dot)
}

fn raw_segment_is_dot(segment: &str) -> bool {
    let lower = segment.to_ascii_lowercase();
    let mut remainder = lower.as_str();
    let mut dots = 0;
    while !remainder.is_empty() {
        if let Some(rest) = remainder.strip_prefix('.') {
            remainder = rest;
        } else if let Some(rest) = remainder.strip_prefix("%2e") {
            remainder = rest;
        } else {
            return false;
        }
        dots += 1;
    }
    matches!(dots, 1 | 2)
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
