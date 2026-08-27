//! Single Local/Remote seam for Skills CLI.
//!
//! Commands freeze a [`crate::targets::TargetContext`], then resolve this
//! transport once via [`SkillsCliTransport::for_target`]. Business logic must
//! not `match ActiveTarget`. The only `ActiveTarget` variant match is
//! `target_kind`, used for construction and gates.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tempfile::Builder;

use crate::services::installation::fs_util::{
    create_skills_cli_directory_link, is_reparse_or_symlink, ManagedDirectoryLinkKind,
};
use crate::targets::{
    connect_remote_target, remote_file_type_is_dir, ActiveTarget, ConnectedRemoteTarget,
    ProcessClass, RemoteDirEntry, RemotePathInfo, TargetsError,
};

use super::argv::{
    is_node_version_supported, parse_node_version, quote_remote_cli_command, NodeLauncher,
    SKILLS_CLI_MIN_NODE_DISPLAY, SKILLS_CLI_NPM_SPEC,
};
use super::error::SkillsCliError;
use super::lock::{remote_lock_path, skills_cli_lock_path_from_env};
use super::probe::{build_path_probe_script, parse_path_probe_output, PathProbe, PathProbeKind};
use super::remote_scripts::{
    build_atomic_replace_script, build_create_managed_link_script,
    build_create_managed_links_script, build_extract_tar_command,
    build_remote_launcher_probe_script, build_remove_canonical_backup_script,
    build_remove_update_scratch_script, build_rename_script, build_verified_link_remove_script,
    is_skillport_canonical_backup_path, is_skillport_update_scratch_path, is_windows_remote_os,
    parse_remote_launcher_probe, parse_verified_link_remove_output, VerifiedLinkRemoveStatus,
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

/// Inventory/mutate children call these FS primitives. This seam ships both
/// backends so later tasks do not reshape the trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum SkillsCliPathKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct SkillsCliPathInfo {
    pub kind: SkillsCliPathKind,
    pub symlink_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct SkillsCliDirEntry {
    pub name: String,
    pub kind: SkillsCliPathKind,
    pub symlink_target: Option<String>,
}

#[async_trait]
#[allow(dead_code)]
pub(crate) trait SkillsCliFs: Send + Sync {
    async fn inspect_path(&self, path: &str) -> Result<Option<SkillsCliPathInfo>, SkillsCliError>;
    async fn read_file_bounded(
        &self,
        path: &str,
        max_bytes: u64,
    ) -> Result<Vec<u8>, SkillsCliError>;
    async fn atomic_write(&self, path: &str, bytes: &[u8]) -> Result<(), SkillsCliError>;
    async fn list_dir(&self, path: &str) -> Result<Vec<SkillsCliDirEntry>, SkillsCliError>;
    async fn remove_tree(&self, path: &str) -> Result<(), SkillsCliError>;
    async fn create_dir_all(&self, path: &str) -> Result<(), SkillsCliError>;
    async fn exists(&self, path: &str) -> Result<bool, SkillsCliError>;
    async fn probe_paths(&self, paths: &[String]) -> Result<Vec<PathProbe>, SkillsCliError>;
    async fn create_managed_link(
        &self,
        target: &str,
        link: &str,
    ) -> Result<SkillsCliManagedLinkKind, SkillsCliError>;
    async fn create_managed_links(&self, pairs: &[(String, String)]) -> Result<(), SkillsCliError>;
    async fn remove_verified_link(
        &self,
        link: &str,
    ) -> Result<VerifiedLinkRemoveStatus, SkillsCliError>;
    async fn remove_verified_links(
        &self,
        links: &[String],
    ) -> Result<Vec<(String, VerifiedLinkRemoveStatus)>, SkillsCliError>;
    async fn rename(&self, from: &str, to: &str) -> Result<(), SkillsCliError>;
}

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

struct LocalSkillsCliFs {
    #[allow(dead_code)]
    writes: Arc<AtomicUsize>,
}

#[allow(dead_code)]
struct RemoteSkillsCliFs {
    connection: Arc<ConnectedRemoteTarget>,
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

fn local_remove_verified_link(
    link: &str,
    writes: &AtomicUsize,
) -> Result<VerifiedLinkRemoveStatus, SkillsCliError> {
    let path = Path::new(link);
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(VerifiedLinkRemoveStatus::Absent);
        }
        Err(error) => {
            return Err(SkillsCliError::Io {
                context: "inspect link",
                source: error,
            });
        }
    };
    if !is_reparse_or_symlink(&metadata) {
        return Ok(VerifiedLinkRemoveStatus::SkippedNotLink);
    }
    writes.fetch_add(1, Ordering::SeqCst);
    let result = if cfg!(windows) {
        std::fs::remove_dir(path).or_else(|_| std::fs::remove_file(path))
    } else {
        std::fs::remove_file(path).or_else(|_| std::fs::remove_dir(path))
    };
    result
        .map(|_| VerifiedLinkRemoveStatus::Removed)
        .map_err(|error| SkillsCliError::Io {
            context: "remove verified link",
            source: error,
        })
}

fn to_ipc_link_kind(kind: ManagedDirectoryLinkKind) -> SkillsCliManagedLinkKind {
    match kind {
        ManagedDirectoryLinkKind::WindowsJunction => SkillsCliManagedLinkKind::WindowsJunction,
        ManagedDirectoryLinkKind::Symlink => SkillsCliManagedLinkKind::Symlink,
    }
}

fn map_directory_link_error(
    error: crate::services::installation::InstallationError,
) -> SkillsCliError {
    match error {
        crate::services::installation::InstallationError::ManagedDirectoryLinkUnsupported => {
            SkillsCliError::PlacementUnavailable
        }
        crate::services::installation::InstallationError::ManagedDirectoryLinkTargetMismatch => {
            SkillsCliError::PlacementConflict
        }
        other => SkillsCliError::Io {
            context: "managed directory link",
            source: std::io::Error::other(other.to_string()),
        },
    }
}

fn map_create_link_error(error: TargetsError) -> SkillsCliError {
    match error {
        TargetsError::ProcessTimedOut { timeout_ms, .. } => {
            SkillsCliError::Timeout(Duration::from_millis(timeout_ms as u64))
        }
        TargetsError::ProcessCancelled(_) => SkillsCliError::Cancelled,
        _ => {
            tracing::warn!("Skills CLI remote managed link could not be created");
            SkillsCliError::PlacementUnavailable
        }
    }
}

fn map_connect_error(error: TargetsError) -> SkillsCliError {
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

fn map_remote_error(context: &'static str, error: TargetsError) -> SkillsCliError {
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

fn probe_local_path(path: &str) -> Result<PathProbe, SkillsCliError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PathProbe {
                path: path.to_string(),
                kind: PathProbeKind::Absent,
                link_target: None,
            });
        }
        Err(error) => {
            return Err(SkillsCliError::Io {
                context: "probe path",
                source: error,
            });
        }
    };
    if is_reparse_or_symlink(&metadata) {
        let link_target = std::fs::read_link(path)
            .ok()
            .map(|target| target.to_string_lossy().into_owned());
        return Ok(PathProbe {
            path: path.to_string(),
            kind: PathProbeKind::Link,
            link_target,
        });
    }
    let kind = if metadata.is_dir() {
        PathProbeKind::Dir
    } else {
        PathProbeKind::File
    };
    Ok(PathProbe {
        path: path.to_string(),
        kind,
        link_target: None,
    })
}

#[allow(dead_code)]
fn kind_from_local(file_type: std::fs::FileType) -> SkillsCliPathKind {
    if file_type.is_symlink() {
        SkillsCliPathKind::Symlink
    } else if file_type.is_dir() {
        SkillsCliPathKind::Directory
    } else if file_type.is_file() {
        SkillsCliPathKind::File
    } else {
        SkillsCliPathKind::Other
    }
}

#[allow(dead_code)]
fn kind_from_remote(info: &RemotePathInfo) -> SkillsCliPathKind {
    if info.symlink_target.is_some()
        || info.file_type == "symlink"
        || info.file_type.contains("Symlink")
    {
        SkillsCliPathKind::Symlink
    } else if remote_file_type_is_dir(&info.file_type) {
        SkillsCliPathKind::Directory
    } else if info.file_type == "file" || info.file_type.contains("File") {
        SkillsCliPathKind::File
    } else {
        SkillsCliPathKind::Other
    }
}

#[allow(dead_code)]
fn kind_from_remote_entry(entry: &RemoteDirEntry) -> SkillsCliPathKind {
    kind_from_remote(&RemotePathInfo {
        file_type: entry.file_type.clone(),
        symlink_target: entry.symlink_target.clone(),
    })
}

#[async_trait]
#[allow(dead_code)]
impl SkillsCliFs for LocalSkillsCliFs {
    async fn inspect_path(&self, path: &str) -> Result<Option<SkillsCliPathInfo>, SkillsCliError> {
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(SkillsCliError::Io {
                    context: "inspect path",
                    source: error,
                });
            }
        };
        let symlink_target = if metadata.file_type().is_symlink() {
            std::fs::read_link(path)
                .ok()
                .map(|target| target.to_string_lossy().into_owned())
        } else {
            None
        };
        Ok(Some(SkillsCliPathInfo {
            kind: kind_from_local(metadata.file_type()),
            symlink_target,
        }))
    }

    async fn read_file_bounded(
        &self,
        path: &str,
        max_bytes: u64,
    ) -> Result<Vec<u8>, SkillsCliError> {
        let metadata = std::fs::metadata(path).map_err(|error| SkillsCliError::Io {
            context: "read file",
            source: error,
        })?;
        if metadata.len() > max_bytes {
            return Err(SkillsCliError::Io {
                context: "read file",
                source: std::io::Error::other("file exceeds bound"),
            });
        }
        std::fs::read(path).map_err(|error| SkillsCliError::Io {
            context: "read file",
            source: error,
        })
    }

    async fn atomic_write(&self, path: &str, bytes: &[u8]) -> Result<(), SkillsCliError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        let target = Path::new(path);
        let parent = target
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut temp = Builder::new()
            .prefix(".skillport-skills-cli-")
            .tempfile_in(parent)
            .map_err(|error| SkillsCliError::Io {
                context: "atomic write",
                source: error,
            })?;
        temp.write_all(bytes).map_err(|error| SkillsCliError::Io {
            context: "atomic write",
            source: error,
        })?;
        temp.flush().map_err(|error| SkillsCliError::Io {
            context: "atomic write",
            source: error,
        })?;
        temp.as_file()
            .sync_all()
            .map_err(|error| SkillsCliError::Io {
                context: "atomic write",
                source: error,
            })?;
        temp.persist(target).map_err(|error| SkillsCliError::Io {
            context: "atomic write",
            source: std::io::Error::other(error.to_string()),
        })?;
        Ok(())
    }

    async fn list_dir(&self, path: &str) -> Result<Vec<SkillsCliDirEntry>, SkillsCliError> {
        let mut entries = Vec::new();
        let read = std::fs::read_dir(path).map_err(|error| SkillsCliError::Io {
            context: "list directory",
            source: error,
        })?;
        for entry in read {
            let entry = entry.map_err(|error| SkillsCliError::Io {
                context: "list directory",
                source: error,
            })?;
            let metadata =
                std::fs::symlink_metadata(entry.path()).map_err(|error| SkillsCliError::Io {
                    context: "list directory",
                    source: error,
                })?;
            let symlink_target = if metadata.file_type().is_symlink() {
                std::fs::read_link(entry.path())
                    .ok()
                    .map(|target| target.to_string_lossy().into_owned())
            } else {
                None
            };
            entries.push(SkillsCliDirEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                kind: kind_from_local(metadata.file_type()),
                symlink_target,
            });
        }
        Ok(entries)
    }

    async fn remove_tree(&self, path: &str) -> Result<(), SkillsCliError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        let target = Path::new(path);
        let result = if target.is_dir() {
            std::fs::remove_dir_all(target)
        } else {
            std::fs::remove_file(target)
        };
        result.map_err(|error| SkillsCliError::Io {
            context: "remove tree",
            source: error,
        })
    }

    async fn create_dir_all(&self, path: &str) -> Result<(), SkillsCliError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        std::fs::create_dir_all(path).map_err(|error| SkillsCliError::Io {
            context: "create directory",
            source: error,
        })
    }

    async fn exists(&self, path: &str) -> Result<bool, SkillsCliError> {
        Ok(Path::new(path).exists())
    }

    async fn probe_paths(&self, paths: &[String]) -> Result<Vec<PathProbe>, SkillsCliError> {
        paths.iter().map(|path| probe_local_path(path)).collect()
    }

    async fn create_managed_link(
        &self,
        target: &str,
        link: &str,
    ) -> Result<SkillsCliManagedLinkKind, SkillsCliError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        let kind = create_skills_cli_directory_link(Path::new(target), Path::new(link))
            .map_err(map_directory_link_error)?;
        Ok(to_ipc_link_kind(kind))
    }

    async fn create_managed_links(&self, pairs: &[(String, String)]) -> Result<(), SkillsCliError> {
        for (target, link) in pairs {
            self.create_managed_link(target, link).await?;
        }
        Ok(())
    }

    async fn remove_verified_link(
        &self,
        link: &str,
    ) -> Result<VerifiedLinkRemoveStatus, SkillsCliError> {
        local_remove_verified_link(link, &self.writes)
    }

    async fn remove_verified_links(
        &self,
        links: &[String],
    ) -> Result<Vec<(String, VerifiedLinkRemoveStatus)>, SkillsCliError> {
        let mut out = Vec::with_capacity(links.len());
        for link in links {
            out.push((
                link.clone(),
                local_remove_verified_link(link, &self.writes)?,
            ));
        }
        Ok(out)
    }

    async fn rename(&self, from: &str, to: &str) -> Result<(), SkillsCliError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        std::fs::rename(from, to).map_err(|error| SkillsCliError::Io {
            context: "rename path",
            source: error,
        })
    }
}

#[async_trait]
#[allow(dead_code)]
impl SkillsCliFs for RemoteSkillsCliFs {
    async fn inspect_path(&self, path: &str) -> Result<Option<SkillsCliPathInfo>, SkillsCliError> {
        let info = self
            .connection
            .inspect_path(path)
            .await
            .map_err(|error| map_remote_error("inspect path", error))?;
        Ok(info.map(|info| SkillsCliPathInfo {
            kind: kind_from_remote(&info),
            symlink_target: info.symlink_target,
        }))
    }

    async fn read_file_bounded(
        &self,
        path: &str,
        max_bytes: u64,
    ) -> Result<Vec<u8>, SkillsCliError> {
        match self.connection.read_file_bounded(path, max_bytes).await {
            Ok(bytes) => Ok(bytes),
            // Bounded-read never emits RemotePathMissing: missing files and other
            // non-44/45 failures become RemoteFileReadFailed. Inventory maps that
            // to NotFound so a missing lock is an empty snapshot, not an error.
            Err(TargetsError::RemotePathMissing(_))
            | Err(TargetsError::WslPathMissing(_))
            | Err(TargetsError::RemoteFileReadFailed { .. }) => Err(SkillsCliError::Io {
                context: "read file",
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "remote path missing"),
            }),
            Err(error) => Err(map_remote_error("read file", error)),
        }
    }

    async fn atomic_write(&self, path: &str, bytes: &[u8]) -> Result<(), SkillsCliError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        let parent = crate::targets::remote_parent(path).unwrap_or_else(|| ".".to_string());
        let temp = format!(
            "{}/.skillport-skills-cli-lock-{}",
            parent.trim_end_matches('/'),
            uuid::Uuid::new_v4()
        );
        self.connection
            .write_file(&temp, bytes)
            .await
            .map_err(|error| map_remote_error("atomic write", error))?;
        let script = build_atomic_replace_script(&temp, path);
        self.connection
            .run_script(&script, &[])
            .await
            .map(|_| ())
            .map_err(|error| map_remote_error("atomic write", error))
    }

    async fn list_dir(&self, path: &str) -> Result<Vec<SkillsCliDirEntry>, SkillsCliError> {
        let entries = self
            .connection
            .list_dir(path)
            .await
            .map_err(|error| map_remote_error("list directory", error))?;
        Ok(entries
            .iter()
            .map(|entry| SkillsCliDirEntry {
                name: entry.name.clone(),
                kind: kind_from_remote_entry(entry),
                symlink_target: entry.symlink_target.clone(),
            })
            .collect())
    }

    async fn remove_tree(&self, path: &str) -> Result<(), SkillsCliError> {
        if !is_skillport_canonical_backup_path(path) && !is_skillport_update_scratch_path(path) {
            return Err(SkillsCliError::Io {
                context: "remove tree",
                source: std::io::Error::other("refused non-backup recursive delete"),
            });
        }
        self.writes.fetch_add(1, Ordering::SeqCst);
        let script = if is_skillport_update_scratch_path(path) {
            build_remove_update_scratch_script(&[path])?
        } else {
            build_remove_canonical_backup_script(path)?
        };
        self.connection
            .run_script(&script, &[])
            .await
            .map(|_| ())
            .map_err(|error| map_remote_error("remove backup", error))
    }

    async fn create_dir_all(&self, path: &str) -> Result<(), SkillsCliError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        self.connection
            .mkdir_p(path)
            .await
            .map_err(|error| map_remote_error("create directory", error))
    }

    async fn exists(&self, path: &str) -> Result<bool, SkillsCliError> {
        self.connection
            .exists(path)
            .await
            .map_err(|error| map_remote_error("exists", error))
    }

    async fn probe_paths(&self, paths: &[String]) -> Result<Vec<PathProbe>, SkillsCliError> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let script = build_path_probe_script(paths);
        let stdout = self
            .connection
            .run_script(&script, &[])
            .await
            .map_err(|error| map_remote_error("probe paths", error))?;
        Ok(parse_path_probe_output(paths, &stdout))
    }

    async fn create_managed_link(
        &self,
        target: &str,
        link: &str,
    ) -> Result<SkillsCliManagedLinkKind, SkillsCliError> {
        let windows = is_windows_remote_os(self.connection.remote_os());
        let script = build_create_managed_link_script(windows, target, link)?;
        self.writes.fetch_add(1, Ordering::SeqCst);
        if let Err(error) = self.connection.run_script(&script, &[]).await {
            let _ = self.remove_verified_link(link).await;
            return Err(map_create_link_error(error));
        }
        let link_path = link.to_string();
        let probes = self.probe_paths(std::slice::from_ref(&link_path)).await;
        let probe = match probes {
            Ok(list) => list.into_iter().next(),
            Err(error) => {
                let _ = self.remove_verified_link(link).await;
                return Err(error);
            }
        };
        let kind = if windows {
            SkillsCliManagedLinkKind::WindowsJunction
        } else {
            SkillsCliManagedLinkKind::Symlink
        };
        let posix = true;
        let slot = probe
            .as_ref()
            .map(|item| super::probe::observed_slot_from_probe(item, target, kind, posix))
            .unwrap_or(super::placement::ObservedSlot::Absent);
        match slot {
            super::placement::ObservedSlot::ManagedLink {
                resolves_to_canonical: true,
                kind: confirmed,
            } => Ok(confirmed),
            _ => {
                let _ = self.remove_verified_link(link).await;
                Err(SkillsCliError::PlacementUnavailable)
            }
        }
    }

    async fn create_managed_links(&self, pairs: &[(String, String)]) -> Result<(), SkillsCliError> {
        if pairs.is_empty() {
            return Ok(());
        }
        let windows = is_windows_remote_os(self.connection.remote_os());
        let script = build_create_managed_links_script(windows, pairs)?;
        self.writes.fetch_add(1, Ordering::SeqCst);
        self.connection
            .run_script(&script, &[])
            .await
            .map(|_| ())
            .map_err(map_create_link_error)
    }

    async fn remove_verified_link(
        &self,
        link: &str,
    ) -> Result<VerifiedLinkRemoveStatus, SkillsCliError> {
        let link_path = link.to_string();
        let results = self
            .remove_verified_links(std::slice::from_ref(&link_path))
            .await?;
        Ok(results
            .into_iter()
            .next()
            .map(|(_, status)| status)
            .unwrap_or(VerifiedLinkRemoveStatus::Absent))
    }

    async fn remove_verified_links(
        &self,
        links: &[String],
    ) -> Result<Vec<(String, VerifiedLinkRemoveStatus)>, SkillsCliError> {
        if links.is_empty() {
            return Ok(Vec::new());
        }
        let windows = is_windows_remote_os(self.connection.remote_os());
        let script = build_verified_link_remove_script(windows, links);
        self.writes.fetch_add(1, Ordering::SeqCst);
        let stdout = self
            .connection
            .run_script(&script, &[])
            .await
            .map_err(|error| map_remote_error("remove verified link", error))?;
        Ok(parse_verified_link_remove_output(links, &stdout))
    }

    async fn rename(&self, from: &str, to: &str) -> Result<(), SkillsCliError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        let script = build_rename_script(from, to);
        self.connection
            .run_script(&script, &[])
            .await
            .map(|_| ())
            .map_err(|error| map_remote_error("rename path", error))
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
