use super::*;
pub(super) async fn cleanup_expired_preview_workspaces_for_connection(
    connection: &ConnectedRemoteTarget,
) {
    for workspace in prune_expired_preview_workspaces(Utc::now()) {
        if workspace.target_id == connection.target_id() {
            let _ = connection
                .remove_tree(&workspace.remote_workspace_dir)
                .await;
        }
    }
}

pub(super) async fn create_remote_preview_workspace(
    connection: &ConnectedRemoteTarget,
    resolved: &ResolvedGitHubRepoSource,
    auth: Option<&str>,
) -> Result<GitHubPreviewWorkspace, String> {
    let archive_url = github_archive_url(&resolved.repo);
    let script = remote_workspace_download_script(auth)?;
    let output = connection
        .run_script(
            &script,
            &[archive_url.as_str(), crate::commands::APP_USER_AGENT],
        )
        .await?;
    let remote_workspace_dir = output
        .lines()
        .last()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .ok_or_else(|| "Remote GitHub preview did not return a workspace path.".to_string())?
        .to_string();
    let remote_repo_dir = remote_join(&remote_workspace_dir, "repo");
    let now = Utc::now();

    Ok(GitHubPreviewWorkspace {
        id: format!("github-preview-{}", uuid::Uuid::new_v4()),
        target_id: connection.target_id().to_string(),
        repo: resolved.repo.clone(),
        source_path: resolved.source_path.clone(),
        remote_workspace_dir,
        remote_repo_dir,
        created_at: now,
        expires_at: now + Duration::minutes(REMOTE_PREVIEW_WORKSPACE_TTL_MINUTES),
    })
}

pub(super) fn github_archive_url(repo: &GitHubRepoRef) -> String {
    format!(
        "https://api.github.com/repos/{}/{}/tarball/{}",
        repo.owner, repo.repo, repo.branch
    )
}

pub(super) fn remote_workspace_download_script(auth: Option<&str>) -> Result<String, String> {
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
for tool in sh curl tar find mktemp; do
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
tar -xzf "$archive_file" -C "$repo_dir" --strip-components=1
rm -f -- "$archive_file"
printf '%s\n' "$workspace"
"#
    ))
}

pub(super) fn curl_auth_header_config_line(token: &str) -> Result<String, String> {
    if token.contains('\n') || token.contains('\r') {
        return Err("GitHub token contains unsupported newline characters.".to_string());
    }
    let escaped = token.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!("header = \"Authorization: Bearer {escaped}\""))
}
pub(crate) async fn import_github_repo_skills_remote_with_auth(
    pool: &DbPool,
    active_target: &ActiveTarget,
    repo_url: &str,
    selections: Vec<GitHubSkillImportSelection>,
    preview_workspace_id: Option<&str>,
    app: Option<&AppHandle>,
    auth: Option<&str>,
) -> Result<GitHubRepoImportResult, String> {
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
        return Err("Select at least one skill to import.".to_string());
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    let central_root = central.global_skills_dir;
    let connection = connect_remote_target(active_target).await?;
    connection.mkdir_p(&central_root).await?;

    let resolved = resolve_repo_source(repo_url, auth).await?;
    let workspace =
        resolve_remote_import_workspace(&connection, &resolved, preview_workspace_id, auth).await?;
    let candidates = build_remote_repo_skill_candidates_from_workspace(
        &connection,
        &resolved.repo,
        &workspace.remote_repo_dir,
        resolved.source_path.as_deref(),
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
        if connection.exists(&target_dir).await? && op.resolution != DuplicateResolution::Overwrite
        {
            for path in created_paths.iter().rev() {
                let _ = connection.remove_tree(path).await;
            }
            return Err(format!("Target directory '{}' already exists.", target_dir));
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
            return Err(error);
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
            name: frontmatter.name.clone(),
            description: frontmatter.description.clone(),
            file_path: skill_md_path,
            canonical_path: Some(target_dir.clone()),
            is_central: true,
            source: Some(format!(
                "github:{}/{}",
                resolved.repo.owner, resolved.repo.repo
            )),
            content: None,
            scanned_at: Utc::now().to_rfc3339(),
            fs_created_at: None,
            fs_updated_at: None,
        };
        db::upsert_skill(pool, &db_skill).await?;
        db::assign_github_repository_to_skill(
            pool,
            &resolved.repo.owner,
            &resolved.repo.repo,
            &resolved.repo.branch,
            &resolved.repo.normalized_url,
            &op.final_skill_id,
            &op.candidate.source_path,
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

    let _ = take_preview_workspace(&workspace.id);
    let _ = connection
        .remove_tree(&workspace.remote_workspace_dir)
        .await;

    Ok(GitHubRepoImportResult {
        repo: resolved.repo,
        imported_skills,
        skipped_skills,
    })
}

pub(super) async fn resolve_remote_import_workspace(
    connection: &ConnectedRemoteTarget,
    resolved: &ResolvedGitHubRepoSource,
    preview_workspace_id: Option<&str>,
    auth: Option<&str>,
) -> Result<GitHubPreviewWorkspace, String> {
    cleanup_expired_preview_workspaces_for_connection(connection).await;

    if let Some(workspace_id) = preview_workspace_id {
        if let Some(workspace) = get_preview_workspace(workspace_id) {
            if !workspace.matches_source(
                connection.target_id(),
                &resolved.repo,
                resolved.source_path.as_deref(),
            ) {
                return Err(
                    "GitHub preview workspace does not match the active target or repository. Preview the repository again."
                        .to_string(),
                );
            }
            if !workspace.is_expired(Utc::now()) {
                return Ok(workspace);
            }
            let _ = take_preview_workspace(workspace_id);
            let _ = connection
                .remove_tree(&workspace.remote_workspace_dir)
                .await;
        }
    }

    let workspace = create_remote_preview_workspace(connection, resolved, auth).await?;
    register_preview_workspace(workspace.clone());
    Ok(workspace)
}

pub(super) fn remote_skill_source_dir(
    remote_repo_dir: &str,
    source_path: &str,
) -> Result<String, String> {
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

pub(crate) async fn fetch_github_skill_markdown_from_remote_workspace(
    state: &State<'_, AppState>,
    workspace_id: &str,
    source_path: Option<&str>,
) -> Result<String, String> {
    let workspace = get_preview_workspace(workspace_id).ok_or_else(|| {
        "GitHub preview workspace has expired. Preview the repository again.".to_string()
    })?;
    if workspace.is_expired(Utc::now()) {
        let _ = take_preview_workspace(workspace_id);
        return Err(
            "GitHub preview workspace has expired. Preview the repository again.".to_string(),
        );
    }
    let source_path = source_path.ok_or_else(|| {
        "A source path is required for remote GitHub markdown preview.".to_string()
    })?;
    let active_target = state.active_target().await?;
    if active_target.is_remote_like() && active_target.id() != workspace.target_id {
        return Err(
            "The active remote target changed after preview. Preview the repository again."
                .to_string(),
        );
    }
    if !active_target.is_remote_like() {
        return Err(
            "Remote GitHub preview workspace is only available on its remote target.".to_string(),
        );
    }
    let skill_md_path = if source_path == "." {
        "SKILL.md".to_string()
    } else {
        join_repo_path(source_path, "SKILL.md")?
    };
    let connection = connect_remote_target(&active_target).await?;
    let bytes = connection
        .read_file(&remote_join(&workspace.remote_repo_dir, &skill_md_path))
        .await?;
    String::from_utf8(bytes).map_err(|e| format!("Remote SKILL.md is not valid UTF-8: {}", e))
}

pub(crate) async fn discard_preview_workspace_for_active_target(
    state: &State<'_, AppState>,
    workspace_id: &str,
) {
    let Some(workspace) = take_preview_workspace(workspace_id) else {
        return;
    };
    let Ok(active_target) = state.active_target().await else {
        return;
    };
    if !active_target.is_remote_like() {
        return;
    }
    if active_target.id() != workspace.target_id {
        return;
    }
    if let Ok(connection) = connect_remote_target(&active_target).await {
        let _ = connection
            .remove_tree(&workspace.remote_workspace_dir)
            .await;
    }
}
