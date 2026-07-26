use super::*;
use crate::secrets::{SecretStorageState, SecretStore, GITHUB_PAT_SECRET_KEY};
use serde_json::json;
use std::time::Duration;

pub(super) const LEGACY_GITHUB_PAT_SETTING_KEY: &str = "github_pat";
pub(super) const GITHUB_PAT_MIGRATION_SETTING_KEY: &str = "github_pat_keyring_migration_v1";
pub(super) const GITHUB_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const GITHUB_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn normalize_github_pat(value: impl AsRef<str>) -> Option<String> {
    let token = value.as_ref().trim().to_string();
    (!token.is_empty()).then_some(token)
}

pub(super) fn map_secret_error(action: &str, error: crate::secrets::SecretError) -> String {
    format!("Failed to {} GitHub token: {}", action, error)
}

pub(super) async fn record_github_pat_migration_failure(pool: &DbPool, error: &str, reason: &str) {
    crate::operation_log::record_operation_log_best_effort(
        pool,
        crate::operation_log::local_target_context(),
        crate::operation_log::OperationLogEvent::new(
            "settings",
            "settings.github_pat_migration",
            "failed",
            "GitHub token migration to secure storage failed",
        )
        .subject("setting", LEGACY_GITHUB_PAT_SETTING_KEY, "GitHub token")
        .error(error)
        .details(json!({
            "key": LEGACY_GITHUB_PAT_SETTING_KEY,
            "reason": reason,
            "legacySettingRetained": true,
        })),
    )
    .await;
}

pub(crate) fn github_client() -> Result<reqwest::Client, GithubImportError> {
    match GITHUB_SHARED_CLIENT.get_or_init(|| {
        let builder = reqwest::Client::builder()
            .user_agent(crate::commands::APP_USER_AGENT)
            .connect_timeout(GITHUB_CONNECT_TIMEOUT)
            .timeout(GITHUB_REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none());
        #[cfg(test)]
        let builder = builder.no_proxy();
        builder.build()
    }) {
        Ok(client) => Ok(client.clone()),
        Err(error) => Err(GithubImportError::Http(error.to_string())),
    }
}

pub(super) async fn legacy_github_pat_from_settings(
    pool: &DbPool,
) -> Result<Option<String>, GithubImportError> {
    Ok(db::get_setting(pool, LEGACY_GITHUB_PAT_SETTING_KEY)
        .await?
        .and_then(normalize_github_pat))
}

pub(super) async fn mark_github_pat_migration_complete(
    pool: &DbPool,
) -> Result<(), GithubImportError> {
    Ok(db::set_setting(pool, GITHUB_PAT_MIGRATION_SETTING_KEY, "1").await?)
}

pub(super) async fn is_github_pat_migration_marked(
    pool: &DbPool,
) -> Result<bool, GithubImportError> {
    Ok(db::get_setting(pool, GITHUB_PAT_MIGRATION_SETTING_KEY)
        .await?
        .as_deref()
        == Some("1"))
}

pub(super) async fn delete_legacy_github_pat_setting(
    pool: &DbPool,
) -> Result<(), GithubImportError> {
    Ok(db::delete_setting(pool, LEGACY_GITHUB_PAT_SETTING_KEY).await?)
}

pub(super) fn log_github_pat_migration_warning(message: impl AsRef<str>) {
    tracing::warn!(message = %message.as_ref(), "GitHub PAT migration warning");
}

pub(super) async fn migrate_github_pat_to_secret_store(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<Option<String>, GithubImportError> {
    if is_github_pat_migration_marked(pool).await? {
        return Ok(None);
    }

    let Some(token) = legacy_github_pat_from_settings(pool).await? else {
        return Ok(None);
    };

    let storage_state = match secrets.set(GITHUB_PAT_SECRET_KEY, &token) {
        Ok(storage_state) => storage_state,
        Err(error) => {
            let mapped_error = map_secret_error("migrate", error);
            log_github_pat_migration_warning(format!(
                "GitHub PAT migration failed; keeping legacy settings value: {}",
                mapped_error
            ));
            record_github_pat_migration_failure(pool, &mapped_error, "secret_store_set").await;
            return Ok(Some(mapped_error));
        }
    };
    if !storage_state.is_available() {
        let mapped_error = format!(
            "Failed to migrate GitHub token: unavailable storage state {:?}",
            storage_state
        );
        log_github_pat_migration_warning(format!(
            "GitHub PAT migration did not produce an available secret state: {:?}",
            storage_state
        ));
        record_github_pat_migration_failure(pool, &mapped_error, "unavailable_storage_state").await;
        return Ok(Some(mapped_error));
    }

    match secrets.get(GITHUB_PAT_SECRET_KEY) {
        Ok(Some(saved)) if normalize_github_pat(&saved).as_deref() == Some(token.as_str()) => {
            delete_legacy_github_pat_setting(pool).await?;
            mark_github_pat_migration_complete(pool).await?;
            Ok(None)
        }
        Ok(_) => {
            let mapped_error =
                "Failed to verify migrated GitHub token; keeping legacy settings value.";
            log_github_pat_migration_warning(
                "GitHub PAT migration readback did not match; keeping legacy settings value.",
            );
            record_github_pat_migration_failure(pool, mapped_error, "readback_mismatch").await;
            Ok(Some(mapped_error.to_string()))
        }
        Err(error) => {
            let mapped_error = map_secret_error("verify migrated", error);
            log_github_pat_migration_warning(format!(
                "GitHub PAT migration readback failed; keeping legacy settings value: {}",
                mapped_error
            ));
            record_github_pat_migration_failure(pool, &mapped_error, "readback_error").await;
            Ok(Some(mapped_error))
        }
    }
}

pub async fn migrate_github_pat_on_startup(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<(), GithubImportError> {
    let _ = migrate_github_pat_to_secret_store(pool, secrets).await?;
    Ok(())
}

pub(crate) async fn github_direct_auth_from_secret_store(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<Option<String>, GithubImportError> {
    let mut secret_error = None;
    match secrets.get(GITHUB_PAT_SECRET_KEY) {
        Ok(secret) => {
            if let Some(token) = secret.and_then(normalize_github_pat) {
                return Ok(Some(token));
            }
        }
        Err(error) => {
            secret_error = Some(map_secret_error("read", error));
        }
    }

    let migration_error = migrate_github_pat_to_secret_store(pool, secrets).await?;
    match secrets.get(GITHUB_PAT_SECRET_KEY) {
        Ok(secret) => {
            if let Some(token) = secret.and_then(normalize_github_pat) {
                return Ok(Some(token));
            }
        }
        Err(error) => {
            secret_error.get_or_insert_with(|| map_secret_error("read", error));
        }
    }

    let legacy_token = legacy_github_pat_from_settings(pool).await?;
    if legacy_token.is_some() {
        return Ok(legacy_token);
    }

    if let Some(error) = migration_error.or(secret_error) {
        return Err(GithubImportError::Secret(error));
    }

    Ok(None)
}

pub(crate) async fn get_github_pat_state_impl(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<GitHubPatState, GithubImportError> {
    let migration_error = migrate_github_pat_to_secret_store(pool, secrets).await?;

    match secrets.state(GITHUB_PAT_SECRET_KEY) {
        Ok(storage_state) if storage_state.is_available() => Ok(GitHubPatState {
            configured: true,
            storage_state,
            error: migration_error,
        }),
        Ok(SecretStorageState::Unreadable) => {
            let legacy_token = legacy_github_pat_from_settings(pool).await?;
            Ok(GitHubPatState {
                configured: legacy_token.is_some(),
                storage_state: SecretStorageState::Unreadable,
                error: migration_error,
            })
        }
        Ok(_) => {
            let legacy_token = legacy_github_pat_from_settings(pool).await?;
            Ok(GitHubPatState {
                configured: legacy_token.is_some(),
                storage_state: SecretStorageState::Missing,
                error: migration_error,
            })
        }
        Err(error) => {
            let legacy_token = legacy_github_pat_from_settings(pool).await?;
            Ok(GitHubPatState {
                configured: legacy_token.is_some(),
                storage_state: SecretStorageState::Unreadable,
                error: Some(migration_error.unwrap_or_else(|| map_secret_error("inspect", error))),
            })
        }
    }
}

pub(crate) async fn reveal_github_pat_impl(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<Option<String>, GithubImportError> {
    github_direct_auth_from_secret_store(pool, secrets).await
}

pub(crate) async fn set_github_pat_impl(
    pool: &DbPool,
    secrets: &dyn SecretStore,
    value: String,
) -> Result<GitHubPatState, GithubImportError> {
    let token = normalize_github_pat(&value).ok_or(GithubImportError::PatTokenEmpty)?;
    let storage_state = secrets
        .set(GITHUB_PAT_SECRET_KEY, &token)
        .map_err(|error| GithubImportError::Secret(map_secret_error("save", error)))?;
    if !storage_state.is_available() {
        return Err(GithubImportError::Secret(format!(
            "Failed to save GitHub token: unavailable storage state {:?}",
            storage_state
        )));
    }

    let saved = secrets
        .get(GITHUB_PAT_SECRET_KEY)
        .map_err(|error| GithubImportError::Secret(map_secret_error("verify", error)))?
        .and_then(normalize_github_pat);
    if saved.as_deref() != Some(token.as_str()) {
        return Err(GithubImportError::Secret(
            "Failed to verify saved GitHub token.".to_string(),
        ));
    }

    delete_legacy_github_pat_setting(pool).await?;
    mark_github_pat_migration_complete(pool).await?;
    Ok(GitHubPatState {
        configured: true,
        storage_state,
        error: None,
    })
}

pub(crate) async fn clear_github_pat_impl(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<GitHubPatState, GithubImportError> {
    secrets
        .delete(GITHUB_PAT_SECRET_KEY)
        .map_err(|error| GithubImportError::Secret(map_secret_error("clear", error)))?;
    delete_legacy_github_pat_setting(pool).await?;
    mark_github_pat_migration_complete(pool).await?;
    Ok(GitHubPatState {
        configured: false,
        storage_state: SecretStorageState::Missing,
        error: None,
    })
}

pub(crate) async fn test_github_pat_impl(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<GitHubPatTestResult, GithubImportError> {
    let Some(token) = github_direct_auth_from_secret_store(pool, secrets).await? else {
        return Ok(GitHubPatTestResult {
            configured: false,
            ok: false,
            status: None,
            message_key: "settings.githubPatTestNoToken".to_string(),
            message: "No GitHub token is configured.".to_string(),
        });
    };

    let client = github_client()?;
    let response = client
        .get("https://api.github.com/rate_limit")
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| GithubImportError::Http(format!("Failed to test GitHub token: {}", e)))?;
    let status = response.status();
    if status.is_success() {
        return Ok(GitHubPatTestResult {
            configured: true,
            ok: true,
            status: Some(status.as_u16()),
            message_key: "settings.githubPatTestSuccess".to_string(),
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
        message_key: "settings.githubPatTestFailure".to_string(),
        message: denial,
    })
}
