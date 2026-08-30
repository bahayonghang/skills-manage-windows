use super::*;
pub(super) const TARGETS_SETTING_KEY: &str = "ssh_targets_v1";
pub(super) const WSL_TARGETS_SETTING_KEY: &str = "wsl_targets_v1";
pub(super) const ACTIVE_TARGET_SETTING_KEY: &str = "active_target_id_v1";
pub(super) const SSH_PASSWORD_SERVICE: &str = "SkillPort SSH Targets";
pub(super) const SSH_ASKPASS_HELPER_ENV: &str = "SKILLPORT_SSH_ASKPASS_HELPER";
pub(super) const SSH_PASSWORD_ENV: &str = "SKILLPORT_SSH_PASSWORD";
pub const LOCAL_TARGET_ID: &str = "local";
pub(super) const TARGET_CONFIG_QUARANTINE_SETTING_KEY: &str = "target_config_quarantine_v1";
#[cfg(windows)]
pub(super) const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(all(windows, test))]
pub(super) static LAST_HIDDEN_CHILD_CREATION_FLAGS: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0);

#[cfg(windows)]
pub(super) fn hidden_child_creation_flags() -> u32 {
    CREATE_NO_WINDOW
}

#[cfg(windows)]
pub(super) fn hide_child_window(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    let flags = hidden_child_creation_flags();
    command.creation_flags(flags);
    #[cfg(test)]
    LAST_HIDDEN_CHILD_CREATION_FLAGS.store(flags, std::sync::atomic::Ordering::SeqCst);
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Local,
    Ssh,
    Wsl,
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
    pub(super) fn is_available(self) -> bool {
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
pub struct WslTargetConfig {
    pub id: String,
    pub label: String,
    pub distribution: String,
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
pub struct UpdateSshTargetRequest {
    pub id: String,
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
pub struct CreateWslTargetRequest {
    pub label: String,
    pub distribution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWslTargetRequest {
    pub id: String,
    pub label: String,
    pub distribution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestWslTargetRequest {
    pub id: Option<String>,
    pub label: Option<String>,
    pub distribution: Option<String>,
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
    pub distribution: Option<String>,
    pub auth_method: Option<SshAuthMethod>,
    pub key_path: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WslTargetTestResult {
    pub ok: bool,
    pub remote_home: Option<String>,
    pub remote_os: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WslDistributionSummary {
    pub name: String,
    pub is_default: bool,
    pub state: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ActiveTarget {
    Local,
    Ssh(Box<RemoteTargetConfig>),
    Wsl(Box<WslTargetConfig>),
}

impl ActiveTarget {
    pub fn is_remote_like(&self) -> bool {
        !matches!(self, ActiveTarget::Local)
    }

    pub fn remote_home(&self) -> Option<&str> {
        match self {
            ActiveTarget::Local => None,
            ActiveTarget::Ssh(target) => Some(target.remote_home.as_str()),
            ActiveTarget::Wsl(target) => Some(target.remote_home.as_str()),
        }
    }

    pub fn id(&self) -> &str {
        match self {
            ActiveTarget::Local => LOCAL_TARGET_ID,
            ActiveTarget::Ssh(target) => target.id.as_str(),
            ActiveTarget::Wsl(target) => target.id.as_str(),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            ActiveTarget::Local => "Local",
            ActiveTarget::Ssh(target) => target.label.as_str(),
            ActiveTarget::Wsl(target) => target.label.as_str(),
        }
    }

    pub fn kind(&self) -> TargetKind {
        match self {
            ActiveTarget::Local => TargetKind::Local,
            ActiveTarget::Ssh(_) => TargetKind::Ssh,
            ActiveTarget::Wsl(_) => TargetKind::Wsl,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum TargetConfigDomain {
    Ssh,
    Wsl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TargetConfigQuarantineIncident {
    pub domain: TargetConfigDomain,
    pub detected_at: String,
    pub reason_code: String,
    pub source_bytes: u64,
    pub source_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TargetConfigQuarantineStatus {
    pub version: u8,
    pub incidents: Vec<TargetConfigQuarantineIncident>,
    pub active_target_reset: bool,
}

impl Default for TargetConfigQuarantineStatus {
    fn default() -> Self {
        Self {
            version: 1,
            incidents: Vec::new(),
            active_target_reset: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TargetContext {
    target: ActiveTarget,
    db: DbPool,
}

impl TargetContext {
    pub(crate) fn new(target: ActiveTarget, db: DbPool) -> Self {
        Self { target, db }
    }

    pub fn target(&self) -> &ActiveTarget {
        &self.target
    }

    pub fn db(&self) -> &DbPool {
        &self.db
    }

    pub fn id(&self) -> &str {
        self.target.id()
    }

    pub fn label(&self) -> &str {
        self.target.label()
    }

    pub fn kind(&self) -> TargetKind {
        self.target.kind()
    }
}
