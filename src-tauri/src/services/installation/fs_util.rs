//! Filesystem utilities for skill installation:
//! relative-path computation, platform-specific symlink primitives,
//! and recursive directory copy.

use std::path::{Path, PathBuf};

use crate::services::resource_budget::ResourceBudget;

use super::error::InstallationError;

pub(crate) use super::directory_link::{
    create_skills_cli_directory_link, inspect_managed_directory_link, is_reparse_or_symlink,
    observe_directory_slot, remove_directory_link_slot, remove_verified_directory_link,
    slot_is_directory_link, DirectorySlotObservation, ManagedDirectoryLinkKind, REASON_BROKEN_LINK,
    REASON_NOT_A_DIRECTORY, REASON_WRONG_LINK_TARGET,
};

/// Run a synchronous filesystem task on the blocking-thread pool with
/// installation-domain errors. Thin typed wrapper over
/// [`crate::fs_util::run_blocking_fs_with`].
pub(crate) async fn run_blocking_fs<T, F>(
    label: &'static str,
    task: F,
) -> Result<T, InstallationError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, InstallationError> + Send + 'static,
{
    crate::fs_util::run_blocking_fs_with(label, task, InstallationError::task_join).await
}

pub(crate) async fn path_exists_blocking(path: &Path) -> Result<bool, InstallationError> {
    let path = path.to_path_buf();
    run_blocking_fs("path existence check", move || Ok(path.exists())).await
}

pub(crate) fn resolved_symlink_target(link_path: &Path, raw_target: &Path) -> PathBuf {
    if raw_target.is_absolute() {
        raw_target.to_path_buf()
    } else {
        link_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(raw_target)
    }
}

pub(crate) fn symlink_points_to(
    link_path: &Path,
    expected_target: &Path,
) -> Result<bool, InstallationError> {
    let metadata = match std::fs::symlink_metadata(link_path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(false),
    };
    if !metadata.file_type().is_symlink() {
        return Ok(false);
    }

    let raw_target = std::fs::read_link(link_path).map_err(|e| {
        InstallationError::io(
            format!("Failed to inspect symlink target '{}'", link_path.display()),
            e,
        )
    })?;
    let resolved_target = resolved_symlink_target(link_path, &raw_target);
    Ok(crate::paths::paths_equivalent(
        &resolved_target,
        expected_target,
    ))
}

pub(crate) fn dirs_have_same_contents(
    left: &Path,
    right: &Path,
) -> Result<bool, InstallationError> {
    let left_metadata = match std::fs::symlink_metadata(left) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(false),
    };
    let right_metadata = match std::fs::symlink_metadata(right) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(false),
    };

    if left_metadata.file_type().is_symlink() || right_metadata.file_type().is_symlink() {
        return Ok(false);
    }
    if left_metadata.is_dir() != right_metadata.is_dir() {
        return Ok(false);
    }
    if left_metadata.is_file() != right_metadata.is_file() {
        return Ok(false);
    }

    if left_metadata.is_file() {
        let left_bytes = std::fs::read(left).map_err(|e| {
            InstallationError::io(format!("Failed to read '{}'", left.display()), e)
        })?;
        let right_bytes = std::fs::read(right).map_err(|e| {
            InstallationError::io(format!("Failed to read '{}'", right.display()), e)
        })?;
        return Ok(left_bytes == right_bytes);
    }

    if !left_metadata.is_dir() {
        return Ok(false);
    }

    let mut left_entries = directory_entry_names(left)?;
    let mut right_entries = directory_entry_names(right)?;
    left_entries.sort();
    right_entries.sort();
    if left_entries != right_entries {
        return Ok(false);
    }

    for name in left_entries {
        if !dirs_have_same_contents(&left.join(&name), &right.join(&name))? {
            return Ok(false);
        }
    }

    Ok(true)
}

fn directory_entry_names(dir: &Path) -> Result<Vec<std::ffi::OsString>, InstallationError> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        InstallationError::io(format!("Failed to read directory '{}'", dir.display()), e)
    })?;
    let mut names = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|e| InstallationError::io("Failed to read directory entry", e))?;
        names.push(entry.file_name());
    }
    Ok(names)
}

/// Compute a relative path from `from_dir` to `to_path`.
///
/// Both paths must be absolute. The resulting path can be used as a symlink
/// target placed inside `from_dir`.
///
/// Examples:
/// - `make_relative_path("/a/b/c", "/a/d/e/f")` -> `"../../d/e/f"`
/// - `make_relative_path("/home/user/.claude/skills", "/home/user/.agents/skills/my-skill")`
///   -> `"../../.agents/skills/my-skill"`
pub fn make_relative_path(from_dir: &Path, to_path: &Path) -> PathBuf {
    let from_components: Vec<_> = from_dir.components().collect();
    let to_components: Vec<_> = to_path.components().collect();

    // Find the length of the common path prefix.
    let common_len = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // Number of ".." hops needed to climb out of `from_dir`.
    let up_count = from_components.len() - common_len;

    let mut result = PathBuf::new();
    for _ in 0..up_count {
        result.push("..");
    }
    for component in &to_components[common_len..] {
        result.push(component.as_os_str());
    }

    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    }
}

// ─── Platform-specific symlink creation ──────────────────────────────────────

#[cfg(unix)]
pub fn create_symlink(target: &Path, link: &Path) -> Result<(), InstallationError> {
    std::os::unix::fs::symlink(target, link).map_err(InstallationError::SymlinkCreate)
}

#[cfg(windows)]
pub fn create_symlink(target: &Path, link: &Path) -> Result<(), InstallationError> {
    std::os::windows::fs::symlink_dir(target, link).map_err(InstallationError::SymlinkCreate)
}

#[cfg(not(any(unix, windows)))]
pub fn create_symlink(_target: &Path, _link: &Path) -> Result<(), InstallationError> {
    Err(InstallationError::SymlinkUnsupported)
}

/// Remove the symlink entry at `path`. Callers attach their own operation
/// context (e.g. "Failed to remove existing symlink").
#[cfg(windows)]
pub(crate) fn remove_symlink_path(path: &Path) -> std::io::Result<()> {
    std::fs::remove_dir(path)
}

#[cfg(not(windows))]
pub(crate) fn remove_symlink_path(path: &Path) -> std::io::Result<()> {
    std::fs::remove_file(path)
}

/// Choose between an absolute path (cross-volume on Windows) or a relative
/// path. Used to compute symlink target strings.
pub fn symlink_target_path(from_dir: &Path, to_path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let from_prefix = from_dir.components().next();
        let to_prefix = to_path.components().next();
        if from_prefix != to_prefix {
            return to_path.to_path_buf();
        }
    }

    make_relative_path(from_dir, to_path)
}

// ─── Recursive Directory Copy ─────────────────────────────────────────────────

#[derive(Debug)]
struct CopyBudgetTracker {
    limits: ResourceBudget,
    root: PathBuf,
    remaining_entries: usize,
    remaining_bytes: u64,
}

impl CopyBudgetTracker {
    fn new(root: &Path, limits: ResourceBudget) -> Self {
        Self {
            remaining_entries: limits.copy_entries,
            remaining_bytes: limits.copy_bytes,
            limits,
            root: root.to_path_buf(),
        }
    }

    fn consume_entry(&mut self) -> Result<(), InstallationError> {
        if self.remaining_entries == 0 {
            return Err(InstallationError::CopyEntryBudgetExceeded {
                root: self.root.display().to_string(),
                limit: self.limits.copy_entries,
            });
        }
        self.remaining_entries -= 1;
        Ok(())
    }

    fn consume_file_bytes(&mut self, path: &Path, bytes: u64) -> Result<(), InstallationError> {
        if bytes > self.remaining_bytes {
            return Err(InstallationError::CopyByteBudgetExceeded {
                root: self.root.display().to_string(),
                limit: self.limits.copy_bytes,
                path: path.display().to_string(),
            });
        }
        self.remaining_bytes -= bytes;
        Ok(())
    }
}

/// Recursively copy a directory tree from `src` to `dst`.
///
/// `dst` must not exist prior to the call (or may be an empty dir).
/// The behaviour mirrors `cp -r src dst` on Unix.
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), InstallationError> {
    copy_dir_all_with_budget(src, dst, ResourceBudget::default_skill())
}

pub(crate) fn copy_dir_all_with_budget(
    src: &Path,
    dst: &Path,
    limits: ResourceBudget,
) -> Result<(), InstallationError> {
    let mut tracker = CopyBudgetTracker::new(src, limits);
    copy_dir_all_with_tracker(src, dst, &mut tracker)
}

fn copy_dir_all_with_tracker(
    src: &Path,
    dst: &Path,
    tracker: &mut CopyBudgetTracker,
) -> Result<(), InstallationError> {
    std::fs::create_dir_all(dst).map_err(|e| {
        InstallationError::io(
            format!("Failed to create destination directory '{}'", dst.display()),
            e,
        )
    })?;

    for entry in std::fs::read_dir(src).map_err(|e| {
        InstallationError::io(
            format!("Failed to read source directory '{}'", src.display()),
            e,
        )
    })? {
        let entry =
            entry.map_err(|e| InstallationError::io("Failed to read directory entry", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        tracker.consume_entry()?;

        let file_type = entry
            .file_type()
            .map_err(|e| InstallationError::io("Failed to determine file type", e))?;

        if file_type.is_dir() {
            copy_dir_all_with_tracker(&src_path, &dst_path, tracker)?;
        } else {
            let file_size = entry
                .metadata()
                .map_err(|e| {
                    InstallationError::io(format!("Failed to inspect '{}'", src_path.display()), e)
                })?
                .len();
            tracker.consume_file_bytes(&src_path, file_size)?;
            std::fs::copy(&src_path, &dst_path).map_err(|e| {
                InstallationError::io(
                    format!(
                        "Failed to copy '{}' -> '{}'",
                        src_path.display(),
                        dst_path.display()
                    ),
                    e,
                )
            })?;
        }
    }

    Ok(())
}

pub(crate) async fn copy_dir_all_blocking(src: &Path, dst: &Path) -> Result<(), InstallationError> {
    let src = src.to_path_buf();
    let dst = dst.to_path_buf();
    run_blocking_fs("directory copy", move || copy_dir_all(&src, &dst)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_file(path: &Path, size: usize) {
        std::fs::write(path, vec![b'a'; size]).unwrap();
    }

    #[test]
    fn copy_dir_all_with_budget_rejects_total_bytes() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("src");
        let dst = temp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        write_file(&src.join("big.txt"), 8);

        let budget = ResourceBudget {
            copy_bytes: 4,
            ..ResourceBudget::default()
        };
        let err = copy_dir_all_with_budget(&src, &dst, budget).unwrap_err();

        assert!(matches!(
            err,
            InstallationError::CopyByteBudgetExceeded { .. }
        ));
        assert!(err.to_string().contains("total copied bytes"));
    }

    #[test]
    fn copy_dir_all_with_budget_rejects_entry_limit() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("src");
        let dst = temp.path().join("dst");
        std::fs::create_dir_all(src.join("nested")).unwrap();
        write_file(&src.join("nested").join("a.txt"), 1);
        write_file(&src.join("nested").join("b.txt"), 1);

        let budget = ResourceBudget {
            copy_entries: 2,
            ..ResourceBudget::default()
        };
        let err = copy_dir_all_with_budget(&src, &dst, budget).unwrap_err();

        assert!(matches!(
            err,
            InstallationError::CopyEntryBudgetExceeded { .. }
        ));
        assert!(err.to_string().contains("exceeds 2 entries"));
    }
}
