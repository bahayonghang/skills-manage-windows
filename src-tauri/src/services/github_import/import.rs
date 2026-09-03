use super::tree_import::{try_prepare_tree_import, TreeImportOutcome, TreeSelectionScope};
use super::*;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) async fn import_github_repo_skills_impl(
    pool: &DbPool,
    secrets: &dyn crate::secrets::SecretStore,
    repo_url: &str,
    selections: Vec<GitHubSkillImportSelection>,
    app: Option<&AppHandle>,
) -> Result<GitHubRepoImportResult, GithubImportError> {
    let auth = github_direct_auth_from_secret_store(pool, secrets).await?;
    import_github_repo_skills_with_auth(pool, repo_url, selections, app, auth.as_deref()).await
}

pub(crate) async fn import_github_repo_skills_with_auth(
    pool: &DbPool,
    repo_url: &str,
    selections: Vec<GitHubSkillImportSelection>,
    app: Option<&AppHandle>,
    auth: Option<&str>,
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
    let resolved = resolve_repo_source(repo_url, auth).await?;
    let client = github_client()?;
    let (snapshot, candidates) = match try_prepare_tree_import(
        &client,
        &resolved.repo,
        &resolved.repo,
        resolved.source_path.as_deref(),
        TreeSelectionScope::Selected(&selections),
        auth,
        false,
    )
    .await?
    {
        TreeImportOutcome::Ready {
            snapshot,
            inspected,
        } => (snapshot, inspected.valid_candidates),
        TreeImportOutcome::Fallback(_reason) => {
            let snapshot = download_repo_snapshot(&client, &resolved.repo, auth).await?;
            let candidates = build_repo_skill_candidates_from_snapshot_at_path(
                &resolved.repo,
                &snapshot,
                resolved.source_path.as_deref(),
            )?;
            (snapshot, candidates)
        }
    };
    if candidates.is_empty() {
        return Err(GithubImportError::NoImportableSkills);
    }
    let central_root = central_skills_root(pool).await?;
    std::fs::create_dir_all(&central_root)
        .map_err(|e| GithubImportError::io("Failed to create central skills directory", e))?;

    import_github_repo_skills_from_snapshot(
        pool,
        &resolved.repo,
        &snapshot,
        &candidates,
        selections,
        &central_root,
        None,
        app,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn import_github_repo_skills_from_snapshot(
    pool: &DbPool,
    repo: &GitHubRepoRef,
    snapshot: &GitHubRepoSnapshot,
    candidates: &[RemoteSkillCandidate],
    selections: Vec<GitHubSkillImportSelection>,
    central_root: &Path,
    provenance: Option<&ImportProvenance>,
    app: Option<&AppHandle>,
) -> Result<GitHubRepoImportResult, GithubImportError> {
    let (mut staging_ops, skipped_skills) =
        plan_import_staging(pool, candidates, selections).await?;

    for op in &mut staging_ops {
        op.source_files = collect_snapshot_source_files(snapshot, &op.candidate.source_path)?;
    }

    let total_files = staging_ops
        .iter()
        .map(|op| op.source_files.len())
        .sum::<usize>();
    let total_bytes = staging_ops
        .iter()
        .flat_map(|op| op.source_files.iter())
        .map(|file| file.byte_len as u64)
        .sum::<u64>();
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

    for op in &staging_ops {
        let summary = import_single_staged_skill(
            pool,
            &crate::services::central_updates::CentralFs::Local,
            repo,
            snapshot,
            central_root,
            op,
            provenance,
            &mut progress_state,
            app,
            true,
        )
        .await?;
        imported_skills.push(summary);
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

/// Import Central Update selections from a repository snapshot already bound
/// to an immutable commit. This is deliberately narrower than the generic
/// URL-based importer: it performs no source resolution or network request.
pub(crate) async fn import_github_repo_skills_from_pinned_snapshot(
    pool: &DbPool,
    repo: &GitHubRepoRef,
    resolved_commit_sha: &str,
    snapshot: &GitHubRepoSnapshot,
    selections: Vec<GitHubSkillImportSelection>,
    app: Option<&AppHandle>,
) -> Result<GitHubRepoImportResult, GithubImportError> {
    validate_commit_sha(resolved_commit_sha)?;
    let candidates = build_repo_skill_candidates_from_snapshot_at_path(repo, snapshot, None)?;
    if candidates.is_empty() {
        return Err(GithubImportError::NoImportableSkills);
    }
    let content_digest_by_source_path = candidates
        .iter()
        .map(|candidate| {
            Ok((
                candidate.source_path.clone(),
                candidate_content_digest_from_snapshot(snapshot, &candidate.source_path)?,
            ))
        })
        .collect::<Result<HashMap<_, _>, GithubImportError>>()?;
    let provenance = ImportProvenance {
        resolved_commit_sha: resolved_commit_sha.to_string(),
        content_digest_by_source_path,
    };
    let central_root = central_skills_root(pool).await?;
    std::fs::create_dir_all(&central_root).map_err(|error| {
        GithubImportError::io("Failed to create central skills directory", error)
    })?;
    import_github_repo_skills_from_snapshot(
        pool,
        repo,
        snapshot,
        &candidates,
        selections,
        &central_root,
        Some(&provenance),
        app,
    )
    .await
}

pub(crate) async fn import_github_repo_skills_partially_with_auth(
    pool: &DbPool,
    repo_url: &str,
    selections: Vec<GitHubSkillImportSelection>,
    app: Option<&AppHandle>,
    auth: Option<&str>,
) -> Result<PartialGitHubRepoImportResult, GithubImportError> {
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

    let resolved = resolve_repo_source(repo_url, auth).await?;
    let client = github_client()?;
    let (snapshot, inspected) = match try_prepare_tree_import(
        &client,
        &resolved.repo,
        &resolved.repo,
        resolved.source_path.as_deref(),
        TreeSelectionScope::Selected(&selections),
        auth,
        true,
    )
    .await?
    {
        TreeImportOutcome::Ready {
            snapshot,
            inspected,
        } => (snapshot, inspected),
        TreeImportOutcome::Fallback(_reason) => {
            let snapshot = download_repo_snapshot(&client, &resolved.repo, auth).await?;
            let inspected = inspect_repo_skill_candidates_from_snapshot_at_path(
                &resolved.repo,
                &snapshot,
                resolved.source_path.as_deref(),
            )?;
            (snapshot, inspected)
        }
    };

    let central_root = central_skills_root(pool).await?;
    std::fs::create_dir_all(&central_root)
        .map_err(|e| GithubImportError::io("Failed to create central skills directory", e))?;

    import_github_repo_skills_from_snapshot_partially(
        pool,
        &resolved.repo,
        &snapshot,
        inspected,
        selections,
        &central_root,
        app,
    )
    .await
}

pub(super) struct StagedImport {
    pub(super) candidate: RemoteSkillCandidate,
    pub(super) final_skill_id: String,
    pub(super) resolution: DuplicateResolution,
    pub(super) source_files: Vec<SnapshotSourceFile>,
}

pub(super) async fn plan_import_staging(
    pool: &DbPool,
    candidates: &[RemoteSkillCandidate],
    selections: Vec<GitHubSkillImportSelection>,
) -> Result<(Vec<StagedImport>, Vec<String>), GithubImportError> {
    if selections.is_empty() {
        return Err(GithubImportError::NoSelections);
    }

    let mut selected_paths = HashSet::new();
    let mut occupied_ids = current_central_skill_ids(pool).await?;
    let mut staging_ops = Vec::new();
    let mut skipped_skills = Vec::new();

    for selection in selections {
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.source_path == selection.source_path)
            .ok_or_else(|| {
                GithubImportError::SelectionUnavailable(selection.source_path.clone())
            })?;

        if !selected_paths.insert(candidate.source_path.clone()) {
            return Err(GithubImportError::DuplicateSelection(
                candidate.source_path.clone(),
            ));
        }

        match selection.resolution {
            DuplicateResolution::Skip => {
                skipped_skills.push(candidate.source_path.clone());
            }
            DuplicateResolution::Overwrite => {
                occupied_ids.insert(candidate.skill_id.clone());
                staging_ops.push(StagedImport {
                    candidate: candidate.clone(),
                    final_skill_id: candidate.skill_id.clone(),
                    resolution: DuplicateResolution::Overwrite,
                    source_files: Vec::new(),
                });
            }
            DuplicateResolution::Rename => {
                let requested_id =
                    sanitize_skill_id(selection.renamed_skill_id.as_deref().ok_or_else(|| {
                        GithubImportError::RenameIdRequired(candidate.source_path.clone())
                    })?)?;
                if occupied_ids.contains(&requested_id) {
                    return Err(GithubImportError::RenameIdInUse(requested_id));
                }
                occupied_ids.insert(requested_id.clone());
                staging_ops.push(StagedImport {
                    candidate: candidate.clone(),
                    final_skill_id: requested_id,
                    resolution: DuplicateResolution::Rename,
                    source_files: Vec::new(),
                });
            }
        }
    }

    if staging_ops.is_empty() && skipped_skills.is_empty() {
        return Err(GithubImportError::NoValidOperations);
    }

    Ok((staging_ops, skipped_skills))
}

pub(crate) async fn import_github_repo_skills_from_snapshot_partially(
    pool: &DbPool,
    repo: &GitHubRepoRef,
    snapshot: &GitHubRepoSnapshot,
    inspected: InspectedGitHubRepoSkills,
    selections: Vec<GitHubSkillImportSelection>,
    central_root: &Path,
    app: Option<&AppHandle>,
) -> Result<PartialGitHubRepoImportResult, GithubImportError> {
    let invalid_by_path = inspected
        .invalid_candidates
        .into_iter()
        .map(|candidate| (candidate.source_path.clone(), candidate))
        .collect::<HashMap<_, _>>();
    let valid_by_path = inspected
        .valid_candidates
        .iter()
        .map(|candidate| (candidate.source_path.clone(), candidate.clone()))
        .collect::<HashMap<_, _>>();

    let mut accepted_selections = Vec::new();
    let mut failed_skills = Vec::new();
    for selection in selections {
        if let Some(invalid) = invalid_by_path.get(&selection.source_path) {
            failed_skills.push(PartialGitHubRepoImportFailure {
                source_path: selection.source_path.clone(),
                error: invalid.detail.clone(),
            });
            continue;
        }
        if valid_by_path.contains_key(&selection.source_path) {
            accepted_selections.push(selection);
            continue;
        }
        failed_skills.push(PartialGitHubRepoImportFailure {
            source_path: selection.source_path.clone(),
            error: format!(
                "Selected skill '{}' is no longer available in the preview.",
                selection.source_path
            ),
        });
    }

    if accepted_selections.is_empty() {
        emit_github_import_progress(
            app,
            GitHubImportProgressPayload {
                phase: GitHubImportProgressPhase::Finalizing,
                current_skill: None,
                current_path: None,
                completed_files: 0,
                total_files: 0,
                completed_bytes: 0,
                total_bytes: 0,
            },
        );
        return Ok(PartialGitHubRepoImportResult {
            repo: repo.clone(),
            imported_skills: Vec::new(),
            skipped_skills: Vec::new(),
            failed_skills,
        });
    }

    let (staging_ops, skipped_skills) =
        plan_import_staging(pool, &inspected.valid_candidates, accepted_selections).await?;
    let mut runnable_ops = Vec::new();
    for mut op in staging_ops {
        match collect_snapshot_source_files(snapshot, &op.candidate.source_path) {
            Ok(files) => {
                op.source_files = files;
                runnable_ops.push(op);
            }
            Err(error) => failed_skills.push(PartialGitHubRepoImportFailure {
                source_path: op.candidate.source_path.clone(),
                error: error.to_string(),
            }),
        }
    }

    let total_files = runnable_ops
        .iter()
        .map(|op| op.source_files.len())
        .sum::<usize>();
    let total_bytes = runnable_ops
        .iter()
        .flat_map(|op| op.source_files.iter())
        .map(|file| file.byte_len as u64)
        .sum::<u64>();
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
    for op in &runnable_ops {
        match import_single_staged_skill(
            pool,
            &crate::services::central_updates::CentralFs::Local,
            repo,
            snapshot,
            central_root,
            op,
            None,
            &mut progress_state,
            app,
            true,
        )
        .await
        {
            Ok(summary) => imported_skills.push(summary),
            Err(error) => failed_skills.push(PartialGitHubRepoImportFailure {
                source_path: op.candidate.source_path.clone(),
                error: error.to_string(),
            }),
        }
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

    Ok(PartialGitHubRepoImportResult {
        repo: repo.clone(),
        imported_skills,
        skipped_skills,
        failed_skills,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn import_single_staged_skill(
    pool: &DbPool,
    fs: &crate::services::central_updates::CentralFs,
    repo: &GitHubRepoRef,
    snapshot: &GitHubRepoSnapshot,
    central_root: &Path,
    op: &StagedImport,
    provenance: Option<&ImportProvenance>,
    progress_state: &mut GitHubImportProgressState,
    app: Option<&AppHandle>,
    emit_per_file_progress: bool,
) -> Result<ImportedGitHubSkillSummary, GithubImportError> {
    let target_dir = match fs {
        crate::services::central_updates::CentralFs::Local => central_root.join(&op.final_skill_id),
        crate::services::central_updates::CentralFs::Remote(_) => PathBuf::from(remote_join(
            &central_root.to_string_lossy().replace('\\', "/"),
            &op.final_skill_id,
        )),
    };
    let frontmatter = frontmatter_from_snapshot(snapshot, &op.candidate.source_path)?;
    for file in &op.source_files {
        if !is_safe_repo_relative_path(&file.relative_path) {
            return Err(GithubImportError::RepoContainsUnsupportedPath(
                file.repo_path.clone(),
            ));
        }
        if emit_per_file_progress {
            progress_state.completed_files += 1;
            progress_state.completed_bytes += file.byte_len as u64;
            emit_github_import_progress(
                app,
                GitHubImportProgressPayload {
                    phase: GitHubImportProgressPhase::Writing,
                    current_skill: Some(op.candidate.source_path.clone()),
                    current_path: Some(file.relative_path.clone()),
                    completed_files: progress_state.completed_files,
                    total_files: progress_state.total_files,
                    completed_bytes: progress_state.completed_bytes,
                    total_bytes: progress_state.total_bytes,
                },
            );
        }
    }
    if !emit_per_file_progress {
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
    }

    if op.resolution != DuplicateResolution::Overwrite
        && matches!(fs, crate::services::central_updates::CentralFs::Local)
        && target_dir.exists()
    {
        return Err(GithubImportError::TargetDirExists(
            target_dir.display().to_string(),
        ));
    }

    let existing = db::get_skill_by_id(pool, &op.final_skill_id).await?;
    let uid = existing
        .as_ref()
        .filter(|skill| skill.is_central)
        .map(|skill| skill.uid.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let (file_path, canonical_path) = import_skill_persistence_paths(fs, &target_dir);
    let mut candidate = op.candidate.clone();
    candidate.skill_name = frontmatter.name.clone();
    candidate.description = frontmatter.description.clone();
    let db_skill = Skill {
        id: op.final_skill_id.clone(),
        uid,
        name: frontmatter.name.clone(),
        description: frontmatter.description.clone(),
        file_path,
        canonical_path: Some(canonical_path),
        is_central: true,
        source: Some(format!("github:{}/{}", repo.owner, repo.repo)),
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    let (resolved_commit_sha, content_digest) =
        provenance_for(provenance, &op.candidate.source_path);
    crate::services::central_updates::journaled_central_content_upsert_with_fs(
        pool,
        fs,
        crate::services::central_updates::JournaledCentralContentUpsert {
            skill: db_skill,
            repo: repo.clone(),
            candidate,
            snapshot,
            target_dir: target_dir.clone(),
            resolved_commit_sha,
            content_digest,
        },
    )
    .await
    .map_err(map_journaled_upsert_error)?;

    Ok(ImportedGitHubSkillSummary {
        source_path: op.candidate.source_path.clone(),
        original_skill_id: op.candidate.skill_id.clone(),
        imported_skill_id: op.final_skill_id.clone(),
        skill_name: frontmatter.name,
        target_directory: target_dir.to_string_lossy().into_owned(),
        resolution: op.resolution.clone(),
    })
}

fn import_skill_persistence_paths(
    fs: &crate::services::central_updates::CentralFs,
    target_dir: &Path,
) -> (String, String) {
    match fs {
        crate::services::central_updates::CentralFs::Local => (
            target_dir.join("SKILL.md").to_string_lossy().into_owned(),
            target_dir.to_string_lossy().into_owned(),
        ),
        crate::services::central_updates::CentralFs::Remote(_) => {
            let canonical_path = target_dir.to_string_lossy().replace('\\', "/");
            (
                crate::targets::remote_join(&canonical_path, "SKILL.md"),
                canonical_path,
            )
        }
    }
}

fn frontmatter_from_snapshot(
    snapshot: &GitHubRepoSnapshot,
    source_path: &str,
) -> Result<SkillFrontmatter, GithubImportError> {
    let bytes = snapshot
        .files
        .iter()
        .find_map(|(repo_path, bytes)| {
            let relative = repo_file_relative_to_source(repo_path, source_path)?;
            relative
                .eq_ignore_ascii_case("SKILL.md")
                .then_some(bytes.as_slice())
        })
        .ok_or_else(|| {
            GithubImportError::ImportedSkillMissingFrontmatter(source_path.to_string())
        })?;
    let raw = std::str::from_utf8(bytes).map_err(|error| {
        GithubImportError::Parse(format!("Skill metadata is not valid UTF-8: {error}"))
    })?;
    parse_frontmatter(raw)
        .ok_or_else(|| GithubImportError::ImportedSkillMissingFrontmatter(source_path.to_string()))
}

fn map_journaled_upsert_error(
    error: crate::services::central_updates::CentralUpdatesError,
) -> GithubImportError {
    use crate::services::central_updates::CentralUpdatesError;
    match error {
        CentralUpdatesError::GithubImport(inner) => inner,
        CentralUpdatesError::Db(error) => GithubImportError::Db(error),
        CentralUpdatesError::Remote(message) => GithubImportError::Remote(message),
        CentralUpdatesError::FirstUpsertTargetExists(path) => {
            GithubImportError::TargetDirExists(path)
        }
        other => GithubImportError::CentralApply(Box::new(other)),
    }
}

pub(crate) async fn central_skills_root(pool: &DbPool) -> Result<PathBuf, GithubImportError> {
    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or(GithubImportError::CentralAgentMissing)?;
    Ok(PathBuf::from(central.global_skills_dir))
}

pub(super) async fn current_central_skill_ids(
    pool: &DbPool,
) -> Result<HashSet<String>, GithubImportError> {
    let rows = sqlx::query("SELECT id FROM skills WHERE is_central = 1")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .map(|row| row.get::<String, _>("id"))
        .collect::<HashSet<_>>())
}

pub(crate) async fn build_preview_skills(
    pool: &DbPool,
    candidates: &[RemoteSkillCandidate],
) -> Result<Vec<GitHubSkillPreview>, GithubImportError> {
    let skill_ids = candidates
        .iter()
        .map(|candidate| candidate.skill_id.clone())
        .collect::<Vec<_>>();
    let existing_by_id = db::get_skills_by_ids(pool, &skill_ids).await?;
    let mut skills = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let conflict = existing_by_id
            .get(&candidate.skill_id)
            .and_then(|existing| {
                if existing.is_central {
                    Some(GitHubSkillConflict {
                        existing_skill_id: existing.id.clone(),
                        existing_name: existing.name.clone(),
                        existing_canonical_path: existing.canonical_path.clone(),
                        proposed_skill_id: candidate.skill_id.clone(),
                        proposed_name: candidate.skill_name.clone(),
                    })
                } else {
                    None
                }
            });

        skills.push(GitHubSkillPreview {
            source_path: candidate.source_path.clone(),
            skill_id: candidate.skill_id.clone(),
            skill_name: candidate.skill_name.clone(),
            description: candidate.description.clone(),
            plugin_name: candidate.plugin_name.clone(),
            root_directory: candidate.root_directory.clone(),
            skill_directory_name: candidate.skill_directory_name.clone(),
            download_url: candidate.download_url.clone(),
            conflict,
            files: None,
        });
    }
    Ok(skills)
}
