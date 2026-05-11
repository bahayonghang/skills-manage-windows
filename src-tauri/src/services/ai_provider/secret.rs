use crate::db::{self, DbPool};
use crate::secrets::{SecretStorageState, SecretStore, AI_API_KEY_SECRET_KEY};
use serde::Serialize;
use serde_json::json;

const LEGACY_AI_API_KEY_SETTING_KEY: &str = AI_API_KEY_SECRET_KEY;
const AI_API_KEY_MIGRATION_SETTING_KEY: &str = "ai_api_key_keyring_migration_v1";

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiApiKeyState {
    pub configured: bool,
    pub storage_state: SecretStorageState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn normalize_ai_api_key(value: impl AsRef<str>) -> Option<String> {
    let token = value.as_ref().trim().to_string();
    (!token.is_empty()).then_some(token)
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
) -> Result<Option<String>, String> {
    let mut secret_error = None;
    match secrets.get(AI_API_KEY_SECRET_KEY) {
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
    match secrets.get(AI_API_KEY_SECRET_KEY) {
        Ok(secret) => {
            if let Some(token) = secret.and_then(normalize_ai_api_key) {
                return Ok(Some(token));
            }
        }
        Err(error) => {
            secret_error.get_or_insert_with(|| map_secret_error("read", error));
        }
    }

    let legacy_token = legacy_ai_api_key_from_settings(pool).await?;
    if legacy_token.is_some() {
        return Ok(legacy_token);
    }

    if let Some(error) = migration_error.or(secret_error) {
        return Err(error);
    }

    Ok(None)
}

pub async fn get_ai_api_key_state_impl(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<AiApiKeyState, String> {
    let migration_error = migrate_ai_api_key_to_secret_store(pool, secrets).await?;

    match secrets.state(AI_API_KEY_SECRET_KEY) {
        Ok(storage_state) if storage_state.is_available() => Ok(AiApiKeyState {
            configured: true,
            storage_state,
            error: migration_error,
        }),
        Ok(SecretStorageState::Unreadable) => {
            let legacy_token = legacy_ai_api_key_from_settings(pool).await?;
            Ok(AiApiKeyState {
                configured: legacy_token.is_some(),
                storage_state: SecretStorageState::Unreadable,
                error: migration_error,
            })
        }
        Ok(_) => {
            let legacy_token = legacy_ai_api_key_from_settings(pool).await?;
            Ok(AiApiKeyState {
                configured: legacy_token.is_some(),
                storage_state: SecretStorageState::Missing,
                error: migration_error,
            })
        }
        Err(error) => {
            let legacy_token = legacy_ai_api_key_from_settings(pool).await?;
            Ok(AiApiKeyState {
                configured: legacy_token.is_some(),
                storage_state: SecretStorageState::Unreadable,
                error: Some(migration_error.unwrap_or_else(|| map_secret_error("inspect", error))),
            })
        }
    }
}

pub async fn set_ai_api_key_impl(
    pool: &DbPool,
    secrets: &dyn SecretStore,
    value: String,
) -> Result<AiApiKeyState, String> {
    let token = normalize_ai_api_key(&value)
        .ok_or_else(|| "AI API key cannot be empty; clear the key instead.".to_string())?;
    let storage_state = secrets
        .set(AI_API_KEY_SECRET_KEY, &token)
        .map_err(|error| map_secret_error("save", error))?;
    if !storage_state.is_available() {
        return Err(format!(
            "Failed to save AI API key: unavailable storage state {:?}",
            storage_state
        ));
    }

    let saved = secrets
        .get(AI_API_KEY_SECRET_KEY)
        .map_err(|error| map_secret_error("verify", error))?
        .and_then(normalize_ai_api_key);
    if saved.as_deref() != Some(token.as_str()) {
        return Err("Failed to verify saved AI API key.".to_string());
    }

    delete_legacy_ai_api_key_setting(pool).await?;
    mark_ai_api_key_migration_complete(pool).await?;
    Ok(AiApiKeyState {
        configured: true,
        storage_state,
        error: None,
    })
}

pub async fn clear_ai_api_key_impl(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<AiApiKeyState, String> {
    secrets
        .delete(AI_API_KEY_SECRET_KEY)
        .map_err(|error| map_secret_error("clear", error))?;
    delete_legacy_ai_api_key_setting(pool).await?;
    mark_ai_api_key_migration_complete(pool).await?;
    Ok(AiApiKeyState {
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

        let state = get_ai_api_key_state_impl(&pool, &secrets).await.unwrap();

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
            ai_api_key_from_secret_store(&pool, &secrets)
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

        let state = set_ai_api_key_impl(&pool, &secrets, "  sk-new  ".to_string())
            .await
            .unwrap();

        assert!(state.configured);
        assert_eq!(
            secrets
                .get(AI_API_KEY_SECRET_KEY)
                .expect("secret read")
                .as_deref(),
            Some("sk-new")
        );
        assert_eq!(
            db::get_setting(&pool, LEGACY_AI_API_KEY_SETTING_KEY)
                .await
                .unwrap(),
            None
        );
    }
}
