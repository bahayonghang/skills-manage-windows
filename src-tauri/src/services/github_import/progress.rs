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
            let relative_path = repo_file_relative_to_source(path, source_path)?;

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

pub(super) fn emit_github_import_progress(
    app: Option<&AppHandle>,
    payload: GitHubImportProgressPayload,
) {
    if let Some(app) = app {
        let _ = app.emit("github-import:progress", payload);
    }
}
