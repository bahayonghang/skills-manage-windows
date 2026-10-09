use super::RulesError;
use crate::services::resource_budget::ResourceBudget;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub(super) fn fingerprint(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn validate_name(name: &str) -> Result<(), RulesError> {
    let lower = name.to_lowercase();
    let stem = lower.strip_suffix(".md").ok_or(RulesError::InvalidName)?;
    let reserved = stem.split('.').next().unwrap_or_default();
    if stem.is_empty()
        || name.len() > 240
        || name.starts_with('.')
        || name.ends_with([' ', '.'])
        || stem.ends_with([' ', '.'])
        || name.contains("..")
        || name
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
        || stem == "rules"
        || stem.starts_with("rules@")
        || ["con", "prn", "aux", "nul", "conin$", "conout$"].contains(&reserved)
        || reserved
            .strip_prefix("com")
            .or_else(|| reserved.strip_prefix("lpt"))
            .is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
    {
        return Err(RulesError::InvalidName);
    }
    Ok(())
}

/// Every existing ancestor below the selected home must be a real directory.
/// This also checks private receipt/backup parents before any read or write.
pub(super) fn safe_dir(home: &Path, dir: &Path, create: bool) -> Result<(), RulesError> {
    let relative = dir
        .strip_prefix(home)
        .map_err(|_| RulesError::TargetConflict)?;
    let mut current = home.to_path_buf();
    let canonical_home = home.canonicalize()?;
    for part in relative.components() {
        if !matches!(part, std::path::Component::Normal(_)) {
            return Err(RulesError::TargetConflict);
        }
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) if !meta.file_type().is_symlink() && meta.is_dir() => {
                if !current.canonicalize()?.starts_with(&canonical_home) {
                    return Err(RulesError::TargetConflict);
                }
            }
            Ok(_) => return Err(RulesError::TargetConflict),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if create {
                    fs::create_dir(&current)?;
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

pub(super) fn read_bounded(path: &Path) -> Result<Vec<u8>, RulesError> {
    let limit = ResourceBudget::default_skill().file_bytes;
    let file = File::open(path)?;
    if file.metadata()?.len() > limit {
        return Err(RulesError::BudgetExceeded);
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(RulesError::BudgetExceeded);
    }
    Ok(bytes)
}

pub(super) fn regular_bytes(home: &Path, path: &Path) -> Result<Vec<u8>, RulesError> {
    safe_dir(
        home,
        path.parent().ok_or(RulesError::TargetConflict)?,
        false,
    )?;
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(RulesError::TargetConflict);
    }
    read_bounded(path)
}

pub(super) fn slot_metadata(path: &Path) -> Result<Option<fs::Metadata>, RulesError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn atomic_write(
    home: &Path,
    path: &Path,
    bytes: &[u8],
    new: bool,
) -> Result<(), RulesError> {
    atomic_write_checked(home, path, bytes, new, None)
}

pub(super) fn atomic_write_checked(
    home: &Path,
    path: &Path,
    bytes: &[u8],
    new: bool,
    expected_revision: Option<&str>,
) -> Result<(), RulesError> {
    if bytes.len() as u64 > ResourceBudget::default_skill().file_bytes {
        return Err(RulesError::BudgetExceeded);
    }
    let parent = path.parent().ok_or(RulesError::TargetConflict)?;
    safe_dir(home, parent, true)?;
    if let Some(meta) = slot_metadata(path)? {
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(RulesError::TargetConflict);
        }
        if new {
            return Err(RulesError::TargetConflict);
        }
    }
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    if let Some(expected) = expected_revision {
        if fingerprint(&regular_bytes(home, path)?) != expected {
            return Err(RulesError::RevisionConflict);
        }
    }
    if new {
        temporary
            .persist_noclobber(path)
            .map_err(|e| RulesError::from(e.error))?;
    } else {
        temporary
            .persist(path)
            .map_err(|e| RulesError::from(e.error))?;
    }
    Ok(())
}

pub(super) fn names(home: &Path, dir: &Path) -> Result<Vec<String>, RulesError> {
    safe_dir(home, dir, false)?;
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut names = Vec::new();
    let mut count = 0;
    for entry in entries {
        let entry = entry?;
        count += 1;
        if count > ResourceBudget::default_skill().tree_entries {
            return Err(RulesError::BudgetExceeded);
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.')
            || !name.to_ascii_lowercase().ends_with(".md")
            || validate_name(&name).is_err()
            || entry.file_type()?.is_dir()
        {
            continue;
        }
        names.push(name);
    }
    names.sort_by_key(|n| n.to_lowercase());
    if names
        .windows(2)
        .any(|pair| pair[0].to_lowercase() == pair[1].to_lowercase())
    {
        return Err(RulesError::TargetConflict);
    }
    Ok(names)
}

pub(super) fn link_resolved(path: &Path) -> Result<PathBuf, RulesError> {
    let target = fs::read_link(path)?;
    let raw = if target.is_absolute() {
        target
    } else {
        path.parent()
            .ok_or(RulesError::TargetConflict)?
            .join(target)
    };
    // Collapse lexical parent components so a broken link can still prove ownership.
    let mut result = PathBuf::new();
    for part in raw.components() {
        match part {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            other => result.push(other.as_os_str()),
        }
    }
    Ok(result)
}

pub(super) fn points_to(path: &Path, central: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
        && link_resolved(path).is_ok_and(|target| crate::paths::paths_equivalent(&target, central))
}

pub(super) fn create_file_link(target: &Path, path: &Path) -> Result<(), RulesError> {
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, path)?;
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(target, path)?;
    Ok(())
}
