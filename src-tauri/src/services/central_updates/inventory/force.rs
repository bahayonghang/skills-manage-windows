use std::collections::HashMap;

use tauri::AppHandle;

use crate::db::{self, DbPool};
use crate::services::central_skills::{
    self, BatchDeleteCentralSkillRequest, BatchDeleteCentralSkillResult,
};
use crate::services::central_updates::{
    collect_remote_added_skills, load_remote_skill_content, prepare_skill_updates,
    prepare_snapshots_for_repo_refs_with_policy, repo_cache_key, state_from_remote,
    update_skills_batch, CentralFs, CentralUpdateSnapshotCache, CentralUpdatesError,
    RemoteSkillLoadError, SkillUpdatePlan, SkillUpdateStatus, SnapshotCachePolicy,
};
use crate::services::github_import::{
    self, DuplicateResolution, GitHubSkillImportSelection, ImportedGitHubSkillSummary,
};
use crate::targets::ActiveTarget;

use super::{
    load_syncable_github_repositories, normalize_ids, prepared_repo_ref, repository_id_for_state,
    repository_import_url, ForceRepositoryMirrorRequest, ForceRepositoryMirrorResult,
    ForceSkillUpdateFailure, ForceSkillUpdateRequest, ForceSkillUpdateResult, ForceSkillUpdateSkip,
    ForceSkillUpdateSuccess,
};

pub(crate) async fn force_update_central_skills_impl(
    pool: &DbPool,
    fs: &CentralFs,
    auth_token: Option<&str>,
    client: &reqwest::Client,
    snapshots_cache: &CentralUpdateSnapshotCache,
    cache_policy: SnapshotCachePolicy,
    request: ForceSkillUpdateRequest,
) -> Result<ForceSkillUpdateResult, CentralUpdatesError> {
    let skill_ids = normalize_ids(request.skill_ids);
    if skill_ids.is_empty() {
        return Err(CentralUpdatesError::NoForceUpdateSelection);
    }
    let skills = db::get_central_skills_by_ids(pool, &skill_ids).await?;
    let prepared = prepare_skill_updates(pool, fs, skills, auth_token, false).await?;
    let snapshot_repos = prepared
        .iter()
        .filter_map(prepared_repo_ref)
        .collect::<Vec<_>>();
    let snapshots = prepare_snapshots_for_repo_refs_with_policy(
        client,
        auth_token,
        &snapshot_repos,
        snapshots_cache,
        cache_policy,
    )
    .await?;

    let mut result = ForceSkillUpdateResult::default();
    let mut pending_updates = Vec::new();
    for prepared_skill in prepared {
        let skill = &prepared_skill.skill;
        match load_remote_skill_content(&prepared_skill, &snapshots) {
            Ok(Some(remote)) => {
                let before = state_from_remote(skill, &remote, false);
                pending_updates.push((skill.clone(), remote, before));
            }
            Ok(None) => result.skipped.push(ForceSkillUpdateSkip {
                skill_id: skill.id.clone(),
                reason: "Unsupported source".to_string(),
            }),
            Err(RemoteSkillLoadError::RemoteMissing(reason)) => {
                result.skipped.push(ForceSkillUpdateSkip {
                    skill_id: skill.id.clone(),
                    reason,
                });
            }
            Err(RemoteSkillLoadError::Other(_error)) => {
                result.failed.push(ForceSkillUpdateFailure {
                    skill_id: skill.id.clone(),
                    repository_id: None,
                    source_path: None,
                    error: "This update item could not be applied.".to_string(),
                });
            }
        }
    }
    let plans = pending_updates
        .iter()
        .map(|(skill, remote, _)| SkillUpdatePlan {
            skill: skill.clone(),
            remote: remote.clone(),
            refresh_copies: request.refresh_copy_installations,
            first_upsert: false,
        })
        .collect();
    for ((skill, _, before), outcome) in pending_updates
        .into_iter()
        .zip(update_skills_batch(pool, fs, plans, None).await)
    {
        debug_assert_eq!(skill.id, outcome.skill_id);
        match outcome.result {
            Ok(state) => {
                db::upsert_skill_update_state(pool, &state).await?;
                result.overwritten.push(ForceSkillUpdateSuccess {
                    skill_id: skill.id,
                    repository_id: repository_id_for_state_from_db(pool, &before).await?,
                    source_path: before.source_path,
                    local_hash: before.last_remote_hash,
                    remote_hash: before.latest_remote_hash,
                    bytes_changed: before.status == SkillUpdateStatus::UpdateAvailable,
                    copy_installations_refreshed: request.refresh_copy_installations,
                });
            }
            Err(error) => result.failed.push(ForceSkillUpdateFailure {
                skill_id: skill.id,
                repository_id: repository_id_for_state_from_db(pool, &before).await?,
                source_path: before.source_path,
                error: error.error().public_update_message().to_string(),
            }),
        }
    }
    let succeeded = result
        .overwritten
        .iter()
        .map(|item| item.skill_id.clone())
        .collect::<Vec<_>>();
    db::delete_skill_update_inventory_entries_for_skills(pool, &succeeded).await?;
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn force_mirror_central_repositories_impl(
    app: Option<&AppHandle>,
    pool: &DbPool,
    active_target: &ActiveTarget,
    fs: &CentralFs,
    auth_token: Option<&str>,
    client: &reqwest::Client,
    snapshots_cache: &CentralUpdateSnapshotCache,
    cache_policy: SnapshotCachePolicy,
    request: ForceRepositoryMirrorRequest,
) -> Result<ForceRepositoryMirrorResult, CentralUpdatesError> {
    if !request.delete_missing && !request.import_added && !request.overwrite_tracked {
        return Err(CentralUpdatesError::NoMirrorOperation);
    }
    let repository_ids = normalize_ids(request.repository_ids);
    if repository_ids.is_empty() {
        return Err(CentralUpdatesError::NoMirrorRepositorySelection);
    }

    let valid_repositories =
        load_syncable_github_repositories(pool, &repository_ids, auth_token).await?;
    let repo_refs = valid_repositories
        .iter()
        .map(|(_, repo)| repo.clone())
        .collect::<Vec<_>>();
    let snapshots = prepare_snapshots_for_repo_refs_with_policy(
        client,
        auth_token,
        &repo_refs,
        snapshots_cache,
        cache_policy,
    )
    .await?;

    let mut overwritten = Vec::new();
    let mut skipped = Vec::new();
    let mut failed_items = Vec::new();
    let mut remote_missing_skill_ids = Vec::new();

    let mut skill_ids = Vec::new();
    for repository_id in &repository_ids {
        skill_ids.extend(db::get_central_skill_ids_by_repository(pool, repository_id).await?);
    }
    let skills = if skill_ids.is_empty() {
        Vec::new()
    } else {
        db::get_central_skills_by_ids(pool, &normalize_ids(skill_ids)).await?
    };
    let prepared = prepare_skill_updates(pool, fs, skills, auth_token, false).await?;
    let mut pending_overwrites = Vec::new();
    for prepared_skill in prepared {
        let skill = &prepared_skill.skill;
        match load_remote_skill_content(&prepared_skill, &snapshots) {
            Ok(Some(remote)) if request.overwrite_tracked => {
                let before = state_from_remote(skill, &remote, false);
                pending_overwrites.push((skill.clone(), remote, before));
            }
            Ok(Some(_)) => {}
            Ok(None) => skipped.push(ForceSkillUpdateSkip {
                skill_id: skill.id.clone(),
                reason: "Unsupported source".to_string(),
            }),
            Err(RemoteSkillLoadError::RemoteMissing(_reason)) if request.delete_missing => {
                remote_missing_skill_ids.push(skill.id.clone());
            }
            Err(RemoteSkillLoadError::RemoteMissing(reason)) => {
                skipped.push(ForceSkillUpdateSkip {
                    skill_id: skill.id.clone(),
                    reason,
                });
            }
            Err(RemoteSkillLoadError::Other(_error)) => {
                failed_items.push(ForceSkillUpdateFailure {
                    skill_id: skill.id.clone(),
                    repository_id: None,
                    source_path: None,
                    error: "This update item could not be applied.".to_string(),
                })
            }
        }
    }
    let overwrite_plans = pending_overwrites
        .iter()
        .map(|(skill, remote, _)| SkillUpdatePlan {
            skill: skill.clone(),
            remote: remote.clone(),
            refresh_copies: true,
            first_upsert: false,
        })
        .collect();
    for ((skill, _, before), outcome) in pending_overwrites
        .into_iter()
        .zip(update_skills_batch(pool, fs, overwrite_plans, None).await)
    {
        debug_assert_eq!(skill.id, outcome.skill_id);
        match outcome.result {
            Ok(state) => {
                db::upsert_skill_update_state(pool, &state).await?;
                overwritten.push(ForceSkillUpdateSuccess {
                    skill_id: skill.id,
                    repository_id: repository_id_for_state_from_db(pool, &before).await?,
                    source_path: before.source_path,
                    local_hash: before.last_remote_hash,
                    remote_hash: before.latest_remote_hash,
                    bytes_changed: before.status == SkillUpdateStatus::UpdateAvailable,
                    copy_installations_refreshed: true,
                });
            }
            Err(error) => failed_items.push(ForceSkillUpdateFailure {
                skill_id: skill.id,
                repository_id: repository_id_for_state_from_db(pool, &before).await?,
                source_path: before.source_path,
                error: error.error().public_update_message().to_string(),
            }),
        }
    }

    let (imported, import_failures) = if request.import_added {
        force_import_remote_added(
            app,
            pool,
            active_target,
            auth_token,
            &repository_ids,
            &valid_repositories,
            &snapshots,
        )
        .await?
    } else {
        (Vec::new(), Vec::new())
    };
    failed_items.extend(import_failures);

    let deleted = if request.delete_missing {
        force_delete_remote_missing(
            pool,
            active_target,
            &remote_missing_skill_ids,
            request.remove_copy_installations_for_deleted,
        )
        .await?
    } else {
        BatchDeleteCentralSkillResult {
            succeeded: Vec::new(),
            failed: Vec::new(),
        }
    };

    let mut affected = overwritten
        .iter()
        .map(|item| item.skill_id.clone())
        .collect::<Vec<_>>();
    affected.extend(imported.iter().map(|item| item.imported_skill_id.clone()));
    affected.extend(deleted.succeeded.iter().map(|item| item.skill_id.clone()));
    db::delete_skill_update_inventory_entries_for_skills(pool, &normalize_ids(affected)).await?;

    Ok(ForceRepositoryMirrorResult {
        overwritten,
        imported,
        deleted,
        skipped,
        failed_repositories: Vec::new(),
        failed_items,
    })
}

async fn repository_id_for_state_from_db(
    pool: &DbPool,
    state: &db::SkillUpdateState,
) -> Result<Option<String>, CentralUpdatesError> {
    let repo_with_stats = db::get_skill_repositories_with_stats(pool).await?;
    let repo_by_id = repo_with_stats
        .iter()
        .map(|r| (r.repository.id.clone(), r.repository.clone()))
        .collect::<HashMap<_, _>>();
    Ok(repository_id_for_state(&repo_by_id, state))
}

async fn force_import_remote_added(
    app: Option<&AppHandle>,
    pool: &DbPool,
    active_target: &ActiveTarget,
    auth_token: Option<&str>,
    repository_ids: &[String],
    valid_repositories: &[(db::SkillRepository, github_import::GitHubRepoRef)],
    snapshots: &super::super::snapshots::SharedGitHubSnapshots,
) -> Result<
    (
        Vec<ImportedGitHubSkillSummary>,
        Vec<ForceSkillUpdateFailure>,
    ),
    CentralUpdatesError,
> {
    let mut failed = Vec::new();
    let collection = collect_remote_added_skills(
        pool,
        repository_ids,
        valid_repositories,
        snapshots,
        &mut failed,
    )
    .await?;
    let mut by_repository: HashMap<String, Vec<GitHubSkillImportSelection>> = HashMap::new();
    for item in collection
        .remote_added
        .into_iter()
        .chain(collection.skipped_remote_added)
    {
        by_repository
            .entry(item.repository_id)
            .or_default()
            .push(GitHubSkillImportSelection {
                source_path: item.preview.source_path,
                resolution: DuplicateResolution::Overwrite,
                renamed_skill_id: None,
            });
    }

    let mut imported = Vec::new();
    let mut failed_items = failed
        .into_iter()
        .map(|failure| ForceSkillUpdateFailure {
            skill_id: failure.repository_id.clone(),
            repository_id: Some(failure.repository_id),
            source_path: None,
            error: failure.error,
        })
        .collect::<Vec<_>>();
    for (repository_id, selections) in by_repository {
        let Some(repository) = db::get_skill_repository_by_id(pool, &repository_id).await? else {
            continue;
        };
        let Some(repo_url) = repository_import_url(&repository) else {
            continue;
        };
        let result = match active_target {
            ActiveTarget::Local => {
                let Some(repo_ref) = valid_repositories
                    .iter()
                    .find(|(repo, _)| repo.id == repository_id)
                    .map(|(_, repo_ref)| repo_ref)
                else {
                    continue;
                };
                let Some(snapshot) = snapshots.get(&repo_cache_key(repo_ref)) else {
                    failed_items.push(ForceSkillUpdateFailure {
                        skill_id: repository_id.clone(),
                        repository_id: Some(repository_id.clone()),
                        source_path: None,
                        error: "GitHub repository snapshot is unavailable.".to_string(),
                    });
                    continue;
                };
                let inspected = github_import::inspect_repo_skill_candidates_from_snapshot_at_path(
                    repo_ref, snapshot, None,
                )?;
                let central_root = github_import::central_skills_root(pool).await?;
                std::fs::create_dir_all(&central_root).map_err(|error| {
                    CentralUpdatesError::io(
                        format!(
                            "Failed to create central skills directory '{}'",
                            central_root.display()
                        ),
                        error,
                    )
                })?;
                let partial = github_import::import_github_repo_skills_from_snapshot_partially(
                    pool,
                    repo_ref,
                    snapshot,
                    inspected,
                    selections,
                    &central_root,
                    app,
                )
                .await?;
                failed_items.extend(partial.failed_skills.into_iter().map(|failure| {
                    ForceSkillUpdateFailure {
                        skill_id: failure.source_path.clone(),
                        repository_id: Some(repository_id.clone()),
                        source_path: Some(failure.source_path),
                        error: failure.error,
                    }
                }));
                github_import::GitHubRepoImportResult {
                    repo: partial.repo,
                    imported_skills: partial.imported_skills,
                    skipped_skills: partial.skipped_skills,
                }
            }
            ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                github_import::import_github_repo_skills_remote_with_auth(
                    pool,
                    active_target,
                    &repo_url,
                    selections,
                    app,
                    auth_token,
                )
                .await?
            }
        };
        imported.extend(result.imported_skills);
    }
    Ok((imported, failed_items))
}

async fn force_delete_remote_missing(
    pool: &DbPool,
    active_target: &ActiveTarget,
    skill_ids: &[String],
    remove_copy_installations: bool,
) -> Result<BatchDeleteCentralSkillResult, CentralUpdatesError> {
    if skill_ids.is_empty() {
        return Ok(BatchDeleteCentralSkillResult {
            succeeded: Vec::new(),
            failed: Vec::new(),
        });
    }
    let preview = central_skills::preview_delete_central_skills_impl(pool, skill_ids).await?;
    let requests = preview
        .previews
        .into_iter()
        .map(|preview| BatchDeleteCentralSkillRequest {
            skill_id: preview.skill_id,
            remove_agent_ids: if remove_copy_installations {
                preview
                    .copy_installations
                    .into_iter()
                    .map(|installation| installation.agent_id)
                    .collect()
            } else {
                Vec::new()
            },
            force: false,
        })
        .collect::<Vec<_>>();
    Ok(match active_target {
        ActiveTarget::Local => central_skills::delete_central_skills_impl(pool, &requests).await?,
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            central_skills::delete_central_skills_remote_impl(pool, active_target, &requests)
                .await?
        }
    })
}
