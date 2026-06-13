use crate::services::resource_budget::ResourceBudget;

use super::*;
pub(crate) async fn fetch_raw_text(
    client: &reqwest::Client,
    url: &str,
    auth_token: Option<&str>,
) -> Result<String, GithubImportError> {
    let response = send_github_request_with_fallback(
        client,
        GitHubFetchSurface::Raw,
        |endpoint| {
            if let Some(path) = raw_url_to_repo_path(url) {
                raw_file_url(endpoint, &path.repo, &path.file_path)
            } else {
                url.to_string()
            }
        },
        "Failed to download skill metadata",
        auth_token,
    )
    .await?;

    if !response.status().is_success() {
        return Err(
            classify_github_denial_response(response, "downloading skill metadata")
                .await
                .unwrap_or_else(|| {
                    GithubImportError::Http("Failed to download skill metadata.".to_string())
                }),
        );
    }

    let budget = ResourceBudget::default_skill();
    if let Some(content_length) = response.content_length() {
        budget
            .reject_file_read_size(url, content_length)
            .map_err(GithubImportError::Budget)?;
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| GithubImportError::Http(format!("Failed to read skill metadata: {}", e)))?;
    budget
        .reject_file_read_size(url, bytes.len() as u64)
        .map_err(GithubImportError::Budget)?;
    String::from_utf8(bytes.to_vec())
        .map_err(|e| GithubImportError::Parse(format!("Skill metadata is not valid UTF-8: {}", e)))
}

#[derive(Debug, Clone)]
pub(super) struct RawRepoPath {
    pub(super) repo: GitHubRepoRef,
    pub(super) file_path: String,
}

pub(super) fn raw_url_to_repo_path(url: &str) -> Option<RawRepoPath> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    if host != "raw.githubusercontent.com" {
        return None;
    }

    let segments = parsed.path_segments()?;
    let parts = segments.collect::<Vec<_>>();
    if parts.len() < 4 {
        return None;
    }

    Some(RawRepoPath {
        repo: GitHubRepoRef {
            owner: parts[0].to_string(),
            repo: parts[1].to_string(),
            branch: parts[2].to_string(),
            normalized_url: format!("https://github.com/{}/{}", parts[0], parts[1]),
        },
        file_path: parts[3..].join("/"),
    })
}

pub(super) fn github_endpoint_url(
    endpoint: &GitHubMirrorEndpoint,
    surface: GitHubFetchSurface,
    path: &str,
) -> String {
    let base = match surface {
        GitHubFetchSurface::Api => endpoint.api_base,
        GitHubFetchSurface::Raw => endpoint.raw_base,
    };
    format!("{}{}", base.trim_end_matches('/'), path)
}

pub(super) fn raw_file_url(
    endpoint: &GitHubMirrorEndpoint,
    repo: &GitHubRepoRef,
    file_path: &str,
) -> String {
    github_endpoint_url(
        endpoint,
        GitHubFetchSurface::Raw,
        &format!(
            "/{}/{}/{}/{}",
            repo.owner,
            repo.repo,
            repo.branch,
            file_path.trim_start_matches('/')
        ),
    )
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
    let mut attempts = Vec::new();
    let mut last_retryable_denial = None;

    for endpoint in GITHUB_MIRROR_ENDPOINTS {
        let url = build_url(endpoint);
        wait_for_github_host_slot(&url).await?;
        let mut request = client.get(&url);
        let mirrors_share_same_url = GITHUB_MIRROR_ENDPOINTS
            .iter()
            .filter(|candidate| candidate.label != "github")
            .any(|candidate| build_url(candidate) == url);
        if endpoint.label == "github" && !mirrors_share_same_url {
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

                if status.is_success() {
                    return Ok(response);
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
                    return Ok(response);
                }

                if should_retry_via_mirror_status(surface, status) {
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
                if is_retryable_github_transport_error(&error) {
                    attempts.push(MirrorAttemptOutcome {
                        status: error.status(),
                        error_message: format!(
                            "{} mirror '{}' failed: {}",
                            surface_label(surface),
                            endpoint.label,
                            error
                        ),
                    });
                    continue;
                }

                return Err(GithubImportError::Http(format!(
                    "{}: {}",
                    failure_prefix, error
                )));
            }
        }
    }

    if let Some(denial) = last_retryable_denial {
        return Err(GithubImportError::from_denial(denial));
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
    let body = response.text().await.ok();
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
        github_message,
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
