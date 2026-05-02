//! Filesystem utilities for skill installation:
//! relative-path computation, platform-specific symlink primitives,
//! and recursive directory copy.

use std::path::{Path, PathBuf};

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
pub fn create_symlink(target: &Path, link: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, link).map_err(|e| format!("Failed to create symlink: {}", e))
}

#[cfg(windows)]
pub fn create_symlink(target: &Path, link: &Path) -> Result<(), String> {
    std::os::windows::fs::symlink_dir(target, link)
        .map_err(|e| format!("Failed to create symlink: {}", e))
}

#[cfg(not(any(unix, windows)))]
pub fn create_symlink(_target: &Path, _link: &Path) -> Result<(), String> {
    Err("Symlink creation is only supported on Unix systems".to_string())
}

#[cfg(windows)]
pub(crate) fn remove_symlink_path(path: &Path) -> Result<(), String> {
    std::fs::remove_dir(path).map_err(|e| format!("Failed to remove existing symlink: {}", e))
}

#[cfg(not(windows))]
pub(crate) fn remove_symlink_path(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| format!("Failed to remove existing symlink: {}", e))
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

/// Recursively copy a directory tree from `src` to `dst`.
///
/// `dst` must not exist prior to the call (or may be an empty dir).
/// The behaviour mirrors `cp -r src dst` on Unix.
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| {
        format!(
            "Failed to create destination directory '{}': {}",
            dst.display(),
            e
        )
    })?;

    for entry in std::fs::read_dir(src)
        .map_err(|e| format!("Failed to read source directory '{}': {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to determine file type: {}", e))?;

        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "Failed to copy '{}' -> '{}': {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                )
            })?;
        }
    }

    Ok(())
}
