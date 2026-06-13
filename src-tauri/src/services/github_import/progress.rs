use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SnapshotSourceFile {
    pub(super) repo_path: String,
    pub(super) relative_path: String,
    pub(super) byte_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct GitHubImportProgressState {
    pub(super) completed_files: usize,
    pub(super) total_files: usize,
    pub(super) completed_bytes: u64,
    pub(super) total_bytes: u64,
}

pub(super) fn collect_snapshot_source_files(
    snapshot: &GitHubRepoSnapshot,
    source_path: &str,
) -> Result<Vec<SnapshotSourceFile>, GithubImportError> {
    let mut files = snapshot
        .files
        .iter()
        .filter_map(|(path, bytes)| {
            let relative_path = if source_path == "." {
                if path.contains('/') {
                    return None;
                }
                path.clone()
            } else {
                let prefix = format!("{}/", source_path.trim_matches('/'));
                let relative = path.strip_prefix(&prefix)?;
                if relative.is_empty() {
                    return None;
                }
                relative.to_string()
            };

            Some(SnapshotSourceFile {
                repo_path: path.clone(),
                relative_path,
                byte_len: bytes.len(),
            })
        })
        .collect::<Vec<_>>();

    files.sort_by(|left, right| left.repo_path.cmp(&right.repo_path));

    if files.is_empty() {
        return Err(GithubImportError::RepoPathGone(source_path.to_string()));
    }

    Ok(files)
}

pub(super) async fn write_snapshot_source_to_target(
    snapshot: &GitHubRepoSnapshot,
    files: &[SnapshotSourceFile],
    target_dir: &Path,
    source_path: &str,
    progress_state: &mut GitHubImportProgressState,
    app: Option<&AppHandle>,
) -> Result<(), GithubImportError> {
    // Same ordering as the original synchronous version: create the target
    // directory first, then validate-and-write one file at a time. Each
    // blocking write owns a transient clone of just that file's bytes. The
    // `AppHandle` deliberately stays on the async side (by reference): moving
    // it into a blocking closure links Tauri's dialog/menu drop-glue into test
    // binaries, which then fail to load on Windows (comctl32 v6
    // `TaskDialogIndirect` needs an app manifest).
    let target_dir_for_create = target_dir.to_path_buf();
    crate::fs_util::run_blocking_fs_with(
        "import target directory creation",
        move || {
            std::fs::create_dir_all(&target_dir_for_create)
                .map_err(|e| GithubImportError::io("Failed to create import target directory", e))
        },
        GithubImportError::task_join,
    )
    .await?;

    for file in files {
        if !is_safe_repo_relative_path(&file.relative_path) {
            return Err(GithubImportError::RepoContainsUnsupportedPath(
                file.repo_path.clone(),
            ));
        }

        let bytes = snapshot
            .files
            .get(&file.repo_path)
            .ok_or_else(|| GithubImportError::RepoFileGone(file.repo_path.clone()))?
            .clone();

        let destination = target_dir.join(&file.relative_path);
        crate::fs_util::run_blocking_fs_with(
            "import file write",
            move || {
                let parent = destination
                    .parent()
                    .ok_or(GithubImportError::ImportParentDirUnknown)?;
                std::fs::create_dir_all(parent).map_err(|e| {
                    GithubImportError::io("Failed to create imported file parent directory", e)
                })?;
                std::fs::write(&destination, &bytes).map_err(|e| {
                    GithubImportError::io(
                        format!("Failed to write imported file '{}'", destination.display()),
                        e,
                    )
                })
            },
            GithubImportError::task_join,
        )
        .await?;

        progress_state.completed_files += 1;
        progress_state.completed_bytes += file.byte_len as u64;
        emit_github_import_progress(
            app,
            GitHubImportProgressPayload {
                phase: GitHubImportProgressPhase::Writing,
                current_skill: Some(source_path.to_string()),
                current_path: Some(file.relative_path.clone()),
                completed_files: progress_state.completed_files,
                total_files: progress_state.total_files,
                completed_bytes: progress_state.completed_bytes,
                total_bytes: progress_state.total_bytes,
            },
        );
    }

    Ok(())
}

pub(super) fn emit_github_import_progress(
    app: Option<&AppHandle>,
    payload: GitHubImportProgressPayload,
) {
    if let Some(app) = app {
        let _ = app.emit("github-import:progress", payload);
    }
}
