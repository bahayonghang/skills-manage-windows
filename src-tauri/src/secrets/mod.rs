//! Application-level secret storage.
//!
//! This module is intentionally separate from SSH target credentials. SSH keeps
//! its existing target-specific semantics, while app secrets such as GitHub PATs
//! and AI API keys use this injectable store through [`crate::AppState`].

mod system;

use serde::{Deserialize, Serialize};

pub use system::{SecretStoreBackend, SystemSecretStore};

/// Keyring service name for application-level secrets.
///
/// SSH passwords deliberately keep their own `SkillPort SSH Targets` service so
/// future migrations cannot accidentally cross-read target credentials.
pub const SKILLPORT_SECRET_SERVICE: &str = "SkillPort Secrets";

/// Secret key for the GitHub personal access token.
pub const GITHUB_PAT_SECRET_KEY: &str = "github_pat";

/// Secret key for the AI provider API key.
pub const AI_API_KEY_SECRET_KEY: &str = "ai_api_key";

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SecretStorageState {
    Stored,
    Session,
    #[default]
    Missing,
    Unreadable,
}

impl SecretStorageState {
    pub fn is_available(self) -> bool {
        matches!(self, Self::Stored | Self::Session)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    NoEntry,
    Other(String),
}

impl SecretError {
    pub fn message(&self) -> String {
        match self {
            SecretError::NoEntry => "No saved secret is available.".to_string(),
            SecretError::Other(error) => error.clone(),
        }
    }
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message())
    }
}

impl std::error::Error for SecretError {}

pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>, SecretError>;
    fn set(&self, key: &str, value: &str) -> Result<SecretStorageState, SecretError>;
    fn delete(&self, key: &str) -> Result<(), SecretError>;
    fn state(&self, key: &str) -> Result<SecretStorageState, SecretError>;
}

#[cfg(test)]
pub struct MockSecretStore {
    values: std::sync::Mutex<std::collections::HashMap<String, String>>,
    set_state: std::sync::Mutex<SecretStorageState>,
    get_error: std::sync::Mutex<Option<SecretError>>,
    set_error: std::sync::Mutex<Option<SecretError>>,
    delete_error: std::sync::Mutex<Option<SecretError>>,
}

#[cfg(test)]
impl Default for MockSecretStore {
    fn default() -> Self {
        Self {
            values: std::sync::Mutex::new(std::collections::HashMap::new()),
            set_state: std::sync::Mutex::new(SecretStorageState::Stored),
            get_error: std::sync::Mutex::new(None),
            set_error: std::sync::Mutex::new(None),
            delete_error: std::sync::Mutex::new(None),
        }
    }
}

#[cfg(test)]
impl MockSecretStore {
    pub fn with_value(key: &str, value: &str) -> Self {
        let store = Self::default();
        store
            .values
            .lock()
            .expect("mock secret values")
            .insert(key.to_string(), value.to_string());
        store
    }

    pub fn set_get_error(&self, error: SecretError) {
        *self.get_error.lock().expect("mock get error") = Some(error);
    }

    pub fn set_set_error(&self, error: SecretError) {
        *self.set_error.lock().expect("mock set error") = Some(error);
    }

    pub fn set_delete_error(&self, error: SecretError) {
        *self.delete_error.lock().expect("mock delete error") = Some(error);
    }

    pub fn set_next_state(&self, state: SecretStorageState) {
        *self.set_state.lock().expect("mock set state") = state;
    }
}

#[cfg(test)]
impl SecretStore for MockSecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, SecretError> {
        if let Some(error) = self.get_error.lock().expect("mock get error").clone() {
            return Err(error);
        }
        Ok(self
            .values
            .lock()
            .expect("mock secret values")
            .get(key)
            .cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<SecretStorageState, SecretError> {
        if let Some(error) = self.set_error.lock().expect("mock set error").clone() {
            return Err(error);
        }
        self.values
            .lock()
            .expect("mock secret values")
            .insert(key.to_string(), value.to_string());
        Ok(*self.set_state.lock().expect("mock set state"))
    }

    fn delete(&self, key: &str) -> Result<(), SecretError> {
        if let Some(error) = self.delete_error.lock().expect("mock delete error").clone() {
            return Err(error);
        }
        self.values.lock().expect("mock secret values").remove(key);
        Ok(())
    }

    fn state(&self, key: &str) -> Result<SecretStorageState, SecretError> {
        if self.get(key)?.is_some() {
            Ok(*self.set_state.lock().expect("mock set state"))
        } else {
            Ok(SecretStorageState::Missing)
        }
    }
}
