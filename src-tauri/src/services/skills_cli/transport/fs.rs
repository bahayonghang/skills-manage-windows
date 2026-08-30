//! Local and Remote filesystem backends for [`super::SkillsCliTransport`].

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tempfile::Builder;

use crate::services::installation::fs_util::{
    create_skills_cli_directory_link, is_reparse_or_symlink, ManagedDirectoryLinkKind,
};
use crate::targets::{
    remote_file_type_is_dir, ConnectedRemoteTarget, RemoteDirEntry, RemotePathInfo, TargetsError,
};

use super::super::error::SkillsCliError;
use super::super::probe::{
    build_path_probe_script, parse_path_probe_output, PathProbe, PathProbeKind,
};
use super::super::remote_scripts::{
    build_atomic_replace_script, build_create_managed_link_script,
    build_create_managed_links_script, build_remove_canonical_backup_script,
    build_remove_update_scratch_script, build_rename_script, build_verified_link_remove_script,
    is_skillport_canonical_backup_path, is_skillport_update_scratch_path, is_windows_remote_os,
    parse_verified_link_remove_output, VerifiedLinkRemoveStatus,
};
use super::super::SkillsCliManagedLinkKind;
use super::map_remote_error;

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

pub(super) struct LocalSkillsCliFs {
    #[allow(dead_code)]
    pub(super) writes: Arc<AtomicUsize>,
}

#[allow(dead_code)]
pub(super) struct RemoteSkillsCliFs {
    pub(super) connection: Arc<ConnectedRemoteTarget>,
    pub(super) writes: Arc<AtomicUsize>,
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
            .map(|item| super::super::probe::observed_slot_from_probe(item, target, kind, posix))
            .unwrap_or(super::super::placement::ObservedSlot::Absent);
        match slot {
            super::super::placement::ObservedSlot::ManagedLink {
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
