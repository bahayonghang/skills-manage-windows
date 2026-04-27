use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::db::{self, DbPool};

const TARGETS_SETTING_KEY: &str = "ssh_targets_v1";
const ACTIVE_TARGET_SETTING_KEY: &str = "active_target_id_v1";
const SSH_PASSWORD_SERVICE: &str = "SkillPort SSH Targets";
pub const LOCAL_TARGET_ID: &str = "local";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Local,
    Ssh,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SshAuthMethod {
    #[default]
    Key,
    Password,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TargetCredentialStatus {
    Stored,
    Session,
    Missing,
    Unreadable,
}

impl TargetCredentialStatus {
    fn is_available(self) -> bool {
        matches!(
            self,
            TargetCredentialStatus::Stored | TargetCredentialStatus::Session
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTargetConfig {
    pub id: String,
    pub label: String,
    pub host: String,
    pub username: String,
    pub port: u16,
    #[serde(default)]
    pub auth_method: SshAuthMethod,
    #[serde(default)]
    pub key_path: String,
    #[serde(default)]
    pub credential_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected_password: Option<String>,
    #[serde(skip)]
    pub password: Option<String>,
    pub remote_home: String,
    pub remote_os: String,
    pub symlink_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSshTargetRequest {
    pub label: String,
    pub host: String,
    pub username: String,
    pub port: Option<u16>,
    pub auth_method: Option<SshAuthMethod>,
    pub key_path: Option<String>,
    pub password: Option<String>,
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestSshTargetRequest {
    pub id: Option<String>,
    pub label: Option<String>,
    pub host: Option<String>,
    pub username: Option<String>,
    pub port: Option<u16>,
    pub auth_method: Option<SshAuthMethod>,
    pub key_path: Option<String>,
    pub password: Option<String>,
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSummary {
    pub id: String,
    pub kind: TargetKind,
    pub label: String,
    pub host: Option<String>,
    pub username: Option<String>,
    pub port: Option<u16>,
    pub auth_method: Option<SshAuthMethod>,
    pub remote_home: Option<String>,
    pub remote_os: Option<String>,
    pub cache_db_path: Option<String>,
    pub has_stored_password: Option<bool>,
    pub credential_status: Option<TargetCredentialStatus>,
    pub credential_error: Option<String>,
    pub symlink_enabled: Option<bool>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTargetTestResult {
    pub ok: bool,
    pub remote_home: Option<String>,
    pub remote_os: Option<String>,
    pub credential_status: Option<TargetCredentialStatus>,
    pub credential_error: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum ActiveTarget {
    Local,
    Ssh(Box<RemoteTargetConfig>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CredentialStoreError {
    NoEntry,
    Other(String),
}

impl CredentialStoreError {
    fn message(&self) -> String {
        match self {
            CredentialStoreError::NoEntry => "No saved SSH password is available.".to_string(),
            CredentialStoreError::Other(error) => error.clone(),
        }
    }
}

trait CredentialBackend: Send + Sync {
    fn set_password(
        &self,
        credential_key: &str,
        password: &str,
    ) -> Result<(), CredentialStoreError>;
    fn get_password(&self, credential_key: &str) -> Result<String, CredentialStoreError>;
    fn delete_credential(&self, credential_key: &str) -> Result<(), CredentialStoreError>;
}

struct SystemCredentialBackend;

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("Protected password payload is not valid hex.".to_string());
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            let high = (chunk[0] as char)
                .to_digit(16)
                .ok_or_else(|| "Protected password payload is not valid hex.".to_string())?;
            let low = (chunk[1] as char)
                .to_digit(16)
                .ok_or_else(|| "Protected password payload is not valid hex.".to_string())?;
            Ok(((high << 4) | low) as u8)
        })
        .collect()
}

#[cfg(windows)]
mod protected_credentials {
    use std::ffi::c_void;
    use std::io;
    use std::ptr::{null, null_mut};
    use std::slice;

    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct DataBlob {
        cbData: u32,
        pbData: *mut u8,
    }

    #[link(name = "Crypt32")]
    unsafe extern "system" {
        fn CryptProtectData(
            pDataIn: *mut DataBlob,
            szDataDescr: *const u16,
            pOptionalEntropy: *mut DataBlob,
            pvReserved: *mut c_void,
            pPromptStruct: *mut c_void,
            dwFlags: u32,
            pDataOut: *mut DataBlob,
        ) -> i32;

        fn CryptUnprotectData(
            pDataIn: *mut DataBlob,
            ppszDataDescr: *mut *mut u16,
            pOptionalEntropy: *mut DataBlob,
            pvReserved: *mut c_void,
            pPromptStruct: *mut c_void,
            dwFlags: u32,
            pDataOut: *mut DataBlob,
        ) -> i32;
    }

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn LocalFree(hmem: *mut c_void) -> *mut c_void;
    }

    fn last_error(action: &str) -> String {
        format!(
            "Failed to {} SSH password with Windows DPAPI: {}",
            action,
            io::Error::last_os_error()
        )
    }

    pub fn protect(password: &str) -> Result<String, String> {
        let mut input = password.as_bytes().to_vec();
        let mut input_blob = DataBlob {
            cbData: input
                .len()
                .try_into()
                .map_err(|_| "SSH password is too large to protect.".to_string())?,
            pbData: input.as_mut_ptr(),
        };
        let mut output_blob = DataBlob {
            cbData: 0,
            pbData: null_mut(),
        };

        let ok = unsafe {
            CryptProtectData(
                &mut input_blob,
                null(),
                null_mut(),
                null_mut(),
                null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output_blob,
            )
        };
        if ok == 0 {
            return Err(last_error("protect"));
        }

        let protected = unsafe {
            slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize).to_vec()
        };
        unsafe {
            LocalFree(output_blob.pbData.cast::<c_void>());
        }
        Ok(super::hex_encode(&protected))
    }

    pub fn unprotect(protected: &str) -> Result<String, String> {
        let mut input = super::hex_decode(protected)?;
        let mut input_blob = DataBlob {
            cbData: input
                .len()
                .try_into()
                .map_err(|_| "Protected SSH password payload is too large.".to_string())?,
            pbData: input.as_mut_ptr(),
        };
        let mut output_blob = DataBlob {
            cbData: 0,
            pbData: null_mut(),
        };

        let ok = unsafe {
            CryptUnprotectData(
                &mut input_blob,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output_blob,
            )
        };
        if ok == 0 {
            return Err(last_error("unprotect"));
        }

        let plaintext = unsafe {
            slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize).to_vec()
        };
        unsafe {
            LocalFree(output_blob.pbData.cast::<c_void>());
        }
        String::from_utf8(plaintext)
            .map_err(|e| format!("Protected SSH password is not valid UTF-8: {}", e))
    }
}

#[cfg(not(windows))]
mod protected_credentials {
    pub fn protect(_password: &str) -> Result<String, String> {
        Err("App-local protected SSH password fallback is only available on Windows.".to_string())
    }

    pub fn unprotect(_protected: &str) -> Result<String, String> {
        Err("App-local protected SSH password fallback is only available on Windows.".to_string())
    }
}

impl SystemCredentialBackend {
    fn entry(credential_key: &str) -> Result<keyring::Entry, CredentialStoreError> {
        keyring::Entry::new(SSH_PASSWORD_SERVICE, credential_key).map_err(|e| {
            CredentialStoreError::Other(format!("Failed to access system credential store: {}", e))
        })
    }
}

impl CredentialBackend for SystemCredentialBackend {
    fn set_password(
        &self,
        credential_key: &str,
        password: &str,
    ) -> Result<(), CredentialStoreError> {
        Self::entry(credential_key)?
            .set_password(password)
            .map_err(|e| {
                CredentialStoreError::Other(format!(
                    "Failed to store SSH password in system credential store: {}",
                    e
                ))
            })
    }

    fn get_password(&self, credential_key: &str) -> Result<String, CredentialStoreError> {
        match Self::entry(credential_key)?.get_password() {
            Ok(password) => Ok(password),
            Err(keyring::Error::NoEntry) => Err(CredentialStoreError::NoEntry),
            Err(e) => Err(CredentialStoreError::Other(format!(
                "Failed to read SSH password from system credential store: {}",
                e
            ))),
        }
    }

    fn delete_credential(&self, credential_key: &str) -> Result<(), CredentialStoreError> {
        match Self::entry(credential_key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(CredentialStoreError::Other(format!(
                "Failed to delete SSH password from system credential store: {}",
                e
            ))),
        }
    }
}

fn protected_password_for_target(target: &RemoteTargetConfig) -> Result<Option<String>, String> {
    target
        .protected_password
        .as_deref()
        .map(protected_credentials::unprotect)
        .transpose()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TargetCredentialState {
    status: TargetCredentialStatus,
    error: Option<String>,
}

pub struct TargetRegistry {
    pools: Mutex<HashMap<String, DbPool>>,
    session_passwords: Mutex<HashMap<String, String>>,
    credential_backend: Arc<dyn CredentialBackend>,
}

impl Default for TargetRegistry {
    fn default() -> Self {
        Self {
            pools: Mutex::new(HashMap::new()),
            session_passwords: Mutex::new(HashMap::new()),
            credential_backend: Arc::new(SystemCredentialBackend),
        }
    }
}

impl TargetRegistry {
    #[cfg(test)]
    fn with_credential_backend(credential_backend: Arc<dyn CredentialBackend>) -> Self {
        Self {
            pools: Mutex::new(HashMap::new()),
            session_passwords: Mutex::new(HashMap::new()),
            credential_backend,
        }
    }

    fn get_session_password(&self, credential_key: &str) -> Option<String> {
        self.session_passwords
            .lock()
            .ok()
            .and_then(|passwords| passwords.get(credential_key).cloned())
    }

    fn set_session_password(&self, credential_key: &str, password: &str) -> Result<(), String> {
        self.session_passwords
            .lock()
            .map_err(|_| "Failed to update SSH password session cache.".to_string())?
            .insert(credential_key.to_string(), password.to_string());
        Ok(())
    }

    fn clear_session_password(&self, credential_key: &str) {
        if let Ok(mut passwords) = self.session_passwords.lock() {
            passwords.remove(credential_key);
        }
    }

    fn target_credential_state(
        &self,
        target: &RemoteTargetConfig,
    ) -> Option<TargetCredentialState> {
        let credential_key = credential_key_for_password_target(target)?;
        match self.credential_backend.get_password(&credential_key) {
            Ok(password) if !password.is_empty() => Some(TargetCredentialState {
                status: TargetCredentialStatus::Stored,
                error: None,
            }),
            Ok(_) => {
                if let Ok(Some(_)) = protected_password_for_target(target) {
                    return Some(TargetCredentialState {
                        status: TargetCredentialStatus::Stored,
                        error: Some(
                            "System credential store returned an empty SSH password; using the Windows protected local credential."
                                .to_string(),
                        ),
                    });
                }
                if self.get_session_password(&credential_key).is_some() {
                    Some(TargetCredentialState {
                        status: TargetCredentialStatus::Session,
                        error: Some(
                            "Stored SSH password is empty; using the current session password."
                                .to_string(),
                        ),
                    })
                } else {
                    Some(TargetCredentialState {
                        status: TargetCredentialStatus::Missing,
                        error: None,
                    })
                }
            }
            Err(CredentialStoreError::NoEntry) => {
                match protected_password_for_target(target) {
                    Ok(Some(_)) => {
                        return Some(TargetCredentialState {
                            status: TargetCredentialStatus::Stored,
                            error: None,
                        });
                    }
                    Err(error) => {
                        if self.get_session_password(&credential_key).is_none() {
                            return Some(TargetCredentialState {
                                status: TargetCredentialStatus::Unreadable,
                                error: Some(error),
                            });
                        }
                    }
                    Ok(None) => {}
                }
                if self.get_session_password(&credential_key).is_some() {
                    Some(TargetCredentialState {
                        status: TargetCredentialStatus::Session,
                        error: None,
                    })
                } else {
                    Some(TargetCredentialState {
                        status: TargetCredentialStatus::Missing,
                        error: None,
                    })
                }
            }
            Err(error) => {
                match protected_password_for_target(target) {
                    Ok(Some(_)) => {
                        return Some(TargetCredentialState {
                            status: TargetCredentialStatus::Stored,
                            error: Some(error.message()),
                        });
                    }
                    Err(protected_error) => {
                        if self.get_session_password(&credential_key).is_none() {
                            return Some(TargetCredentialState {
                                status: TargetCredentialStatus::Unreadable,
                                error: Some(format!("{}; {}", error.message(), protected_error)),
                            });
                        }
                    }
                    Ok(None) => {}
                }
                if self.get_session_password(&credential_key).is_some() {
                    Some(TargetCredentialState {
                        status: TargetCredentialStatus::Session,
                        error: Some(error.message()),
                    })
                } else {
                    Some(TargetCredentialState {
                        status: TargetCredentialStatus::Unreadable,
                        error: Some(error.message()),
                    })
                }
            }
        }
    }

    fn attach_available_password(&self, target: &mut RemoteTargetConfig) {
        if target.auth_method != SshAuthMethod::Password
            || target
                .password
                .as_deref()
                .is_some_and(|password| !password.is_empty())
        {
            return;
        }
        let Some(credential_key) = credential_key_for_password_target(target) else {
            return;
        };
        match self.credential_backend.get_password(&credential_key) {
            Ok(password) if !password.is_empty() => {
                target.password = Some(password);
            }
            _ => {
                if let Ok(Some(password)) = protected_password_for_target(target) {
                    target.password = Some(password);
                    return;
                }
                if let Some(password) = self.get_session_password(&credential_key) {
                    target.password = Some(password);
                }
            }
        }
    }

    fn save_target_password(
        &self,
        target: &mut RemoteTargetConfig,
    ) -> Result<TargetCredentialState, String> {
        let credential_key = credential_key_for_password_target(target)
            .ok_or_else(|| "Password target is missing its credential key.".to_string())?;
        let password = target
            .password
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Password is required for password authentication.".to_string())?
            .to_string();

        let fallback_to_protected =
            |registry: &TargetRegistry,
             target: &mut RemoteTargetConfig,
             error: CredentialStoreError| {
                match protected_credentials::protect(&password) {
                    Ok(protected_password) => {
                        target.protected_password = Some(protected_password);
                        registry.set_session_password(&credential_key, &password)?;
                        Ok(TargetCredentialState {
                            status: TargetCredentialStatus::Stored,
                            error: Some(error.message()),
                        })
                    }
                    Err(protected_error) => {
                        registry.set_session_password(&credential_key, &password)?;
                        Ok(TargetCredentialState {
                            status: TargetCredentialStatus::Session,
                            error: Some(format!("{}; {}", error.message(), protected_error)),
                        })
                    }
                }
            };

        match self
            .credential_backend
            .set_password(&credential_key, &password)
            .and_then(|_| self.credential_backend.get_password(&credential_key))
        {
            Ok(saved_password) if saved_password == password => {
                target.protected_password = None;
                self.clear_session_password(&credential_key);
                Ok(TargetCredentialState {
                    status: TargetCredentialStatus::Stored,
                    error: None,
                })
            }
            Ok(_) => fallback_to_protected(
                self,
                target,
                CredentialStoreError::Other(
                    "System credential store returned a different SSH password after saving."
                        .to_string(),
                ),
            ),
            Err(error) => fallback_to_protected(self, target, error),
        }
    }

    fn delete_target_password(&self, target: &mut RemoteTargetConfig) -> Result<(), String> {
        let Some(credential_key) = credential_key_for_password_target(target) else {
            return Ok(());
        };
        self.clear_session_password(&credential_key);
        target.protected_password = None;
        self.credential_backend
            .delete_credential(&credential_key)
            .map_err(|error| error.message())
    }

    fn target_summary(&self, target: &RemoteTargetConfig, active_id: &str) -> TargetSummary {
        let credential_state = self.target_credential_state(target);
        TargetSummary {
            id: target.id.clone(),
            kind: TargetKind::Ssh,
            label: target.label.clone(),
            host: Some(target.host.clone()),
            username: Some(target.username.clone()),
            port: Some(target.port),
            auth_method: Some(target.auth_method),
            remote_home: Some(target.remote_home.clone()),
            remote_os: Some(target.remote_os.clone()),
            cache_db_path: remote_cache_db_path(&target.id)
                .ok()
                .map(|path| path.to_string_lossy().into_owned()),
            has_stored_password: credential_state
                .as_ref()
                .map(|state| state.status.is_available()),
            credential_status: credential_state.as_ref().map(|state| state.status),
            credential_error: credential_state.and_then(|state| state.error),
            symlink_enabled: Some(remote_symlink_allowed(target)),
            is_active: target.id == active_id,
        }
    }

    pub async fn list_targets(&self, local_db: &DbPool) -> Result<Vec<TargetSummary>, String> {
        let active_id = active_target_id(local_db).await?;
        let mut targets = vec![TargetSummary {
            id: LOCAL_TARGET_ID.to_string(),
            kind: TargetKind::Local,
            label: "Local".to_string(),
            host: None,
            username: None,
            port: None,
            auth_method: None,
            remote_home: None,
            remote_os: None,
            cache_db_path: None,
            has_stored_password: None,
            credential_status: None,
            credential_error: None,
            symlink_enabled: None,
            is_active: active_id == LOCAL_TARGET_ID,
        }];

        for target in load_remote_targets(local_db).await? {
            targets.push(self.target_summary(&target, active_id.as_str()));
        }

        Ok(targets)
    }

    pub async fn active_target(&self, local_db: &DbPool) -> Result<ActiveTarget, String> {
        let active_id = active_target_id(local_db).await?;
        if active_id == LOCAL_TARGET_ID {
            return Ok(ActiveTarget::Local);
        }

        let mut target = load_remote_targets(local_db)
            .await?
            .into_iter()
            .find(|target| target.id == active_id)
            .ok_or_else(|| {
                format!(
                    "Active target '{}' no longer exists. Switch back to Local.",
                    active_id
                )
            })?;
        self.attach_available_password(&mut target);
        Ok(ActiveTarget::Ssh(Box::new(target)))
    }

    pub async fn active_db(&self, local_db: &DbPool) -> Result<DbPool, String> {
        match self.active_target(local_db).await? {
            ActiveTarget::Local => Ok(local_db.clone()),
            ActiveTarget::Ssh(target) => self.remote_db(&target).await,
        }
    }

    pub async fn remote_db(&self, target: &RemoteTargetConfig) -> Result<DbPool, String> {
        if let Ok(pools) = self.pools.lock() {
            if let Some(pool) = pools.get(&target.id) {
                return Ok(pool.clone());
            }
        }

        let db_path = remote_cache_db_path(&target.id)?;
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create target cache directory '{}': {}",
                    parent.display(),
                    e
                )
            })?;
        }

        let db_path = db_path.to_string_lossy().into_owned();
        let pool = db::create_pool(&db_path).await?;
        db::init_database_for_remote_home(&pool, &target.remote_home).await?;

        if let Ok(mut pools) = self.pools.lock() {
            pools.insert(target.id.clone(), pool.clone());
        }

        Ok(pool)
    }

    fn drop_remote_pool(&self, target_id: &str) {
        if let Ok(mut pools) = self.pools.lock() {
            pools.remove(target_id);
        }
    }
}

pub async fn create_ssh_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    request: CreateSshTargetRequest,
) -> Result<TargetSummary, String> {
    let target_id = format!("ssh-{}", Uuid::new_v4());
    let base = request_to_config(request, target_id)?;
    let probe = probe_ssh_target(&base).await?;
    if !is_supported_remote_os(&probe.remote_os) {
        return Err(format!(
            "Remote OS '{}' is not supported in this version. Linux and macOS are supported.",
            probe.remote_os
        ));
    }

    let mut targets = load_remote_targets(local_db).await?;
    let mut target = base;
    target.remote_home = probe.remote_home;
    target.remote_os = probe.remote_os;
    let credential_state = if target.auth_method == SshAuthMethod::Password {
        Some(registry.save_target_password(&mut target)?)
    } else {
        None
    };
    if target.auth_method == SshAuthMethod::Password {
        target.password = None;
    }

    let stored_target = target.clone();
    targets.push(stored_target.clone());
    if let Err(error) = save_remote_targets(local_db, &targets).await {
        let mut cleanup_target = stored_target.clone();
        let _ = registry.delete_target_password(&mut cleanup_target);
        return Err(error);
    }
    registry.remote_db(&stored_target).await?;

    let active_id = active_target_id(local_db).await?;
    let mut summary = registry.target_summary(&stored_target, active_id.as_str());
    if let Some(state) = credential_state {
        summary.credential_status = Some(state.status);
        summary.credential_error = state.error;
        summary.has_stored_password = Some(state.status.is_available());
    }
    Ok(summary)
}

pub async fn test_ssh_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    request: TestSshTargetRequest,
) -> Result<SshTargetTestResult, String> {
    let supplied_password = request.password.clone();
    let has_existing_target_id = request
        .id
        .as_deref()
        .is_some_and(|id| !id.trim().is_empty());
    let mut should_store_password = false;
    let mut target = match request.id.as_deref() {
        Some(id) if !id.trim().is_empty() => load_remote_targets(local_db)
            .await?
            .into_iter()
            .find(|target| target.id == id)
            .ok_or_else(|| format!("Target '{}' not found", id))?,
        _ => test_request_to_config(request)?,
    };
    if has_existing_target_id
        && apply_supplied_password_to_existing_target(&mut target, supplied_password.as_deref())?
    {
        should_store_password = true;
    }

    match probe_ssh_target(&target).await {
        Ok(probe) if is_supported_remote_os(&probe.remote_os) => {
            let credential_state = if should_store_password {
                target.remote_home = probe.remote_home.clone();
                target.remote_os = probe.remote_os.clone();
                let credential_state = registry.save_target_password(&mut target)?;
                target.password = None;
                let mut targets = load_remote_targets(local_db).await?;
                if let Some(existing) = targets.iter_mut().find(|item| item.id == target.id) {
                    *existing = target.clone();
                    save_remote_targets(local_db, &targets).await?;
                }
                Some(credential_state)
            } else {
                None
            };
            let message =
                ssh_success_message("SSH connection is available.", credential_state.as_ref());
            Ok(SshTargetTestResult {
                ok: true,
                remote_home: Some(probe.remote_home),
                remote_os: Some(probe.remote_os),
                credential_status: credential_state.as_ref().map(|state| state.status),
                credential_error: credential_state.and_then(|state| state.error),
                message,
            })
        }
        Ok(probe) => Ok(SshTargetTestResult {
            ok: false,
            remote_home: Some(probe.remote_home),
            remote_os: Some(probe.remote_os.clone()),
            credential_status: None,
            credential_error: None,
            message: format!(
                "Remote OS '{}' is not supported in this version. Linux and macOS are supported.",
                probe.remote_os
            ),
        }),
        Err(error) => Ok(SshTargetTestResult {
            ok: false,
            remote_home: None,
            remote_os: None,
            credential_status: None,
            credential_error: None,
            message: error,
        }),
    }
}

pub async fn update_ssh_target_password_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target_id: &str,
    password: &str,
) -> Result<SshTargetTestResult, String> {
    let mut targets = load_remote_targets(local_db).await?;
    let target = targets
        .iter_mut()
        .find(|target| target.id == target_id)
        .ok_or_else(|| format!("Target '{}' not found", target_id))?;

    if target.auth_method != SshAuthMethod::Password {
        return Err("This SSH target does not use password authentication.".to_string());
    }
    apply_supplied_password_to_existing_target(target, Some(password))?;

    match probe_ssh_target(target).await {
        Ok(probe) if is_supported_remote_os(&probe.remote_os) => {
            target.remote_home = probe.remote_home.clone();
            target.remote_os = probe.remote_os.clone();
            let credential_state = registry.save_target_password(target)?;
            target.password = None;
            save_remote_targets(local_db, &targets).await?;

            let message = ssh_success_message(
                "SSH password was verified and saved.",
                Some(&credential_state),
            );
            Ok(SshTargetTestResult {
                ok: true,
                remote_home: Some(probe.remote_home),
                remote_os: Some(probe.remote_os),
                credential_status: Some(credential_state.status),
                credential_error: credential_state.error,
                message,
            })
        }
        Ok(probe) => Ok(SshTargetTestResult {
            ok: false,
            remote_home: Some(probe.remote_home),
            remote_os: Some(probe.remote_os.clone()),
            credential_status: None,
            credential_error: None,
            message: format!(
                "Remote OS '{}' is not supported in this version. Linux and macOS are supported.",
                probe.remote_os
            ),
        }),
        Err(error) => Ok(SshTargetTestResult {
            ok: false,
            remote_home: None,
            remote_os: None,
            credential_status: None,
            credential_error: None,
            message: error,
        }),
    }
}

pub async fn delete_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target_id: &str,
) -> Result<(), String> {
    if target_id == LOCAL_TARGET_ID {
        return Err("Local target cannot be deleted.".to_string());
    }

    let mut targets = load_remote_targets(local_db).await?;
    let original_len = targets.len();
    targets.retain(|target| target.id != target_id);
    if targets.len() == original_len {
        return Err(format!("Target '{}' not found", target_id));
    }
    if let Some(mut removed) = load_remote_targets(local_db)
        .await?
        .into_iter()
        .find(|target| target.id == target_id)
    {
        registry.delete_target_password(&mut removed)?;
    }

    save_remote_targets(local_db, &targets).await?;
    if active_target_id(local_db).await? == target_id {
        db::set_setting(local_db, ACTIVE_TARGET_SETTING_KEY, LOCAL_TARGET_ID).await?;
    }
    registry.drop_remote_pool(target_id);
    Ok(())
}

pub async fn set_active_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target_id: &str,
) -> Result<TargetSummary, String> {
    if target_id != LOCAL_TARGET_ID
        && !load_remote_targets(local_db)
            .await?
            .iter()
            .any(|target| target.id == target_id)
    {
        return Err(format!("Target '{}' not found", target_id));
    }

    db::set_setting(local_db, ACTIVE_TARGET_SETTING_KEY, target_id).await?;
    let targets = registry.list_targets(local_db).await?;
    targets
        .into_iter()
        .find(|target| target.id == target_id)
        .ok_or_else(|| format!("Target '{}' not found", target_id))
}

pub async fn get_active_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
) -> Result<TargetSummary, String> {
    let active_id = active_target_id(local_db).await?;
    registry
        .list_targets(local_db)
        .await?
        .into_iter()
        .find(|target| target.id == active_id)
        .ok_or_else(|| format!("Target '{}' not found", active_id))
}

pub async fn active_target_id(local_db: &DbPool) -> Result<String, String> {
    Ok(db::get_setting(local_db, ACTIVE_TARGET_SETTING_KEY)
        .await?
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| LOCAL_TARGET_ID.to_string()))
}

pub async fn load_remote_targets(local_db: &DbPool) -> Result<Vec<RemoteTargetConfig>, String> {
    let Some(raw) = db::get_setting(local_db, TARGETS_SETTING_KEY).await? else {
        return Ok(Vec::new());
    };
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&raw).map_err(|e| format!("Failed to parse remote targets: {}", e))
}

async fn save_remote_targets(
    local_db: &DbPool,
    targets: &[RemoteTargetConfig],
) -> Result<(), String> {
    let raw = serde_json::to_string(targets).map_err(|e| e.to_string())?;
    db::set_setting(local_db, TARGETS_SETTING_KEY, &raw).await
}

fn request_to_config(
    request: CreateSshTargetRequest,
    target_id: String,
) -> Result<RemoteTargetConfig, String> {
    let auth_method = request.auth_method.unwrap_or_default();
    if auth_method == SshAuthMethod::Key
        && request
            .passphrase
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    {
        return Err(
            "Passphrase-protected keys are not supported yet. Use ssh-agent or an unencrypted key."
                .to_string(),
        );
    }

    let key_path = match auth_method {
        SshAuthMethod::Key => required_field("keyPath", request.key_path.as_deref().unwrap_or(""))?,
        SshAuthMethod::Password => String::new(),
    };
    let password = match auth_method {
        SshAuthMethod::Key => None,
        SshAuthMethod::Password => Some(required_field(
            "password",
            request.password.as_deref().unwrap_or(""),
        )?),
    };
    let credential_key = match auth_method {
        SshAuthMethod::Key => None,
        SshAuthMethod::Password => Some(credential_key_for_target(&target_id)),
    };

    Ok(RemoteTargetConfig {
        id: target_id,
        label: required_field("label", &request.label)?,
        host: required_field("host", &request.host)?,
        username: required_field("username", &request.username)?,
        port: request.port.unwrap_or(22),
        auth_method,
        key_path,
        credential_key,
        protected_password: None,
        password,
        remote_home: String::new(),
        remote_os: String::new(),
        symlink_enabled: false,
    })
}

fn test_request_to_config(request: TestSshTargetRequest) -> Result<RemoteTargetConfig, String> {
    let auth_method = request.auth_method.unwrap_or_default();
    if auth_method == SshAuthMethod::Key
        && request
            .passphrase
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    {
        return Err(
            "Passphrase-protected keys are not supported yet. Use ssh-agent or an unencrypted key."
                .to_string(),
        );
    }

    let id = request.id.unwrap_or_else(|| "test".to_string());
    let key_path = match auth_method {
        SshAuthMethod::Key => required_field("keyPath", request.key_path.as_deref().unwrap_or(""))?,
        SshAuthMethod::Password => String::new(),
    };
    let password = match auth_method {
        SshAuthMethod::Key => None,
        SshAuthMethod::Password => Some(required_field(
            "password",
            request.password.as_deref().unwrap_or(""),
        )?),
    };
    let credential_key = match auth_method {
        SshAuthMethod::Key => None,
        SshAuthMethod::Password => Some(credential_key_for_target(&id)),
    };

    Ok(RemoteTargetConfig {
        id,
        label: required_field("label", request.label.as_deref().unwrap_or("SSH target"))?,
        host: required_field("host", request.host.as_deref().unwrap_or(""))?,
        username: required_field("username", request.username.as_deref().unwrap_or(""))?,
        port: request.port.unwrap_or(22),
        auth_method,
        key_path,
        credential_key,
        protected_password: None,
        password,
        remote_home: String::new(),
        remote_os: String::new(),
        symlink_enabled: false,
    })
}

fn required_field(name: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("{} is required.", name))
    } else {
        Ok(trimmed.to_string())
    }
}

fn apply_supplied_password_to_existing_target(
    target: &mut RemoteTargetConfig,
    password: Option<&str>,
) -> Result<bool, String> {
    if target.auth_method != SshAuthMethod::Password {
        return Ok(false);
    }

    let Some(password) = password.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(false);
    };
    if target.credential_key.is_none() {
        target.credential_key = Some(credential_key_for_target(&target.id));
    }
    target.password = Some(password.to_string());
    Ok(true)
}

pub fn remote_cache_db_path(target_id: &str) -> Result<PathBuf, String> {
    let target_id = sanitize_target_id(target_id)?;
    Ok(crate::paths::app_data_dir()
        .join("targets")
        .join(target_id)
        .join("db.sqlite"))
}

fn sanitize_target_id(target_id: &str) -> Result<String, String> {
    let trimmed = target_id.trim();
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err("Invalid target id.".to_string());
    }
    Ok(trimmed.to_string())
}

fn credential_key_for_target(target_id: &str) -> String {
    format!("{}:ssh-password", target_id)
}

fn credential_key_for_password_target(target: &RemoteTargetConfig) -> Option<String> {
    if target.auth_method != SshAuthMethod::Password {
        return None;
    }
    Some(
        target
            .credential_key
            .clone()
            .unwrap_or_else(|| credential_key_for_target(&target.id)),
    )
}

fn ssh_success_message(default_message: &str, state: Option<&TargetCredentialState>) -> String {
    match state.map(|state| state.status) {
        Some(TargetCredentialStatus::Session) => {
            "SSH password was verified for this session. The system credential store could not be read back, so enter it again after restarting SkillPort.".to_string()
        }
        _ => default_message.to_string(),
    }
}

fn load_target_password(target: &RemoteTargetConfig) -> Result<String, String> {
    if let Some(password) = target.password.as_deref().filter(|value| !value.is_empty()) {
        return Ok(password.to_string());
    }
    let Some(credential_key) = credential_key_for_password_target(target) else {
        return Err("Password target is missing its credential key.".to_string());
    };
    match SystemCredentialBackend.get_password(&credential_key) {
        Ok(password) => Ok(password),
        Err(CredentialStoreError::NoEntry) => protected_password_for_target(target)?.ok_or_else(|| {
            format!(
                "SSH password for target '{}' is not available. Open Settings, enter the password for this target, save it, and retry.",
                target.label
            )
        }),
        Err(error) => protected_password_for_target(target)?.ok_or_else(|| error.message()),
    }
}

struct SshProbe {
    remote_home: String,
    remote_os: String,
}

async fn probe_ssh_target(target: &RemoteTargetConfig) -> Result<SshProbe, String> {
    let connection = connect_ssh_target(target).await?;
    let output = connection
        .run_command("printf '%s\\n' \"$HOME\"; uname -s 2>/dev/null || printf '%s\\n' unknown")
        .await?;
    let mut lines = output.lines();
    let remote_home = lines
        .next()
        .map(str::trim)
        .filter(|line| line.starts_with('/'))
        .ok_or_else(|| "Remote HOME probe did not return an absolute POSIX path.".to_string())?
        .to_string();
    let remote_os = lines.next().map(str::trim).unwrap_or("unknown").to_string();

    connection.ensure_dir(&remote_home).await?;
    connection
        .mkdir_p(&remote_join(&remote_home, ".skillsmanage"))
        .await?;
    connection
        .mkdir_p(&remote_join(&remote_home, ".skillsmanage/skills"))
        .await?;

    Ok(SshProbe {
        remote_home,
        remote_os,
    })
}

fn is_supported_remote_os(remote_os: &str) -> bool {
    matches!(remote_os, "Linux" | "Darwin")
}

pub fn remote_symlink_allowed(target: &RemoteTargetConfig) -> bool {
    target.symlink_enabled || is_supported_remote_os(&target.remote_os)
}

#[derive(Debug, Clone)]
pub struct RemoteDirEntry {
    pub name: String,
    pub file_type: String,
    pub symlink_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePathInfo {
    pub file_type: String,
    pub symlink_target: Option<String>,
}

pub struct ConnectedSshTarget {
    target: RemoteTargetConfig,
    password: Option<String>,
    askpass_path: Option<PathBuf>,
}

pub async fn connect_ssh_target(target: &RemoteTargetConfig) -> Result<ConnectedSshTarget, String> {
    let password = match target.auth_method {
        SshAuthMethod::Key => None,
        SshAuthMethod::Password => Some(load_target_password(target)?),
    };
    let askpass_path = match password.as_deref() {
        Some(_) => Some(create_askpass_script()?),
        None => None,
    };
    let connection = ConnectedSshTarget {
        target: target.clone(),
        password,
        askpass_path,
    };
    connection
        .run_command("printf '%s' connected >/dev/null")
        .await?;
    Ok(connection)
}

fn create_askpass_script() -> Result<PathBuf, String> {
    let extension = if cfg!(windows) { "cmd" } else { "sh" };
    let path = env::temp_dir().join(format!(
        "skillport-ssh-askpass-{}.{}",
        Uuid::new_v4(),
        extension
    ));
    let content = if cfg!(windows) {
        "@echo off\r\n<nul set /p \"=%SKILLPORT_SSH_PASSWORD%\"\r\nexit /b 0\r\n"
    } else {
        "#!/bin/sh\nprintf '%s' \"$SKILLPORT_SSH_PASSWORD\"\n"
    };
    fs::write(&path, content).map_err(|e| {
        format!(
            "Failed to create SSH askpass helper '{}': {}",
            path.display(),
            e
        )
    })?;
    set_askpass_permissions(&path)?;
    Ok(path)
}

#[cfg(unix)]
fn set_askpass_permissions(path: &PathBuf) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|e| {
            format!(
                "Failed to inspect askpass helper '{}': {}",
                path.display(),
                e
            )
        })?
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions).map_err(|e| {
        format!(
            "Failed to set askpass helper permissions '{}': {}",
            path.display(),
            e
        )
    })
}

#[cfg(not(unix))]
fn set_askpass_permissions(_path: &PathBuf) -> Result<(), String> {
    Ok(())
}

impl Drop for ConnectedSshTarget {
    fn drop(&mut self) {
        if let Some(path) = &self.askpass_path {
            let _ = fs::remove_file(path);
        }
    }
}

fn ssh_program() -> std::ffi::OsString {
    #[cfg(windows)]
    {
        if let Ok(windir) = env::var("WINDIR") {
            let candidate = PathBuf::from(windir)
                .join("System32")
                .join("OpenSSH")
                .join("ssh.exe");
            if candidate.is_file() {
                return candidate.into_os_string();
            }
        }
    }
    "ssh".into()
}

impl ConnectedSshTarget {
    fn base_command(&self) -> Command {
        let mut command = Command::new(ssh_program());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        command
            .arg("-p")
            .arg(self.target.port.to_string())
            .arg("-o")
            .arg("StrictHostKeyChecking=accept-new");

        match self.target.auth_method {
            SshAuthMethod::Key => {
                command
                    .arg("-i")
                    .arg(&self.target.key_path)
                    .arg("-o")
                    .arg("BatchMode=yes")
                    .arg("-o")
                    .arg("PreferredAuthentications=publickey");
            }
            SshAuthMethod::Password => {
                command
                    .arg("-o")
                    .arg("BatchMode=no")
                    .arg("-o")
                    .arg("PreferredAuthentications=password,keyboard-interactive")
                    .arg("-o")
                    .arg("PubkeyAuthentication=no")
                    .arg("-o")
                    .arg("NumberOfPasswordPrompts=1");
                if let (Some(path), Some(password)) = (&self.askpass_path, &self.password) {
                    command
                        .env("SSH_ASKPASS", path)
                        .env("SSH_ASKPASS_REQUIRE", "force")
                        .env("DISPLAY", "skillport")
                        .env("SKILLPORT_SSH_PASSWORD", password);
                }
            }
        }
        command.arg(format!("{}@{}", self.target.username, self.target.host));
        command
    }

    fn remote_command_error(&self, status: ExitStatus, stderr: &[u8]) -> String {
        format!(
            "Remote command failed with status {}: {}",
            status,
            self.ssh_failure_detail(stderr)
        )
    }

    fn ssh_failure_detail(&self, stderr: &[u8]) -> String {
        let detail = String::from_utf8_lossy(stderr);
        let detail = detail.trim();
        if let Some(message) = self.ssh_auth_failure_message(detail) {
            if detail.is_empty() {
                message
            } else {
                format!("{} Raw ssh error: {}", message, detail)
            }
        } else if detail.is_empty() {
            "ssh exited without stderr.".to_string()
        } else {
            detail.to_string()
        }
    }

    fn ssh_auth_failure_message(&self, stderr: &str) -> Option<String> {
        if !stderr.to_ascii_lowercase().contains("permission denied") {
            return None;
        }

        let destination = format!("{}@{}", self.target.username, self.target.host);
        let message = match self.target.auth_method {
            SshAuthMethod::Key => {
                let key_hint = self.target.key_path.trim();
                if key_hint.is_empty() {
                    format!(
                        "SSH authentication failed for {} using key authentication. Check that the target uses the correct username and port, and configure a private key path for this Windows machine.",
                        destination
                    )
                } else {
                    format!(
                        "SSH authentication failed for {} using key authentication with '{}'. Check that the private key path is correct on this Windows machine and that the matching public key is installed in the remote user's ~/.ssh/authorized_keys.",
                        destination, key_hint
                    )
                }
            }
            SshAuthMethod::Password => format!(
                "SSH authentication failed for {} using password authentication. Check that the password is correct and that the remote sshd allows password or keyboard-interactive login for this user.",
                destination
            ),
        };
        Some(message)
    }

    pub async fn run_script(&self, script: &str, args: &[&str]) -> Result<String, String> {
        let output =
            self.run_command_with_stdin(&remote_script_command(args), script.as_bytes())?;
        String::from_utf8(output).map_err(|e| format!("Remote stdout is not valid UTF-8: {}", e))
    }

    pub async fn run_command(&self, command: &str) -> Result<String, String> {
        let output = self
            .base_command()
            .arg(command)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("Failed to start ssh: {}", e))?;
        if output.status.success() {
            String::from_utf8(output.stdout)
                .map_err(|e| format!("Remote stdout is not valid UTF-8: {}", e))
        } else {
            Err(self.remote_command_error(output.status, &output.stderr))
        }
    }

    fn run_command_with_stdin(&self, command: &str, stdin: &[u8]) -> Result<Vec<u8>, String> {
        let mut child = self
            .base_command()
            .arg(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start ssh: {}", e))?;

        if let Some(mut child_stdin) = child.stdin.take() {
            child_stdin
                .write_all(stdin)
                .map_err(|e| format!("Failed to write ssh stdin: {}", e))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for ssh: {}", e))?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(self.remote_command_error(output.status, &output.stderr))
        }
    }

    fn run_command_bytes(&self, command: &str) -> Result<Vec<u8>, String> {
        let output = self
            .base_command()
            .arg(command)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("Failed to start ssh: {}", e))?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(self.remote_command_error(output.status, &output.stderr))
        }
    }

    pub async fn ensure_dir(&self, path: &str) -> Result<(), String> {
        if self.exists(path).await? {
            return Ok(());
        }
        Err(format!("Remote path '{}' does not exist.", path))
    }

    pub async fn exists(&self, path: &str) -> Result<bool, String> {
        let command = format!("test -e {}", shell_quote(path));
        let output = self
            .base_command()
            .arg(command)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("Failed to start ssh: {}", e))?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(format!(
                "Failed to inspect remote path '{}': {}",
                path,
                self.ssh_failure_detail(&output.stderr)
            )),
        }
    }

    pub async fn inspect_path(&self, path: &str) -> Result<Option<RemotePathInfo>, String> {
        let command = format!(
            r#"p={path}; if [ -L "$p" ]; then printf 'symlink\t%s\n' "$(readlink "$p" || true)"; elif [ -d "$p" ]; then printf 'dir\t\n'; elif [ -f "$p" ]; then printf 'file\t\n'; elif [ -e "$p" ]; then printf 'other\t\n'; else exit 1; fi"#,
            path = shell_quote(path)
        );
        let output = self
            .base_command()
            .arg(command)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("Failed to start ssh: {}", e))?;
        match output.status.code() {
            Some(0) => {
                let stdout = String::from_utf8(output.stdout)
                    .map_err(|e| format!("Remote stdout is not valid UTF-8: {}", e))?;
                let mut parts = stdout.trim_end().splitn(2, '\t');
                let file_type = parts.next().unwrap_or("other").to_string();
                let symlink_target = parts
                    .next()
                    .map(str::to_string)
                    .filter(|value| !value.is_empty());
                Ok(Some(RemotePathInfo {
                    file_type,
                    symlink_target,
                }))
            }
            Some(1) => Ok(None),
            _ => Err(format!(
                "Failed to inspect remote path '{}': {}",
                path,
                self.ssh_failure_detail(&output.stderr)
            )),
        }
    }

    pub async fn mkdir_p(&self, path: &str) -> Result<(), String> {
        self.run_command(&format!("mkdir -p {}", shell_quote(path)))
            .await
            .map(|_| ())
    }

    pub async fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
        if let Some(parent) = remote_parent(path) {
            self.mkdir_p(&parent).await?;
        }
        let command = format!("cat > {}", shell_quote(path));
        self.run_command_with_stdin(&command, bytes).map(|_| ())
    }

    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        self.run_command_bytes(&format!("cat {}", shell_quote(path)))
    }

    pub async fn copy_dir(&self, source: &str, target: &str) -> Result<(), String> {
        let command = format!(
            "mkdir -p {target} && cp -R {source}/. {target}/",
            source = shell_quote(source),
            target = shell_quote(target)
        );
        self.run_command(&command).await.map(|_| ())
    }

    pub async fn remove_tree(&self, path: &str) -> Result<(), String> {
        self.run_command(&format!("rm -rf -- {}", shell_quote(path)))
            .await
            .map(|_| ())
    }

    pub async fn remove_file(&self, path: &str) -> Result<(), String> {
        self.run_command(&format!("rm -f -- {}", shell_quote(path)))
            .await
            .map(|_| ())
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<RemoteDirEntry>, String> {
        let command = format!(
            r#"d={dir}; for p in "$d"/* "$d"/.[!.]* "$d"/..?*; do [ -e "$p" ] || continue; name=${{p##*/}}; link=""; if [ -L "$p" ]; then kind="symlink"; link=$(readlink "$p" || true); elif [ -d "$p" ]; then kind="dir"; elif [ -f "$p" ]; then kind="file"; else kind="other"; fi; printf '%s\t%s\t%s\n' "$name" "$kind" "$link"; done"#,
            dir = shell_quote(path)
        );
        let output = self.run_command(&command).await?;
        Ok(output
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, '\t');
                let name = parts.next()?.to_string();
                let file_type = parts.next().unwrap_or("other").to_string();
                let symlink_target = parts
                    .next()
                    .map(str::to_string)
                    .filter(|value| !value.is_empty());
                Some(RemoteDirEntry {
                    name,
                    file_type,
                    symlink_target,
                })
            })
            .collect())
    }
}

fn remote_script_command(args: &[&str]) -> String {
    let mut command = "sh -s --".to_string();
    for arg in args {
        command.push(' ');
        command.push_str(&shell_quote(arg));
    }
    command
}

pub fn remote_join(parent: &str, child: &str) -> String {
    let parent = parent.trim_end_matches('/');
    let child = child.trim_start_matches('/');
    if parent.is_empty() || parent == "/" {
        format!("/{}", child)
    } else if child.is_empty() {
        parent.to_string()
    } else {
        format!("{}/{}", parent, child)
    }
}

pub fn remote_parent(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('/');
    let index = trimmed.rfind('/')?;
    if index == 0 {
        Some("/".to_string())
    } else {
        Some(trimmed[..index].to_string())
    }
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn remote_file_type_is_dir(debug_value: &str) -> bool {
    matches!(debug_value, "dir" | "Directory") || debug_value.contains("Directory")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use std::path::Path;

    #[derive(Default)]
    struct MemoryCredentialBackend {
        passwords: Mutex<HashMap<String, String>>,
        set_error: Mutex<Option<CredentialStoreError>>,
        get_error: Mutex<Option<CredentialStoreError>>,
        delete_error: Mutex<Option<CredentialStoreError>>,
    }

    impl MemoryCredentialBackend {
        fn with_get_error(error: CredentialStoreError) -> Arc<Self> {
            Arc::new(Self {
                get_error: Mutex::new(Some(error)),
                ..Self::default()
            })
        }

        fn with_set_error(error: CredentialStoreError) -> Arc<Self> {
            Arc::new(Self {
                set_error: Mutex::new(Some(error)),
                ..Self::default()
            })
        }
    }

    impl CredentialBackend for MemoryCredentialBackend {
        fn set_password(
            &self,
            credential_key: &str,
            password: &str,
        ) -> Result<(), CredentialStoreError> {
            if let Some(error) = self.set_error.lock().unwrap().clone() {
                return Err(error);
            }
            self.passwords
                .lock()
                .unwrap()
                .insert(credential_key.to_string(), password.to_string());
            Ok(())
        }

        fn get_password(&self, credential_key: &str) -> Result<String, CredentialStoreError> {
            if let Some(error) = self.get_error.lock().unwrap().clone() {
                return Err(error);
            }
            self.passwords
                .lock()
                .unwrap()
                .get(credential_key)
                .cloned()
                .ok_or(CredentialStoreError::NoEntry)
        }

        fn delete_credential(&self, credential_key: &str) -> Result<(), CredentialStoreError> {
            if let Some(error) = self.delete_error.lock().unwrap().clone() {
                return Err(error);
            }
            self.passwords.lock().unwrap().remove(credential_key);
            Ok(())
        }
    }

    async fn memory_db() -> DbPool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        db::init_database(&pool).await.unwrap();
        pool
    }

    fn password_target() -> RemoteTargetConfig {
        RemoteTargetConfig {
            id: "ssh-demo".to_string(),
            label: "Lab".to_string(),
            host: "lab.local".to_string(),
            username: "alice".to_string(),
            port: 22,
            auth_method: SshAuthMethod::Password,
            key_path: String::new(),
            credential_key: Some("ssh-demo:ssh-password".to_string()),
            protected_password: None,
            password: Some("secret".to_string()),
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: false,
        }
    }

    fn command_arg_strings(command: Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    fn ssh_connection_for(target: RemoteTargetConfig) -> ConnectedSshTarget {
        ConnectedSshTarget {
            password: target.password.clone(),
            target,
            askpass_path: None,
        }
    }

    #[tokio::test]
    async fn active_target_defaults_to_local() {
        let pool = memory_db().await;
        assert_eq!(active_target_id(&pool).await.unwrap(), LOCAL_TARGET_ID);
    }

    #[test]
    fn remote_cache_path_is_scoped_by_target_id() {
        let path = remote_cache_db_path("ssh-demo_1").unwrap();
        assert!(path.ends_with(Path::new("targets").join("ssh-demo_1").join("db.sqlite")));
    }

    #[test]
    fn remote_join_uses_posix_separators() {
        assert_eq!(
            remote_join("/home/alice", ".skillsmanage/skills"),
            "/home/alice/.skillsmanage/skills"
        );
    }

    #[test]
    fn remote_script_command_runs_script_via_stdin_with_quoted_args() {
        let command = remote_script_command(&["/tmp/demo path", "plain"]);

        assert_eq!(command, "sh -s -- '/tmp/demo path' 'plain'");
    }

    #[test]
    fn askpass_helper_avoids_powershell_on_windows() {
        let path = create_askpass_script().expect("askpass");
        let content = fs::read_to_string(&path).expect("read askpass");
        let _ = fs::remove_file(&path);

        assert!(!content.to_ascii_lowercase().contains("powershell"));
        if cfg!(windows) {
            assert!(content.contains("%SKILLPORT_SSH_PASSWORD%"));
            assert!(content.contains("<nul set /p"));
            assert!(content.contains("exit /b 0"));
        } else {
            assert!(content.contains("$SKILLPORT_SSH_PASSWORD"));
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_askpass_helper_outputs_password_with_success_status() {
        let path = create_askpass_script().expect("askpass");
        let output = Command::new("cmd")
            .arg("/C")
            .arg(&path)
            .env("SKILLPORT_SSH_PASSWORD", "pa&ss")
            .output()
            .expect("run askpass");
        let _ = fs::remove_file(&path);

        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "pa&ss");
    }

    #[test]
    fn ssh_base_command_places_key_options_before_destination() {
        let mut target = password_target();
        target.auth_method = SshAuthMethod::Key;
        target.key_path = "C:\\Users\\alice\\.ssh\\id_ed25519".to_string();
        target.credential_key = None;
        target.password = None;
        let args = command_arg_strings(ssh_connection_for(target).base_command());
        let destination = args
            .iter()
            .position(|arg| arg == "alice@lab.local")
            .expect("destination");
        let key_option = args.iter().position(|arg| arg == "-i").expect("key option");
        let key_path = args
            .iter()
            .position(|arg| arg == "C:\\Users\\alice\\.ssh\\id_ed25519")
            .expect("key path");
        let batch_mode = args
            .iter()
            .position(|arg| arg == "BatchMode=yes")
            .expect("batch mode");
        let preferred = args
            .iter()
            .position(|arg| arg == "PreferredAuthentications=publickey")
            .expect("preferred auth");

        assert!(key_option < destination);
        assert!(key_path < destination);
        assert!(batch_mode < destination);
        assert!(preferred < destination);
        assert_eq!(args.last().map(String::as_str), Some("alice@lab.local"));
    }

    #[test]
    fn ssh_base_command_places_password_options_before_destination() {
        let args = command_arg_strings(ssh_connection_for(password_target()).base_command());
        let destination = args
            .iter()
            .position(|arg| arg == "alice@lab.local")
            .expect("destination");
        let batch_mode = args
            .iter()
            .position(|arg| arg == "BatchMode=no")
            .expect("batch mode");
        let preferred = args
            .iter()
            .position(|arg| arg == "PreferredAuthentications=password,keyboard-interactive")
            .expect("preferred auth");
        let pubkey_disabled = args
            .iter()
            .position(|arg| arg == "PubkeyAuthentication=no")
            .expect("pubkey disabled");
        let prompt_count = args
            .iter()
            .position(|arg| arg == "NumberOfPasswordPrompts=1")
            .expect("prompt count");

        assert!(batch_mode < destination);
        assert!(preferred < destination);
        assert!(pubkey_disabled < destination);
        assert!(prompt_count < destination);
        assert_eq!(args.last().map(String::as_str), Some("alice@lab.local"));
    }

    #[test]
    fn ssh_auth_failure_detail_explains_password_denied() {
        let connection = ssh_connection_for(password_target());
        let detail = connection
            .ssh_failure_detail(b"alice@lab.local: Permission denied (publickey,password).\n");

        assert!(detail.contains("using password authentication"));
        assert!(detail.contains("remote sshd allows password"));
        assert!(detail.contains("Raw ssh error"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_ssh_hidden_window_flag_matches_create_no_window() {
        assert_eq!(CREATE_NO_WINDOW, 0x08000000);
    }

    #[cfg(windows)]
    #[test]
    fn protected_password_roundtrips_with_windows_dpapi() {
        let protected = protected_credentials::protect("secret").unwrap();

        assert_ne!(protected, "secret");
        assert_eq!(
            protected_credentials::unprotect(&protected).unwrap(),
            "secret"
        );
    }

    #[test]
    fn key_target_requires_key_path() {
        let result = request_to_config(
            CreateSshTargetRequest {
                label: "Lab".to_string(),
                host: "lab.local".to_string(),
                username: "alice".to_string(),
                port: Some(22),
                auth_method: Some(SshAuthMethod::Key),
                key_path: None,
                password: None,
                passphrase: None,
            },
            "ssh-test".to_string(),
        );

        assert!(result.unwrap_err().contains("keyPath"));
    }

    #[test]
    fn password_target_uses_credential_key_without_key_path() {
        let target = request_to_config(
            CreateSshTargetRequest {
                label: "Lab".to_string(),
                host: "lab.local".to_string(),
                username: "alice".to_string(),
                port: Some(22),
                auth_method: Some(SshAuthMethod::Password),
                key_path: None,
                password: Some("secret".to_string()),
                passphrase: None,
            },
            "ssh-test".to_string(),
        )
        .unwrap();

        assert_eq!(target.auth_method, SshAuthMethod::Password);
        assert!(target.key_path.is_empty());
        assert_eq!(
            target.credential_key.as_deref(),
            Some("ssh-test:ssh-password")
        );
        assert_eq!(target.password.as_deref(), Some("secret"));
    }

    #[test]
    fn supported_remote_os_allows_symlink_for_legacy_targets() {
        let mut target = password_target();
        target.remote_os = "Linux".to_string();
        target.symlink_enabled = false;

        assert!(remote_symlink_allowed(&target));
    }

    #[test]
    fn unsupported_remote_os_keeps_symlink_disabled_without_override() {
        let mut target = password_target();
        target.remote_os = "unknown".to_string();
        target.symlink_enabled = false;

        assert!(!remote_symlink_allowed(&target));
    }

    #[test]
    fn password_override_repairs_existing_password_target() {
        let mut target = RemoteTargetConfig {
            id: "ssh-demo".to_string(),
            label: "Lab".to_string(),
            host: "lab.local".to_string(),
            username: "alice".to_string(),
            port: 22,
            auth_method: SshAuthMethod::Password,
            key_path: String::new(),
            credential_key: None,
            protected_password: None,
            password: None,
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: false,
        };

        let changed =
            apply_supplied_password_to_existing_target(&mut target, Some("secret")).unwrap();

        assert!(changed);
        assert_eq!(target.password.as_deref(), Some("secret"));
        assert_eq!(
            target.credential_key.as_deref(),
            Some("ssh-demo:ssh-password")
        );
    }

    #[test]
    fn password_override_ignores_key_targets() {
        let mut target = RemoteTargetConfig {
            id: "ssh-demo".to_string(),
            label: "Lab".to_string(),
            host: "lab.local".to_string(),
            username: "alice".to_string(),
            port: 22,
            auth_method: SshAuthMethod::Key,
            key_path: "~/.ssh/id_ed25519".to_string(),
            credential_key: None,
            protected_password: None,
            password: None,
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: false,
        };

        let changed =
            apply_supplied_password_to_existing_target(&mut target, Some("secret")).unwrap();

        assert!(!changed);
        assert!(target.password.is_none());
    }

    #[test]
    fn credential_save_reports_stored_after_readback() {
        let backend = Arc::new(MemoryCredentialBackend::default());
        let registry = TargetRegistry::with_credential_backend(backend);
        let mut target = password_target();

        let state = registry.save_target_password(&mut target).unwrap();
        let summary = registry.target_summary(&target, LOCAL_TARGET_ID);

        assert_eq!(state.status, TargetCredentialStatus::Stored);
        assert_eq!(
            summary.credential_status,
            Some(TargetCredentialStatus::Stored)
        );
        assert_eq!(summary.has_stored_password, Some(true));
    }

    #[test]
    fn credential_save_remains_available_when_readback_fails() {
        let backend = MemoryCredentialBackend::with_get_error(CredentialStoreError::Other(
            "credential vault locked".to_string(),
        ));
        let registry = TargetRegistry::with_credential_backend(backend);
        let mut target = password_target();

        let state = registry.save_target_password(&mut target).unwrap();
        let mut runtime_target = RemoteTargetConfig {
            password: None,
            ..target.clone()
        };
        registry.attach_available_password(&mut runtime_target);
        let summary = registry.target_summary(&target, LOCAL_TARGET_ID);

        #[cfg(windows)]
        assert_eq!(state.status, TargetCredentialStatus::Stored);
        #[cfg(not(windows))]
        assert_eq!(state.status, TargetCredentialStatus::Session);
        assert_eq!(runtime_target.password.as_deref(), Some("secret"));
        #[cfg(windows)]
        assert_eq!(
            summary.credential_status,
            Some(TargetCredentialStatus::Stored)
        );
        #[cfg(not(windows))]
        assert_eq!(
            summary.credential_status,
            Some(TargetCredentialStatus::Session)
        );
        assert_eq!(summary.has_stored_password, Some(true));
    }

    #[test]
    fn credential_save_remains_available_when_write_fails() {
        let backend = MemoryCredentialBackend::with_set_error(CredentialStoreError::Other(
            "credential vault denied write".to_string(),
        ));
        let registry = TargetRegistry::with_credential_backend(backend);
        let mut target = password_target();

        let state = registry.save_target_password(&mut target).unwrap();
        let mut runtime_target = RemoteTargetConfig {
            password: None,
            ..target.clone()
        };
        registry.attach_available_password(&mut runtime_target);

        #[cfg(windows)]
        assert_eq!(state.status, TargetCredentialStatus::Stored);
        #[cfg(not(windows))]
        assert_eq!(state.status, TargetCredentialStatus::Session);
        assert_eq!(runtime_target.password.as_deref(), Some("secret"));
    }

    #[tokio::test]
    async fn active_target_attaches_available_password() {
        let backend = MemoryCredentialBackend::with_get_error(CredentialStoreError::NoEntry);
        let registry = TargetRegistry::with_credential_backend(backend);
        let pool = memory_db().await;
        let mut target = password_target();
        registry.save_target_password(&mut target).unwrap();
        target.password = None;
        save_remote_targets(&pool, &[target]).await.unwrap();
        db::set_setting(&pool, ACTIVE_TARGET_SETTING_KEY, "ssh-demo")
            .await
            .unwrap();

        let active = registry.active_target(&pool).await.unwrap();

        match active {
            ActiveTarget::Ssh(target) => {
                assert_eq!(target.password.as_deref(), Some("secret"));
            }
            ActiveTarget::Local => panic!("expected ssh target"),
        }
    }

    #[test]
    fn credential_summary_reports_missing_and_unreadable() {
        let missing =
            TargetRegistry::with_credential_backend(Arc::new(MemoryCredentialBackend::default()));
        let unreadable =
            TargetRegistry::with_credential_backend(MemoryCredentialBackend::with_get_error(
                CredentialStoreError::Other("credential vault unavailable".to_string()),
            ));
        let mut target = password_target();
        target.password = None;

        let missing_summary = missing.target_summary(&target, LOCAL_TARGET_ID);
        let unreadable_summary = unreadable.target_summary(&target, LOCAL_TARGET_ID);

        assert_eq!(
            missing_summary.credential_status,
            Some(TargetCredentialStatus::Missing)
        );
        assert_eq!(missing_summary.has_stored_password, Some(false));
        assert_eq!(
            unreadable_summary.credential_status,
            Some(TargetCredentialStatus::Unreadable)
        );
        assert_eq!(unreadable_summary.has_stored_password, Some(false));
        assert!(unreadable_summary.credential_error.is_some());
    }

    #[test]
    fn deleting_target_password_clears_session_cache() {
        let backend = MemoryCredentialBackend::with_get_error(CredentialStoreError::NoEntry);
        let registry = TargetRegistry::with_credential_backend(backend);
        let mut target = password_target();

        registry.save_target_password(&mut target).unwrap();
        #[cfg(windows)]
        assert_eq!(
            registry
                .target_summary(&target, LOCAL_TARGET_ID)
                .credential_status,
            Some(TargetCredentialStatus::Stored)
        );
        #[cfg(not(windows))]
        assert_eq!(
            registry
                .target_summary(&target, LOCAL_TARGET_ID)
                .credential_status,
            Some(TargetCredentialStatus::Session)
        );

        registry.delete_target_password(&mut target).unwrap();

        assert_eq!(
            registry
                .target_summary(&target, LOCAL_TARGET_ID)
                .credential_status,
            Some(TargetCredentialStatus::Missing)
        );
    }

    #[test]
    fn key_auth_targets_do_not_report_password_credential_state() {
        let registry =
            TargetRegistry::with_credential_backend(Arc::new(MemoryCredentialBackend::default()));
        let mut target = password_target();
        target.auth_method = SshAuthMethod::Key;
        target.key_path = "~/.ssh/id_ed25519".to_string();
        target.credential_key = None;
        target.password = None;

        let summary = registry.target_summary(&target, LOCAL_TARGET_ID);

        assert_eq!(summary.credential_status, None);
        assert_eq!(summary.has_stored_password, None);
        assert_eq!(summary.credential_error, None);
    }
}
