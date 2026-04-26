use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use uuid::Uuid;

use crate::db::{self, DbPool};

const TARGETS_SETTING_KEY: &str = "ssh_targets_v1";
const ACTIVE_TARGET_SETTING_KEY: &str = "active_target_id_v1";
const SSH_PASSWORD_SERVICE: &str = "SkillPort SSH Targets";
pub const LOCAL_TARGET_ID: &str = "local";

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
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTargetTestResult {
    pub ok: bool,
    pub remote_home: Option<String>,
    pub remote_os: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum ActiveTarget {
    Local,
    Ssh(Box<RemoteTargetConfig>),
}

#[derive(Default)]
pub struct TargetRegistry {
    pools: Mutex<HashMap<String, DbPool>>,
}

impl TargetRegistry {
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
            is_active: active_id == LOCAL_TARGET_ID,
        }];

        for target in load_remote_targets(local_db).await? {
            targets.push(target_summary(&target, active_id.as_str()));
        }

        Ok(targets)
    }

    pub async fn active_target(&self, local_db: &DbPool) -> Result<ActiveTarget, String> {
        let active_id = active_target_id(local_db).await?;
        if active_id == LOCAL_TARGET_ID {
            return Ok(ActiveTarget::Local);
        }

        load_remote_targets(local_db)
            .await?
            .into_iter()
            .find(|target| target.id == active_id)
            .map(Box::new)
            .map(ActiveTarget::Ssh)
            .ok_or_else(|| {
                format!(
                    "Active target '{}' no longer exists. Switch back to Local.",
                    active_id
                )
            })
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
    if target.auth_method == SshAuthMethod::Password {
        store_target_password(&target)?;
    }

    let mut stored_target = target.clone();
    stored_target.password = None;
    targets.push(stored_target.clone());
    if let Err(error) = save_remote_targets(local_db, &targets).await {
        let _ = delete_target_password(&stored_target);
        return Err(error);
    }
    registry.remote_db(&stored_target).await?;

    let active_id = active_target_id(local_db).await?;
    Ok(target_summary(&stored_target, active_id.as_str()))
}

pub async fn test_ssh_target_impl(
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
            if should_store_password {
                store_target_password(&target)?;
            }
            Ok(SshTargetTestResult {
                ok: true,
                remote_home: Some(probe.remote_home),
                remote_os: Some(probe.remote_os),
                message: "SSH connection is available.".to_string(),
            })
        }
        Ok(probe) => Ok(SshTargetTestResult {
            ok: false,
            remote_home: Some(probe.remote_home),
            remote_os: Some(probe.remote_os.clone()),
            message: format!(
                "Remote OS '{}' is not supported in this version. Linux and macOS are supported.",
                probe.remote_os
            ),
        }),
        Err(error) => Ok(SshTargetTestResult {
            ok: false,
            remote_home: None,
            remote_os: None,
            message: error,
        }),
    }
}

pub async fn update_ssh_target_password_impl(
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
            store_target_password(target)?;
            target.password = None;
            save_remote_targets(local_db, &targets).await?;

            Ok(SshTargetTestResult {
                ok: true,
                remote_home: Some(probe.remote_home),
                remote_os: Some(probe.remote_os),
                message: "SSH password was verified and saved.".to_string(),
            })
        }
        Ok(probe) => Ok(SshTargetTestResult {
            ok: false,
            remote_home: Some(probe.remote_home),
            remote_os: Some(probe.remote_os.clone()),
            message: format!(
                "Remote OS '{}' is not supported in this version. Linux and macOS are supported.",
                probe.remote_os
            ),
        }),
        Err(error) => Ok(SshTargetTestResult {
            ok: false,
            remote_home: None,
            remote_os: None,
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
    if let Some(removed) = load_remote_targets(local_db)
        .await?
        .into_iter()
        .find(|target| target.id == target_id)
    {
        delete_target_password(&removed)?;
    }

    save_remote_targets(local_db, &targets).await?;
    if active_target_id(local_db).await? == target_id {
        db::set_setting(local_db, ACTIVE_TARGET_SETTING_KEY, LOCAL_TARGET_ID).await?;
    }
    registry.drop_remote_pool(target_id);
    Ok(())
}

pub async fn set_active_target_impl(
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
    let targets = TargetRegistry::default().list_targets(local_db).await?;
    targets
        .into_iter()
        .find(|target| target.id == target_id)
        .ok_or_else(|| format!("Target '{}' not found", target_id))
}

pub async fn get_active_target_impl(local_db: &DbPool) -> Result<TargetSummary, String> {
    let active_id = active_target_id(local_db).await?;
    TargetRegistry::default()
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

fn target_summary(target: &RemoteTargetConfig, active_id: &str) -> TargetSummary {
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
        is_active: target.id == active_id,
    }
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

fn credential_entry(credential_key: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SSH_PASSWORD_SERVICE, credential_key)
        .map_err(|e| format!("Failed to access system credential store: {}", e))
}

fn store_target_password(target: &RemoteTargetConfig) -> Result<(), String> {
    let Some(credential_key) = target.credential_key.as_deref() else {
        return Err("Password target is missing its credential key.".to_string());
    };
    let password = target
        .password
        .as_deref()
        .ok_or_else(|| "Password is required for password authentication.".to_string())?;
    credential_entry(credential_key)?
        .set_password(password)
        .map_err(|e| {
            format!(
                "Failed to store SSH password in system credential store: {}",
                e
            )
        })
}

fn load_target_password(target: &RemoteTargetConfig) -> Result<String, String> {
    if let Some(password) = target.password.as_deref().filter(|value| !value.is_empty()) {
        return Ok(password.to_string());
    }
    let Some(credential_key) = target.credential_key.as_deref() else {
        return Err("Password target is missing its credential key.".to_string());
    };
    match credential_entry(credential_key)?.get_password() {
        Ok(password) => Ok(password),
        Err(keyring::Error::NoEntry) => Err(format!(
            "SSH password for target '{}' is not available. Open Settings, enter the password for this target, save it, and retry.",
            target.label
        )),
        Err(e) => Err(format!(
            "Failed to read SSH password from system credential store: {}",
            e
        )),
    }
}

fn delete_target_password(target: &RemoteTargetConfig) -> Result<(), String> {
    if target.auth_method != SshAuthMethod::Password {
        return Ok(());
    }
    let Some(credential_key) = target.credential_key.as_deref() else {
        return Ok(());
    };
    match credential_entry(credential_key)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!(
            "Failed to delete SSH password from system credential store: {}",
            e
        )),
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

#[derive(Debug, Clone)]
pub struct RemoteDirEntry {
    pub name: String,
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
        "@echo off\r\npowershell -NoLogo -NoProfile -Command \"[Console]::Out.Write($env:SKILLPORT_SSH_PASSWORD)\"\r\n"
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

impl ConnectedSshTarget {
    fn base_command(&self) -> Command {
        let mut command = Command::new("ssh");
        command
            .arg("-p")
            .arg(self.target.port.to_string())
            .arg("-o")
            .arg("StrictHostKeyChecking=accept-new")
            .arg(format!("{}@{}", self.target.username, self.target.host));

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
        command
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
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "Remote command failed with status {}: {}",
                output.status,
                stderr.trim()
            ))
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
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "Remote command failed with status {}: {}",
                output.status,
                stderr.trim()
            ))
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
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "Remote command failed with status {}: {}",
                output.status,
                stderr.trim()
            ))
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
            _ => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!(
                    "Failed to inspect remote path '{}': {}",
                    path,
                    stderr.trim()
                ))
            }
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

    async fn memory_db() -> DbPool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        db::init_database(&pool).await.unwrap();
        pool
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
}
