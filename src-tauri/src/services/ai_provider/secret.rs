use crate::db::{self, DbPool};
use crate::secrets::{SecretStorageState, SecretStore, AI_API_KEY_SECRET_KEY};
use serde::Serialize;
use serde_json::json;

const LEGACY_AI_API_KEY_SETTING_KEY: &str = AI_API_KEY_SECRET_KEY;
const DEFAULT_AI_PROVIDER: &str = "claude";
const AI_API_KEY_MIGRATION_SETTING_KEY: &str = "ai_api_key_keyring_migration_v1";

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiApiKeyState {
    pub provider: String,
    pub configured: bool,
    pub storage_state: SecretStorageState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn normalize_ai_api_key(value: impl AsRef<str>) -> Option<String> {
    let token = value.as_ref().trim().to_string();
    (!token.is_empty()).then_some(token)
}

fn normalize_provider(provider: Option<&str>) -> &str {
    provider
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_AI_PROVIDER)
}

async fn resolve_operation_provider(pool: &DbPool, provider: Option<&str>) -> String {
    if let Some(provider) = provider.map(str::trim).filter(|value| !value.is_empty()) {
        return provider.to_string();
    }

    super::get_ai_setting(pool, "ai_provider")
        .await
        .unwrap_or_else(|| DEFAULT_AI_PROVIDER.to_string())
}

fn provider_secret_key(provider: Option<&str>) -> String {
    let provider = normalize_provider(provider);
    format!("{}__{}", AI_API_KEY_SECRET_KEY, provider)
}

fn map_secret_error(action: &str, error: crate::secrets::SecretError) -> String {
    format!("Failed to {} AI API key: {}", action, error)
}

async fn record_ai_api_key_migration_failure(pool: &DbPool, error: &str, reason: &str) {
    crate::operation_log::record_operation_log_best_effort(
        pool,
        crate::operation_log::local_target_context(),
        crate::operation_log::OperationLogEvent::new(
            "settings",
            "settings.ai_api_key_migration",
            "failed",
            "AI API key migration to secure storage failed",
        )
        .subject("setting", LEGACY_AI_API_KEY_SETTING_KEY, "AI API key")
        .error(error)
        .details(json!({
            "key": LEGACY_AI_API_KEY_SETTING_KEY,
            "reason": reason,
            "legacySettingRetained": true,
        })),
    )
    .await;
}

async fn legacy_ai_api_key_from_settings(pool: &DbPool) -> Result<Option<String>, String> {
    Ok(db::get_setting(pool, LEGACY_AI_API_KEY_SETTING_KEY)
        .await?
        .and_then(normalize_ai_api_key))
}

async fn mark_ai_api_key_migration_complete(pool: &DbPool) -> Result<(), String> {
    db::set_setting(pool, AI_API_KEY_MIGRATION_SETTING_KEY, "1").await
}

async fn is_ai_api_key_migration_marked(pool: &DbPool) -> Result<bool, String> {
    Ok(db::get_setting(pool, AI_API_KEY_MIGRATION_SETTING_KEY)
        .await?
        .as_deref()
        == Some("1"))
}

async fn delete_legacy_ai_api_key_setting(pool: &DbPool) -> Result<(), String> {
    db::delete_setting(pool, LEGACY_AI_API_KEY_SETTING_KEY).await
}

fn log_ai_api_key_migration_warning(message: impl AsRef<str>) {
    tracing::warn!(message = %message.as_ref(), "AI API key migration warning");
}

async fn migrate_ai_api_key_to_secret_store(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<Option<String>, String> {
    if is_ai_api_key_migration_marked(pool).await? {
        return Ok(None);
    }

    let Some(token) = legacy_ai_api_key_from_settings(pool).await? else {
        return Ok(None);
    };

    let storage_state = match secrets.set(AI_API_KEY_SECRET_KEY, &token) {
        Ok(storage_state) => storage_state,
        Err(error) => {
            let mapped_error = map_secret_error("migrate", error);
            log_ai_api_key_migration_warning(format!(
                "AI API key migration failed; keeping legacy settings value: {}",
                mapped_error
            ));
            record_ai_api_key_migration_failure(pool, &mapped_error, "secret_store_set").await;
            return Ok(Some(mapped_error));
        }
    };
    if !storage_state.is_available() {
        let mapped_error = format!(
            "Failed to migrate AI API key: unavailable storage state {:?}",
            storage_state
        );
        log_ai_api_key_migration_warning(format!(
            "AI API key migration did not produce an available secret state: {:?}",
            storage_state
        ));
        record_ai_api_key_migration_failure(pool, &mapped_error, "unavailable_storage_state").await;
        return Ok(Some(mapped_error));
    }

    match secrets.get(AI_API_KEY_SECRET_KEY) {
        Ok(Some(saved)) if normalize_ai_api_key(&saved).as_deref() == Some(token.as_str()) => {
            delete_legacy_ai_api_key_setting(pool).await?;
            mark_ai_api_key_migration_complete(pool).await?;
            Ok(None)
        }
        Ok(_) => {
            let mapped_error =
                "Failed to verify migrated AI API key; keeping legacy settings value.";
            log_ai_api_key_migration_warning(
                "AI API key migration readback did not match; keeping legacy settings value.",
            );
            record_ai_api_key_migration_failure(pool, mapped_error, "readback_mismatch").await;
            Ok(Some(mapped_error.to_string()))
        }
        Err(error) => {
            let mapped_error = map_secret_error("verify migrated", error);
            log_ai_api_key_migration_warning(format!(
                "AI API key migration readback failed; keeping legacy settings value: {}",
                mapped_error
            ));
            record_ai_api_key_migration_failure(pool, &mapped_error, "readback_error").await;
            Ok(Some(mapped_error))
        }
    }
}

async fn migrate_ai_api_key_to_provider_secret_store(
    pool: &DbPool,
    secrets: &dyn SecretStore,
    provider: &str,
) -> Result<Option<String>, String> {
    let Some(token) = legacy_ai_api_key_from_settings(pool).await? else {
        return Ok(None);
    };

    let key = provider_secret_key(Some(provider));
    let storage_state = match secrets.set(&key, &token) {
        Ok(storage_state) => storage_state,
        Err(error) => {
            let mapped_error = map_secret_error("migrate", error);
            log_ai_api_key_migration_warning(format!(
                "AI API key migration to provider secret failed; keeping legacy settings value: {}",
                mapped_error
            ));
            record_ai_api_key_migration_failure(pool, &mapped_error, "provider_secret_store_set")
                .await;
            return Ok(Some(mapped_error));
        }
    };
    if !storage_state.is_available() {
        let mapped_error = format!(
            "Failed to migrate AI API key: unavailable storage state {:?}",
            storage_state
        );
        log_ai_api_key_migration_warning(format!(
            "AI API key migration to provider secret did not produce an available secret state: {:?}",
            storage_state
        ));
        record_ai_api_key_migration_failure(
            pool,
            &mapped_error,
            "provider_unavailable_storage_state",
        )
        .await;
        return Ok(Some(mapped_error));
    }

    match secrets.get(&key) {
        Ok(Some(saved)) if normalize_ai_api_key(&saved).as_deref() == Some(token.as_str()) => {
            delete_legacy_ai_api_key_setting(pool).await?;
            mark_ai_api_key_migration_complete(pool).await?;
            Ok(None)
        }
        Ok(_) => {
            let mapped_error =
                "Failed to verify migrated AI API key; keeping legacy settings value.";
            log_ai_api_key_migration_warning(
                "AI API key provider migration readback did not match; keeping legacy settings value.",
            );
            record_ai_api_key_migration_failure(pool, mapped_error, "provider_readback_mismatch")
                .await;
            Ok(Some(mapped_error.to_string()))
        }
        Err(error) => {
            let mapped_error = map_secret_error("verify migrated", error);
            log_ai_api_key_migration_warning(format!(
                "AI API key provider migration readback failed; keeping legacy settings value: {}",
                mapped_error
            ));
            record_ai_api_key_migration_failure(pool, &mapped_error, "provider_readback_error")
                .await;
            Ok(Some(mapped_error))
        }
    }
}

pub async fn migrate_ai_api_key_on_startup(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<(), String> {
    let _ = migrate_ai_api_key_to_secret_store(pool, secrets).await?;
    Ok(())
}

pub(crate) async fn ai_api_key_from_secret_store(
    pool: &DbPool,
    secrets: &dyn SecretStore,
    provider: Option<&str>,
) -> Result<Option<String>, String> {
    let key = provider_secret_key(provider);
    let provider_id = normalize_provider(provider);
    let mut secret_error = None;
    match secrets.get(&key) {
        Ok(secret) => {
            if let Some(token) = secret.and_then(normalize_ai_api_key) {
                return Ok(Some(token));
            }
        }
        Err(error) => {
            secret_error = Some(map_secret_error("read", error));
        }
    }

    let migration_error = migrate_ai_api_key_to_secret_store(pool, secrets).await?;
    match secrets.get(&key) {
        Ok(secret) => {
            if let Some(token) = secret.and_then(normalize_ai_api_key) {
                return Ok(Some(token));
            }
        }
        Err(error) => {
            secret_error.get_or_insert_with(|| map_secret_error("read", error));
        }
    }

    match secrets.get(AI_API_KEY_SECRET_KEY) {
        Ok(secret) => {
            if let Some(token) = secret.and_then(normalize_ai_api_key) {
                return Ok(Some(token));
            }
        }
        Err(error) => {
            secret_error.get_or_insert_with(|| map_secret_error("read legacy", error));
        }
    }

    let legacy_token = legacy_ai_api_key_from_settings(pool).await?;
    if let Some(token) = legacy_token {
        match migrate_ai_api_key_to_provider_secret_store(pool, secrets, provider_id).await? {
            None => return Ok(Some(token)),
            Some(error) => {
                secret_error.get_or_insert(error);
                return Ok(Some(token));
            }
        }
    }

    if let Some(error) = migration_error.or(secret_error) {
        return Err(error);
    }

    Ok(None)
}

fn available_secret_state(
    secrets: &dyn SecretStore,
    key: &str,
    action: &str,
) -> Result<Option<SecretStorageState>, String> {
    match secrets.state(key) {
        Ok(state) if state.is_available() => Ok(Some(state)),
        Ok(_) => Ok(None),
        Err(error) => Err(map_secret_error(action, error)),
    }
}

pub async fn get_ai_api_key_state_impl(
    pool: &DbPool,
    secrets: &dyn SecretStore,
    provider: Option<&str>,
) -> Result<AiApiKeyState, String> {
    let provider_id = resolve_operation_provider(pool, provider).await;
    let key = provider_secret_key(Some(&provider_id));
    let mut migration_error = migrate_ai_api_key_to_secret_store(pool, secrets).await?;
    let mut state_error = None;

    match available_secret_state(secrets, &key, "inspect") {
        Ok(Some(storage_state)) => {
            return Ok(AiApiKeyState {
                provider: provider_id,
                configured: true,
                storage_state,
                error: migration_error,
            });
        }
        Ok(None) => {}
        Err(error) => state_error = Some(error),
    }

    match available_secret_state(secrets, AI_API_KEY_SECRET_KEY, "inspect legacy") {
        Ok(Some(storage_state)) => {
            return Ok(AiApiKeyState {
                provider: provider_id,
                configured: true,
                storage_state,
                error: migration_error,
            });
        }
        Ok(None) => {}
        Err(error) => {
            state_error.get_or_insert(error);
        }
    };

    let legacy_token = legacy_ai_api_key_from_settings(pool).await?;
    if legacy_token.is_some() {
        if let Some(error) =
            migrate_ai_api_key_to_provider_secret_store(pool, secrets, &provider_id).await?
        {
            migration_error = migration_error.or(Some(error));
        }
    }
    Ok(AiApiKeyState {
        provider: provider_id,
        configured: legacy_token.is_some(),
        storage_state: if state_error.is_some() {
            SecretStorageState::Unreadable
        } else {
            SecretStorageState::Missing
        },
        error: migration_error.or(state_error),
    })
}

pub async fn set_ai_api_key_impl(
    pool: &DbPool,
    secrets: &dyn SecretStore,
    value: String,
    provider: Option<&str>,
) -> Result<AiApiKeyState, String> {
    let provider_id = resolve_operation_provider(pool, provider).await;
    let key = provider_secret_key(Some(&provider_id));
    let token = normalize_ai_api_key(&value)
        .ok_or_else(|| "AI API key cannot be empty; clear the key instead.".to_string())?;
    let storage_state = secrets
        .set(&key, &token)
        .map_err(|error| map_secret_error("save", error))?;
    if !storage_state.is_available() {
        return Err(format!(
            "Failed to save AI API key: unavailable storage state {:?}",
            storage_state
        ));
    }

    let saved = secrets
        .get(&key)
        .map_err(|error| map_secret_error("verify", error))?
        .and_then(normalize_ai_api_key);
    if saved.as_deref() != Some(token.as_str()) {
        return Err("Failed to verify saved AI API key.".to_string());
    }

    delete_legacy_ai_api_key_setting(pool).await?;
    mark_ai_api_key_migration_complete(pool).await?;
    Ok(AiApiKeyState {
        provider: provider_id,
        configured: true,
        storage_state,
        error: None,
    })
}

pub async fn clear_ai_api_key_impl(
    pool: &DbPool,
    secrets: &dyn SecretStore,
    provider: Option<&str>,
) -> Result<AiApiKeyState, String> {
    let provider_id = resolve_operation_provider(pool, provider).await;
    let key = provider_secret_key(Some(&provider_id));
    secrets
        .delete(&key)
        .map_err(|error| map_secret_error("clear", error))?;
    secrets
        .delete(AI_API_KEY_SECRET_KEY)
        .map_err(|error| map_secret_error("clear legacy", error))?;
    delete_legacy_ai_api_key_setting(pool).await?;
    mark_ai_api_key_migration_complete(pool).await?;
    Ok(AiApiKeyState {
        provider: provider_id,
        configured: false,
        storage_state: SecretStorageState::Missing,
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::{MockSecretStore, SecretError};

    async fn setup_test_db() -> DbPool {
        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        db::init_database(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn legacy_ai_api_key_migrates_to_secret_store_and_deletes_setting() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();
        db::set_setting(&pool, LEGACY_AI_API_KEY_SETTING_KEY, "  sk-test  ")
            .await
            .unwrap();

        migrate_ai_api_key_on_startup(&pool, &secrets)
            .await
            .unwrap();

        assert_eq!(
            secrets
                .get(AI_API_KEY_SECRET_KEY)
                .expect("secret read")
                .as_deref(),
            Some("sk-test")
        );
        assert_eq!(
            db::get_setting(&pool, LEGACY_AI_API_KEY_SETTING_KEY)
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            db::get_setting(&pool, AI_API_KEY_MIGRATION_SETTING_KEY)
                .await
                .unwrap()
                .as_deref(),
            Some("1")
        );
    }

    #[tokio::test]
    async fn legacy_ai_api_key_migration_failure_keeps_setting_and_marker_absent() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();
        secrets.set_set_error(SecretError::Other("keyring down".to_string()));
        db::set_setting(&pool, LEGACY_AI_API_KEY_SETTING_KEY, "sk-legacy")
            .await
            .unwrap();

        let state = get_ai_api_key_state_impl(&pool, &secrets, None)
            .await
            .unwrap();

        assert!(state.configured);
        assert!(state
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("keyring down"));
        assert_eq!(
            db::get_setting(&pool, LEGACY_AI_API_KEY_SETTING_KEY)
                .await
                .unwrap()
                .as_deref(),
            Some("sk-legacy")
        );
        assert_eq!(
            db::get_setting(&pool, AI_API_KEY_MIGRATION_SETTING_KEY)
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            ai_api_key_from_secret_store(&pool, &secrets, None)
                .await
                .unwrap()
                .as_deref(),
            Some("sk-legacy")
        );
    }

    #[tokio::test]
    async fn set_ai_api_key_uses_secret_store_and_removes_legacy_setting() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();
        db::set_setting(&pool, LEGACY_AI_API_KEY_SETTING_KEY, "sk-legacy")
            .await
            .unwrap();

        let state = set_ai_api_key_impl(&pool, &secrets, "  sk-new  ".to_string(), None)
            .await
            .unwrap();

        assert!(state.configured);
        assert_eq!(state.provider, DEFAULT_AI_PROVIDER);
        assert_eq!(
            secrets
                .get("ai_api_key__claude")
                .expect("secret read")
                .as_deref(),
            Some("sk-new")
        );
        assert_eq!(
            secrets
                .get(AI_API_KEY_SECRET_KEY)
                .expect("legacy secret read")
                .as_deref(),
            None
        );
        assert_eq!(
            db::get_setting(&pool, LEGACY_AI_API_KEY_SETTING_KEY)
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn provider_scoped_ai_api_key_uses_provider_secret_with_legacy_fallback() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();
        secrets
            .set(AI_API_KEY_SECRET_KEY, "sk-legacy")
            .expect("legacy secret");

        assert_eq!(
            ai_api_key_from_secret_store(&pool, &secrets, Some("deepseek"))
                .await
                .unwrap()
                .as_deref(),
            Some("sk-legacy")
        );

        let state =
            set_ai_api_key_impl(&pool, &secrets, "sk-deepseek".to_string(), Some("deepseek"))
                .await
                .unwrap();

        assert_eq!(state.provider, "deepseek");
        assert_eq!(
            secrets
                .get("ai_api_key__deepseek")
                .expect("secret read")
                .as_deref(),
            Some("sk-deepseek")
        );
        assert_eq!(
            secrets
                .get(AI_API_KEY_SECRET_KEY)
                .expect("legacy secret read")
                .as_deref(),
            Some("sk-legacy")
        );
        assert_eq!(
            ai_api_key_from_secret_store(&pool, &secrets, Some("deepseek"))
                .await
                .unwrap()
                .as_deref(),
            Some("sk-deepseek")
        );
    }

    #[tokio::test]
    async fn omitted_provider_uses_current_ai_provider_for_state_and_write() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::default();
        db::set_setting(&pool, "ai_provider", "deepseek")
            .await
            .unwrap();
        secrets
            .set("ai_api_key__deepseek", "sk-deepseek")
            .expect("provider secret");

        let state = get_ai_api_key_state_impl(&pool, &secrets, None)
            .await
            .unwrap();

        assert_eq!(state.provider, "deepseek");
        assert!(state.configured);

        let state = set_ai_api_key_impl(&pool, &secrets, "sk-new".to_string(), None)
            .await
            .unwrap();

        assert_eq!(state.provider, "deepseek");
        assert_eq!(
            secrets
                .get("ai_api_key__deepseek")
                .expect("deepseek secret")
                .as_deref(),
            Some("sk-new")
        );
        assert_eq!(
            secrets
                .get("ai_api_key__claude")
                .expect("claude secret")
                .as_deref(),
            None
        );
    }
}
