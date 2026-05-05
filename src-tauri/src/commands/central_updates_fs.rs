//! 中央技能更新使用的本地/远程文件系统层。
//!
//! 把无 DB、无 Tauri 状态的纯文件 IO/哈希/路径工具集中放在这里，让
//! `central_updates.rs` 专注于 commands 与 orchestration。两侧（本地与
//! 远程）共享同一组接口，避免上层在 target 类型上分支。

use std::collections::VecDeque;
use std::path::{Component, Path, PathBuf};

use uuid::Uuid;

use crate::targets::{connect_ssh_target, remote_parent, ActiveTarget, ConnectedSshTarget};

use super::github_import::GitHubRepoSnapshot;
use super::linker;

/// Atomically replace a remote skill directory with newly written staging
/// content. `$1` is the canonical directory; `$2` is the staging directory
/// already populated with new files; `$3` is a backup path used to roll back
/// when `mv` fails.
const REMOTE_CENTRAL_UPDATE_SCRIPT: &str = r#"
set -eu

target_dir=$1
staging_dir=$2
backup_dir=$3

if [ -e "$target_dir" ]; then
  mv "$target_dir" "$backup_dir"
fi

if mv "$staging_dir" "$target_dir"; then
  rm -rf -- "$backup_dir" 2>/dev/null || true
else
  rc=$?
  if [ -e "$backup_dir" ]; then mv "$backup_dir" "$target_dir"; fi
  exit "$rc"
fi
"#;

#[derive(Debug, Clone)]
pub(super) struct RemoteSkillFile {
    pub repo_path: String,
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

/// Filesystem façade used by Central skill updates.
///
/// Local mode operates on `std::fs` paths; SSH mode delegates to a connected
/// remote and runs equivalent shell-side primitives. Both modes intentionally
/// expose the same operations so the orchestration code in
/// [`super::central_updates`] never branches on target type.
pub(super) enum CentralFs {
    Local,
    Remote(Box<ConnectedSshTarget>),
}

impl CentralFs {
    pub(super) async fn from_active_target(target: ActiveTarget) -> Result<Self, String> {
        match target {
            ActiveTarget::Local => Ok(Self::Local),
            ActiveTarget::Ssh(remote) => {
                let conn = connect_ssh_target(&remote).await?;
                Ok(Self::Remote(Box::new(conn)))
            }
        }
    }

    /// Compute a stable hash over every regular file under `root`.
    ///
    /// The remote variant only reads `file` entries (skipping symlinks and
    /// directories) and feeds them through the shared [`hash_entries`] helper
    /// so local and remote skills produce comparable digests.
    pub(super) async fn hash_directory(&self, root: &Path) -> Result<String, String> {
        match self {
            Self::Local => hash_local_directory(root),
            Self::Remote(conn) => hash_remote_directory(conn, &posix_path(root)).await,
        }
    }

    /// Atomically replace `target_dir` with the contents of `files`.
    ///
    /// Both variants write into a sibling staging directory first, then swap
    /// it into place using a rename pair so a failure mid-write never leaves
    /// the canonical directory empty.
    pub(super) async fn write_skill_dir_atomic(
        &self,
        skill_id: &str,
        target_dir: &Path,
        files: &[RemoteSkillFile],
    ) -> Result<(), String> {
        match self {
            Self::Local => write_skill_dir_atomic_local(skill_id, target_dir, files),
            Self::Remote(conn) => {
                write_skill_dir_atomic_remote(conn, skill_id, &posix_path(target_dir), files).await
            }
        }
    }

    /// Refresh a copy-installation so that `target` mirrors `source_dir`.
    ///
    /// `target` must end with `skill_id` to keep the operation scoped to the
    /// expected installation slot. The remote variant uses `cp -R` over SSH;
    /// the local variant delegates to [`linker::copy_dir_all`].
    pub(super) async fn refresh_copy_install(
        &self,
        skill_id: &str,
        source_dir: &Path,
        target: &str,
    ) -> Result<(), String> {
        match self {
            Self::Local => refresh_copy_install_local(skill_id, source_dir, target),
            Self::Remote(conn) => {
                refresh_copy_install_remote(conn, skill_id, &posix_path(source_dir), target).await
            }
        }
    }
}

fn posix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(super) fn collect_remote_skill_files(
    snapshot: &GitHubRepoSnapshot,
    source_path: &str,
) -> Result<Vec<RemoteSkillFile>, String> {
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
        return Err(format!(
            "Repository path '{}' is no longer available.",
            source_path
        ));
    }
    Ok(files)
}

pub(super) fn ensure_remote_skill_manifest(files: &[RemoteSkillFile]) -> Result<(), String> {
    let has_manifest = files
        .iter()
        .any(|file| file.relative_path.eq_ignore_ascii_case("SKILL.md"));
    if has_manifest {
        Ok(())
    } else {
        Err("Remote skill no longer contains SKILL.md.".to_string())
    }
}

fn write_remote_skill_files(files: &[RemoteSkillFile], target_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(target_dir).map_err(|e| {
        format!(
            "Failed to create update staging directory '{}': {}",
            target_dir.display(),
            e
        )
    })?;

    for file in files {
        if !is_safe_relative_path(&file.relative_path) {
            return Err(format!(
                "Repository contains an unsupported path '{}'.",
                file.repo_path
            ));
        }

        let destination = target_dir.join(&file.relative_path);
        let parent = destination.parent().ok_or_else(|| {
            format!(
                "Failed to determine parent directory for '{}'.",
                destination.display()
            )
        })?;
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create update file parent '{}': {}",
                parent.display(),
                e
            )
        })?;
        std::fs::write(&destination, &file.bytes).map_err(|e| {
            format!(
                "Failed to write update file '{}': {}",
                destination.display(),
                e
            )
        })?;
    }

    Ok(())
}

fn replace_target_dir(target_dir: &Path, temp_dir: &Path, backup_dir: &Path) -> Result<(), String> {
    let had_target = std::fs::symlink_metadata(target_dir).is_ok();
    if had_target {
        std::fs::rename(target_dir, backup_dir).map_err(|e| {
            format!(
                "Failed to stage existing skill directory '{}' for replacement: {}",
                target_dir.display(),
                e
            )
        })?;
    }

    if let Err(error) = std::fs::rename(temp_dir, target_dir) {
        if had_target {
            let _ = std::fs::rename(backup_dir, target_dir);
        }
        return Err(format!(
            "Failed to replace skill directory '{}': {}",
            target_dir.display(),
            error
        ));
    }

    if had_target {
        remove_path(backup_dir).map_err(|e| {
            format!(
                "Updated skill directory, but failed to remove backup '{}': {}",
                backup_dir.display(),
                e
            )
        })?;
    }

    Ok(())
}

fn write_skill_dir_atomic_local(
    skill_id: &str,
    target_dir: &Path,
    files: &[RemoteSkillFile],
) -> Result<(), String> {
    let parent = target_dir
        .parent()
        .ok_or_else(|| format!("Skill '{}' target directory has no parent.", skill_id))?;
    std::fs::create_dir_all(parent).map_err(|e| {
        format!(
            "Failed to create parent directory '{}': {}",
            parent.display(),
            e
        )
    })?;

    let temp_dir = parent.join(format!(".skillport-update-{}-{}", skill_id, Uuid::new_v4()));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).map_err(|e| {
            format!(
                "Failed to clear stale update directory '{}': {}",
                temp_dir.display(),
                e
            )
        })?;
    }

    write_remote_skill_files(files, &temp_dir)?;

    let backup_dir = parent.join(format!(".skillport-backup-{}-{}", skill_id, Uuid::new_v4()));
    replace_target_dir(target_dir, &temp_dir, &backup_dir)
}

async fn write_skill_dir_atomic_remote(
    conn: &ConnectedSshTarget,
    skill_id: &str,
    target_dir: &str,
    files: &[RemoteSkillFile],
) -> Result<(), String> {
    let parent = remote_parent(target_dir).ok_or_else(|| {
        format!(
            "Skill '{}' target directory '{}' has no parent.",
            skill_id, target_dir
        )
    })?;
    conn.mkdir_p(&parent).await?;

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

    if conn.exists(&staging_dir).await? {
        conn.remove_tree(&staging_dir).await?;
    }
    conn.mkdir_p(&staging_dir).await?;

    for file in files {
        if !is_safe_relative_path(&file.relative_path) {
            // Best-effort cleanup before bailing out so we never leave a half
            // populated staging directory behind.
            let _ = conn.remove_tree(&staging_dir).await;
            return Err(format!(
                "Repository contains an unsupported path '{}'.",
                file.repo_path
            ));
        }
        let destination = format!(
            "{}/{}",
            staging_dir.trim_end_matches('/'),
            file.relative_path.trim_start_matches('/')
        );
        // `write_file` already mkdir_p's the parent, but we call it here too
        // so a missing intermediate segment surfaces a helpful error.
        if let Some(file_parent) = remote_parent(&destination) {
            conn.mkdir_p(&file_parent).await?;
        }
        conn.write_file(&destination, &file.bytes).await?;
    }

    conn.run_script(
        REMOTE_CENTRAL_UPDATE_SCRIPT,
        &[target_dir, &staging_dir, &backup_dir],
    )
    .await
    .map(|_| ())
    .map_err(|err| {
        // The script aborts before staging touches `target_dir`, so the
        // canonical directory remains untouched on early failures.
        format!(
            "Remote update script failed for '{}': {}",
            target_dir, err
        )
    })
}

fn refresh_copy_install_local(
    skill_id: &str,
    source_dir: &Path,
    target: &str,
) -> Result<(), String> {
    let target_path = PathBuf::from(target);
    if target_path.file_name().and_then(|value| value.to_str()) != Some(skill_id) {
        return Err(format!(
            "Refusing to refresh copy install outside expected skill directory '{}'.",
            target_path.display()
        ));
    }
    if std::fs::symlink_metadata(&target_path).is_ok() {
        remove_path(&target_path)?;
    }
    linker::copy_dir_all(source_dir, &target_path)
}

async fn refresh_copy_install_remote(
    conn: &ConnectedSshTarget,
    skill_id: &str,
    source_dir: &str,
    target: &str,
) -> Result<(), String> {
    let basename = target.trim_end_matches('/').rsplit('/').next();
    if basename != Some(skill_id) {
        return Err(format!(
            "Refusing to refresh copy install outside expected skill directory '{}'.",
            target
        ));
    }
    if conn.exists(target).await? {
        conn.remove_tree(target).await?;
    }
    conn.copy_dir(source_dir, target).await
}

pub(super) fn hash_remote_files(
    _snapshot: &GitHubRepoSnapshot,
    files: &[RemoteSkillFile],
) -> Result<String, String> {
    let mut entries = Vec::with_capacity(files.len());
    for file in files {
        entries.push((file.relative_path.clone(), file.bytes.clone()));
    }
    Ok(hash_entries(entries))
}

fn hash_local_directory(root: &Path) -> Result<String, String> {
    let mut entries = Vec::new();
    collect_local_hash_entries(root, root, &mut entries)?;
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(hash_entries(entries))
}

/// BFS-walk a remote directory and return a digest comparable to
/// [`hash_local_directory`]. Symlinks and special entries are skipped, only
/// regular files contribute to the hash.
async fn hash_remote_directory(conn: &ConnectedSshTarget, root: &str) -> Result<String, String> {
    if !conn.exists(root).await? {
        // Treat a missing canonical directory as an empty hash so the upper
        // layer can still mark this skill as `update_available`.
        return Ok(hash_entries(Vec::new()));
    }

    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(root.to_string());

    while let Some(current) = queue.pop_front() {
        let dir_entries = conn.list_dir(&current).await?;
        for entry in dir_entries {
            let child_path = format!("{}/{}", current.trim_end_matches('/'), entry.name);
            match entry.file_type.as_str() {
                "dir" => queue.push_back(child_path),
                "file" => {
                    let relative = remote_relative_path(root, &child_path)?;
                    if !is_safe_relative_path(&relative) {
                        return Err(format!(
                            "Remote skill path '{}' contains unsupported components.",
                            child_path
                        ));
                    }
                    let bytes = conn.read_file(&child_path).await?;
                    entries.push((relative, bytes));
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

fn remote_relative_path(root: &str, child: &str) -> Result<String, String> {
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
        .ok_or_else(|| {
            format!(
                "Remote path '{}' is not under expected root '{}'.",
                child, root
            )
        })
}

fn collect_local_hash_entries(
    root: &Path,
    current: &Path,
    entries: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(current).map_err(|e| {
        format!(
            "Failed to read local skill directory '{}': {}",
            current.display(),
            e
        )
    })? {
        let entry = entry.map_err(|e| format!("Failed to read local skill entry: {}", e))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "Failed to inspect local skill entry '{}': {}",
                path.display(),
                e
            )
        })?;
        if file_type.is_dir() {
            collect_local_hash_entries(root, &path, entries)?;
        } else if file_type.is_file() {
            let relative_path = relative_path_string(root, &path)?;
            let bytes = std::fs::read(&path).map_err(|e| {
                format!(
                    "Failed to read local skill file '{}': {}",
                    path.display(),
                    e
                )
            })?;
            entries.push((relative_path, bytes));
        }
    }
    Ok(())
}

fn hash_entries(mut entries: Vec<(String, Vec<u8>)>) -> String {
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hash = 0xcbf29ce484222325u64;
    for (path, bytes) in entries {
        hash = fnv1a(hash, path.as_bytes());
        hash = fnv1a(hash, &[0xff]);
        hash = fnv1a(hash, &bytes);
        hash = fnv1a(hash, &[0xfe]);
    }
    format!("fnv1a64:{hash:016x}")
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn relative_path_string(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path.strip_prefix(root).map_err(|e| {
        format!(
            "Failed to compute relative path for '{}': {}",
            path.display(),
            e
        )
    })?;
    let parts = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value.to_string_lossy().into_owned()),
            _ => Err(format!(
                "Local skill path '{}' contains unsupported components.",
                path.display()
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(parts.join("/"))
}

pub(super) fn normalize_repo_path(path: &str) -> Result<String, String> {
    let normalized = path.trim().trim_matches('/').replace('\\', "/");
    let normalized = if normalized.is_empty() {
        ".".to_string()
    } else {
        normalized
    };
    if !is_safe_repo_path(&normalized) {
        return Err(format!("Repository path '{}' is not supported.", path));
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

fn remove_path(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => remove_symlink_path(path),
        Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to remove directory '{}': {}", path.display(), e)),
        Ok(_) => std::fs::remove_file(path)
            .map_err(|e| format!("Failed to remove file '{}': {}", path.display(), e)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to inspect '{}': {}", path.display(), error)),
    }
}

#[cfg(windows)]
fn remove_symlink_path(path: &Path) -> Result<(), String> {
    std::fs::remove_dir(path)
        .map_err(|e| format!("Failed to remove symlink '{}': {}", path.display(), e))
}

#[cfg(not(windows))]
fn remove_symlink_path(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path)
        .map_err(|e| format!("Failed to remove symlink '{}': {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn hash_entries_is_stable_across_input_order() {
        let left = hash_entries(vec![
            ("b.txt".to_string(), b"two".to_vec()),
            ("a.txt".to_string(), b"one".to_vec()),
        ]);
        let right = hash_entries(vec![
            ("a.txt".to_string(), b"one".to_vec()),
            ("b.txt".to_string(), b"two".to_vec()),
        ]);

        assert_eq!(left, right);
    }

    #[test]
    fn local_hash_changes_when_file_content_changes() {
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("demo");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), b"one").unwrap();

        let first = hash_local_directory(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), b"two").unwrap();
        let second = hash_local_directory(&skill_dir).unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn collect_remote_skill_files_requires_source_path() {
        let snapshot = GitHubRepoSnapshot {
            files: HashMap::from([(
                "skills/demo/SKILL.md".to_string(),
                b"---\nname: Demo\n---".to_vec(),
            )]),
        };

        let files = collect_remote_skill_files(&snapshot, "skills/demo").unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "SKILL.md");
    }
}
