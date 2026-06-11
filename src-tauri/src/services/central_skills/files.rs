use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use crate::db::{self, DbPool};
use crate::fs_util::run_blocking_fs;
use crate::services::resource_budget::ResourceBudget;
use crate::targets::{
    connect_remote_target, remote_file_type_is_dir, ActiveTarget, ConnectedRemoteTarget,
};

use super::query::get_skill_detail_with_row_impl;
use super::types::DirectoryTreeEntry;

#[derive(Debug, Clone)]
pub struct SkillPathAccessContext {
    pub skill_id: String,
    pub agent_id: Option<String>,
    pub row_id: Option<String>,
}

#[derive(Debug, Clone)]
struct DirectoryTreeBudget {
    limits: ResourceBudget,
    remaining_entries: usize,
}

impl DirectoryTreeBudget {
    fn new(limits: ResourceBudget) -> Self {
        Self {
            remaining_entries: limits.tree_entries,
            limits,
        }
    }

    fn consume_entry(&mut self, path: &str) -> Result<(), String> {
        if self.remaining_entries == 0 {
            return Err(format!(
                "Refusing to traverse '{}': directory tree exceeds {} entries.",
                path, self.limits.tree_entries
            ));
        }
        self.remaining_entries -= 1;
        Ok(())
    }
}

pub async fn read_skill_content_for_target_impl(
    pool: &DbPool,
    active_target: ActiveTarget,
    skill_id: &str,
) -> Result<String, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;

    match active_target {
        ActiveTarget::Local => {
            let file_path = skill.file_path.clone();
            run_blocking_fs("skill content read", move || {
                read_skill_file_content(&file_path)
            })
            .await
        }
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
    pool: &DbPool,
    active_target: ActiveTarget,
    path: &str,
    access: &SkillPathAccessContext,
) -> Result<String, String> {
    let access_root = resolve_skill_access_root(pool, access).await?;
    match active_target {
        ActiveTarget::Local => {
            let path = path.to_string();
            run_blocking_fs("skill file read", move || {
                read_file_by_path_impl(&path, &access_root)
            })
            .await
        }
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            let connection = connect_remote_target(&active_target).await?;
            read_remote_file_by_path_impl(&connection, path, &access_root).await
        }
    }
}

pub(super) fn read_file_by_path_impl(path: &str, access_root: &str) -> Result<String, String> {
    let budget = ResourceBudget::default_skill();
    let resolved = resolve_local_allowed_path(access_root, path)?;
    let metadata = std::fs::metadata(&resolved)
        .map_err(|e| format!("Failed to inspect '{}': {}", resolved.display(), e))?;
    if !metadata.is_file() {
        return Err(format!("Path is not a file: {}", resolved.display()));
    }
    budget.reject_file_read_size(&resolved.to_string_lossy(), metadata.len())?;
    std::fs::read_to_string(&resolved)
        .map_err(|e| format!("Failed to read '{}': {}", resolved.display(), e))
}

pub async fn list_directory_tree_for_target_impl(
    pool: &DbPool,
    active_target: ActiveTarget,
    path: &str,
    access: &SkillPathAccessContext,
) -> Result<Vec<DirectoryTreeEntry>, String> {
    let access_root = resolve_skill_access_root(pool, access).await?;
    match active_target {
        ActiveTarget::Local => {
            let path = path.to_string();
            run_blocking_fs("skill directory tree listing", move || {
                list_directory_tree_impl(&path, &access_root)
            })
            .await
        }
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            let connection = connect_remote_target(&active_target).await?;
            list_remote_directory_tree_impl(&connection, path, &access_root).await
        }
    }
}

pub(super) fn list_directory_tree_impl(
    path: &str,
    access_root: &str,
) -> Result<Vec<DirectoryTreeEntry>, String> {
    let limits = ResourceBudget::default_skill();
    let directory = resolve_local_allowed_path(access_root, path)?;
    if !directory.is_dir() {
        return Err(format!("Path is not a directory: {}", directory.display()));
    }
    let mut budget = DirectoryTreeBudget::new(limits);
    list_directory_tree_impl_with_budget(&directory, 0, &mut budget)
}

fn list_directory_tree_impl_with_budget(
    directory: &Path,
    depth: usize,
    budget: &mut DirectoryTreeBudget,
) -> Result<Vec<DirectoryTreeEntry>, String> {
    if depth >= budget.limits.tree_depth {
        return Err(format!(
            "Refusing to traverse '{}': directory depth exceeds {}.",
            directory.display(),
            budget.limits.tree_depth
        ));
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(directory)
        .map_err(|e| format!("Failed to list directory '{}': {}", directory.display(), e))?
    {
        let entry = entry
            .map_err(|e| format!("Failed to read directory '{}': {}", directory.display(), e))?;
        let entry_path = entry.path();
        budget.consume_entry(&entry_path.to_string_lossy())?;
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
            list_directory_tree_impl_with_budget(&entry_path, depth + 1, budget)?
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
    access_root: &str,
) -> Result<Vec<DirectoryTreeEntry>, String> {
    let allowed_path = normalize_remote_allowed_path(access_root, path)?;
    let info = connection
        .inspect_path(&allowed_path)
        .await?
        .ok_or_else(|| format!("Remote path '{}' does not exist.", allowed_path))?;
    if info.file_type == "symlink" {
        return Err(format!(
            "Refusing to traverse remote symlink path '{}'.",
            allowed_path
        ));
    }
    if !remote_file_type_is_dir(&info.file_type) {
        return Err(format!(
            "Remote path '{}' is not a directory.",
            allowed_path
        ));
    }

    let mut budget = DirectoryTreeBudget::new(ResourceBudget::default_skill());
    let mut root_entries =
        fetch_remote_directory_entries(connection, &allowed_path, &mut budget).await?;
    sort_directory_entries(&mut root_entries);

    let mut queue: VecDeque<(usize, &mut DirectoryTreeEntry)> =
        root_entries.iter_mut().map(|entry| (1, entry)).collect();
    while let Some((depth, entry)) = queue.pop_front() {
        if depth >= budget.limits.tree_depth {
            return Err(format!(
                "Refusing to traverse '{}': directory depth exceeds {}.",
                entry.path, budget.limits.tree_depth
            ));
        }
        if entry.file_type == "dir" {
            let mut children =
                fetch_remote_directory_entries(connection, &entry.path, &mut budget).await?;
            sort_directory_entries(&mut children);
            entry.children = children;
            for child in &mut entry.children {
                if child.file_type == "dir" {
                    queue.push_back((depth + 1, child));
                }
            }
        }
    }

    Ok(root_entries)
}

async fn fetch_remote_directory_entries(
    connection: &ConnectedRemoteTarget,
    path: &str,
    budget: &mut DirectoryTreeBudget,
) -> Result<Vec<DirectoryTreeEntry>, String> {
    let entries = connection.list_dir(path).await?;
    if entries.len() > budget.remaining_entries {
        return Err(format!(
            "Refusing to traverse '{}': directory tree exceeds {} entries.",
            path, budget.limits.tree_entries
        ));
    }
    budget.remaining_entries -= entries.len();
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

pub async fn open_in_file_manager_for_target_impl(
    pool: &DbPool,
    active_target: ActiveTarget,
    path: &str,
    access: &SkillPathAccessContext,
) -> Result<(), String> {
    if active_target.is_remote_like() {
        return Err("Remote paths cannot be opened in the local file manager. Copy the remote path instead.".to_string());
    }
    let access_root = resolve_skill_access_root(pool, access).await?;
    open_in_file_manager_checked_impl(path, &access_root)
}

pub(super) fn open_in_file_manager_checked_impl(
    path: &str,
    access_root: &str,
) -> Result<(), String> {
    let resolved = resolve_local_allowed_path(access_root, path)?;
    if !resolved.exists() {
        return Err(format!("Path does not exist: {}", resolved.display()));
    }
    open_in_file_manager_impl(&resolved.to_string_lossy())
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

async fn read_remote_file_by_path_impl(
    connection: &ConnectedRemoteTarget,
    path: &str,
    access_root: &str,
) -> Result<String, String> {
    let budget = ResourceBudget::default_skill();
    let allowed_path = normalize_remote_allowed_path(access_root, path)?;
    let info = connection
        .inspect_path(&allowed_path)
        .await?
        .ok_or_else(|| format!("Remote path '{}' does not exist.", allowed_path))?;
    if info.file_type == "symlink" {
        return Err(format!(
            "Refusing to read remote symlink path '{}'.",
            allowed_path
        ));
    }
    if info.file_type != "file" {
        return Err(format!("Remote path '{}' is not a file.", allowed_path));
    }
    let bytes = connection.read_file(&allowed_path).await?;
    budget.reject_file_read_size(&allowed_path, bytes.len() as u64)?;
    String::from_utf8(bytes)
        .map_err(|e| format!("Remote file '{}' is not valid UTF-8: {}", allowed_path, e))
}

async fn resolve_skill_access_root(
    pool: &DbPool,
    access: &SkillPathAccessContext,
) -> Result<String, String> {
    let detail = get_skill_detail_with_row_impl(
        pool,
        &access.skill_id,
        access.agent_id.as_deref(),
        access.row_id.as_deref(),
    )
    .await?;
    Ok(detail.dir_path)
}

fn resolve_local_allowed_path(access_root: &str, requested_path: &str) -> Result<PathBuf, String> {
    let root = Path::new(access_root);
    let root_canonical = root.canonicalize().map_err(|e| {
        format!(
            "Failed to resolve allowed skill root '{}': {}",
            access_root, e
        )
    })?;
    if !root_canonical.is_dir() {
        return Err(format!(
            "Allowed skill root '{}' is not a directory.",
            root_canonical.display()
        ));
    }

    let requested = Path::new(requested_path);
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root_canonical.join(requested)
    };
    let candidate_canonical = candidate
        .canonicalize()
        .map_err(|e| format!("Failed to resolve '{}': {}", candidate.display(), e))?;
    if candidate_canonical != root_canonical && !candidate_canonical.starts_with(&root_canonical) {
        return Err(format!(
            "Refusing to access '{}': path escapes skill root '{}'.",
            candidate_canonical.display(),
            root_canonical.display()
        ));
    }
    Ok(candidate_canonical)
}

fn normalize_remote_allowed_path(
    access_root: &str,
    requested_path: &str,
) -> Result<String, String> {
    let root = normalize_remote_posix_path(access_root)?;
    let requested = requested_path.trim();
    let candidate = if requested.is_empty() {
        root.clone()
    } else if requested.starts_with('/') {
        normalize_remote_posix_path(requested)?
    } else {
        normalize_remote_posix_path(&format!("{}/{}", root.trim_end_matches('/'), requested))?
    };
    if !remote_path_is_within(&root, &candidate) {
        return Err(format!(
            "Refusing to access '{}': path escapes skill root '{}'.",
            candidate, root
        ));
    }
    Ok(candidate)
}

fn normalize_remote_posix_path(path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Skill path context is empty.".to_string());
    }
    if trimmed.contains('\\') {
        return Err(format!(
            "Refusing to access '{}': backslashes are not allowed in remote paths.",
            trimmed
        ));
    }
    let is_absolute = trimmed.starts_with('/');
    let mut segments = Vec::new();
    for segment in trimmed.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return Err(format!(
                "Refusing to access '{}': parent traversal is not allowed.",
                trimmed
            ));
        }
        segments.push(segment);
    }
    let joined = segments.join("/");
    Ok(match (is_absolute, joined.is_empty()) {
        (true, true) => "/".to_string(),
        (true, false) => format!("/{}", joined),
        (false, true) => ".".to_string(),
        (false, false) => joined,
    })
}

fn remote_path_is_within(root: &str, candidate: &str) -> bool {
    if root == "/" {
        return candidate.starts_with('/');
    }
    candidate == root || candidate.starts_with(&format!("{root}/"))
}

#[cfg(test)]
mod path_guard_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn local_guard_allows_file_within_skill_root() {
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let skill_file = skill_dir.join("SKILL.md");
        std::fs::write(&skill_file, "# hello").unwrap();

        let content =
            read_file_by_path_impl(&skill_file.to_string_lossy(), &skill_dir.to_string_lossy())
                .unwrap();

        assert_eq!(content, "# hello");
    }

    #[test]
    fn local_guard_blocks_path_outside_skill_root() {
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let outside_file = temp.path().join("outside.txt");
        std::fs::write(&outside_file, "secret").unwrap();

        let error = read_file_by_path_impl(
            &outside_file.to_string_lossy(),
            &skill_dir.to_string_lossy(),
        )
        .unwrap_err();

        assert!(error.contains("escapes skill root"));
    }

    #[cfg(unix)]
    #[test]
    fn local_guard_blocks_symlink_escape() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let outside_file = temp.path().join("outside.txt");
        std::fs::write(&outside_file, "secret").unwrap();
        let link = skill_dir.join("outside-link.txt");
        symlink(&outside_file, &link).unwrap();

        let error = read_file_by_path_impl(&link.to_string_lossy(), &skill_dir.to_string_lossy())
            .unwrap_err();

        assert!(error.contains("escapes skill root"));
    }

    #[test]
    fn remote_guard_blocks_parent_traversal() {
        let error = normalize_remote_allowed_path(
            "/home/alice/.claude/skills/demo",
            "/home/alice/.claude/skills/../.ssh/id_rsa",
        )
        .unwrap_err();

        assert!(error.contains("parent traversal"));
    }

    #[test]
    fn remote_guard_allows_file_within_skill_root() {
        let allowed = normalize_remote_allowed_path(
            "/home/alice/.claude/skills/demo",
            "/home/alice/.claude/skills/demo/docs/README.md",
        )
        .unwrap();

        assert_eq!(allowed, "/home/alice/.claude/skills/demo/docs/README.md");
    }
}
