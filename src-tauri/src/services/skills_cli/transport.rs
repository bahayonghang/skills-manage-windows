//! Single Local/Remote seam for Skills CLI.
//!
//! Commands freeze a [`crate::targets::TargetContext`], then resolve this
//! transport once via [`SkillsCliTransport::for_target`]. Business logic must
//! not `match ActiveTarget`. The only `ActiveTarget` variant match is
//! `target_kind`, used for construction and gates.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use crate::targets::{
    connect_remote_target, ActiveTarget, ConnectedRemoteTarget, ProcessClass, TargetsError,
};

use super::argv::{
    is_node_version_supported, parse_node_version, quote_remote_cli_command, NodeLauncher,
    SKILLS_CLI_MIN_NODE_DISPLAY, SKILLS_CLI_NPM_SPEC,
};
use super::error::SkillsCliError;
use super::lock::{remote_lock_path, skills_cli_lock_path_from_env};
use super::remote_scripts::{
    build_extract_tar_command, build_remote_launcher_probe_script,
    build_remove_update_scratch_script, is_windows_remote_os, parse_remote_launcher_probe,
    REMOTE_SKILL_HASH_SCRIPT,
};
use super::runner::{CliOutput, NodeProcessRunner, RunnerRequest, SkillsCliRunner};
use super::SkillsCliManagedLinkKind;
use super::{doctor_with_program, resolve_node_program_from_env, SkillsCliDoctorReport};

const DOCTOR_PROBE_SCRIPT: &str = r#"printf 'XDG=%s\n' "${XDG_STATE_HOME-}"
printf 'HOME=%s\n' "$HOME"
if command -v node >/dev/null 2>&1; then
  printf 'NODEV=%s\n' "$(node --version 2>/dev/null)"
else
  printf 'NODEV=\n'
fi
"#;

/// Inventory opened ListGlobal / InstallTargets / ReadSkillMd / ExportInventory
/// on Remote. Mutate opened Link/Unlink/PreviewRemove/RemoveGlobal/LeftoverScan.
/// This install/update task opens PreviewSource, AddGlobal, CancelJob,
/// CheckUpdates, UpdateInventory, VerifyUpdateBaseline, ApplyUpdates, and
/// RetryUpdateRecovery. RevealFolder stays permanently unsupported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkillsCliCapability {
    Doctor,
    ListGlobal,
    InstallTargets,
    ReadSkillMd,
    RevealFolder,
    ExportInventory,
    PreviewSource,
    AddGlobal,
    LinkPlatform,
    UnlinkPlatform,
    PreviewRemove,
    RemoveGlobal,
    LeftoverScan,
    CancelJob,
    CheckUpdates,
    UpdateInventory,
    VerifyUpdateBaseline,
    ApplyUpdates,
    RetryUpdateRecovery,
}

impl SkillsCliCapability {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const ALL: &'static [Self] = &[
        Self::Doctor,
        Self::ListGlobal,
        Self::InstallTargets,
        Self::ReadSkillMd,
        Self::RevealFolder,
        Self::ExportInventory,
        Self::PreviewSource,
        Self::AddGlobal,
        Self::LinkPlatform,
        Self::UnlinkPlatform,
        Self::PreviewRemove,
        Self::RemoveGlobal,
        Self::LeftoverScan,
        Self::CancelJob,
        Self::CheckUpdates,
        Self::UpdateInventory,
        Self::VerifyUpdateBaseline,
        Self::ApplyUpdates,
        Self::RetryUpdateRecovery,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteCapabilitySupport {
    Supported,
    #[allow(dead_code)]
    UnsupportedOnRemote,
    /// Remote has no host file manager; never opened by later tasks.
    PermanentlyUnsupported,
}

fn remote_capability_support(cap: SkillsCliCapability) -> RemoteCapabilitySupport {
    match cap {
        SkillsCliCapability::Doctor
        | SkillsCliCapability::ListGlobal
        | SkillsCliCapability::InstallTargets
        | SkillsCliCapability::ReadSkillMd
        | SkillsCliCapability::ExportInventory
        | SkillsCliCapability::LinkPlatform
        | SkillsCliCapability::UnlinkPlatform
        | SkillsCliCapability::PreviewRemove
        | SkillsCliCapability::RemoveGlobal
        | SkillsCliCapability::LeftoverScan
        | SkillsCliCapability::PreviewSource
        | SkillsCliCapability::AddGlobal
        | SkillsCliCapability::CancelJob
        | SkillsCliCapability::CheckUpdates
        | SkillsCliCapability::UpdateInventory
        | SkillsCliCapability::VerifyUpdateBaseline
        | SkillsCliCapability::ApplyUpdates
        | SkillsCliCapability::RetryUpdateRecovery => RemoteCapabilitySupport::Supported,
        SkillsCliCapability::RevealFolder => RemoteCapabilitySupport::PermanentlyUnsupported,
    }
}

mod fs;
use fs::{LocalSkillsCliFs, RemoteSkillsCliFs, SkillsCliFs};

#[derive(Debug, Clone)]
pub(crate) struct SkillsCliPaths {
    canonical_root: String,
    lock_path: String,
    posix: bool,
}

impl SkillsCliPaths {
    fn for_local() -> Self {
        let home = crate::paths::skills_cli_local_home();
        Self {
            canonical_root: crate::paths::universal_skills_dir_from_home(&home)
                .to_string_lossy()
                .into_owned(),
            lock_path: skills_cli_lock_path_from_env(
                std::env::var("XDG_STATE_HOME").ok().as_deref(),
                &home,
            )
            .to_string_lossy()
            .into_owned(),
            posix: false,
        }
    }

    fn for_remote(remote_home: &str, xdg_state_home: Option<&str>) -> Self {
        Self {
            canonical_root: crate::targets::remote_join(
                remote_home,
                crate::paths::UNIVERSAL_SKILLS_REL,
            ),
            lock_path: remote_lock_path(xdg_state_home, remote_home),
            posix: true,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn canonical_root(&self) -> &str {
        &self.canonical_root
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn lock_path(&self) -> &str {
        &self.lock_path
    }

    pub(crate) fn canonical_root_path(&self) -> PathBuf {
        PathBuf::from(&self.canonical_root)
    }

    pub(crate) fn lock_path_buf(&self) -> PathBuf {
        PathBuf::from(&self.lock_path)
    }

    pub(crate) fn uses_posix(&self) -> bool {
        self.posix
    }

    pub(crate) fn join_child(&self, parent: &str, child: &str) -> String {
        if self.posix {
            crate::targets::remote_join(parent, child)
        } else {
            Path::new(parent).join(child).to_string_lossy().into_owned()
        }
    }

    pub(crate) fn parent_of(&self, path: &str) -> Option<String> {
        if self.posix {
            crate::targets::remote_parent(path)
        } else {
            Path::new(path)
                .parent()
                .map(|parent| parent.to_string_lossy().into_owned())
                .filter(|value| !value.is_empty())
        }
    }
}

pub(crate) enum SkillsCliScope {
    Local,
    Remote(Arc<ConnectedRemoteTarget>),
}

pub struct SkillsCliTransport {
    scope: SkillsCliScope,
    paths: Mutex<SkillsCliPaths>,
    #[allow(dead_code)]
    fs: Arc<dyn SkillsCliFs>,
    runner: Arc<dyn SkillsCliRunner>,
    writes: Arc<AtomicUsize>,
}

struct RemoteNodeRunner {
    connection: Arc<ConnectedRemoteTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillsCliTargetKind {
    Local,
    Remote,
}

impl SkillsCliTransport {
    /// Single `ActiveTarget` variant match: construction (`for_target`) and
    /// gates (`ensure_capability_for_target`, `uses_local_cli_lock`) call this.
    fn target_kind(target: &ActiveTarget) -> SkillsCliTargetKind {
        match target {
            ActiveTarget::Local => SkillsCliTargetKind::Local,
            ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => SkillsCliTargetKind::Remote,
        }
    }

    pub async fn for_target(target: &ActiveTarget) -> Result<Self, SkillsCliError> {
        match Self::target_kind(target) {
            SkillsCliTargetKind::Local => Ok(Self::for_local()),
            SkillsCliTargetKind::Remote => {
                let connection = connect_remote_target(target)
                    .await
                    .map_err(map_connect_error)?;
                Ok(Self::for_remote(Arc::new(connection)))
            }
        }
    }

    pub fn for_local() -> Self {
        Self::for_local_with_runner(Arc::new(NodeProcessRunner))
    }

    pub(crate) fn for_local_with_runner(runner: Arc<dyn SkillsCliRunner>) -> Self {
        let writes = Arc::new(AtomicUsize::new(0));
        Self {
            scope: SkillsCliScope::Local,
            paths: Mutex::new(SkillsCliPaths::for_local()),
            fs: Arc::new(LocalSkillsCliFs {
                writes: Arc::clone(&writes),
            }),
            runner,
            writes,
        }
    }

    fn for_remote(connection: Arc<ConnectedRemoteTarget>) -> Self {
        let writes = Arc::new(AtomicUsize::new(0));
        let paths = SkillsCliPaths::for_remote(connection.remote_home(), None);
        let runner: Arc<dyn SkillsCliRunner> = Arc::new(RemoteNodeRunner {
            connection: Arc::clone(&connection),
        });
        Self {
            scope: SkillsCliScope::Remote(Arc::clone(&connection)),
            paths: Mutex::new(paths),
            fs: Arc::new(RemoteSkillsCliFs {
                connection,
                writes: Arc::clone(&writes),
            }),
            runner,
            writes,
        }
    }

    /// Leftover lock protection and origin annotation consult this machine's
    /// lock only for Local. Same predicate as the former `is_local_target`.
    pub fn uses_local_cli_lock(target: &ActiveTarget) -> bool {
        matches!(Self::target_kind(target), SkillsCliTargetKind::Local)
    }

    pub(crate) fn paths(&self) -> SkillsCliPaths {
        self.paths
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    #[allow(dead_code)]
    pub(crate) fn fs(&self) -> &dyn SkillsCliFs {
        self.fs.as_ref()
    }

    pub(crate) fn is_remote(&self) -> bool {
        matches!(self.scope, SkillsCliScope::Remote(_))
    }

    pub(crate) fn mutation_target(&self) -> ActiveTarget {
        match &self.scope {
            SkillsCliScope::Local => ActiveTarget::Local,
            SkillsCliScope::Remote(connection) => connection.active_target(),
        }
    }

    /// Static `local` / `ssh` / `wsl` label for structured warn fields.
    pub(crate) fn warn_target_kind(&self) -> &'static str {
        match &self.scope {
            SkillsCliScope::Local => "local",
            SkillsCliScope::Remote(connection) => match connection.active_target() {
                ActiveTarget::Local => "local",
                ActiveTarget::Ssh(_) => "ssh",
                ActiveTarget::Wsl(_) => "wsl",
            },
        }
    }

    pub(crate) fn remote_connection(&self) -> Option<&ConnectedRemoteTarget> {
        match &self.scope {
            SkillsCliScope::Remote(connection) => Some(connection.as_ref()),
            SkillsCliScope::Local => None,
        }
    }

    pub(crate) async fn resolve_launcher(&self) -> Result<NodeLauncher, SkillsCliError> {
        match &self.scope {
            SkillsCliScope::Local => {
                let path = std::env::var("PATH").unwrap_or_default();
                crate::services::skills_cli::resolve_node_launcher(&path)
            }
            SkillsCliScope::Remote(connection) => {
                let stdout = connection
                    .run_script(&build_remote_launcher_probe_script(), &[])
                    .await
                    .map_err(|_| SkillsCliError::CliUnavailable)?;
                parse_remote_launcher_probe(&stdout)
            }
        }
    }

    pub(crate) async fn digest_remote_skill_dirs(
        &self,
        roots: &[String],
    ) -> Result<std::collections::HashMap<String, String>, SkillsCliError> {
        let connection = self
            .remote_connection()
            .ok_or(SkillsCliError::LocalTargetOnly)?;
        if roots.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let args: Vec<&str> = roots.iter().map(String::as_str).collect();
        let stdout = connection
            .run_script(REMOTE_SKILL_HASH_SCRIPT, &args)
            .await
            .map_err(|error| map_remote_error("hash skill directories", error))?;
        super::updates::parse_remote_skill_hash_output(&stdout)
    }

    pub(crate) async fn extract_tar_stdin_cancellable(
        &self,
        staging: &str,
        archive: &[u8],
        cancel: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<(), SkillsCliError> {
        let connection = self
            .remote_connection()
            .ok_or(SkillsCliError::LocalTargetOnly)?;
        self.writes.fetch_add(1, Ordering::SeqCst);
        let command = build_extract_tar_command(staging);
        connection
            .run_command_with_stdin_bytes_cancellable(&command, archive, cancel)
            .await
            .map(|_| ())
            .map_err(map_runner_error_from_targets)
    }

    pub(crate) async fn run_remote_script(
        &self,
        script: &str,
        mutate: bool,
    ) -> Result<String, SkillsCliError> {
        let connection = self
            .remote_connection()
            .ok_or(SkillsCliError::LocalTargetOnly)?;
        if mutate {
            self.writes.fetch_add(1, Ordering::SeqCst);
        }
        connection
            .run_script(script, &[])
            .await
            .map_err(|error| map_remote_error("run remote script", error))
    }

    pub(crate) async fn remove_update_scratch(&self, paths: &[&str]) -> Result<(), SkillsCliError> {
        if paths.is_empty() {
            return Ok(());
        }
        let script = build_remove_update_scratch_script(paths)?;
        self.run_remote_script(&script, true).await.map(|_| ())
    }

    pub(crate) fn recovery_target_id(&self) -> Option<&str> {
        match &self.scope {
            SkillsCliScope::Local => None,
            SkillsCliScope::Remote(connection) => Some(connection.target_id()),
        }
    }

    pub(crate) fn managed_link_kind(&self) -> SkillsCliManagedLinkKind {
        match &self.scope {
            SkillsCliScope::Local => {
                if cfg!(windows) {
                    SkillsCliManagedLinkKind::WindowsJunction
                } else {
                    SkillsCliManagedLinkKind::Symlink
                }
            }
            SkillsCliScope::Remote(connection) => {
                if is_windows_remote_os(connection.remote_os()) {
                    SkillsCliManagedLinkKind::WindowsJunction
                } else {
                    SkillsCliManagedLinkKind::Symlink
                }
            }
        }
    }

    pub(crate) fn runner(&self) -> &dyn SkillsCliRunner {
        self.runner.as_ref()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn write_count(&self) -> usize {
        self.writes.load(Ordering::SeqCst)
    }

    /// Gate before [`Self::for_target`] so unsupported Remote capabilities
    /// return `local_target_only` without an SSH handshake or FS write.
    pub fn ensure_capability_for_target(
        target: &ActiveTarget,
        cap: SkillsCliCapability,
    ) -> Result<(), SkillsCliError> {
        match Self::target_kind(target) {
            SkillsCliTargetKind::Local => Ok(()),
            SkillsCliTargetKind::Remote => Self::ensure_remote_capability(cap),
        }
    }

    fn ensure_remote_capability(cap: SkillsCliCapability) -> Result<(), SkillsCliError> {
        match remote_capability_support(cap) {
            RemoteCapabilitySupport::Supported => Ok(()),
            RemoteCapabilitySupport::UnsupportedOnRemote
            | RemoteCapabilitySupport::PermanentlyUnsupported => {
                Err(SkillsCliError::LocalTargetOnly)
            }
        }
    }

    pub fn ensure_capability(&self, cap: SkillsCliCapability) -> Result<(), SkillsCliError> {
        match &self.scope {
            SkillsCliScope::Local => Ok(()),
            SkillsCliScope::Remote(_) => Self::ensure_remote_capability(cap),
        }
    }

    pub(crate) async fn doctor(&self) -> Result<SkillsCliDoctorReport, SkillsCliError> {
        match &self.scope {
            SkillsCliScope::Local => {
                doctor_with_program(self.runner.as_ref(), &resolve_node_program_from_env()?).await
            }
            SkillsCliScope::Remote(connection) => self.doctor_remote(connection.as_ref()).await,
        }
    }

    async fn doctor_remote(
        &self,
        connection: &ConnectedRemoteTarget,
    ) -> Result<SkillsCliDoctorReport, SkillsCliError> {
        let stdout = connection
            .run_script(DOCTOR_PROBE_SCRIPT, &[])
            .await
            .map_err(map_doctor_remote_error)?;
        let probe = parse_doctor_probe(&stdout);
        if let Some(probed_home) = probe.home.as_deref().filter(|value| !value.is_empty()) {
            if probed_home != connection.remote_home() {
                tracing::warn!("Skills CLI remote HOME does not match configured remote_home");
            }
        }
        {
            let mut paths = self.paths.lock().unwrap_or_else(|error| error.into_inner());
            *paths = SkillsCliPaths::for_remote(
                connection.remote_home(),
                probe.xdg_state_home.as_deref(),
            );
        }
        if probe.node_version.trim().is_empty() {
            return Err(SkillsCliError::NodeMissing);
        }
        let found_version = probe.node_version.trim().to_string();
        let parsed =
            parse_node_version(&probe.node_version).ok_or_else(|| SkillsCliError::NodeTooOld {
                required: SKILLS_CLI_MIN_NODE_DISPLAY,
                found: found_version.clone(),
            })?;
        if !is_node_version_supported(parsed) {
            return Err(SkillsCliError::NodeTooOld {
                required: SKILLS_CLI_MIN_NODE_DISPLAY,
                found: found_version,
            });
        }
        Ok(SkillsCliDoctorReport {
            node_version: found_version,
            npm_spec: SKILLS_CLI_NPM_SPEC.to_string(),
        })
    }
}

struct DoctorProbe {
    xdg_state_home: Option<String>,
    home: Option<String>,
    node_version: String,
}

fn parse_doctor_probe(stdout: &str) -> DoctorProbe {
    let mut xdg_state_home = None;
    let mut home = None;
    let mut node_version = String::new();
    for line in stdout.lines() {
        if let Some(value) = line.strip_prefix("XDG=") {
            xdg_state_home = Some(value.to_string()).filter(|item| !item.is_empty());
        } else if let Some(value) = line.strip_prefix("HOME=") {
            home = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("NODEV=") {
            node_version = value.to_string();
        }
    }
    DoctorProbe {
        xdg_state_home,
        home,
        node_version,
    }
}

pub(super) fn map_connect_error(error: TargetsError) -> SkillsCliError {
    match error {
        TargetsError::ProcessTimedOut { timeout_ms, .. } => {
            SkillsCliError::Timeout(Duration::from_millis(timeout_ms as u64))
        }
        TargetsError::ProcessCancelled(_) => SkillsCliError::Cancelled,
        _ => {
            tracing::warn!("Skills CLI remote host is unavailable");
            SkillsCliError::RemoteUnavailable
        }
    }
}

fn map_doctor_remote_error(error: TargetsError) -> SkillsCliError {
    match error {
        TargetsError::ProcessTimedOut { timeout_ms, .. } => {
            SkillsCliError::Timeout(Duration::from_millis(timeout_ms as u64))
        }
        TargetsError::ProcessCancelled(_) => SkillsCliError::Cancelled,
        _ => {
            tracing::warn!("Skills CLI remote doctor probe failed");
            SkillsCliError::NodeMissing
        }
    }
}

pub(super) fn map_remote_error(context: &'static str, error: TargetsError) -> SkillsCliError {
    match error {
        TargetsError::ProcessTimedOut { timeout_ms, .. } => {
            SkillsCliError::Timeout(Duration::from_millis(timeout_ms as u64))
        }
        TargetsError::ProcessCancelled(_) => SkillsCliError::Cancelled,
        TargetsError::ProcessOutputLimitExceeded { stream, .. } => {
            SkillsCliError::OutputLimitExceeded { stream }
        }
        TargetsError::RemoteCommandFailed { .. } | TargetsError::WslCommandFailed { .. } => {
            tracing::warn!(context, "Skills CLI remote host is unavailable");
            SkillsCliError::RemoteUnavailable
        }
        _ => {
            tracing::warn!(context, "Skills CLI remote transport failed");
            SkillsCliError::Io {
                context,
                source: std::io::Error::other("remote access failed"),
            }
        }
    }
}

#[async_trait]
impl SkillsCliRunner for RemoteNodeRunner {
    async fn run(&self, request: RunnerRequest<'_>) -> Result<CliOutput, SkillsCliError> {
        let command = quote_remote_cli_command(&request.program, &request.args);
        let result = if request.policy.class == ProcessClass::BulkTransfer {
            self.connection
                .run_script_cancellable(&command, &[], request.cancel)
                .await
        } else {
            self.connection.run_command(&command).await
        };
        match result {
            Ok(stdout) => Ok(CliOutput {
                status_success: true,
                exit_code: Some(0),
                stdout: stdout.into_bytes(),
                stderr: Vec::new(),
            }),
            Err(TargetsError::ProcessTimedOut { timeout_ms, .. }) => Err(SkillsCliError::Timeout(
                Duration::from_millis(timeout_ms as u64),
            )),
            Err(TargetsError::ProcessCancelled(_)) => Err(SkillsCliError::Cancelled),
            Err(TargetsError::ProcessOutputLimitExceeded { stream, .. }) => {
                Err(SkillsCliError::OutputLimitExceeded { stream })
            }
            Err(TargetsError::RemoteCommandFailed { status, .. })
            | Err(TargetsError::WslCommandFailed { status, .. }) => Ok(CliOutput {
                status_success: false,
                exit_code: status.code(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            }),
            Err(error) => Err(map_runner_error_from_targets(error)),
        }
    }
}

fn map_runner_error_from_targets(error: TargetsError) -> SkillsCliError {
    match error {
        TargetsError::ProcessTimedOut { timeout_ms, .. } => {
            SkillsCliError::Timeout(Duration::from_millis(timeout_ms as u64))
        }
        TargetsError::ProcessCancelled(_) => SkillsCliError::Cancelled,
        TargetsError::ProcessOutputLimitExceeded { stream, .. } => {
            SkillsCliError::OutputLimitExceeded { stream }
        }
        _ => {
            tracing::warn!("Skills CLI remote process failed to start");
            SkillsCliError::CliUnavailable
        }
    }
}

#[cfg(test)]
impl SkillsCliTransport {
    pub(crate) fn for_tests_remote(connection: ConnectedRemoteTarget) -> Self {
        Self::for_remote(Arc::new(connection))
    }

    pub(crate) fn map_remote_error_for_tests(error: TargetsError) -> SkillsCliError {
        map_remote_error("test remote", error)
    }

    pub(crate) fn map_connect_error_for_tests(error: TargetsError) -> SkillsCliError {
        map_connect_error(error)
    }

    pub(crate) async fn doctor_ignoring_platforms(
        &self,
        _platforms: &[&str],
    ) -> Result<SkillsCliDoctorReport, SkillsCliError> {
        self.doctor().await
    }
}
