//! Exact versioned local digest using the GitHub skill-content framing.

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::services::github_import::skill_content_digest_from_file_bytes;
use crate::services::resource_budget::{DEFAULT_COPY_BYTES, DEFAULT_COPY_ENTRIES};

use super::super::SkillsCliError;

pub const DIGEST_MARKER_DIR_PREFIX: &str = ".skillport-update-op-";

pub fn digest_skill_directory(root: &Path) -> Result<String, SkillsCliError> {
    let files = collect_skill_files(root)?;
    Ok(skill_content_digest_from_file_bytes(&files))
}

pub fn collect_skill_files(root: &Path) -> Result<Vec<(String, Vec<u8>)>, SkillsCliError> {
    let canonical_root = fs::canonicalize(root).map_err(|source| SkillsCliError::Io {
        context: "canonicalize skill directory",
        source,
    })?;
    let mut files = Vec::new();
    let mut total_bytes = 0_u64;
    walk_skill_files(&canonical_root, &canonical_root, &mut files, &mut total_bytes)?;
    files.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    Ok(files)
}

fn walk_skill_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<(String, Vec<u8>)>,
    total_bytes: &mut u64,
) -> Result<(), SkillsCliError> {
    let entries = fs::read_dir(current).map_err(|source| SkillsCliError::Io {
        context: "read skill directory",
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| SkillsCliError::Io {
            context: "read skill directory entry",
            source,
        })?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(DIGEST_MARKER_DIR_PREFIX) {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).map_err(|source| SkillsCliError::Io {
            context: "stat skill path",
            source,
        })?;
        if metadata.file_type().is_symlink() {
            return Err(SkillsCliError::UpdateIntegrity);
        }
        if metadata.is_dir() {
            walk_skill_files(root, &path, files, total_bytes)?;
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        if files.len() >= DEFAULT_COPY_ENTRIES {
            return Err(SkillsCliError::UpdateIntegrity);
        }
        let relative = relative_posix(root, &path)?;
        let bytes = fs::read(&path).map_err(|source| SkillsCliError::Io {
            context: "read skill file",
            source,
        })?;
        *total_bytes = total_bytes
            .checked_add(bytes.len() as u64)
            .ok_or(SkillsCliError::UpdateIntegrity)?;
        if *total_bytes > DEFAULT_COPY_BYTES {
            return Err(SkillsCliError::UpdateIntegrity);
        }
        files.push((relative, bytes));
    }
    Ok(())
}

fn relative_posix(root: &Path, file: &Path) -> Result<String, SkillsCliError> {
    let relative = file.strip_prefix(root).map_err(|_| SkillsCliError::UpdateIntegrity)?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            _ => return Err(SkillsCliError::UpdateIntegrity),
        }
    }
    Ok(parts.join("/"))
}

pub fn write_skill_files(root: &Path, files: &[(String, Vec<u8>)]) -> Result<(), SkillsCliError> {
    if root.exists() {
        fs::remove_dir_all(root).map_err(|source| SkillsCliError::Io {
            context: "replace owned canonical",
            source,
        })?;
    }
    fs::create_dir_all(root).map_err(|source| SkillsCliError::Io {
        context: "create owned canonical",
        source,
    })?;
    for (relative, bytes) in files {
        if relative.contains("..") || PathBuf::from(relative).is_absolute() {
            return Err(SkillsCliError::UpdateIntegrity);
        }
        let dest = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|source| SkillsCliError::Io {
                context: "create skill file parent",
                source,
            })?;
        }
        fs::write(&dest, bytes).map_err(|source| SkillsCliError::Io {
            context: "write skill file",
            source,
        })?;
    }
    Ok(())
}

pub fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<(), SkillsCliError> {
    fs::create_dir_all(dest).map_err(|source_err| SkillsCliError::Io {
        context: "create backup directory",
        source: source_err,
    })?;
    for entry in fs::read_dir(source).map_err(|source_err| SkillsCliError::Io {
        context: "read backup source",
        source: source_err,
    })? {
        let entry = entry.map_err(|source_err| SkillsCliError::Io {
            context: "read backup entry",
            source: source_err,
        })?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let metadata = fs::symlink_metadata(&from).map_err(|source_err| SkillsCliError::Io {
            context: "stat backup entry",
            source: source_err,
        })?;
        if metadata.file_type().is_symlink() {
            return Err(SkillsCliError::UpdateIntegrity);
        }
        if metadata.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if metadata.is_file() {
            fs::copy(&from, &to).map_err(|source_err| SkillsCliError::Io {
                context: "copy backup file",
                source: source_err,
            })?;
        }
    }
    Ok(())
}
