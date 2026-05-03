pub(crate) async fn github_direct_auth_from_settings(
    pool: &DbPool,
) -> Result<Option<String>, String> {
    Ok(db::get_setting(pool, GITHUB_PAT_SETTING_KEY)
        .await?
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty()))
}

pub(crate) fn github_client() -> Result<reqwest::Client, String> {
    GITHUB_SHARED_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .user_agent(crate::commands::APP_USER_AGENT)
                .build()
                .map_err(|e| e.to_string())
        })
        .clone()
}
pub(crate) async fn set_github_pat_impl(pool: &DbPool, value: String) -> Result<(), String> {
    db::set_setting(pool, GITHUB_PAT_SETTING_KEY, value.trim()).await
}

pub(crate) async fn clear_github_pat_impl(pool: &DbPool) -> Result<(), String> {
    db::set_setting(pool, GITHUB_PAT_SETTING_KEY, "").await
}

pub(crate) async fn test_github_pat_impl(pool: &DbPool) -> Result<GitHubPatTestResult, String> {
    let Some(token) = github_direct_auth_from_settings(pool).await? else {
        return Ok(GitHubPatTestResult {
            configured: false,
            ok: false,
            status: None,
            message: "No GitHub token is configured.".to_string(),
        });
    };

    let client = github_client()?;
    let response = client
        .get("https://api.github.com/rate_limit")
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("Failed to test GitHub token: {}", e))?;
    let status = response.status();
    if status.is_success() {
        return Ok(GitHubPatTestResult {
            configured: true,
            ok: true,
            status: Some(status.as_u16()),
            message: "GitHub token is usable for authenticated GitHub requests.".to_string(),
        });
    }

    let denial = parse_github_denial_response(response, "testing GitHub token", true)
        .await
        .map(|denial| denial.to_string())
        .unwrap_or_else(|| format!("GitHub token test returned HTTP {}", status.as_u16()));

    Ok(GitHubPatTestResult {
        configured: true,
        ok: false,
        status: Some(status.as_u16()),
        message: denial,
    })
}
