//! Local/remote filesystem layer used by Central skill updates.
//!
//! Pure file IO / hashing / path utilities with no DB and no Tauri state.
//! Both sides (local and remote) share the same interface so the
//! orchestration code in this domain never branches on target type.

use flate2::{write::GzEncoder, Compression};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::path::{Component, Path, PathBuf};

use uuid::Uuid;

use crate::fs_util::run_blocking_fs_with;
use crate::services::github_import::GitHubRepoSnapshot;
use crate::services::installation::copy_dir_all;
use crate::targets::{
    connect_remote_target, remote_parent, shell_quote, ActiveTarget, ConnectedRemoteTarget,
};

use super::error::CentralUpdatesError;

mod remote_scripts;

#[cfg(test)]
mod tests;

use remote_scripts::{
    REMOTE_CENTRAL_UPDATE_SCRIPT, REMOTE_HASH_SCRIPT, REMOTE_HASH_UNSUPPORTED_EXIT_CODE,
    REMOTE_REFRESH_COPY_SCRIPT,
};

#[derive(Debug, Clone)]
pub(crate) struct RemoteSkillFile {
    pub repo_path: String,
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

/// Filesystem façade used by Central skill updates.
///
/// Local mode operates on `std::fs` paths; SSH mode delegates to a connected
/// remote and runs equivalent shell-side primitives. Both modes intentionally
/// expose the same operations so the update orchestration never branches on
/// target type.
pub(crate) enum CentralFs {
    Local,
    Remote(Box<ConnectedRemoteTarget>),
}

impl CentralFs {
    pub(crate) async fn from_active_target(
        target: ActiveTarget,
    ) -> Result<Self, CentralUpdatesError> {
        match target {
            ActiveTarget::Local => Ok(Self::Local),
            ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                let conn = connect_remote_target(&target)
                    .await
                    .map_err(|e| CentralUpdatesError::Remote(e.to_string()))?;
                Ok(Self::Remote(Box::new(conn)))
            }
        }
    }

    /// Compute hashes for many skill directories. SSH mode batches roots into
    /// one remote script per chunk; local mode stays simple and deterministic.
    pub(crate) async fn hash_directories(
        &self,
        roots: &[PathBuf],
    ) -> Result<HashMap<PathBuf, String>, CentralUpdatesError> {
        match self {
            Self::Local => {
                let roots = roots.to_vec();
                run_blocking_fs_with(
                    "central skill hashing",
                    move || {
                        let mut hashes = HashMap::with_capacity(roots.len());
                        for root in &roots {
                            hashes.insert(root.clone(), hash_local_directory(root)?);
                        }
                        Ok(hashes)
                    },
                    CentralUpdatesError::task_join,
                )
                .await
            }
            Self::Remote(conn) => hash_remote_directories(conn, roots).await,
        }
    }

    /// Atomically replace `target_dir` with the contents of `files`.
    ///
    /// Both variants write into a sibling staging directory first, then swap
    /// it into place using a rename pair so a failure mid-write never leaves
    /// the canonical directory empty.
    pub(crate) async fn write_skill_dir_atomic(
        &self,
        skill_id: &str,
        target_dir: &Path,
        files: &[RemoteSkillFile],
    ) -> Result<(), CentralUpdatesError> {
        match self {
            Self::Local => {
                let skill_id = skill_id.to_string();
                let target_dir = target_dir.to_path_buf();
                let files = files.to_vec();
                run_blocking_fs_with(
                    "central skill atomic write",
                    move || write_skill_dir_atomic_local(&skill_id, &target_dir, &files),
                    CentralUpdatesError::task_join,
                )
                .await
            }
            Self::Remote(conn) => {
                write_skill_dir_atomic_remote(conn, skill_id, &posix_path(target_dir), files).await
            }
        }
    }

    /// Refresh a copy-installation so that `target` mirrors `source_dir`.
    ///
    /// `target` must end with `skill_id` to keep the operation scoped to the
    /// expected installation slot. The remote variant uses `cp -R` over SSH;
    /// the local variant delegates to the installation domain's
    /// [`copy_dir_all`].
    pub(crate) async fn refresh_copy_install(
        &self,
        skill_id: &str,
        source_dir: &Path,
        target: &str,
    ) -> Result<(), CentralUpdatesError> {
        match self {
            Self::Local => {
                let skill_id = skill_id.to_string();
                let source_dir = source_dir.to_path_buf();
                let target = target.to_string();
                run_blocking_fs_with(
                    "copy install refresh",
                    move || refresh_copy_install_local(&skill_id, &source_dir, &target),
                    CentralUpdatesError::task_join,
                )
                .await
            }
            Self::Remote(conn) => {
                refresh_copy_install_remote(conn, skill_id, &posix_path(source_dir), target).await
            }
        }
    }
}

fn posix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(crate) fn collect_remote_skill_files(
    snapshot: &GitHubRepoSnapshot,
    source_path: &str,
) -> Result<Vec<RemoteSkillFile>, CentralUpdatesError> {
    let mut files = snapshot
        .files
        .iter()
        .filter_map(|(repo_path, bytes)| {
            let relative_path = if source_path == "." {
                if repo_path.contains('/') {
                    return None;
                }
                repo_path.clone()
            } else {
                let prefix = format!("{}/", source_path.trim_matches('/'));
                let relative = repo_path.strip_prefix(&prefix)?;
                if relative.is_empty() {
                    return None;
                }
                relative.to_string()
            };

            Some(RemoteSkillFile {
                repo_path: repo_path.clone(),
                relative_path,
                bytes: bytes.clone(),
            })
        })
        .collect::<Vec<_>>();

    files.sort_by(|left, right| left.repo_path.cmp(&right.repo_path));
    if files.is_empty() {
        return Err(CentralUpdatesError::RepoPathUnavailable(
            source_path.to_string(),
        ));
    }
    Ok(files)
}

pub(crate) fn ensure_remote_skill_manifest(
    files: &[RemoteSkillFile],
) -> Result<(), CentralUpdatesError> {
    let has_manifest = files
        .iter()
        .any(|file| file.relative_path.eq_ignore_ascii_case("SKILL.md"));
    if has_manifest {
        Ok(())
    } else {
        Err(CentralUpdatesError::RemoteManifestMissing)
    }
}

fn write_remote_skill_files(
    files: &[RemoteSkillFile],
    target_dir: &Path,
) -> Result<(), CentralUpdatesError> {
    std::fs::create_dir_all(target_dir).map_err(|e| {
        CentralUpdatesError::io(
            format!(
                "Failed to create update staging directory '{}'",
                target_dir.display()
            ),
            e,
        )
    })?;

    for file in files {
        if !is_safe_relative_path(&file.relative_path) {
            return Err(CentralUpdatesError::UnsupportedRepoFilePath(
                file.repo_path.clone(),
            ));
        }

        let destination = target_dir.join(&file.relative_path);
        let parent = destination.parent().ok_or_else(|| {
            CentralUpdatesError::NoParentDirectory(destination.display().to_string())
        })?;
        std::fs::create_dir_all(parent).map_err(|e| {
            CentralUpdatesError::io(
                format!("Failed to create update file parent '{}'", parent.display()),
                e,
            )
        })?;
        std::fs::write(&destination, &file.bytes).map_err(|e| {
            CentralUpdatesError::io(
                format!("Failed to write update file '{}'", destination.display()),
                e,
            )
        })?;
    }

    Ok(())
}

fn replace_target_dir(
    target_dir: &Path,
    temp_dir: &Path,
    backup_dir: &Path,
) -> Result<(), CentralUpdatesError> {
    let had_target = std::fs::symlink_metadata(target_dir).is_ok();
    if had_target {
        std::fs::rename(target_dir, backup_dir).map_err(|e| {
            CentralUpdatesError::io(
                format!(
                    "Failed to stage existing skill directory '{}' for replacement",
                    target_dir.display()
                ),
                e,
            )
        })?;
    }

    if let Err(error) = std::fs::rename(temp_dir, target_dir) {
        if had_target {
            let _ = std::fs::rename(backup_dir, target_dir);
        }
        return Err(CentralUpdatesError::io(
            format!("Failed to replace skill directory '{}'", target_dir.display()),
            error,
        ));
    }

    if had_target {
        remove_path(backup_dir).map_err(|e| CentralUpdatesError::BackupCleanup {
            path: backup_dir.display().to_string(),
            message: e.to_string(),
        })?;
    }

    Ok(())
}

fn write_skill_dir_atomic_local(
    skill_id: &str,
    target_dir: &Path,
    files: &[RemoteSkillFile],
) -> Result<(), CentralUpdatesError> {
    let parent = target_dir
        .parent()
        .ok_or_else(|| CentralUpdatesError::TargetDirNoParent(skill_id.to_string()))?;
    std::fs::create_dir_all(parent).map_err(|e| {
        CentralUpdatesError::io(
            format!("Failed to create parent directory '{}'", parent.display()),
            e,
        )
    })?;

    let temp_dir = parent.join(format!(".skillport-update-{}-{}", skill_id, Uuid::new_v4()));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).map_err(|e| {
            CentralUpdatesError::io(
                format!(
                    "Failed to clear stale update directory '{}'",
                    temp_dir.display()
                ),
                e,
            )
        })?;
    }

    write_remote_skill_files(files, &temp_dir)?;

    let backup_dir = parent.join(format!(".skillport-backup-{}-{}", skill_id, Uuid::new_v4()));
    replace_target_dir(target_dir, &temp_dir, &backup_dir)
}

async fn write_skill_dir_atomic_remote(
    conn: &ConnectedRemoteTarget,
    skill_id: &str,
    target_dir: &str,
    files: &[RemoteSkillFile],
) -> Result<(), CentralUpdatesError> {
    let parent =
        remote_parent(target_dir).ok_or_else(|| CentralUpdatesError::RemoteTargetDirNoParent {
            skill_id: skill_id.to_string(),
            target_dir: target_dir.to_string(),
        })?;

    let staging_dir = format!(
        "{}/.skillport-update-{}-{}",
        parent.trim_end_matches('/'),
        skill_id,
        Uuid::new_v4()
    );
    let backup_dir = format!(
        "{}/.skillport-backup-{}-{}",
        parent.trim_end_matches('/'),
        skill_id,
        Uuid::new_v4()
    );

    for file in files {
        if !is_safe_relative_path(&file.relative_path) {
            return Err(CentralUpdatesError::UnsupportedRepoFilePath(
                file.repo_path.clone(),
            ));
        }
    }

    let archive = build_skill_archive(files)?;
    conn.run_command_with_stdin_bytes(
        &remote_update_command(target_dir, &parent, &staging_dir, &backup_dir),
        &archive,
    )
    .map(|_| ())
    .map_err(|err| {
        // The script aborts before staging touches `target_dir`, so the
        // canonical directory remains untouched on early failures.
        CentralUpdatesError::Remote(format!(
            "Remote update script failed for '{}': {}",
            target_dir, err
        ))
    })
}

fn refresh_copy_install_local(
    skill_id: &str,
    source_dir: &Path,
    target: &str,
) -> Result<(), CentralUpdatesError> {
    let target_path = PathBuf::from(target);
    if target_path.file_name().and_then(|value| value.to_str()) != Some(skill_id) {
        return Err(CentralUpdatesError::CopyInstallOutsideSkillDir(
            target_path.display().to_string(),
        ));
    }
    if std::fs::symlink_metadata(&target_path).is_ok() {
        remove_path(&target_path)?;
    }
    copy_dir_all(source_dir, &target_path)?;
    Ok(())
}

async fn refresh_copy_install_remote(
    conn: &ConnectedRemoteTarget,
    skill_id: &str,
    source_dir: &str,
    target: &str,
) -> Result<(), CentralUpdatesError> {
    let basename = target.trim_end_matches('/').rsplit('/').next();
    if basename != Some(skill_id) {
        return Err(CentralUpdatesError::CopyInstallOutsideSkillDir(
            target.to_string(),
        ));
    }
    conn.run_script(REMOTE_REFRESH_COPY_SCRIPT, &[source_dir, target, skill_id])
        .await
        .map(|_| ())
        .map_err(|e| CentralUpdatesError::Remote(e.to_string()))
}

#[cfg_attr(not(test), allow(dead_code))]
fn build_skill_archive(files: &[RemoteSkillFile]) -> Result<Vec<u8>, CentralUpdatesError> {
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = tar::Builder::new(encoder);
    for file in files {
        if !is_safe_relative_path(&file.relative_path) {
            return Err(CentralUpdatesError::UnsupportedRepoFilePath(
                file.repo_path.clone(),
            ));
        }
        let mut header = tar::Header::new_gnu();
        header.set_size(file.bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, &file.relative_path, file.bytes.as_slice())
            .map_err(|e| {
                CentralUpdatesError::io(
                    format!("Failed to build update archive entry '{}'", file.repo_path),
                    e,
                )
            })?;
    }
    let encoder = builder
        .into_inner()
        .map_err(|e| CentralUpdatesError::io("Failed to finalize update archive", e))?;
    encoder
        .finish()
        .map_err(|e| CentralUpdatesError::io("Failed to compress update archive", e))
}

#[cfg_attr(not(test), allow(dead_code))]
fn remote_update_command(
    target_dir: &str,
    parent_dir: &str,
    staging_dir: &str,
    backup_dir: &str,
) -> String {
    format!(
        "sh -c {} -- {} {} {} {}",
        shell_quote(REMOTE_CENTRAL_UPDATE_SCRIPT),
        shell_quote(target_dir),
        shell_quote(parent_dir),
        shell_quote(staging_dir),
        shell_quote(backup_dir)
    )
}

pub(crate) fn hash_remote_files(
    _snapshot: &GitHubRepoSnapshot,
    files: &[RemoteSkillFile],
) -> Result<String, CentralUpdatesError> {
    let mut entries = Vec::with_capacity(files.len());
    for file in files {
        let digest = Sha256::digest(&file.bytes);
        entries.push((file.relative_path.clone(), hex_digest(&digest)));
    }
    Ok(hash_entries(entries))
}

fn hash_local_directory(root: &Path) -> Result<String, CentralUpdatesError> {
    if !root.exists() {
        return Ok(hash_entries(Vec::new()));
    }
    let mut entries = Vec::new();
    collect_local_hash_entries(root, root, &mut entries)?;
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(hash_entries(entries))
}

/// BFS-walk a remote directory and return a digest comparable to
/// [`hash_local_directory`]. Symlinks and special entries are skipped, only
/// regular files contribute to the hash.
async fn hash_remote_directory(
    conn: &ConnectedRemoteTarget,
    root: &str,
) -> Result<String, CentralUpdatesError> {
    if !conn
        .exists(root)
        .await
        .map_err(|e| CentralUpdatesError::Remote(e.to_string()))?
    {
        // Treat a missing canonical directory as an empty hash so the upper
        // layer can still mark this skill as `update_available`.
        return Ok(hash_entries(Vec::new()));
    }

    let mut entries: Vec<(String, String)> = Vec::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(root.to_string());

    while let Some(current) = queue.pop_front() {
        let dir_entries = conn
            .list_dir(&current)
            .await
            .map_err(|e| CentralUpdatesError::Remote(e.to_string()))?;
        for entry in dir_entries {
            let child_path = format!("{}/{}", current.trim_end_matches('/'), entry.name);
            match entry.file_type.as_str() {
                "dir" => queue.push_back(child_path),
                "file" => {
                    let relative = remote_relative_path(root, &child_path)?;
                    if !is_safe_relative_path(&relative) {
                        return Err(CentralUpdatesError::UnsupportedRemotePath(child_path));
                    }
                    let bytes = conn
                        .read_file(&child_path)
                        .await
                        .map_err(|e| CentralUpdatesError::Remote(e.to_string()))?;
                    let digest = Sha256::digest(&bytes);
                    entries.push((relative, hex_digest(&digest)));
                }
                // Skip symlinks and `other` entries — they should not exist
                // inside a managed central skill, and ignoring them keeps the
                // hash stable when sshd reports stale metadata.
                _ => {}
            }
        }
    }

    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(hash_entries(entries))
}

async fn hash_remote_directories(
    conn: &ConnectedRemoteTarget,
    roots: &[PathBuf],
) -> Result<HashMap<PathBuf, String>, CentralUpdatesError> {
    let mut hashes = HashMap::with_capacity(roots.len());
    for chunk in roots.chunks(32) {
        let root_args = chunk
            .iter()
            .map(|root| posix_path(root))
            .collect::<Vec<_>>();
        match conn
            .run_script(
                REMOTE_HASH_SCRIPT,
                &root_args.iter().map(String::as_str).collect::<Vec<_>>(),
            )
            .await
        {
            Ok(output) => {
                let parsed = parse_remote_hash_output(&output)?;
                for (root, hash) in chunk.iter().zip(root_args.iter().map(|arg| {
                    parsed
                        .get(arg)
                        .cloned()
                        .unwrap_or_else(|| hash_entries(Vec::new()))
                })) {
                    hashes.insert(root.clone(), hash);
                }
            }
            Err(error) if error.to_string().contains(REMOTE_HASH_UNSUPPORTED_EXIT_CODE) => {
                tracing::warn!(
                    error = %error,
                    "Remote target has no sha256 tool; falling back to per-file SSH hashing"
                );
                for root in chunk {
                    hashes.insert(
                        root.clone(),
                        hash_remote_directory(conn, &posix_path(root)).await?,
                    );
                }
            }
            Err(error) => return Err(CentralUpdatesError::Remote(error.to_string())),
        }
    }
    Ok(hashes)
}

#[cfg_attr(not(test), allow(dead_code))]
fn parse_remote_hash_output(
    output: &str,
) -> Result<HashMap<String, String>, CentralUpdatesError> {
    let mut hashes = HashMap::new();
    let mut current_root: Option<String> = None;
    let mut entries: Vec<(String, String)> = Vec::new();

    for line in output.lines() {
        if let Some(root) = line.strip_prefix("ROOT\t") {
            if let Some(previous) = current_root.replace(root.to_string()) {
                hashes.insert(previous, hash_entries(std::mem::take(&mut entries)));
            }
            continue;
        }
        if let Some(root) = line.strip_prefix("END\t") {
            let active = current_root.take().ok_or_else(|| {
                CentralUpdatesError::RemoteHashOutput(format!(
                    "Remote hash output ended root '{}' before it started.",
                    root
                ))
            })?;
            if active != root {
                return Err(CentralUpdatesError::RemoteHashOutput(format!(
                    "Remote hash output root mismatch: started '{}', ended '{}'.",
                    active, root
                )));
            }
            hashes.insert(active, hash_entries(std::mem::take(&mut entries)));
            continue;
        }
        if current_root.is_none() || line.trim().is_empty() {
            continue;
        }
        if let Some((digest, path)) = parse_remote_hash_line(line) {
            entries.push((path, digest));
        }
    }

    if let Some(root) = current_root {
        hashes.insert(root, hash_entries(entries));
    }

    Ok(hashes)
}

#[cfg_attr(not(test), allow(dead_code))]
fn parse_remote_hash_line(line: &str) -> Option<(String, String)> {
    let (digest, path) = line.split_once('\t').or_else(|| line.split_once("  "))?;
    let digest = digest.trim().trim_start_matches("(stdin)=").to_string();
    if digest.len() != 64 || !digest.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let path = path
        .trim()
        .trim_start_matches('*')
        .trim_start_matches("./")
        .to_string();
    if path.is_empty() || !is_safe_relative_path(&path) {
        return None;
    }
    Some((digest.to_ascii_lowercase(), path))
}

fn remote_relative_path(root: &str, child: &str) -> Result<String, CentralUpdatesError> {
    let normalized_root = root.trim_end_matches('/');
    let prefix = format!("{}/", normalized_root);
    child
        .strip_prefix(&prefix)
        .map(|rel| rel.to_string())
        .or_else(|| {
            // Allow `child == root` when the file lives directly under `root`
            // and `list_dir` returned its own entry.
            if child == normalized_root {
                Some(String::new())
            } else {
                None
            }
        })
        .ok_or_else(|| CentralUpdatesError::RemotePathOutsideRoot {
            child: child.to_string(),
            root: root.to_string(),
        })
}

fn collect_local_hash_entries(
    root: &Path,
    current: &Path,
    entries: &mut Vec<(String, String)>,
) -> Result<(), CentralUpdatesError> {
    for entry in std::fs::read_dir(current).map_err(|e| {
        CentralUpdatesError::io(
            format!(
                "Failed to read local skill directory '{}'",
                current.display()
            ),
            e,
        )
    })? {
        let entry = entry
            .map_err(|e| CentralUpdatesError::io("Failed to read local skill entry", e))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| {
            CentralUpdatesError::io(
                format!("Failed to inspect local skill entry '{}'", path.display()),
                e,
            )
        })?;
        if file_type.is_dir() {
            collect_local_hash_entries(root, &path, entries)?;
        } else if file_type.is_file() {
            let relative_path = relative_path_string(root, &path)?;
            let bytes = std::fs::read(&path).map_err(|e| {
                CentralUpdatesError::io(
                    format!("Failed to read local skill file '{}'", path.display()),
                    e,
                )
            })?;
            let digest = Sha256::digest(&bytes);
            entries.push((relative_path, hex_digest(&digest)));
        }
    }
    Ok(())
}

fn hash_entries(mut entries: Vec<(String, String)>) -> String {
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hasher = Sha256::new();
    for (path, digest) in entries {
        hasher.update(path.as_bytes());
        hasher.update([0xff]);
        hasher.update(digest.as_bytes());
        hasher.update([0xfe]);
    }
    let digest = hasher.finalize();
    format!("sha256-manifest:{}", hex_digest(&digest))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn relative_path_string(root: &Path, path: &Path) -> Result<String, CentralUpdatesError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|e| CentralUpdatesError::RelativePath {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;
    let parts = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value.to_string_lossy().into_owned()),
            _ => Err(CentralUpdatesError::UnsupportedLocalPath(
                path.display().to_string(),
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(parts.join("/"))
}

pub(crate) fn normalize_repo_path(path: &str) -> Result<String, CentralUpdatesError> {
    let normalized = path.trim().trim_matches('/').replace('\\', "/");
    let normalized = if normalized.is_empty() {
        ".".to_string()
    } else {
        normalized
    };
    if !is_safe_repo_path(&normalized) {
        return Err(CentralUpdatesError::UnsupportedRepoPath(path.to_string()));
    }
    Ok(normalized)
}

fn is_safe_repo_path(path: &str) -> bool {
    path == "." || is_safe_relative_path(path)
}

fn is_safe_relative_path(path: &str) -> bool {
    let relative = Path::new(path);
    !relative.is_absolute()
        && relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn remove_path(path: &Path) -> Result<(), CentralUpdatesError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => remove_symlink_path(path),
        Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(path).map_err(|e| {
            CentralUpdatesError::io(
                format!("Failed to remove directory '{}'", path.display()),
                e,
            )
        }),
        Ok(_) => std::fs::remove_file(path).map_err(|e| {
            CentralUpdatesError::io(format!("Failed to remove file '{}'", path.display()), e)
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(CentralUpdatesError::io(
            format!("Failed to inspect '{}'", path.display()),
            error,
        )),
    }
}

#[cfg(windows)]
fn remove_symlink_path(path: &Path) -> Result<(), CentralUpdatesError> {
    std::fs::remove_dir(path).map_err(|e| {
        CentralUpdatesError::io(format!("Failed to remove symlink '{}'", path.display()), e)
    })
}

#[cfg(not(windows))]
fn remove_symlink_path(path: &Path) -> Result<(), CentralUpdatesError> {
    std::fs::remove_file(path).map_err(|e| {
        CentralUpdatesError::io(format!("Failed to remove symlink '{}'", path.display()), e)
    })
}
