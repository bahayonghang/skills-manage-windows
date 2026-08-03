use crate::services::resource_budget::ResourceBudget;

use super::*;
pub(super) async fn cleanup_expired_preview_snapshots_for_connection(
    connection: &ConnectedRemoteTarget,
) {
    let tickets = sweep_preview_snapshots_for_target(connection.target_id(), Utc::now());
    let _ = cleanup_preview_tickets_for_connection(connection, tickets).await;
}

pub(super) async fn cleanup_preview_tickets_for_connection(
    connection: &ConnectedRemoteTarget,
    tickets: Vec<CleanupTicket>,
) -> bool {
    let mut all_cleaned = true;
    for ticket in tickets {
        let target_kind = ticket.target_kind();
        if ticket.target_id() != connection.target_id()
            || target_kind != connection_target_kind(connection)
        {
            all_cleaned = false;
            continue;
        }
        match connection.remove_tree(ticket.workspace_dir()).await {
            Ok(()) => {
                let _ = ack_preview_snapshot_cleanup(&ticket);
            }
            Err(_) => {
                all_cleaned = false;
                tracing::warn!(
                    count = 1,
                    reason = "remove_failed",
                    target_kind = ?target_kind,
                    "GitHub preview snapshot cleanup remains pending"
                );
            }
        }
    }
    all_cleaned
}

pub(super) async fn cleanup_preview_ticket_for_target(
    active_target: &ActiveTarget,
    ticket: CleanupTicket,
) -> bool {
    if !active_target.is_remote_like()
        || active_target.id() != ticket.target_id()
        || active_target.kind() != ticket.target_kind()
    {
        return false;
    }
    let Ok(connection) = connect_remote_target(active_target).await else {
        return false;
    };
    cleanup_preview_tickets_for_connection(&connection, vec![ticket]).await
}

fn connection_target_kind(connection: &ConnectedRemoteTarget) -> TargetKind {
    match connection {
        ConnectedRemoteTarget::Ssh(_) => TargetKind::Ssh,
        ConnectedRemoteTarget::Wsl(_) => TargetKind::Wsl,
    }
}

pub(super) async fn retry_pending_preview_cleanup_for_target(active_target: &ActiveTarget) {
    if !active_target.is_remote_like() {
        return;
    }
    let Ok(connection) = connect_remote_target(active_target).await else {
        return;
    };
    cleanup_expired_preview_snapshots_for_connection(&connection).await;
}

/// Create a bounded remote workspace holding the extracted repository at
/// `pinned_repo`'s ref (a resolved commit for preview).
pub(super) async fn create_remote_preview_workspace(
    connection: &ConnectedRemoteTarget,
    pinned_repo: &GitHubRepoRef,
    auth: Option<&str>,
) -> Result<GitHubPreviewWorkspace, GithubImportError> {
    let archive_url = github_archive_url(pinned_repo);
    let script = remote_workspace_download_script(auth)?;
    let output = connection
        .run_script(
            &script,
            &[archive_url.as_str(), crate::commands::APP_USER_AGENT],
        )
        .await
        .map_err(|e| GithubImportError::Remote(e.to_string()))?;
    let remote_workspace_dir = output
        .lines()
        .last()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .ok_or(GithubImportError::RemotePreviewNoWorkspacePath)?
        .to_string();
    let remote_repo_dir = remote_join(&remote_workspace_dir, "repo");

    Ok(GitHubPreviewWorkspace {
        remote_workspace_dir,
        remote_repo_dir,
    })
}

pub(super) fn github_archive_url(repo: &GitHubRepoRef) -> String {
    format!(
        "https://api.github.com/repos/{}/{}/tarball/{}",
        repo.owner, repo.repo, repo.branch
    )
}

pub(super) fn remote_workspace_download_script(
    auth: Option<&str>,
) -> Result<String, GithubImportError> {
    let budget = ResourceBudget::default_skill();
    let auth_block = match auth.filter(|token| !token.trim().is_empty()) {
        Some(token) => {
            let header = curl_auth_header_config_line(token)?;
            format!(
                r#"curl_conf="$workspace/curl.conf"
umask 077
cat > "$curl_conf" <<'SKILLPORT_CURL_CONF'
{header}
SKILLPORT_CURL_CONF
"#
            )
        }
        None => String::new(),
    };

    Ok(format!(
        r#"set -eu
archive_url=$1
user_agent=$2
archive_limit={archive_limit}
archive_files_limit={archive_files_limit}
archive_expanded_limit={archive_expanded_limit}
archive_entry_limit={archive_entry_limit}
workspace=""
curl_conf=""
cleanup() {{
  status=$?
  if [ -n "$curl_conf" ]; then
    rm -f -- "$curl_conf" || true
  fi
  if [ "$status" -ne 0 ] && [ -n "$workspace" ]; then
    rm -rf -- "$workspace" || true
  fi
  exit "$status"
}}
trap cleanup EXIT
for tool in sh curl tar find mktemp wc; do
  command -v "$tool" >/dev/null 2>&1 || {{
    printf 'Missing required remote tool: %s\n' "$tool" >&2
    exit 127
  }}
done
workspace=$(mktemp -d "${{TMPDIR:-/tmp}}/skillport-github-preview.XXXXXX")
repo_dir="$workspace/repo"
archive_file="$workspace/repo.tar.gz"
mkdir -p -- "$repo_dir"
{auth_block}
if [ -n "$curl_conf" ]; then
  curl -fL --retry 2 --connect-timeout 30 -A "$user_agent" -K "$curl_conf" -o "$archive_file" "$archive_url"
else
  curl -fL --retry 2 --connect-timeout 30 -A "$user_agent" -o "$archive_file" "$archive_url"
fi
set -- $(wc -c < "$archive_file")
archive_size=$1
if [ "$archive_size" -gt "$archive_limit" ]; then
  printf 'GitHub repository archive exceeds the resource budget (%s bytes > %s bytes).\n' "$archive_size" "$archive_limit" >&2
  exit 24
fi
tar -xzf "$archive_file" -C "$repo_dir" --strip-components=1
file_count=0
expanded_size=0
find "$repo_dir" -type f -print | while IFS= read -r file; do
  file_count=$((file_count + 1))
  if [ "$file_count" -gt "$archive_files_limit" ]; then
    printf 'GitHub repository archive exceeds the resource budget (more than %s files).\n' "$archive_files_limit" >&2
    exit 25
  fi
  set -- $(wc -c < "$file")
  file_size=$1
  if [ "$file_size" -gt "$archive_entry_limit" ]; then
    printf "GitHub repository archive entry '%s' exceeds the resource budget (%s bytes > %s bytes).\n" "$file" "$file_size" "$archive_entry_limit" >&2
    exit 26
  fi
  expanded_size=$((expanded_size + file_size))
  if [ "$expanded_size" -gt "$archive_expanded_limit" ]; then
    printf 'GitHub repository expanded archive contents exceeds the resource budget (%s bytes > %s bytes).\n' "$expanded_size" "$archive_expanded_limit" >&2
    exit 27
  fi
done
rm -f -- "$archive_file"
printf '%s\n' "$workspace"
"#,
        archive_limit = budget.archive_bytes,
        archive_files_limit = budget.archive_files,
        archive_expanded_limit = budget.archive_expanded_bytes,
        archive_entry_limit = budget.archive_entry_bytes,
    ))
}

pub(super) fn curl_auth_header_config_line(token: &str) -> Result<String, GithubImportError> {
    if token.contains('\n') || token.contains('\r') {
        return Err(GithubImportError::PatTokenHasNewline);
    }
    let escaped = token.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!("header = \"Authorization: Bearer {escaped}\""))
}
/// Import from a workspace this call created itself.
///
/// Used by the non-preview backend flows (Central repository sync, portable
/// state import, CLI) that confirm through their own verified inventory rather
/// than a renderer preview token.
pub(crate) async fn import_github_repo_skills_remote_with_auth(
    pool: &DbPool,
    active_target: &ActiveTarget,
    repo_url: &str,
    selections: Vec<GitHubSkillImportSelection>,
    app: Option<&AppHandle>,
    auth: Option<&str>,
) -> Result<GitHubRepoImportResult, GithubImportError> {
    let resolved = resolve_repo_source(repo_url, auth).await?;
    let connection = connect_remote_target(active_target)
        .await
        .map_err(|e| GithubImportError::Remote(e.to_string()))?;
    cleanup_expired_preview_snapshots_for_connection(&connection).await;
    let workspace = create_remote_preview_workspace(&connection, &resolved.repo, auth).await?;

    let result = import_github_repo_skills_remote_from_workspace(
        pool,
        active_target,
        &resolved.repo,
        resolved.source_path.as_deref(),
        &workspace,
        selections,
        None,
        app,
    )
    .await;
    let _ = connection
        .remove_tree(&workspace.remote_workspace_dir)
        .await;
    result
}

/// Import selected skills from an already-acquired remote workspace.
///
/// The workspace holds the exact bytes the caller verified; this function never
/// re-resolves a branch or downloads another archive.
#[allow(clippy::too_many_arguments)]
pub(super) async fn import_github_repo_skills_remote_from_workspace(
    pool: &DbPool,
    active_target: &ActiveTarget,
    repo: &GitHubRepoRef,
    source_path: Option<&str>,
    workspace: &GitHubPreviewWorkspace,
    selections: Vec<GitHubSkillImportSelection>,
    provenance: Option<&ImportProvenance>,
    app: Option<&AppHandle>,
) -> Result<GitHubRepoImportResult, GithubImportError> {
    emit_github_import_progress(
        app,
        GitHubImportProgressPayload {
            phase: GitHubImportProgressPhase::Preparing,
            current_skill: None,
            current_path: None,
            completed_files: 0,
            total_files: 0,
            completed_bytes: 0,
            total_bytes: 0,
        },
    );

    if selections.is_empty() {
        return Err(GithubImportError::NoSelections);
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or(GithubImportError::CentralAgentMissing)?;
    let central_root = central.global_skills_dir;
    let connection = connect_remote_target(active_target)
        .await
        .map_err(|e| GithubImportError::Remote(e.to_string()))?;
    connection
        .mkdir_p(&central_root)
        .await
        .map_err(|e| GithubImportError::Remote(e.to_string()))?;

    let candidates = build_remote_repo_skill_candidates_from_workspace(
        &connection,
        repo,
        &workspace.remote_repo_dir,
        source_path,
    )
    .await?;

    let (staging_ops, skipped_skills) = plan_import_staging(pool, &candidates, selections).await?;

    let total_files = staging_ops.len();
    let total_bytes = 0;
    let mut progress_state = GitHubImportProgressState {
        completed_files: 0,
        total_files,
        completed_bytes: 0,
        total_bytes,
    };

    emit_github_import_progress(
        app,
        GitHubImportProgressPayload {
            phase: GitHubImportProgressPhase::Writing,
            current_skill: None,
            current_path: None,
            completed_files: 0,
            total_files,
            completed_bytes: 0,
            total_bytes,
        },
    );

    let mut imported_skills = Vec::new();
    let mut created_paths: Vec<String> = Vec::new();
    let mut created_stages: Vec<String> = Vec::new();

    for op in &staging_ops {
        let target_dir = remote_join(&central_root, &op.final_skill_id);
        let source_dir =
            remote_skill_source_dir(&workspace.remote_repo_dir, &op.candidate.source_path)?;
        let stage_dir = remote_join(
            &central_root,
            &format!(
                ".skillport-import-{}-{}",
                op.final_skill_id,
                uuid::Uuid::new_v4()
            ),
        );
        if connection
            .exists(&target_dir)
            .await
            .map_err(|e| GithubImportError::Remote(e.to_string()))?
            && op.resolution != DuplicateResolution::Overwrite
        {
            for path in created_paths.iter().rev() {
                let _ = connection.remove_tree(path).await;
            }
            return Err(GithubImportError::TargetDirExists(target_dir));
        }

        created_stages.push(stage_dir.clone());
        let overwrite = if op.resolution == DuplicateResolution::Overwrite {
            "1"
        } else {
            "0"
        };
        if let Err(error) = connection
            .run_script(
                remote_import_skill_script(),
                &[
                    source_dir.as_str(),
                    stage_dir.as_str(),
                    target_dir.as_str(),
                    overwrite,
                ],
            )
            .await
        {
            let _ = connection.remove_tree(&stage_dir).await;
            for path in created_paths.iter().rev() {
                let _ = connection.remove_tree(path).await;
            }
            return Err(GithubImportError::Remote(error.to_string()));
        }
        created_stages.pop();

        created_paths.push(target_dir.clone());

        let skill_md_path = remote_join(&target_dir, "SKILL.md");
        let frontmatter = SkillFrontmatter {
            name: op.candidate.skill_name.clone(),
            description: op.candidate.description.clone(),
        };

        progress_state.completed_files += 1;
        emit_github_import_progress(
            app,
            GitHubImportProgressPayload {
                phase: GitHubImportProgressPhase::Writing,
                current_skill: Some(op.candidate.source_path.clone()),
                current_path: Some("SKILL.md".to_string()),
                completed_files: progress_state.completed_files,
                total_files: progress_state.total_files,
                completed_bytes: progress_state.completed_bytes,
                total_bytes: progress_state.total_bytes,
            },
        );

        let db_skill = Skill {
            id: op.final_skill_id.clone(),
            uid: uuid::Uuid::new_v4().to_string(),
            name: frontmatter.name.clone(),
            description: frontmatter.description.clone(),
            file_path: skill_md_path,
            canonical_path: Some(target_dir.clone()),
            is_central: true,
            source: Some(format!("github:{}/{}", repo.owner, repo.repo)),
            content: None,
            scanned_at: Utc::now().to_rfc3339(),
            fs_created_at: None,
            fs_updated_at: None,
        };
        let (resolved_commit_sha, content_digest) =
            provenance_for(provenance, &op.candidate.source_path);
        db::upsert_skill_with_github_repository(
            pool,
            &db_skill,
            &repo.owner,
            &repo.repo,
            &repo.branch,
            &repo.normalized_url,
            &op.candidate.source_path,
            resolved_commit_sha.as_deref(),
            content_digest.as_deref(),
        )
        .await?;

        imported_skills.push(ImportedGitHubSkillSummary {
            source_path: op.candidate.source_path.clone(),
            original_skill_id: op.candidate.skill_id.clone(),
            imported_skill_id: op.final_skill_id.clone(),
            skill_name: frontmatter.name,
            target_directory: target_dir,
            resolution: op.resolution.clone(),
        });
    }

    for stage in created_stages.iter().rev() {
        let _ = connection.remove_tree(stage).await;
    }

    emit_github_import_progress(
        app,
        GitHubImportProgressPayload {
            phase: GitHubImportProgressPhase::Finalizing,
            current_skill: None,
            current_path: None,
            completed_files: progress_state.completed_files,
            total_files: progress_state.total_files,
            completed_bytes: progress_state.completed_bytes,
            total_bytes: progress_state.total_bytes,
        },
    );

    Ok(GitHubRepoImportResult {
        repo: repo.clone(),
        imported_skills,
        skipped_skills,
    })
}

pub(super) fn remote_skill_source_dir(
    remote_repo_dir: &str,
    source_path: &str,
) -> Result<String, GithubImportError> {
    if source_path == "." {
        return Ok(remote_repo_dir.to_string());
    }
    Ok(remote_join(
        remote_repo_dir,
        &normalize_repo_path(source_path)?,
    ))
}

pub(super) fn remote_import_skill_script() -> &'static str {
    r#"set -eu
source_dir=$1
stage_dir=$2
target_dir=$3
overwrite=$4
backup_dir="${target_dir}.skillport-backup-$$"
rm -rf -- "$stage_dir"
mkdir -p -- "$stage_dir"
cp -a "$source_dir/." "$stage_dir/"
if [ -e "$target_dir" ]; then
  if [ "$overwrite" != "1" ]; then
    rm -rf -- "$stage_dir"
    printf 'Target directory already exists: %s\n' "$target_dir" >&2
    exit 23
  fi
  rm -rf -- "$backup_dir"
  mv "$target_dir" "$backup_dir"
fi
if mv "$stage_dir" "$target_dir"; then
  rm -rf -- "$backup_dir"
else
  status=$?
  if [ -e "$backup_dir" ] && [ ! -e "$target_dir" ]; then
    mv "$backup_dir" "$target_dir" || true
  fi
  rm -rf -- "$stage_dir"
  exit "$status"
fi
"#
}

/// Explicitly discard a registered preview snapshot and release its storage.
pub(crate) async fn discard_preview_snapshot_for_target(
    active_target: &ActiveTarget,
    preview_id: &str,
) {
    let ticket = discard_preview_snapshot_for_target_transition(active_target.id(), preview_id);
    if let Some(ticket) = ticket {
        let _ = cleanup_preview_ticket_for_target(active_target, ticket).await;
    }
    retry_pending_preview_cleanup_for_target(active_target).await;
}
