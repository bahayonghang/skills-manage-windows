use super::*;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) async fn import_github_repo_skills_impl(
    pool: &DbPool,
    secrets: &dyn crate::secrets::SecretStore,
    repo_url: &str,
    selections: Vec<GitHubSkillImportSelection>,
    app: Option<&AppHandle>,
) -> Result<GitHubRepoImportResult, String> {
    let auth = github_direct_auth_from_secret_store(pool, secrets).await?;
    import_github_repo_skills_with_auth(pool, repo_url, selections, app, auth.as_deref()).await
}
pub(crate) async fn import_github_repo_skills_with_auth(
    pool: &DbPool,
    repo_url: &str,
    selections: Vec<GitHubSkillImportSelection>,
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
    let resolved = resolve_repo_source(repo_url, auth).await?;
    let client = github_client()?;
    let snapshot = download_repo_snapshot(&client, &resolved.repo, auth).await?;
    let candidates = build_repo_skill_candidates_from_snapshot_at_path(
        &resolved.repo,
        &snapshot,
        resolved.source_path.as_deref(),
    )?;
    if candidates.is_empty() {
        return Err(NO_IMPORTABLE_SKILLS_ERROR.to_string());
    }
    let central_root = central_skills_root(pool).await?;
    std::fs::create_dir_all(&central_root)
        .map_err(|e| format!("Failed to create central skills directory: {}", e))?;
    let (mut staging_ops, skipped_skills) =
        plan_import_staging(pool, &candidates, selections).await?;

    for op in &mut staging_ops {
        op.source_files = collect_snapshot_source_files(&snapshot, &op.candidate.source_path)?;
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
    let mut created_paths = Vec::new();

    for op in &staging_ops {
        let target_dir = central_root.join(&op.final_skill_id);
        if target_dir.exists() {
            if op.resolution == DuplicateResolution::Overwrite {
                std::fs::remove_dir_all(&target_dir).map_err(|e| {
                    format!(
                        "Failed to replace existing canonical skill '{}': {}",
                        op.final_skill_id, e
                    )
                })?;
            } else {
                cleanup_created_directories(&created_paths);
                return Err(format!(
                    "Target directory '{}' already exists.",
                    target_dir.display()
                ));
            }
        }

        if let Err(error) = write_snapshot_source_to_target(
            &snapshot,
            &op.source_files,
            &target_dir,
            &op.candidate.source_path,
            &mut progress_state,
            app,
        ) {
            cleanup_created_directories(&created_paths);
            if target_dir.exists() {
                let _ = std::fs::remove_dir_all(&target_dir);
            }
            return Err(error);
        }

        created_paths.push(target_dir.clone());

        let skill_md_path = target_dir.join("SKILL.md");
        let raw = std::fs::read_to_string(&skill_md_path)
            .map_err(|e| format!("Failed to read imported SKILL.md: {}", e))?;
        let frontmatter = parse_frontmatter(&raw).ok_or_else(|| {
            format!(
                "Imported skill '{}' is missing valid frontmatter.",
                op.candidate.source_path
            )
        })?;

        let db_skill = Skill {
            id: op.final_skill_id.clone(),
            name: frontmatter.name.clone(),
            description: frontmatter.description.clone(),
            file_path: skill_md_path.to_string_lossy().into_owned(),
            canonical_path: Some(target_dir.to_string_lossy().into_owned()),
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
            target_directory: target_dir.to_string_lossy().into_owned(),
            resolution: op.resolution.clone(),
        });
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
        repo: resolved.repo,
        imported_skills,
        skipped_skills,
    })
}

pub(crate) async fn import_github_repo_skills_partially_with_auth(
    pool: &DbPool,
    repo_url: &str,
    selections: Vec<GitHubSkillImportSelection>,
    app: Option<&AppHandle>,
    auth: Option<&str>,
) -> Result<PartialGitHubRepoImportResult, String> {
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
    let snapshot = download_repo_snapshot(&client, &resolved.repo, auth).await?;
    let inspected = inspect_repo_skill_candidates_from_snapshot_at_path(
        &resolved.repo,
        &snapshot,
        resolved.source_path.as_deref(),
    )?;

    let central_root = central_skills_root(pool).await?;
    std::fs::create_dir_all(&central_root)
        .map_err(|e| format!("Failed to create central skills directory: {}", e))?;

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

#[derive(Debug)]
pub(super) struct ExistingSkillBackup {
    pub(super) path: PathBuf,
}

pub(super) async fn plan_import_staging(
    pool: &DbPool,
    candidates: &[RemoteSkillCandidate],
    selections: Vec<GitHubSkillImportSelection>,
) -> Result<(Vec<StagedImport>, Vec<String>), String> {
    if selections.is_empty() {
        return Err("Select at least one skill to import.".to_string());
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
                format!(
                    "Selected skill '{}' is no longer available in the preview.",
                    selection.source_path
                )
            })?;

        if !selected_paths.insert(candidate.source_path.clone()) {
            return Err(format!(
                "Skill '{}' was selected more than once.",
                candidate.source_path
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
                        format!(
                            "Skill '{}' requires a renamed skill id for rename resolution.",
                            candidate.source_path
                        )
                    })?)?;
                if occupied_ids.contains(&requested_id) {
                    return Err(format!(
                        "Renamed skill id '{}' is already in use.",
                        requested_id
                    ));
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
        return Err("No valid import operations were requested.".to_string());
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
) -> Result<PartialGitHubRepoImportResult, String> {
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
                error,
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
            repo,
            snapshot,
            central_root,
            op,
            &mut progress_state,
            app,
        )
        .await
        {
            Ok(summary) => imported_skills.push(summary),
            Err(error) => failed_skills.push(PartialGitHubRepoImportFailure {
                source_path: op.candidate.source_path.clone(),
                error,
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

pub(super) async fn import_single_staged_skill(
    pool: &DbPool,
    repo: &GitHubRepoRef,
    snapshot: &GitHubRepoSnapshot,
    central_root: &Path,
    op: &StagedImport,
    progress_state: &mut GitHubImportProgressState,
    app: Option<&AppHandle>,
) -> Result<ImportedGitHubSkillSummary, String> {
    let target_dir = central_root.join(&op.final_skill_id);
    let staging_path = create_unique_work_dir(central_root, ".skillport-import-")?;

    if let Err(error) = write_snapshot_source_to_target(
        snapshot,
        &op.source_files,
        &staging_path,
        &op.candidate.source_path,
        progress_state,
        app,
    ) {
        let _ = std::fs::remove_dir_all(&staging_path);
        return Err(error);
    }

    let skill_md_path = staging_path.join("SKILL.md");
    let raw = std::fs::read_to_string(&skill_md_path).map_err(|e| {
        let _ = std::fs::remove_dir_all(&staging_path);
        format!("Failed to read imported SKILL.md: {}", e)
    })?;
    let frontmatter = parse_frontmatter(&raw).ok_or_else(|| {
        let _ = std::fs::remove_dir_all(&staging_path);
        format!(
            "Imported skill '{}' is missing valid frontmatter.",
            op.candidate.source_path
        )
    })?;

    let mut existing_backup = None;
    if target_dir.exists() {
        if op.resolution == DuplicateResolution::Overwrite {
            existing_backup = Some(match backup_existing_skill_dir(central_root, &target_dir) {
                Ok(backup) => backup,
                Err(error) => {
                    let _ = std::fs::remove_dir_all(&staging_path);
                    return Err(error);
                }
            });
        } else {
            let _ = std::fs::remove_dir_all(&staging_path);
            return Err(format!(
                "Target directory '{}' already exists.",
                target_dir.display()
            ));
        }
    }

    if let Err(error) = std::fs::rename(&staging_path, &target_dir) {
        let _ = std::fs::remove_dir_all(&staging_path);
        restore_or_cleanup_target_dir(&target_dir, existing_backup);
        return Err(format!(
            "Failed to finalize import target directory '{}': {}",
            target_dir.display(),
            error
        ));
    }

    let db_skill = Skill {
        id: op.final_skill_id.clone(),
        name: frontmatter.name.clone(),
        description: frontmatter.description.clone(),
        file_path: target_dir.join("SKILL.md").to_string_lossy().into_owned(),
        canonical_path: Some(target_dir.to_string_lossy().into_owned()),
        is_central: true,
        source: Some(format!("github:{}/{}", repo.owner, repo.repo)),
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    if let Err(error) = db::upsert_skill(pool, &db_skill).await {
        restore_or_cleanup_target_dir(&target_dir, existing_backup);
        return Err(error);
    }
    if let Err(error) = db::assign_github_repository_to_skill(
        pool,
        &repo.owner,
        &repo.repo,
        &repo.branch,
        &repo.normalized_url,
        &op.final_skill_id,
        &op.candidate.source_path,
    )
    .await
    {
        restore_or_cleanup_target_dir(&target_dir, existing_backup);
        return Err(error);
    }

    drop_existing_backup(existing_backup);

    Ok(ImportedGitHubSkillSummary {
        source_path: op.candidate.source_path.clone(),
        original_skill_id: op.candidate.skill_id.clone(),
        imported_skill_id: op.final_skill_id.clone(),
        skill_name: frontmatter.name,
        target_directory: target_dir.to_string_lossy().into_owned(),
        resolution: op.resolution.clone(),
    })
}

pub(super) fn backup_existing_skill_dir(
    central_root: &Path,
    target_dir: &Path,
) -> Result<ExistingSkillBackup, String> {
    let backup_path = central_root.join(format!(".skillport-backup-{}", Uuid::new_v4()));
    std::fs::rename(target_dir, &backup_path).map_err(|e| {
        format!(
            "Failed to back up existing canonical skill '{}': {}",
            target_dir.display(),
            e
        )
    })?;
    Ok(ExistingSkillBackup { path: backup_path })
}

pub(super) fn restore_or_cleanup_target_dir(
    target_dir: &Path,
    backup: Option<ExistingSkillBackup>,
) {
    if target_dir.exists() {
        let _ = std::fs::remove_dir_all(target_dir);
    }
    if let Some(backup) = backup {
        let _ = std::fs::rename(&backup.path, target_dir);
    }
}

pub(super) fn drop_existing_backup(backup: Option<ExistingSkillBackup>) {
    if let Some(backup) = backup {
        let _ = std::fs::remove_dir_all(&backup.path);
    }
}

pub(super) fn create_unique_work_dir(parent: &Path, prefix: &str) -> Result<PathBuf, String> {
    let path = parent.join(format!("{prefix}{}", Uuid::new_v4()));
    std::fs::create_dir_all(&path).map_err(|e| {
        format!(
            "Failed to create staging directory '{}': {}",
            path.display(),
            e
        )
    })?;
    Ok(path)
}

pub(super) fn cleanup_created_directories(paths: &[PathBuf]) {
    for path in paths.iter().rev() {
        let _ = std::fs::remove_dir_all(path);
    }
}

pub(crate) async fn central_skills_root(pool: &DbPool) -> Result<PathBuf, String> {
    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    Ok(PathBuf::from(central.global_skills_dir))
}

pub(super) async fn current_central_skill_ids(pool: &DbPool) -> Result<HashSet<String>, String> {
    let rows = sqlx::query("SELECT id FROM skills WHERE is_central = 1")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .iter()
        .map(|row| row.get::<String, _>("id"))
        .collect::<HashSet<_>>())
}

pub(super) async fn build_preview_skills(
    pool: &DbPool,
    candidates: &[RemoteSkillCandidate],
) -> Result<Vec<GitHubSkillPreview>, String> {
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
            root_directory: candidate.root_directory.clone(),
            skill_directory_name: candidate.skill_directory_name.clone(),
            download_url: candidate.download_url.clone(),
            conflict,
        });
    }
    Ok(skills)
}
