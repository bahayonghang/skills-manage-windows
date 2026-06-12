use super::{
    emit_update_progress, error_state_from_assignment, keep_remote_missing_central_skills_impl,
    load_remote_skill_content, load_selected_central_skills, prepare_skill_updates,
    prepare_snapshots_for_repo_refs, remote_missing_state_from_assignment, repo_cache_key,
    state_from_remote, unsupported_state_from_assignment, update_counters_for_state,
    RemoteSkillLoadError, SkillUpdateStatus, UpdateCounters,
};
use crate::commands::central_updates_fs::{normalize_repo_path, CentralFs};
use crate::commands::github_import::{
    self, GitHubRepoImportResult, GitHubRepoRef, GitHubRepoSnapshot, GitHubSkillImportSelection,
    GitHubSkillPreview,
};
use crate::{
    db::{self, DbPool, SkillRepository, SkillUpdateState},
    services::central_skills::{
        self, BatchDeleteCentralSkillRequest, BatchDeleteCentralSkillResult,
    },
    targets::ActiveTarget,
    AppState,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

mod summaries;

use summaries::build_repository_sync_summaries;
pub(crate) use summaries::build_remote_missing_skills;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralRemoteAddedSkill {
    pub repository_id: String,
    pub repo: GitHubRepoRef,
    pub preview: GitHubSkillPreview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralRemoteMissingSkill {
    pub state: SkillUpdateState,
    pub repository_id: Option<String>,
    pub repository_name: String,
    pub repo: Option<GitHubRepoRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralRepositorySyncSummary {
    pub repository_id: String,
    pub name: String,
    pub checked: usize,
    pub update_available: usize,
    pub remote_missing: usize,
    pub unsupported: usize,
    pub failed: usize,
    pub remote_added: usize,
    pub skipped_remote_added: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralRepositorySyncFailure {
    pub repository_id: String,
    pub name: Option<String>,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralRepositorySyncPreview {
    pub states: Vec<SkillUpdateState>,
    pub remote_added: Vec<CentralRemoteAddedSkill>,
    pub skipped_remote_added: Vec<CentralRemoteAddedSkill>,
    pub remote_missing: Vec<CentralRemoteMissingSkill>,
    pub repositories: Vec<CentralRepositorySyncSummary>,
    pub failed_repositories: Vec<CentralRepositorySyncFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralRepositoryAddedSkillSelection {
    pub repository_id: String,
    pub selections: Vec<GitHubSkillImportSelection>,
    pub preview_workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralRepositoryAdditionSkipRequest {
    pub repository_id: String,
    pub source_path: String,
    pub skill_id: String,
    pub skill_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralRepositoryAdditionUnskipRequest {
    pub repository_id: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CentralRemoteAddedCollection {
    pub remote_added: Vec<CentralRemoteAddedSkill>,
    pub skipped_remote_added: Vec<CentralRemoteAddedSkill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralRepositorySyncDecisions {
    pub keep_skill_ids: Vec<String>,
    pub delete_requests: Vec<BatchDeleteCentralSkillRequest>,
    pub additions: Vec<CentralRepositoryAddedSkillSelection>,
    #[serde(default)]
    pub skip_additions: Vec<CentralRepositoryAdditionSkipRequest>,
    #[serde(default)]
    pub unskip_additions: Vec<CentralRepositoryAdditionUnskipRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralRepositorySyncApplyResult {
    pub kept_skill_ids: Vec<String>,
    pub delete_result: BatchDeleteCentralSkillResult,
    pub import_results: Vec<GitHubRepoImportResult>,
    pub skipped_additions: Vec<CentralRepositoryAdditionSkipRequest>,
    pub unskipped_additions: Vec<CentralRepositoryAdditionUnskipRequest>,
    pub failed_repositories: Vec<CentralRepositorySyncFailure>,
    pub states: Vec<SkillUpdateState>,
}

#[tauri::command]
#[deprecated(
    note = "Use refresh_skill_update_inventory with scope=Repositories instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn check_central_repository_sync(
    app: AppHandle,
    state: State<'_, AppState>,
    repository_ids: Vec<String>,
    skill_ids: Option<Vec<String>>,
) -> Result<CentralRepositorySyncPreview, String> {
    let pool = state.active_db().await?;
    let fs = CentralFs::from_active_target(state.active_target().await?).await?;
    let repository_ids = unique_non_empty(repository_ids);
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    let client = github_import::github_client().map_err(|e| e.to_string())?;
    let valid_repositories =
        load_syncable_github_repositories(&pool, &repository_ids, auth.as_deref()).await?;
    let repo_refs = valid_repositories
        .iter()
        .map(|(_, repo)| repo.clone())
        .collect::<Vec<_>>();
    let repo_by_id = valid_repositories
        .iter()
        .map(|(repository, _)| (repository.id.clone(), repository.clone()))
        .collect::<HashMap<_, _>>();

    let skills = load_selected_central_skills(&pool, skill_ids.as_deref()).await?;
    let total = skills.len();
    let mut counters = UpdateCounters::default();
    let mut states = Vec::with_capacity(total);
    let mut failed_repositories = Vec::new();

    state.central_update_cancel.store(false, Ordering::SeqCst);
    emit_update_progress(&app, "checking", "started", total, &counters, None, None);

    let prepared = prepare_skill_updates(&pool, &fs, skills, auth.as_deref(), false).await?;
    let mut snapshot_repos = prepared
        .iter()
        .filter_map(|prepared_skill| {
            prepared_skill
                .source
                .as_ref()
                .map(|source| source.repo.clone())
        })
        .collect::<Vec<_>>();
    snapshot_repos.extend(repo_refs);
    let snapshots = prepare_snapshots_for_repo_refs(
        &client,
        auth.as_deref(),
        &snapshot_repos,
        &state.central_update_snapshots,
    )
    .await?;

    for prepared_skill in prepared {
        let skill = &prepared_skill.skill;
        if state.central_update_cancel.load(Ordering::SeqCst) {
            emit_update_progress(
                &app,
                "checking",
                SkillUpdateStatus::Cancelled.as_str(),
                total,
                &counters,
                None,
                None,
            );
            break;
        }

        emit_update_progress(
            &app,
            "checking",
            "running",
            total,
            &counters,
            Some(skill),
            None,
        );

        let state_result = match load_remote_skill_content(&prepared_skill, &snapshots) {
            Ok(Some(remote)) => state_from_remote(skill, &remote, false),
            Ok(None) => unsupported_state_from_assignment(skill, &prepared_skill.assignment, None),
            Err(RemoteSkillLoadError::RemoteMissing(reason)) => {
                remote_missing_state_from_assignment(skill, &prepared_skill.assignment, &reason)
            }
            Err(RemoteSkillLoadError::Other(error)) => {
                error_state_from_assignment(skill, &prepared_skill.assignment, &error)
            }
        };

        db::upsert_skill_update_state(&pool, &state_result).await?;
        update_counters_for_state(&mut counters, &state_result);
        emit_update_progress(
            &app,
            "checking",
            &state_result.status,
            total,
            &counters,
            Some(skill),
            state_result.error.as_deref(),
        );
        states.push(state_result);
    }

    emit_update_progress(&app, "checking", "completed", total, &counters, None, None);

    let remote_additions = collect_remote_added_skills(
        &pool,
        &repository_ids,
        &valid_repositories,
        &snapshots,
        &mut failed_repositories,
    )
    .await?;
    let remote_missing_states = states
        .iter()
        .filter(|state| state.status == SkillUpdateStatus::RemoteMissing.as_str())
        .cloned()
        .collect::<Vec<_>>();
    let remote_missing = build_remote_missing_skills(&repo_by_id, remote_missing_states);
    let repositories = build_repository_sync_summaries(
        &repo_by_id,
        &states,
        &remote_additions.remote_added,
        &remote_additions.skipped_remote_added,
        &remote_missing,
    );

    Ok(CentralRepositorySyncPreview {
        states,
        remote_added: remote_additions.remote_added,
        skipped_remote_added: remote_additions.skipped_remote_added,
        remote_missing,
        repositories,
        failed_repositories,
    })
}

#[tauri::command]
#[deprecated(
    note = "Use apply_skill_update_decisions instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn apply_central_repository_sync(
    app: AppHandle,
    state: State<'_, AppState>,
    decisions: CentralRepositorySyncDecisions,
) -> Result<CentralRepositorySyncApplyResult, String> {
    let pool = state.active_db().await?;
    let active_target = state.active_target().await?;
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|e| e.to_string())?;

    let kept_skill_ids =
        keep_remote_missing_central_skills_impl(&pool, &decisions.keep_skill_ids).await?;

    let delete_result = if decisions.delete_requests.is_empty() {
        BatchDeleteCentralSkillResult {
            succeeded: Vec::new(),
            failed: Vec::new(),
        }
    } else {
        match &active_target {
            ActiveTarget::Local => {
                central_skills::delete_central_skills_impl(&pool, &decisions.delete_requests)
                    .await
                    .map_err(|e| e.to_string())?
            }
            ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                central_skills::delete_central_skills_remote_impl(
                    &pool,
                    &active_target,
                    &decisions.delete_requests,
                )
                .await
                .map_err(|e| e.to_string())?
            }
        }
    };

    let mut skipped_additions = Vec::new();
    for request in decisions.skip_additions {
        let source_path = normalize_repo_path(&request.source_path)?;
        let saved = db::upsert_skill_repository_sync_skip(
            &pool,
            &request.repository_id,
            &source_path,
            &request.skill_id,
            &request.skill_name,
        )
        .await?;
        skipped_additions.push(CentralRepositoryAdditionSkipRequest {
            repository_id: saved.repository_id,
            source_path: saved.source_path,
            skill_id: saved.skill_id,
            skill_name: saved.skill_name,
        });
    }

    let mut unskipped_additions = Vec::new();
    for request in decisions.unskip_additions {
        let source_path = normalize_repo_path(&request.source_path)?;
        if db::delete_skill_repository_sync_skip(&pool, &request.repository_id, &source_path)
            .await?
        {
            unskipped_additions.push(CentralRepositoryAdditionUnskipRequest {
                repository_id: request.repository_id,
                source_path,
            });
        }
    }

    let mut import_results = Vec::new();
    let mut failed_repositories = Vec::new();
    for addition in decisions.additions {
        let repository_id = addition.repository_id.clone();
        let mut import_selections = Vec::new();
        for selection in addition.selections {
            if selection.resolution == github_import::DuplicateResolution::Skip {
                let source_path = normalize_repo_path(&selection.source_path)?;
                let saved = db::upsert_skill_repository_sync_skip(
                    &pool,
                    &repository_id,
                    &source_path,
                    &source_path,
                    &source_path,
                )
                .await?;
                skipped_additions.push(CentralRepositoryAdditionSkipRequest {
                    repository_id: saved.repository_id,
                    source_path: saved.source_path,
                    skill_id: saved.skill_id,
                    skill_name: saved.skill_name,
                });
            } else {
                import_selections.push(selection);
            }
        }

        if import_selections.is_empty() {
            continue;
        }
        let Some(repository) = db::get_skill_repository_by_id(&pool, &repository_id).await? else {
            failed_repositories.push(CentralRepositorySyncFailure {
                repository_id,
                name: None,
                error: "Repository no longer exists.".to_string(),
            });
            continue;
        };
        let repo_url = match repository_import_url(&repository) {
            Some(repo_url) => repo_url,
            None => {
                failed_repositories.push(CentralRepositorySyncFailure {
                    repository_id: repository.id,
                    name: Some(repository.name),
                    error: "GitHub repository URL is unavailable.".to_string(),
                });
                continue;
            }
        };

        let result = match &active_target {
            ActiveTarget::Local => {
                github_import::import_github_repo_skills_with_auth(
                    &pool,
                    &repo_url,
                    import_selections,
                    Some(&app),
                    auth.as_deref(),
                )
                .await
            }
            ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                github_import::import_github_repo_skills_remote_with_auth(
                    &pool,
                    &active_target,
                    &repo_url,
                    import_selections,
                    addition.preview_workspace_id.as_deref(),
                    Some(&app),
                    auth.as_deref(),
                )
                .await
            }
        };

        match result {
            Ok(result) => {
                for imported in &result.imported_skills {
                    let source_path = normalize_repo_path(&imported.source_path)?;
                    db::delete_skill_repository_sync_skip(&pool, &repository_id, &source_path)
                        .await?;
                }
                import_results.push(result);
            }
            Err(error) => failed_repositories.push(CentralRepositorySyncFailure {
                repository_id: repository.id,
                name: Some(repository.name),
                error: error.to_string(),
            }),
        }
    }

    let states = db::get_skill_update_states(&pool).await?;
    Ok(CentralRepositorySyncApplyResult {
        kept_skill_ids,
        delete_result,
        import_results,
        skipped_additions,
        unskipped_additions,
        failed_repositories,
        states,
    })
}

fn unique_non_empty(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

async fn load_syncable_github_repositories(
    pool: &DbPool,
    repository_ids: &[String],
    auth_token: Option<&str>,
) -> Result<Vec<(SkillRepository, GitHubRepoRef)>, String> {
    let mut repositories = Vec::new();
    for repository_id in repository_ids {
        let Some(repository) = db::get_skill_repository_by_id(pool, repository_id).await? else {
            continue;
        };
        let Some(repo) = resolve_github_repo_ref_from_repository(&repository, auth_token).await?
        else {
            continue;
        };
        repositories.push((repository, repo));
    }
    Ok(repositories)
}

async fn resolve_github_repo_ref_from_repository(
    repository: &SkillRepository,
    auth_token: Option<&str>,
) -> Result<Option<GitHubRepoRef>, String> {
    if repository.is_unknown || repository.source_type != "github" {
        return Ok(None);
    }

    if let (Some(owner), Some(repo), Some(branch)) = (
        repository.owner.as_ref(),
        repository.repo.as_ref(),
        repository.branch.as_ref(),
    ) {
        return Ok(Some(GitHubRepoRef {
            owner: owner.clone(),
            repo: repo.clone(),
            branch: branch.clone(),
            normalized_url: repository
                .url
                .clone()
                .unwrap_or_else(|| format!("https://github.com/{owner}/{repo}")),
        }));
    }

    let Some(url) = repository_import_url(repository) else {
        return Ok(None);
    };
    Ok(Some(
        github_import::resolve_repo_source(&url, auth_token)
            .await
            .map_err(|e| e.to_string())?
            .repo,
    ))
}

fn repository_import_url(repository: &SkillRepository) -> Option<String> {
    if repository.source_type != "github" || repository.is_unknown {
        return None;
    }
    if let Some(url) = repository
        .url
        .as_deref()
        .filter(|url| !url.trim().is_empty())
    {
        if let Some(branch) = repository
            .branch
            .as_deref()
            .filter(|branch| !branch.is_empty())
        {
            return Some(format!(
                "{}/tree/{}",
                url.trim().trim_end_matches('/'),
                branch
            ));
        }
        return Some(url.to_string());
    }
    match (&repository.owner, &repository.repo, &repository.branch) {
        (Some(owner), Some(repo), Some(branch)) => {
            Some(format!("https://github.com/{owner}/{repo}/tree/{branch}"))
        }
        (Some(owner), Some(repo), None) => Some(format!("https://github.com/{owner}/{repo}")),
        _ => None,
    }
}

pub(crate) async fn collect_remote_added_skills(
    pool: &DbPool,
    repository_ids: &[String],
    repositories: &[(SkillRepository, GitHubRepoRef)],
    snapshots: &HashMap<String, GitHubRepoSnapshot>,
    failed_repositories: &mut Vec<CentralRepositorySyncFailure>,
) -> Result<CentralRemoteAddedCollection, String> {
    let members = db::get_central_repository_members_by_repositories(pool, repository_ids).await?;
    let mut source_paths_by_repo = HashMap::<String, HashSet<String>>::new();
    for member in members {
        let Some(source_path) = member
            .source_path
            .as_deref()
            .map(normalize_repo_path)
            .transpose()?
            .filter(|path| !path.is_empty())
        else {
            continue;
        };
        source_paths_by_repo
            .entry(member.repository.id)
            .or_default()
            .insert(source_path);
    }
    let skips = db::get_skill_repository_sync_skips(pool, repository_ids).await?;
    let mut skipped_paths_by_repo = HashMap::<String, HashSet<String>>::new();
    for skip in skips {
        let source_path = normalize_repo_path(&skip.source_path)?;
        skipped_paths_by_repo
            .entry(skip.repository_id)
            .or_default()
            .insert(source_path);
    }

    let mut remote_added = Vec::new();
    let mut skipped_remote_added = Vec::new();
    for (repository, repo) in repositories {
        let Some(snapshot) = snapshots.get(&repo_cache_key(repo)) else {
            failed_repositories.push(CentralRepositorySyncFailure {
                repository_id: repository.id.clone(),
                name: Some(repository.name.clone()),
                error: "GitHub repository snapshot is unavailable.".to_string(),
            });
            continue;
        };

        let inspected = match github_import::inspect_repo_skill_candidates_from_snapshot_at_path(
            repo, snapshot, None,
        ) {
            Ok(inspected) => inspected,
            Err(error) => {
                failed_repositories.push(CentralRepositorySyncFailure {
                    repository_id: repository.id.clone(),
                    name: Some(repository.name.clone()),
                    error: error.to_string(),
                });
                continue;
            }
        };
        for invalid in inspected.invalid_candidates {
            failed_repositories.push(CentralRepositorySyncFailure {
                repository_id: repository.id.clone(),
                name: Some(repository.name.clone()),
                error: invalid.detail,
            });
        }

        let local_source_paths = source_paths_by_repo
            .get(&repository.id)
            .cloned()
            .unwrap_or_default();
        let candidates = inspected
            .valid_candidates
            .into_iter()
            .filter_map(|candidate| {
                normalize_repo_path(&candidate.source_path)
                    .ok()
                    .filter(|path| !local_source_paths.contains(path))
                    .map(|_| candidate)
            })
            .collect::<Vec<_>>();
        let previews = github_import::build_preview_skills(pool, &candidates)
            .await
            .map_err(|e| e.to_string())?;
        let skipped_paths = skipped_paths_by_repo
            .get(&repository.id)
            .cloned()
            .unwrap_or_default();
        for preview in previews {
            let source_path = normalize_repo_path(&preview.source_path)?;
            let item = CentralRemoteAddedSkill {
                repository_id: repository.id.clone(),
                repo: repo.clone(),
                preview,
            };
            if skipped_paths.contains(&source_path) {
                db::upsert_skill_repository_sync_skip(
                    pool,
                    &repository.id,
                    &source_path,
                    &item.preview.skill_id,
                    &item.preview.skill_name,
                )
                .await?;
                skipped_remote_added.push(item);
            } else {
                remote_added.push(item);
            }
        }
    }

    Ok(CentralRemoteAddedCollection {
        remote_added,
        skipped_remote_added,
    })
}
