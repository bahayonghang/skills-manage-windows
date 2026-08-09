use crate::services::bounded_ingestion::{
    read_response_bytes_bounded, read_response_text_bounded, BoundedReadError, ReadLimit,
};
use crate::services::resource_budget::ResourceBudget;

use super::*;

pub(crate) async fn fetch_raw_text(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    file_path: &str,
    auth_token: Option<&str>,
) -> Result<String, GithubImportError> {
    let bytes = fetch_raw_bytes(client, repo, file_path, auth_token).await?;
    String::from_utf8(bytes)
        .map_err(|e| GithubImportError::Parse(format!("Skill metadata is not valid UTF-8: {}", e)))
}

/// Download a single raw blob through the fixed GitHub/mirror endpoint set.
/// The renderer and callers provide repository identity plus a repository-
/// relative path; they never provide the request authority.
pub(super) async fn fetch_raw_bytes(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    file_path: &str,
    auth_token: Option<&str>,
) -> Result<Vec<u8>, GithubImportError> {
    fetch_raw_bytes_with_budget(
        client,
        repo,
        file_path,
        auth_token,
        RawBytesBudget::Metadata,
    )
    .await
}

pub(super) async fn fetch_raw_repo_file(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    file_path: &str,
    auth_token: Option<&str>,
) -> Result<Vec<u8>, GithubImportError> {
    fetch_raw_bytes_with_budget(
        client,
        repo,
        file_path,
        auth_token,
        RawBytesBudget::RepositoryFile,
    )
    .await
}

#[derive(Debug, Clone, Copy)]
pub(super) enum RawBytesBudget {
    Metadata,
    RepositoryFile,
}

async fn fetch_raw_bytes_with_budget(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    file_path: &str,
    auth_token: Option<&str>,
    budget_kind: RawBytesBudget,
) -> Result<Vec<u8>, GithubImportError> {
    validate_repo_ref(repo)?;
    let file_path = normalize_repo_path(file_path)?;
    if file_path.is_empty() {
        return Err(GithubImportError::UnsupportedRepoPath(
            file_path.to_string(),
        ));
    }

    let (failure_prefix, operation) = match budget_kind {
        RawBytesBudget::Metadata => (
            "Failed to download skill metadata",
            "downloading skill metadata",
        ),
        RawBytesBudget::RepositoryFile => (
            "Failed to download repository file",
            "downloading repository file",
        ),
    };
    let response = send_github_request_with_fallback(
        client,
        GitHubFetchSurface::Raw,
        |endpoint| raw_file_url(endpoint, repo, &file_path),
        failure_prefix,
        auth_token,
    )
    .await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(GithubImportError::RepoFileGone(file_path));
    }
    if !response.status().is_success() {
        return Err(classify_github_denial_response(response, operation)
            .await
            .unwrap_or_else(|| GithubImportError::Http(format!("{}.", failure_prefix))));
    }

    read_raw_response_with_budget(
        response,
        ResourceBudget::default_skill(),
        budget_kind,
        &file_path,
    )
    .await
}

pub(super) async fn read_raw_response_with_budget(
    response: reqwest::Response,
    budget: ResourceBudget,
    budget_kind: RawBytesBudget,
    path: &str,
) -> Result<Vec<u8>, GithubImportError> {
    let limit = match budget_kind {
        RawBytesBudget::Metadata => ReadLimit::new("GitHub skill metadata", budget.file_bytes),
        RawBytesBudget::RepositoryFile => {
            ReadLimit::new("GitHub repository file", budget.archive_entry_bytes)
        }
    };
    read_response_bytes_bounded(response, limit)
        .await
        .map_err(|error| map_raw_read_error(error, budget, budget_kind, path))
}

fn map_raw_read_error(
    error: BoundedReadError,
    budget: ResourceBudget,
    budget_kind: RawBytesBudget,
    path: &str,
) -> GithubImportError {
    if let Some((actual, _)) = error.actual_and_limit() {
        let budget_error = match budget_kind {
            RawBytesBudget::Metadata => budget.reject_file_read_size(path, actual),
            RawBytesBudget::RepositoryFile => budget.reject_archive_entry_size(path, actual),
        }
        .expect_err("bounded reader reported an over-limit size");
        return GithubImportError::Budget(budget_error);
    }
    GithubImportError::Http("Failed to read skill metadata.".to_string())
}

pub(super) fn validate_repo_ref(repo: &GitHubRepoRef) -> Result<(), GithubImportError> {
    validate_repo_owner(&repo.owner)?;
    validate_repo_name(&repo.repo)?;
    validate_repo_branch(&repo.branch)
}

fn invalid_repo_component(field: &'static str, value: &str) -> GithubImportError {
    GithubImportError::InvalidRepoComponent {
        field,
        value: value.to_string(),
    }
}

pub(super) fn validate_repo_owner(value: &str) -> Result<(), GithubImportError> {
    if value.is_empty()
        || value.trim() != value
        || value.starts_with('-')
        || value.ends_with('-')
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    {
        return Err(invalid_repo_component("owner", value));
    }
    Ok(())
}

pub(super) fn validate_repo_name(value: &str) -> Result<(), GithubImportError> {
    if value.is_empty()
        || value.trim() != value
        || matches!(value, "." | "..")
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(invalid_repo_component("name", value));
    }
    Ok(())
}

pub(super) fn validate_repo_branch(value: &str) -> Result<(), GithubImportError> {
    if value.is_empty()
        || value.trim() != value
        || matches!(value, "." | "..")
        || value
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    {
        return Err(invalid_repo_component("branch", value));
    }
    Ok(())
}

pub(super) fn github_endpoint_url(
    endpoint: &GitHubMirrorEndpoint,
    surface: GitHubFetchSurface,
    path: &str,
) -> String {
    let base = endpoint_base(endpoint, surface);
    format!("{}{}", base.trim_end_matches('/'), path)
}

pub(super) fn raw_file_url(
    endpoint: &GitHubMirrorEndpoint,
    repo: &GitHubRepoRef,
    file_path: &str,
) -> String {
    let mut url = reqwest::Url::parse(endpoint.raw_base)
        .expect("built-in GitHub raw endpoint must be a valid base URL");
    {
        let mut segments = url
            .path_segments_mut()
            .expect("built-in GitHub raw endpoint must be hierarchical");
        segments.pop_if_empty();
        for value in [&repo.owner, &repo.repo, &repo.branch] {
            segments.push(value);
        }
        for segment in file_path.trim_matches('/').split('/') {
            if !segment.is_empty() {
                segments.push(segment);
            }
        }
    }
    url.to_string()
}

fn endpoint_base(endpoint: &GitHubMirrorEndpoint, surface: GitHubFetchSurface) -> &'static str {
    match surface {
        GitHubFetchSurface::Api => endpoint.api_base,
        GitHubFetchSurface::Raw => endpoint.raw_base,
    }
}

#[cfg(test)]
pub(super) fn validate_github_endpoint_request(
    endpoint: &GitHubMirrorEndpoint,
    surface: GitHubFetchSurface,
    request_url: &str,
) -> Result<(), GithubImportError> {
    validate_endpoint_request(endpoint, surface, request_url, true)
}

fn validate_endpoint_request(
    endpoint: &GitHubMirrorEndpoint,
    surface: GitHubFetchSurface,
    request_url: &str,
    require_https: bool,
) -> Result<(), GithubImportError> {
    let base_url = reqwest::Url::parse(endpoint_base(endpoint, surface)).map_err(|e| {
        GithubImportError::InvalidUrl(format!(
            "Invalid built-in GitHub endpoint '{}': {}",
            endpoint.label, e
        ))
    })?;
    let request_url = reqwest::Url::parse(request_url).map_err(|e| {
        GithubImportError::InvalidUrl(format!("Invalid GitHub URL '{}': {}", request_url, e))
    })?;

    let base_path = base_url.path().trim_end_matches('/');
    let request_path = request_url.path();
    let within_base_path = base_path.is_empty()
        || request_path == base_path
        || request_path
            .strip_prefix(base_path)
            .is_some_and(|suffix| suffix.starts_with('/'));
    let valid_scheme = if require_https {
        request_url.scheme() == "https" && base_url.scheme() == "https"
    } else {
        request_url.scheme() == base_url.scheme()
    };
    let valid_port = if require_https {
        request_url.port_or_known_default() == Some(443)
            && base_url.port_or_known_default() == Some(443)
    } else {
        request_url.port_or_known_default() == base_url.port_or_known_default()
    };
    let valid = valid_scheme
        && request_url.username().is_empty()
        && request_url.password().is_none()
        && request_url.fragment().is_none()
        && request_url.host_str() == base_url.host_str()
        && valid_port
        && within_base_path;

    if !valid {
        return Err(GithubImportError::InvalidUrl(format!(
            "GitHub request URL is outside the built-in '{}' endpoint policy.",
            endpoint.label
        )));
    }
    Ok(())
}

pub(super) async fn send_github_request_with_fallback<F>(
    client: &reqwest::Client,
    surface: GitHubFetchSurface,
    build_url: F,
    failure_prefix: &str,
    auth_token: Option<&str>,
) -> Result<reqwest::Response, GithubImportError>
where
    F: Fn(&GitHubMirrorEndpoint) -> String,
{
    send_github_request_with_endpoints(
        client,
        RequestPolicy::standard(surface),
        GITHUB_MIRROR_ENDPOINTS,
        true,
        build_url,
        failure_prefix,
        auth_token,
    )
    .await
    .map(|response| response.response)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GitHubEndpointProvenance {
    TrustedDirect,
    Mirror,
}

pub(super) struct GitHubArchiveInitialResponse {
    pub(super) response: reqwest::Response,
    pub(super) provenance: GitHubEndpointProvenance,
}

pub(super) async fn send_github_archive_request_with_fallback<F>(
    client: &reqwest::Client,
    build_url: F,
    failure_prefix: &str,
    auth_token: Option<&str>,
) -> Result<GitHubArchiveInitialResponse, GithubImportError>
where
    F: Fn(&GitHubMirrorEndpoint) -> String,
{
    send_github_request_with_endpoints(
        client,
        RequestPolicy::archive(),
        GITHUB_MIRROR_ENDPOINTS,
        true,
        build_url,
        failure_prefix,
        auth_token,
    )
    .await
}

#[cfg(test)]
pub(super) async fn send_github_request_with_test_endpoints<F>(
    client: &reqwest::Client,
    surface: GitHubFetchSurface,
    endpoints: &[GitHubMirrorEndpoint],
    build_url: F,
    failure_prefix: &str,
    auth_token: Option<&str>,
) -> Result<reqwest::Response, GithubImportError>
where
    F: Fn(&GitHubMirrorEndpoint) -> String,
{
    send_github_request_with_endpoints(
        client,
        RequestPolicy::standard(surface),
        endpoints,
        false,
        build_url,
        failure_prefix,
        auth_token,
    )
    .await
    .map(|response| response.response)
}

#[cfg(test)]
pub(super) async fn send_github_archive_request_with_test_endpoints<F>(
    client: &reqwest::Client,
    endpoints: &[GitHubMirrorEndpoint],
    build_url: F,
    failure_prefix: &str,
    auth_token: Option<&str>,
) -> Result<GitHubArchiveInitialResponse, GithubImportError>
where
    F: Fn(&GitHubMirrorEndpoint) -> String,
{
    send_github_request_with_endpoints(
        client,
        RequestPolicy::archive(),
        endpoints,
        false,
        build_url,
        failure_prefix,
        auth_token,
    )
    .await
}

#[derive(Clone, Copy)]
enum ResponseAcceptance {
    SuccessOnly,
    ArchiveInitialRedirect,
}

impl ResponseAcceptance {
    fn accepts(self, status: reqwest::StatusCode) -> bool {
        status.is_success()
            || matches!(self, Self::ArchiveInitialRedirect)
                && matches!(
                    status,
                    reqwest::StatusCode::MOVED_PERMANENTLY | reqwest::StatusCode::FOUND
                )
    }
}

#[derive(Clone, Copy)]
struct RequestPolicy {
    surface: GitHubFetchSurface,
    acceptance: ResponseAcceptance,
}

impl RequestPolicy {
    fn standard(surface: GitHubFetchSurface) -> Self {
        Self {
            surface,
            acceptance: ResponseAcceptance::SuccessOnly,
        }
    }

    fn archive() -> Self {
        Self {
            surface: GitHubFetchSurface::Api,
            acceptance: ResponseAcceptance::ArchiveInitialRedirect,
        }
    }
}

async fn send_github_request_with_endpoints<F>(
    client: &reqwest::Client,
    policy: RequestPolicy,
    endpoints: &[GitHubMirrorEndpoint],
    require_https: bool,
    build_url: F,
    failure_prefix: &str,
    auth_token: Option<&str>,
) -> Result<GitHubArchiveInitialResponse, GithubImportError>
where
    F: Fn(&GitHubMirrorEndpoint) -> String,
{
    let surface = policy.surface;
    let mut attempts = Vec::new();
    let mut last_retryable_denial = None;
    let mut last_archive_error = None;

    let endpoint_urls = endpoints
        .iter()
        .map(|endpoint| {
            let url = build_url(endpoint);
            validate_endpoint_request(endpoint, surface, &url, require_https)?;
            Ok((*endpoint, url))
        })
        .collect::<Result<Vec<_>, GithubImportError>>()?;

    for (endpoint, url) in &endpoint_urls {
        wait_for_github_host_slot(url).await?;
        let mut request = client.get(url);
        let mirrors_share_same_url = endpoint_urls
            .iter()
            .filter(|(candidate, _)| candidate.label != "github")
            .any(|(_, candidate_url)| candidate_url == url);
        let provenance = if endpoint.label == "github" && !mirrors_share_same_url {
            GitHubEndpointProvenance::TrustedDirect
        } else {
            GitHubEndpointProvenance::Mirror
        };
        if provenance == GitHubEndpointProvenance::TrustedDirect {
            if let Some(token) = auth_token {
                request = request.bearer_auth(token);
            }
        }
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                if matches!(
                    status,
                    reqwest::StatusCode::UNAUTHORIZED
                        | reqwest::StatusCode::FORBIDDEN
                        | reqwest::StatusCode::TOO_MANY_REQUESTS
                ) {
                    let denial = parse_github_denial_response(
                        response,
                        "contacting GitHub",
                        auth_token.is_some(),
                    )
                    .await;
                    let can_retry_public_mirror = auth_token.is_none()
                        && denial.as_ref().is_some_and(|denial| {
                            matches!(denial.kind, GitHubAccessDenialKind::RateLimited { .. })
                        });
                    if can_retry_public_mirror {
                        last_retryable_denial = denial;
                        attempts.push(MirrorAttemptOutcome {
                            status: Some(status),
                            error_message: format!(
                                "{} mirror '{}' returned HTTP {} due to rate limiting",
                                surface_label(surface),
                                endpoint.label,
                                status
                            ),
                        });
                        continue;
                    }

                    return Err(denial
                        .map(GithubImportError::from_denial)
                        .unwrap_or_else(|| {
                            GithubImportError::Http(format!("{}: HTTP {}", failure_prefix, status))
                        }));
                }

                if policy.acceptance.accepts(status) {
                    return Ok(GitHubArchiveInitialResponse {
                        response,
                        provenance,
                    });
                }

                if status == reqwest::StatusCode::NOT_FOUND {
                    if last_retryable_denial.is_some() && auth_token.is_none() {
                        attempts.push(MirrorAttemptOutcome {
                            status: Some(status),
                            error_message: format!(
                                "{} mirror '{}' returned HTTP 404 after a prior rate-limit denial",
                                surface_label(surface),
                                endpoint.label
                            ),
                        });
                        continue;
                    }
                    return Ok(GitHubArchiveInitialResponse {
                        response,
                        provenance,
                    });
                }

                if should_retry_via_mirror_status(surface, status) {
                    if matches!(
                        policy.acceptance,
                        ResponseAcceptance::ArchiveInitialRedirect
                    ) {
                        last_archive_error = Some(GithubImportError::ArchiveStatusExhausted);
                    }
                    attempts.push(MirrorAttemptOutcome {
                        status: Some(status),
                        error_message: format!(
                            "{} mirror '{}' returned HTTP {}",
                            surface_label(surface),
                            endpoint.label,
                            status
                        ),
                    });
                    continue;
                }

                return Err(GithubImportError::Http(format!(
                    "{}: HTTP {}",
                    failure_prefix, status
                )));
            }
            Err(error) => {
                let archive_error = matches!(
                    policy.acceptance,
                    ResponseAcceptance::ArchiveInitialRedirect
                )
                .then(|| GithubImportError::from_archive_transport(&error));
                if is_retryable_github_transport_error(&error) {
                    if let Some(error) = archive_error {
                        last_archive_error = Some(error);
                    }
                    attempts.push(MirrorAttemptOutcome {
                        status: error.status(),
                        error_message: format!(
                            "{} mirror '{}' failed: {}",
                            surface_label(surface),
                            endpoint.label,
                            sanitized_github_transport_error(&error)
                        ),
                    });
                    continue;
                }

                if let Some(error) = archive_error {
                    return Err(error);
                }
                return Err(GithubImportError::Http(format!(
                    "{}: {}",
                    failure_prefix,
                    sanitized_github_transport_error(&error)
                )));
            }
        }
    }

    if let Some(denial) = last_retryable_denial {
        return Err(GithubImportError::from_denial(denial));
    }

    if let Some(error) = last_archive_error {
        return Err(error);
    }

    Err(GithubImportError::Http(format!(
        "{}. Direct GitHub access and built-in mirrors were unreachable. Retry later or try a different network path. Last errors: {}",
        failure_prefix,
        summarize_mirror_attempts(&attempts)
    )))
}

pub(super) fn should_retry_via_mirror_status(
    surface: GitHubFetchSurface,
    status: reqwest::StatusCode,
) -> bool {
    match surface {
        GitHubFetchSurface::Api | GitHubFetchSurface::Raw => {
            status.is_server_error()
                || status == reqwest::StatusCode::BAD_GATEWAY
                || status == reqwest::StatusCode::SERVICE_UNAVAILABLE
                || status == reqwest::StatusCode::GATEWAY_TIMEOUT
        }
    }
}

pub(super) fn is_retryable_github_transport_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request() || error.is_body()
}

pub(super) fn sanitized_github_transport_error(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        "request timed out"
    } else if error.is_connect() {
        "connection failed"
    } else if error.is_redirect() {
        "redirect failed"
    } else if error.is_body() {
        "response body failed"
    } else if error.is_decode() {
        "response decoding failed"
    } else if error.is_builder() {
        "request configuration failed"
    } else if error.is_request() {
        "request failed"
    } else {
        "network request failed"
    }
}

pub(super) fn summarize_mirror_attempts(attempts: &[MirrorAttemptOutcome]) -> String {
    attempts
        .iter()
        .map(|attempt| attempt.error_message.clone())
        .collect::<Vec<_>>()
        .join("; ")
}

pub(super) fn surface_label(surface: GitHubFetchSurface) -> &'static str {
    match surface {
        GitHubFetchSurface::Api => "API",
        GitHubFetchSurface::Raw => "raw",
    }
}

pub(super) async fn classify_github_denial_response(
    response: reqwest::Response,
    operation: &'static str,
) -> Option<GithubImportError> {
    parse_github_denial_response(response, operation, false)
        .await
        .map(GithubImportError::from_denial)
}

pub(super) async fn parse_github_denial_response(
    response: reqwest::Response,
    operation: &'static str,
    used_auth: bool,
) -> Option<GitHubAccessDenial> {
    let status = response.status();
    if status != reqwest::StatusCode::UNAUTHORIZED
        && status != reqwest::StatusCode::FORBIDDEN
        && status != reqwest::StatusCode::TOO_MANY_REQUESTS
    {
        return None;
    }

    let headers = response.headers().clone();
    let body =
        read_response_text_bounded(response, ReadLimit::new("GitHub error response", 64 * 1024))
            .await
            .ok();
    let github_message = body.as_deref().and_then(parse_github_error_message);

    let remaining = header_value(&headers, "x-ratelimit-remaining");
    let reset_at = header_value(&headers, "x-ratelimit-reset")
        .as_deref()
        .and_then(parse_rate_limit_reset_epoch);

    let message_lower = github_message
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let remaining_is_zero = remaining.as_deref() == Some("0");
    let kind = if status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || remaining_is_zero
        || message_lower.contains("rate limit")
        || message_lower.contains("api rate limit exceeded")
        || header_value(&headers, "x-ratelimit-resource").is_some()
    {
        GitHubAccessDenialKind::RateLimited {
            reset_at,
            remaining,
        }
    } else {
        GitHubAccessDenialKind::AuthenticationOrPermission
    };

    Some(GitHubAccessDenial {
        kind,
        operation,
        status,
        used_auth,
    })
}

pub(super) fn parse_github_error_message(body: &str) -> Option<String> {
    serde_json::from_str::<GitHubErrorResponse>(body)
        .ok()
        .and_then(|payload| payload.message)
}

pub(super) fn header_value(headers: &reqwest::header::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn parse_rate_limit_reset_epoch(raw: &str) -> Option<String> {
    let epoch = raw.parse::<i64>().ok()?;
    chrono::DateTime::<Utc>::from_timestamp(epoch, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
}
