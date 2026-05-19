use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use crate::db::{self, DbPool};
use crate::targets::{
    connect_remote_target, remote_file_type_is_dir, ActiveTarget, ConnectedRemoteTarget,
};

use super::types::DirectoryTreeEntry;

pub async fn read_skill_content_for_target_impl(
    pool: &DbPool,
    active_target: ActiveTarget,
    skill_id: &str,
) -> Result<String, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;

    match active_target {
        ActiveTarget::Local => read_skill_file_content(&skill.file_path),
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            let connection = connect_remote_target(&active_target).await?;
            let bytes = connection.read_file(&skill.file_path).await?;
            String::from_utf8(bytes).map_err(|e| {
                format!(
                    "Remote file '{}' is not valid UTF-8: {}",
                    skill.file_path, e
                )
            })
        }
    }
}

fn read_skill_file_content(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path, e))
}

pub async fn read_file_by_path_for_target_impl(
    active_target: ActiveTarget,
    path: &str,
) -> Result<String, String> {
    match active_target {
        ActiveTarget::Local => read_file_by_path_impl(path),
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            let connection = connect_remote_target(&active_target).await?;
            let bytes = connection.read_file(path).await?;
            String::from_utf8(bytes)
                .map_err(|e| format!("Remote file '{}' is not valid UTF-8: {}", path, e))
        }
    }
}

pub(super) fn read_file_by_path_impl(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path, e))
}

pub async fn list_directory_tree_for_target_impl(
    active_target: ActiveTarget,
    path: &str,
) -> Result<Vec<DirectoryTreeEntry>, String> {
    match active_target {
        ActiveTarget::Local => list_directory_tree_impl(path),
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            let connection = connect_remote_target(&active_target).await?;
            list_remote_directory_tree_impl(&connection, path).await
        }
    }
}

pub(super) fn list_directory_tree_impl(path: &str) -> Result<Vec<DirectoryTreeEntry>, String> {
    let directory = Path::new(path);
    if !directory.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    if !directory.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(directory)
        .map_err(|e| format!("Failed to list directory '{}': {}", path, e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory '{}': {}", path, e))?;
        let entry_path = entry.path();
        let metadata = std::fs::symlink_metadata(&entry_path)
            .map_err(|e| format!("Failed to inspect '{}': {}", entry_path.display(), e))?;
        let file_type = if metadata.file_type().is_symlink() {
            "symlink".to_string()
        } else if metadata.is_dir() {
            "dir".to_string()
        } else if metadata.is_file() {
            "file".to_string()
        } else {
            "other".to_string()
        };
        let symlink_target = if metadata.file_type().is_symlink() {
            std::fs::read_link(&entry_path)
                .ok()
                .map(|value| value.to_string_lossy().into_owned())
        } else {
            None
        };
        let children = if metadata.is_dir() {
            list_directory_tree_impl(&entry_path.to_string_lossy())?
        } else {
            Vec::new()
        };
        entries.push(DirectoryTreeEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry_path.to_string_lossy().into_owned(),
            file_type,
            symlink_target,
            children,
        });
    }

    sort_directory_entries(&mut entries);
    Ok(entries)
}

async fn list_remote_directory_tree_impl(
    connection: &ConnectedRemoteTarget,
    path: &str,
) -> Result<Vec<DirectoryTreeEntry>, String> {
    if !connection.exists(path).await? {
        return Err(format!("Remote path '{}' does not exist.", path));
    }

    let mut root_entries = fetch_remote_directory_entries(connection, path).await?;
    sort_directory_entries(&mut root_entries);

    let mut queue: VecDeque<&mut DirectoryTreeEntry> = root_entries.iter_mut().collect();
    while let Some(entry) = queue.pop_front() {
        if remote_entry_is_dir(entry) {
            let mut children = fetch_remote_directory_entries(connection, &entry.path).await?;
            sort_directory_entries(&mut children);
            entry.children = children;
            for child in &mut entry.children {
                queue.push_back(child);
            }
        }
    }

    Ok(root_entries)
}

async fn fetch_remote_directory_entries(
    connection: &ConnectedRemoteTarget,
    path: &str,
) -> Result<Vec<DirectoryTreeEntry>, String> {
    let entries = connection.list_dir(path).await?;
    let mut results = Vec::with_capacity(entries.len());
    for entry in entries {
        let child_path = PathBuf::from(path).join(&entry.name);
        results.push(DirectoryTreeEntry {
            name: entry.name,
            path: child_path.to_string_lossy().replace('\\', "/"),
            file_type: entry.file_type,
            symlink_target: entry.symlink_target,
            children: Vec::new(),
        });
    }
    Ok(results)
}

fn remote_entry_is_dir(entry: &DirectoryTreeEntry) -> bool {
    if remote_file_type_is_dir(&entry.file_type) {
        return true;
    }
    entry.file_type == "symlink"
        && entry
            .symlink_target
            .as_deref()
            .is_some_and(|target| target.ends_with('/'))
}

fn sort_directory_entries(entries: &mut [DirectoryTreeEntry]) {
    entries.sort_by(|left, right| {
        directory_sort_rank(&left.file_type)
            .cmp(&directory_sort_rank(&right.file_type))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.path.cmp(&right.path))
    });
}

fn directory_sort_rank(file_type: &str) -> u8 {
    match file_type {
        "dir" => 0,
        "symlink" => 1,
        "file" => 2,
        _ => 3,
    }
}

pub fn open_in_file_manager_for_target_impl(
    active_target: ActiveTarget,
    path: &str,
) -> Result<(), String> {
    if active_target.is_remote_like() {
        return Err("Remote paths cannot be opened in the local file manager. Copy the remote path instead.".to_string());
    }
    open_in_file_manager_checked_impl(path)
}

pub(super) fn open_in_file_manager_checked_impl(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    open_in_file_manager_impl(path)
}

fn open_in_file_manager_impl(path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }

    Ok(())
}
