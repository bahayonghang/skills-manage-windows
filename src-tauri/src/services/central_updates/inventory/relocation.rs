use std::collections::{HashMap, HashSet};

use crate::db::{DbPool, SkillUpdateState};
use crate::services::central_updates::snapshots::SharedGitHubSnapshots;
use crate::services::central_updates::{
    normalize_repo_path, state_from_relocated_source, CentralRemoteAddedSkill, CentralUpdatesError,
    PreparedSkillUpdate, RemoteSkillLoadError, SkillUpdateStatus,
};
use crate::services::github_import;
use chrono::Utc;

use super::{FailedRepository, FailedRepositoryRetry, RepositoryOwnedUpdateState, UpdatableSkill};

pub(super) struct RelocationContext<'a> {
    pub pool: &'a DbPool,
    pub prepared_by_skill_id: &'a HashMap<String, PreparedSkillUpdate>,
    pub snapshots: &'a SharedGitHubSnapshots,
    pub repo_ref_by_id: &'a HashMap<String, github_import::GitHubRepoRef>,
    pub remote_missing_states: &'a mut Vec<RepositoryOwnedUpdateState>,
    pub remote_added_items: &'a mut Vec<CentralRemoteAddedSkill>,
    pub updatable: &'a mut Vec<UpdatableSkill>,
    pub failed_repositories: &'a mut Vec<FailedRepository>,
}

/// Minimal view of a remote skill location, so the incremental path (remote
/// addition previews) and the regular path (whole-snapshot candidates) share
/// one matching rule.
pub(super) struct RelocationCandidateRef<'a> {
    pub skill_id: &'a str,
    pub source_path: &'a str,
}

pub(super) enum RelocationError {
    Load(RemoteSkillLoadError),
    Db(CentralUpdatesError),
}

impl From<RemoteSkillLoadError> for RelocationError {
    fn from(error: RemoteSkillLoadError) -> Self {
        Self::Load(error)
    }
}

impl From<CentralUpdatesError> for RelocationError {
    fn from(error: CentralUpdatesError) -> Self {
        Self::Db(error)
    }
}

/// A relocation is only applied when exactly one remote location carries the
/// same skill id at a different path. Zero matches means the skill was removed
/// upstream; several matches make the new home ambiguous. Both are left to the
/// user instead of being guessed.
pub(super) fn unique_relocation_target(
    skill_id: &str,
    old_path: &str,
    candidates: &[RelocationCandidateRef<'_>],
) -> Option<String> {
    let mut matched = candidates
        .iter()
        .filter(|candidate| candidate.skill_id == skill_id && candidate.source_path != old_path);
    let first = matched.next()?;
    if matched.next().is_some() {
        return None;
    }
    Some(first.source_path.to_string())
}

/// Rebuild the update state against the new path and persist the new source
/// path. Shared by both refresh modes so a relocation always lands the same way.
pub(super) async fn apply_relocation(
    pool: &DbPool,
    prepared: &PreparedSkillUpdate,
    repo: &github_import::GitHubRepoRef,
    repository_id: &str,
    new_path: &str,
    snapshots: &SharedGitHubSnapshots,
) -> Result<SkillUpdateState, RelocationError> {
    let state = state_from_relocated_source(prepared, repo, new_path, snapshots)?;
    persist_relocated_skill(pool, repository_id, &prepared.skill.id, new_path).await?;
    Ok(state)
}

/// A regular-mode skill whose tracked source path is gone from the snapshot.
pub(super) struct PendingRelocation {
    pub skill_id: String,
    pub repository_id: String,
}

/// Regular mode has no remote-addition listing, so the new home of a moved or
/// renamed skill is looked up in the repository snapshot that was already
/// downloaded for the hash comparison. No additional network request is made.
pub(super) async fn resolve_regular_mode_relocations(
    pool: &DbPool,
    pending: &[PendingRelocation],
    prepared_by_skill_id: &HashMap<String, PreparedSkillUpdate>,
    snapshots: &SharedGitHubSnapshots,
    updatable: &mut Vec<UpdatableSkill>,
    failed_repositories: &mut Vec<FailedRepository>,
) -> Result<(), CentralUpdatesError> {
    let mut candidates_by_repo_key = HashMap::<String, Vec<(String, String)>>::new();

    for item in pending {
        let Some(prepared) = prepared_by_skill_id.get(&item.skill_id) else {
            failed_repositories.push(source_missing_failure(&item.repository_id, None));
            continue;
        };
        let Some(source) = prepared.source.as_ref() else {
            failed_repositories.push(source_missing_failure(&item.repository_id, None));
            continue;
        };
        let old_path = source.source_path.clone();
        let repo = source.repo.clone();
        let cache_key = super::super::snapshots::repo_cache_key(&repo);

        if !candidates_by_repo_key.contains_key(&cache_key) {
            let Some(snapshot) = snapshots.get(&cache_key) else {
                failed_repositories.push(FailedRepository {
                    repository_id: item.repository_id.clone(),
                    error: repository_check_failed_message(),
                    error_code: Some(REPOSITORY_CHECK_FAILED_CODE.to_string()),
                    diagnostic_category: None,
                    retry: FailedRepositoryRetry::Retryable,
                    diagnostics: None,
                });
                continue;
            };
            let candidates = github_import::build_repo_skill_candidates_from_snapshot_at_path(
                &repo, snapshot, None,
            )?;
            let normalized = candidates
                .into_iter()
                .filter_map(|candidate| {
                    normalize_repo_path(&candidate.source_path)
                        .ok()
                        .map(|path| (candidate.skill_id, path))
                })
                .collect::<Vec<_>>();
            candidates_by_repo_key.insert(cache_key.clone(), normalized);
        }

        let candidate_refs = candidates_by_repo_key[&cache_key]
            .iter()
            .map(|(skill_id, source_path)| RelocationCandidateRef {
                skill_id,
                source_path,
            })
            .collect::<Vec<_>>();

        let Some(new_path) = unique_relocation_target(&item.skill_id, &old_path, &candidate_refs)
        else {
            failed_repositories.push(source_missing_failure(
                &item.repository_id,
                Some(old_path.clone()),
            ));
            continue;
        };

        let occupied_by = crate::db::get_skill_id_for_repository_source_path(
            pool,
            &item.repository_id,
            &new_path,
        )
        .await?;
        if occupied_by.is_some_and(|owner| owner != item.skill_id) {
            failed_repositories.push(source_missing_failure(
                &item.repository_id,
                Some(old_path.clone()),
            ));
            continue;
        }

        match apply_relocation(
            pool,
            prepared,
            &repo,
            &item.repository_id,
            &new_path,
            snapshots,
        )
        .await
        {
            Ok(state) => {
                if state.status == SkillUpdateStatus::UpdateAvailable {
                    updatable.push(UpdatableSkill {
                        state,
                        repository_id: Some(item.repository_id.clone()),
                        diagnostics: None,
                    });
                }
            }
            Err(RelocationError::Db(error)) => return Err(error),
            Err(RelocationError::Load(error)) => {
                failed_repositories.push(FailedRepository {
                    repository_id: item.repository_id.clone(),
                    error: format!(
                        "Failed to auto-resolve moved skill '{}': {}",
                        item.skill_id,
                        remote_load_error_message(error)
                    ),
                    error_code: Some(RELOCATION_FAILED_CODE.to_string()),
                    diagnostic_category: None,
                    retry: FailedRepositoryRetry::Retryable,
                    diagnostics: None,
                });
            }
        }
    }

    Ok(())
}

pub(super) const RELOCATION_FAILED_CODE: &str = "central_updates.relocation_failed";
pub(super) const SKILL_SOURCE_MISSING_CODE: &str = "central_updates.skill_source_missing";
pub(super) const REPOSITORY_CHECK_FAILED_CODE: &str = "central_updates.repository_check_failed";

pub(super) fn repository_check_failed_message() -> String {
    crate::ipc_error::public_message_for_code(REPOSITORY_CHECK_FAILED_CODE)
        .unwrap_or("The repository could not be checked.")
        .to_string()
}

/// The path a skill is tracked at is gone and no unique new home was found.
/// Only the reviewed sentence and the old repo-relative path are exposed.
fn source_missing_failure(repository_id: &str, source_path: Option<String>) -> FailedRepository {
    FailedRepository {
        repository_id: repository_id.to_string(),
        error: crate::ipc_error::public_message_for_code(SKILL_SOURCE_MISSING_CODE)
            .unwrap_or("The tracked source path no longer contains a skill.")
            .to_string(),
        error_code: Some(SKILL_SOURCE_MISSING_CODE.to_string()),
        diagnostic_category: None,
        retry: FailedRepositoryRetry::DecisionRequired,
        diagnostics: source_path.map(|source_path| super::SkillUpdateDiagnostic {
            source_url: None,
            ref_name: None,
            source_path: Some(source_path),
            local_hash: None,
            baseline_hash: None,
            remote_hash: None,
            local_version: None,
            remote_version: None,
            cache_policy: super::SkillRefreshCachePolicy::Bypass,
            cache_hit: false,
            snapshot_fetched_at: None,
        }),
    }
}

pub(super) async fn reconcile_relocated_remote_skills(
    ctx: &mut RelocationContext<'_>,
) -> Result<(), CentralUpdatesError> {
    let mut missing_by_key = HashMap::<(String, String), Vec<usize>>::new();
    let mut missing_old_path_by_index = HashMap::<usize, String>::new();
    for (index, owned) in ctx.remote_missing_states.iter().enumerate() {
        let state = &owned.state;
        let Some(old_path) = state
            .source_path
            .as_deref()
            .map(normalize_repo_path)
            .transpose()?
            .filter(|path| !path.is_empty())
        else {
            continue;
        };
        missing_old_path_by_index.insert(index, old_path);
        missing_by_key
            .entry((owned.repository_id.clone(), state.skill_id.clone()))
            .or_default()
            .push(index);
    }

    let mut added_by_repository = HashMap::<String, Vec<usize>>::new();
    let mut added_new_path_by_index = HashMap::<usize, String>::new();
    for (index, item) in ctx.remote_added_items.iter().enumerate() {
        let new_path = normalize_repo_path(&item.preview.source_path)?;
        added_new_path_by_index.insert(index, new_path);
        added_by_repository
            .entry(item.repository_id.clone())
            .or_default()
            .push(index);
    }

    let mut resolved_missing = HashSet::<usize>::new();
    let mut resolved_added = HashSet::<usize>::new();
    for ((repository_id, skill_id), missing_indexes) in &missing_by_key {
        if missing_indexes.len() != 1 {
            continue;
        }
        let Some(added_indexes) = added_by_repository.get(repository_id) else {
            continue;
        };
        let missing_index = missing_indexes[0];
        let old_path = &missing_old_path_by_index[&missing_index];

        let candidates = added_indexes
            .iter()
            .map(|index| RelocationCandidateRef {
                skill_id: ctx.remote_added_items[*index].preview.skill_id.as_str(),
                source_path: added_new_path_by_index[index].as_str(),
            })
            .collect::<Vec<_>>();
        let Some(new_path) = unique_relocation_target(skill_id, old_path, &candidates) else {
            continue;
        };
        let Some(added_index) = added_indexes
            .iter()
            .copied()
            .find(|index| added_new_path_by_index[index] == new_path)
        else {
            continue;
        };

        let state = &ctx.remote_missing_states[missing_index].state;
        let item = &ctx.remote_added_items[added_index];
        if item
            .preview
            .conflict
            .as_ref()
            .is_some_and(|conflict| conflict.existing_skill_id != state.skill_id)
        {
            continue;
        }

        let Some(prepared) = ctx.prepared_by_skill_id.get(&state.skill_id) else {
            continue;
        };
        let Some(repo) = ctx.repo_ref_by_id.get(&item.repository_id) else {
            continue;
        };

        let relocated_state = match apply_relocation(
            ctx.pool,
            prepared,
            repo,
            &item.repository_id,
            &new_path,
            ctx.snapshots,
        )
        .await
        {
            Ok(state) => state,
            Err(RelocationError::Db(error)) => return Err(error),
            Err(RelocationError::Load(error)) => {
                ctx.failed_repositories.push(FailedRepository {
                    repository_id: item.repository_id.clone(),
                    error: format!(
                        "Failed to auto-resolve moved skill '{}': {}",
                        state.skill_id,
                        remote_load_error_message(error)
                    ),
                    error_code: Some(RELOCATION_FAILED_CODE.to_string()),
                    diagnostic_category: None,
                    retry: FailedRepositoryRetry::Retryable,
                    diagnostics: None,
                });
                continue;
            }
        };

        if relocated_state.status == SkillUpdateStatus::UpdateAvailable {
            ctx.updatable.push(UpdatableSkill {
                state: relocated_state,
                repository_id: Some(item.repository_id.clone()),
                diagnostics: None,
            });
        }
        resolved_missing.insert(missing_index);
        resolved_added.insert(added_index);
    }

    if !resolved_missing.is_empty() {
        let mut index = 0;
        ctx.remote_missing_states.retain(|_| {
            let keep = !resolved_missing.contains(&index);
            index += 1;
            keep
        });
    }
    if !resolved_added.is_empty() {
        let mut index = 0;
        ctx.remote_added_items.retain(|_| {
            let keep = !resolved_added.contains(&index);
            index += 1;
            keep
        });
    }
    Ok(())
}

pub(super) fn remote_load_error_message(error: RemoteSkillLoadError) -> String {
    match error {
        RemoteSkillLoadError::RemoteMissing(message) | RemoteSkillLoadError::Other(message) => {
            message
        }
    }
}

async fn persist_relocated_skill(
    pool: &DbPool,
    repository_id: &str,
    skill_id: &str,
    source_path: &str,
) -> Result<(), CentralUpdatesError> {
    let now = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO skill_repository_members
         (skill_id, repository_id, source_path, added_at, updated_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(skill_id) DO UPDATE SET
           repository_id = excluded.repository_id,
           source_path = COALESCE(excluded.source_path, skill_repository_members.source_path),
           updated_at = excluded.updated_at",
    )
    .bind(skill_id)
    .bind(repository_id)
    .bind(source_path)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "DELETE FROM skill_repository_pending_additions
         WHERE repository_id = ? AND source_path = ?",
    )
    .bind(repository_id)
    .bind(source_path)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
