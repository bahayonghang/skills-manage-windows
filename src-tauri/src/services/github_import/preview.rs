use super::*;
use crate::services::resource_budget::ResourceBudget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreviewRepositoryFile {
    pub(super) repo_path: String,
    pub(super) byte_len: u64,
}

pub(super) const REMOTE_PREVIEW_FILE_INVENTORY_SCRIPT: &str = r#"set -eu
repo_dir=$1
find "$repo_dir" -type f -exec sh -c '
repo_dir=$1
shift
for file do
  relative=${file#"$repo_dir"/}
  size=$(wc -c < "$file")
  printf "%s\0%s\0" "$relative" "$size"
done
' sh "$repo_dir" {} +
"#;

pub(super) fn snapshot_preview_repository_files(
    snapshot: &GitHubRepoSnapshot,
) -> Vec<PreviewRepositoryFile> {
    snapshot
        .files
        .iter()
        .map(|(repo_path, bytes)| PreviewRepositoryFile {
            repo_path: repo_path.clone(),
            byte_len: bytes.len() as u64,
        })
        .collect()
}

pub(super) fn parse_remote_preview_repository_files(
    output: &str,
) -> Result<Vec<PreviewRepositoryFile>, GithubImportError> {
    if output.is_empty() {
        return Ok(Vec::new());
    }

    let payload = output
        .strip_suffix('\0')
        .ok_or(GithubImportError::RemotePreviewInvalidFileManifest)?;
    let fields = payload.split('\0').collect::<Vec<_>>();
    if fields.len() % 2 != 0 {
        return Err(GithubImportError::RemotePreviewInvalidFileManifest);
    }

    let budget = ResourceBudget::default_skill();
    let mut total_bytes = 0_u64;
    let mut seen_paths = HashSet::new();
    let mut files = Vec::with_capacity(fields.len() / 2);
    for record in fields.chunks_exact(2) {
        if files.len() >= budget.archive_files {
            return Err(GithubImportError::ArchiveFileBudgetExceeded(
                budget.archive_files,
            ));
        }

        let repo_path = normalize_repo_path(record[0])?;
        if repo_path.is_empty() || !seen_paths.insert(repo_path.clone()) {
            return Err(GithubImportError::RemotePreviewInvalidFileManifest);
        }
        let byte_len = record[1]
            .trim()
            .parse::<u64>()
            .map_err(|_| GithubImportError::RemotePreviewInvalidFileManifest)?;
        budget
            .reject_archive_entry_size(&repo_path, byte_len)
            .map_err(GithubImportError::Budget)?;
        total_bytes = total_bytes
            .checked_add(byte_len)
            .ok_or(GithubImportError::ArchiveSizeOverflow)?;
        budget
            .reject_archive_expanded_size(total_bytes)
            .map_err(GithubImportError::Budget)?;
        files.push(PreviewRepositoryFile {
            repo_path,
            byte_len,
        });
    }
    files.sort_by(|left, right| left.repo_path.cmp(&right.repo_path));
    Ok(files)
}

pub(super) async fn remote_preview_repository_files(
    connection: &ConnectedRemoteTarget,
    remote_repo_dir: &str,
) -> Result<Vec<PreviewRepositoryFile>, GithubImportError> {
    let output = connection
        .run_script(REMOTE_PREVIEW_FILE_INVENTORY_SCRIPT, &[remote_repo_dir])
        .await
        .map_err(|error| GithubImportError::Remote(error.to_string()))?;
    parse_remote_preview_repository_files(&output)
}

pub(super) fn attach_preview_file_manifests(
    skills: &mut [GitHubSkillPreview],
    repository_files: &[PreviewRepositoryFile],
) -> Result<(), GithubImportError> {
    for skill in skills {
        let mut files = repository_files
            .iter()
            .filter_map(|file| {
                repo_file_relative_to_source(&file.repo_path, &skill.source_path).map(|path| {
                    GitHubSkillPreviewFile {
                        path,
                        byte_len: file.byte_len,
                    }
                })
            })
            .collect::<Vec<_>>();
        files.sort_by(|left, right| left.path.cmp(&right.path));
        if !files.iter().any(|file| file.path == "SKILL.md") {
            return Err(GithubImportError::PreviewFileManifestIncomplete(
                skill.source_path.clone(),
            ));
        }
        skill.files = Some(files);
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) async fn preview_github_repo_import_impl(
    pool: &DbPool,
    secrets: &dyn crate::secrets::SecretStore,
    repo_url: &str,
) -> Result<GitHubRepoPreview, GithubImportError> {
    let auth = github_direct_auth_from_secret_store(pool, secrets).await?;
    preview_github_repo_import_with_auth(pool, repo_url, auth.as_deref()).await
}

pub(crate) async fn preview_github_repo_import_with_auth(
    pool: &DbPool,
    repo_url: &str,
    auth: Option<&str>,
) -> Result<GitHubRepoPreview, GithubImportError> {
    let resolved = resolve_repo_source(repo_url, auth).await?;
    let client = github_client()?;
    let snapshot = download_repo_snapshot(&client, &resolved.repo, auth).await?;
    let candidates = build_repo_skill_candidates_from_snapshot_at_path(
        &resolved.repo,
        &snapshot,
        resolved.source_path.as_deref(),
    )?;
    let mut skills = build_preview_skills(pool, &candidates).await?;

    if skills.is_empty() {
        return Err(GithubImportError::NoImportableSkills);
    }
    attach_preview_file_manifests(&mut skills, &snapshot_preview_repository_files(&snapshot))?;

    Ok(GitHubRepoPreview {
        repo: resolved.repo,
        skills,
        preview_workspace_id: None,
    })
}

pub(crate) async fn preview_github_repo_import_remote_with_auth(
    pool: &DbPool,
    active_target: &ActiveTarget,
    repo_url: &str,
    auth: Option<&str>,
) -> Result<GitHubRepoPreview, GithubImportError> {
    let resolved = resolve_repo_source(repo_url, auth).await?;
    let connection = connect_remote_target(active_target)
        .await
        .map_err(|e| GithubImportError::Remote(e.to_string()))?;
    cleanup_expired_preview_workspaces_for_connection(&connection).await;

    let workspace = create_remote_preview_workspace(&connection, &resolved, auth).await?;
    let preview_result = async {
        let candidates = build_remote_repo_skill_candidates_from_workspace(
            &connection,
            &resolved.repo,
            &workspace.remote_repo_dir,
            resolved.source_path.as_deref(),
        )
        .await?;
        let mut skills = build_preview_skills(pool, &candidates).await?;
        if skills.is_empty() {
            return Err(GithubImportError::NoImportableSkills);
        }
        let repository_files =
            remote_preview_repository_files(&connection, &workspace.remote_repo_dir).await?;
        attach_preview_file_manifests(&mut skills, &repository_files)?;
        Ok(skills)
    }
    .await;

    match preview_result {
        Ok(skills) => {
            register_preview_workspace(workspace.clone());
            Ok(GitHubRepoPreview {
                repo: resolved.repo,
                skills,
                preview_workspace_id: Some(workspace.id),
            })
        }
        Err(error) => {
            let _ = connection
                .remove_tree(&workspace.remote_workspace_dir)
                .await;
            Err(error)
        }
    }
}
