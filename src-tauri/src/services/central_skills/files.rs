use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use crate::db::{self, DbPool};
use crate::services::{
    bounded_ingestion::{read_file_text_bounded, BoundedReadError, ReadLimit},
    resource_budget::{BudgetExceeded, ResourceBudget},
};
use crate::targets::{
    connect_remote_target, remote_file_type_is_dir, ActiveTarget, ConnectedRemoteTarget,
};

use super::common::run_blocking_fs;
use super::error::CentralSkillsError;
use super::query::get_skill_detail_with_row_impl;
use super::remote_path::resolve_remote_allowed_path;
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

    fn consume_entry(&mut self, path: &str) -> Result<(), CentralSkillsError> {
        if self.remaining_entries == 0 {
            return Err(CentralSkillsError::TreeEntriesExceeded {
                path: path.to_string(),
                limit: self.limits.tree_entries,
            });
        }
        self.remaining_entries -= 1;
        Ok(())
    }
}

pub async fn read_skill_content_for_target_impl(
    pool: &DbPool,
    active_target: ActiveTarget,
    skill_id: &str,
) -> Result<String, CentralSkillsError> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| CentralSkillsError::SkillNotFound(skill_id.to_string()))?;

    match active_target {
        ActiveTarget::Local => {
            let file_path = skill.file_path.clone();
            run_blocking_fs("skill content read", move || {
                read_skill_file_content(&file_path)
            })
            .await
        }
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            let connection = connect_remote_target(&active_target)
                .await
                .map_err(|e| CentralSkillsError::Remote(e.to_string()))?;
            read_remote_skill_content(&connection, &skill.file_path).await
        }
    }
}

pub(super) fn read_skill_file_content(path: &str) -> Result<String, CentralSkillsError> {
    read_file_text_bounded(
        Path::new(path),
        ReadLimit::new(
            "Local skill file",
            ResourceBudget::default_skill().file_bytes,
        ),
    )
    .map_err(|error| map_local_file_read_error(error, path))
}

pub(super) async fn read_remote_skill_content(
    connection: &ConnectedRemoteTarget,
    path: &str,
) -> Result<String, CentralSkillsError> {
    let max_bytes = ResourceBudget::default_skill().file_bytes;
    let bytes = connection
        .read_file_bounded(path, max_bytes)
        .await
        .map_err(|error| map_remote_file_read_error(error, max_bytes))?;
    String::from_utf8(bytes).map_err(|_| CentralSkillsError::SkillFileNotUtf8 { target: "Remote" })
}

fn map_local_file_read_error(error: BoundedReadError, path: &str) -> CentralSkillsError {
    match error {
        BoundedReadError::LimitExceeded { actual, limit, .. } => {
            CentralSkillsError::Budget(BudgetExceeded::new("Local skill file", actual, limit))
        }
        BoundedReadError::InvalidUtf8 { .. } => {
            CentralSkillsError::SkillFileNotUtf8 { target: "Local" }
        }
        BoundedReadError::Io { source, .. } => {
            CentralSkillsError::io(format!("Failed to read '{path}'"), source)
        }
        BoundedReadError::Http { .. } => CentralSkillsError::io(
            "Failed to read local skill file",
            std::io::Error::other("read failed"),
        ),
    }
}

fn map_remote_file_read_error(
    error: crate::targets::TargetsError,
    max_bytes: u64,
) -> CentralSkillsError {
    if matches!(
        error,
        crate::targets::TargetsError::RemoteFileTooLarge { .. }
    ) {
        CentralSkillsError::Budget(BudgetExceeded::new(
            "Remote skill file",
            max_bytes.saturating_add(1),
            max_bytes,
        ))
    } else {
        CentralSkillsError::Remote(error.to_string())
    }
}

pub async fn read_file_by_path_for_target_impl(
    pool: &DbPool,
    active_target: ActiveTarget,
    path: &str,
    access: &SkillPathAccessContext,
) -> Result<String, CentralSkillsError> {
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
            let connection = connect_remote_target(&active_target)
                .await
                .map_err(|e| CentralSkillsError::Remote(e.to_string()))?;
            read_remote_file_by_path_impl(&connection, path, &access_root).await
        }
    }
}

pub(super) fn read_file_by_path_impl(
    path: &str,
    access_root: &str,
) -> Result<String, CentralSkillsError> {
    let budget = ResourceBudget::default_skill();
    let resolved = resolve_local_allowed_path(access_root, path)?;
    let metadata = std::fs::metadata(&resolved).map_err(|e| {
        CentralSkillsError::io(format!("Failed to inspect '{}'", resolved.display()), e)
    })?;
    if !metadata.is_file() {
        return Err(CentralSkillsError::NotAFile(resolved.display().to_string()));
    }
    read_file_text_bounded(
        &resolved,
        ReadLimit::new("Local skill file", budget.file_bytes),
    )
    .map_err(|error| map_local_file_read_error(error, &resolved.to_string_lossy()))
}

pub async fn list_directory_tree_for_target_impl(
    pool: &DbPool,
    active_target: ActiveTarget,
    path: &str,
    access: &SkillPathAccessContext,
) -> Result<Vec<DirectoryTreeEntry>, CentralSkillsError> {
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
            let connection = connect_remote_target(&active_target)
                .await
                .map_err(|e| CentralSkillsError::Remote(e.to_string()))?;
            list_remote_directory_tree_impl(&connection, path, &access_root).await
        }
    }
}

pub(super) fn list_directory_tree_impl(
    path: &str,
    access_root: &str,
) -> Result<Vec<DirectoryTreeEntry>, CentralSkillsError> {
    let limits = ResourceBudget::default_skill();
    let directory = resolve_local_allowed_path(access_root, path)?;
    if !directory.is_dir() {
        return Err(CentralSkillsError::NotADirectory(
            directory.display().to_string(),
        ));
    }
    let mut budget = DirectoryTreeBudget::new(limits);
    list_directory_tree_impl_with_budget(&directory, 0, &mut budget)
}

fn list_directory_tree_impl_with_budget(
    directory: &Path,
    depth: usize,
    budget: &mut DirectoryTreeBudget,
) -> Result<Vec<DirectoryTreeEntry>, CentralSkillsError> {
    if depth >= budget.limits.tree_depth {
        return Err(CentralSkillsError::TreeDepthExceeded {
            path: directory.display().to_string(),
            limit: budget.limits.tree_depth,
        });
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(|e| {
        CentralSkillsError::io(
            format!("Failed to list directory '{}'", directory.display()),
            e,
        )
    })? {
        let entry = entry.map_err(|e| {
            CentralSkillsError::io(
                format!("Failed to read directory '{}'", directory.display()),
                e,
            )
        })?;
        let entry_path = entry.path();
        budget.consume_entry(&entry_path.to_string_lossy())?;
        let metadata = std::fs::symlink_metadata(&entry_path).map_err(|e| {
            CentralSkillsError::io(format!("Failed to inspect '{}'", entry_path.display()), e)
        })?;
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
) -> Result<Vec<DirectoryTreeEntry>, CentralSkillsError> {
    let allowed_path = resolve_remote_allowed_path(connection, access_root, path).await?;
    let info = connection
        .inspect_path(&allowed_path)
        .await
        .map_err(|e| CentralSkillsError::Remote(e.to_string()))?
        .ok_or_else(|| CentralSkillsError::RemotePathMissing(allowed_path.clone()))?;
    if !remote_file_type_is_dir(&info.file_type) {
        return Err(CentralSkillsError::RemotePathNotDirectory(allowed_path));
    }

    let mut budget = DirectoryTreeBudget::new(ResourceBudget::default_skill());
    let mut root_entries =
        fetch_remote_directory_entries(connection, &allowed_path, &mut budget).await?;
    sort_directory_entries(&mut root_entries);

    let mut queue: VecDeque<(usize, &mut DirectoryTreeEntry)> =
        root_entries.iter_mut().map(|entry| (1, entry)).collect();
    while let Some((depth, entry)) = queue.pop_front() {
        if depth >= budget.limits.tree_depth {
            return Err(CentralSkillsError::TreeDepthExceeded {
                path: entry.path.clone(),
                limit: budget.limits.tree_depth,
            });
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
) -> Result<Vec<DirectoryTreeEntry>, CentralSkillsError> {
    let entries = connection
        .list_dir(path)
        .await
        .map_err(|e| CentralSkillsError::Remote(e.to_string()))?;
    if entries.len() > budget.remaining_entries {
        return Err(CentralSkillsError::TreeEntriesExceeded {
            path: path.to_string(),
            limit: budget.limits.tree_entries,
        });
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
) -> Result<(), CentralSkillsError> {
    if active_target.is_remote_like() {
        return Err(CentralSkillsError::RemoteOpenInFileManagerUnsupported);
    }
    let access_root = resolve_skill_access_root(pool, access).await?;
    open_in_file_manager_checked_impl(path, &access_root)
}

pub(super) fn open_in_file_manager_checked_impl(
    path: &str,
    access_root: &str,
) -> Result<(), CentralSkillsError> {
    let resolved = resolve_local_allowed_path(access_root, path)?;
    if !resolved.exists() {
        return Err(CentralSkillsError::PathMissing(
            resolved.display().to_string(),
        ));
    }
    open_in_file_manager_impl(&resolved.to_string_lossy())
}

fn open_in_file_manager_impl(path: &str) -> Result<(), CentralSkillsError> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| CentralSkillsError::io("Failed to open", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| CentralSkillsError::io("Failed to open", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| CentralSkillsError::io("Failed to open", e))?;
    }

    Ok(())
}

async fn read_remote_file_by_path_impl(
    connection: &ConnectedRemoteTarget,
    path: &str,
    access_root: &str,
) -> Result<String, CentralSkillsError> {
    let budget = ResourceBudget::default_skill();
    let allowed_path = resolve_remote_allowed_path(connection, access_root, path).await?;
    let info = connection
        .inspect_path(&allowed_path)
        .await
        .map_err(|e| CentralSkillsError::Remote(e.to_string()))?
        .ok_or_else(|| CentralSkillsError::RemotePathMissing(allowed_path.clone()))?;
    if info.file_type != "file" {
        return Err(CentralSkillsError::RemotePathNotFile(allowed_path));
    }
    let bytes = connection
        .read_file_bounded(&allowed_path, budget.file_bytes)
        .await
        .map_err(|error| map_remote_file_read_error(error, budget.file_bytes))?;
    String::from_utf8(bytes).map_err(|_| CentralSkillsError::SkillFileNotUtf8 { target: "Remote" })
}

async fn resolve_skill_access_root(
    pool: &DbPool,
    access: &SkillPathAccessContext,
) -> Result<String, CentralSkillsError> {
    let detail = get_skill_detail_with_row_impl(
        pool,
        &access.skill_id,
        access.agent_id.as_deref(),
        access.row_id.as_deref(),
    )
    .await?;
    Ok(detail.dir_path)
}

fn resolve_local_allowed_path(
    access_root: &str,
    requested_path: &str,
) -> Result<PathBuf, CentralSkillsError> {
    let root = Path::new(access_root);
    let root_canonical = root.canonicalize().map_err(|e| {
        CentralSkillsError::io(
            format!("Failed to resolve allowed skill root '{}'", access_root),
            e,
        )
    })?;
    if !root_canonical.is_dir() {
        return Err(CentralSkillsError::SkillRootNotDirectory(
            root_canonical.display().to_string(),
        ));
    }

    let requested = Path::new(requested_path);
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root_canonical.join(requested)
    };
    let candidate_canonical = candidate.canonicalize().map_err(|e| {
        CentralSkillsError::io(format!("Failed to resolve '{}'", candidate.display()), e)
    })?;
    if candidate_canonical != root_canonical && !candidate_canonical.starts_with(&root_canonical) {
        return Err(CentralSkillsError::PathEscapesSkillRoot {
            path: candidate_canonical.display().to_string(),
            root: root_canonical.display().to_string(),
        });
    }
    Ok(candidate_canonical)
}

#[cfg(test)]
mod path_guard_tests {
    use std::sync::Arc;

    use crate::targets::{
        ConnectedRemoteTarget, ConnectedSshTarget, ConnectedWslTarget, RemoteTargetConfig,
        SshAuthMethod, WslTargetConfig,
    };
    use crate::test_support::FakeRunner;

    use super::*;
    use tempfile::TempDir;

    fn fake_remote_connection(runner: Arc<FakeRunner>) -> ConnectedRemoteTarget {
        let target = RemoteTargetConfig {
            id: "ssh-test".to_string(),
            label: "SSH test".to_string(),
            host: "example.invalid".to_string(),
            username: "alice".to_string(),
            port: 22,
            auth_method: SshAuthMethod::Key,
            key_path: "~/.ssh/id_ed25519".to_string(),
            credential_key: None,
            protected_password: None,
            password: None,
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: true,
        };
        ConnectedRemoteTarget::Ssh(ConnectedSshTarget::for_tests_with_runner(target, runner))
    }

    fn fake_wsl_connection(runner: Arc<FakeRunner>) -> ConnectedRemoteTarget {
        ConnectedRemoteTarget::Wsl(ConnectedWslTarget::for_tests_with_runner(
            WslTargetConfig {
                id: "wsl-test".to_string(),
                label: "WSL test".to_string(),
                distribution: "Ubuntu-24.04".to_string(),
                remote_home: "/home/alice".to_string(),
                remote_os: "Linux".to_string(),
                symlink_enabled: true,
            },
            runner,
        ))
    }

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
    fn local_skill_reads_reject_oversized_and_invalid_utf8_files() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("SKILL.md");
        std::fs::write(
            &path,
            vec![b'a'; ResourceBudget::default_skill().file_bytes as usize + 1],
        )
        .unwrap();
        assert!(matches!(
            read_skill_file_content(&path.to_string_lossy()).unwrap_err(),
            CentralSkillsError::Budget(_)
        ));
        assert!(!read_skill_file_content(&path.to_string_lossy())
            .unwrap_err()
            .to_string()
            .contains(&path.to_string_lossy().to_string()));

        std::fs::write(&path, [0xff, 0xfe]).unwrap();
        assert!(matches!(
            read_skill_file_content(&path.to_string_lossy()).unwrap_err(),
            CentralSkillsError::SkillFileNotUtf8 { target: "Local" }
        ));
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

        assert!(error.to_string().contains("escapes skill root"));
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

        assert!(error.to_string().contains("escapes skill root"));
    }

    #[cfg(unix)]
    #[test]
    fn local_guard_allows_contained_final_and_intermediate_symlinks() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("skill");
        let real_dir = skill_dir.join("real");
        std::fs::create_dir_all(&real_dir).unwrap();
        let real_file = real_dir.join("README.md");
        std::fs::write(&real_file, "contained").unwrap();
        let final_link = skill_dir.join("final-link.md");
        let intermediate_link = skill_dir.join("docs");
        symlink(&real_file, &final_link).unwrap();
        symlink(&real_dir, &intermediate_link).unwrap();

        assert_eq!(
            read_file_by_path_impl(&final_link.to_string_lossy(), &skill_dir.to_string_lossy())
                .unwrap(),
            "contained"
        );
        assert_eq!(
            read_file_by_path_impl("docs/README.md", &skill_dir.to_string_lossy()).unwrap(),
            "contained"
        );
    }

    #[cfg(unix)]
    #[test]
    fn local_guard_blocks_intermediate_symlink_escape() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("skill");
        let outside_dir = temp.path().join("outside");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::create_dir_all(&outside_dir).unwrap();
        std::fs::write(outside_dir.join("secret.txt"), "secret").unwrap();
        symlink(&outside_dir, skill_dir.join("docs")).unwrap();

        let error =
            read_file_by_path_impl("docs/secret.txt", &skill_dir.to_string_lossy()).unwrap_err();

        assert!(matches!(
            error,
            CentralSkillsError::PathEscapesSkillRoot { .. }
        ));
    }

    #[cfg(unix)]
    #[test]
    fn local_guard_allows_symlink_root_and_explicit_contained_directory_link() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let canonical_root = temp.path().join("canonical-skill");
        let real_docs = canonical_root.join("real-docs");
        std::fs::create_dir_all(&real_docs).unwrap();
        std::fs::write(real_docs.join("README.md"), "docs").unwrap();
        let install_root = temp.path().join("installed-skill");
        symlink(&canonical_root, &install_root).unwrap();
        symlink(&real_docs, canonical_root.join("docs")).unwrap();

        assert_eq!(
            read_file_by_path_impl("docs/README.md", &install_root.to_string_lossy()).unwrap(),
            "docs"
        );

        let explicit = list_directory_tree_impl("docs", &install_root.to_string_lossy()).unwrap();
        assert_eq!(explicit.len(), 1);
        assert_eq!(explicit[0].name, "README.md");

        let discovered = list_directory_tree_impl("", &install_root.to_string_lossy()).unwrap();
        let link = discovered
            .iter()
            .find(|entry| entry.name == "docs")
            .unwrap();
        assert_eq!(link.file_type, "symlink");
        assert!(link.children.is_empty());
    }

    #[tokio::test]
    async fn remote_read_uses_canonical_candidate_for_inspect_and_read() {
        for make_connection in [fake_remote_connection, fake_wsl_connection] {
            let runner = Arc::new(FakeRunner::new());
            runner.push_success("/canonical/skill/docs/README.md\0");
            runner.push_success("file\t\n");
            runner.push_success("canonical content");
            let connection = make_connection(runner.clone());

            let content =
                read_remote_file_by_path_impl(&connection, "docs-link/README.md", "/install/skill")
                    .await
                    .unwrap();

            assert_eq!(content, "canonical content");
            let calls = runner.calls();
            assert_eq!(calls.len(), 3);
            for call in &calls[1..] {
                let command = call.args.last().unwrap();
                assert!(command.contains("/canonical/skill/docs/README.md"));
                assert!(!command.contains("docs-link"));
            }
            let read = calls.last().unwrap();
            assert!(read.args.last().unwrap().contains("wc -c"));
            assert!(read.args.last().unwrap().contains("bs=1048577 count=1"));
        }
    }

    #[tokio::test]
    async fn remote_main_skill_read_has_ssh_wsl_size_and_utf8_parity() {
        for make_connection in [fake_remote_connection, fake_wsl_connection] {
            let runner = Arc::new(FakeRunner::new());
            runner.push_output(44, "", "secret detail");
            runner.push_output_bytes(0, &[0xff, 0xfe], &[]);
            let connection = make_connection(runner);

            assert!(matches!(
                read_remote_skill_content(&connection, "/remote/SKILL.md")
                    .await
                    .unwrap_err(),
                CentralSkillsError::Budget(_)
            ));
            assert!(matches!(
                read_remote_skill_content(&connection, "/remote/SKILL.md")
                    .await
                    .unwrap_err(),
                CentralSkillsError::SkillFileNotUtf8 { target: "Remote" }
            ));
        }
    }

    #[tokio::test]
    async fn remote_directory_entrypoint_uses_canonical_candidate() {
        let runner = Arc::new(FakeRunner::new());
        runner.push_success("/canonical/skill/docs\0");
        runner.push_success("dir\t\n");
        runner.push_success("README.md\tfile\t\n");
        let connection = fake_remote_connection(runner.clone());

        let entries = list_remote_directory_tree_impl(&connection, "docs-link", "/install/skill")
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "/canonical/skill/docs/README.md");
        let calls = runner.calls();
        assert_eq!(calls.len(), 3);
        for call in &calls[1..] {
            let command = call.args.last().unwrap();
            assert!(command.contains("/canonical/skill/docs"));
            assert!(!command.contains("docs-link"));
        }
    }

    #[tokio::test]
    async fn remote_canonical_escape_prevents_inspect_and_read() {
        let runner = Arc::new(FakeRunner::new());
        runner.push_output(44, "", "sensitive resolver detail");
        let connection = fake_remote_connection(runner.clone());

        let error = read_remote_file_by_path_impl(&connection, "docs/passwd", "/install/skill")
            .await
            .unwrap_err();

        assert!(matches!(error, CentralSkillsError::RemoteCanonicalEscape));
        assert_eq!(runner.calls().len(), 1);
        assert!(!error.to_string().contains("sensitive"));
    }
}
