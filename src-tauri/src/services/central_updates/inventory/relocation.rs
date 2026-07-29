use std::collections::{HashMap, HashSet};

use crate::db::{DbPool, SkillRepository, SkillUpdateState};
use crate::services::central_updates::{
    normalize_repo_path, state_from_relocated_source, CentralRemoteAddedSkill, CentralUpdatesError,
    PreparedSkillUpdate, RemoteSkillLoadError, SkillUpdateStatus,
};
use crate::services::github_import;
use chrono::Utc;

use super::{repository_id_for_state, FailedRepository, UpdatableSkill};

pub(crate) struct RelocationContext<'a> {
    pub pool: &'a DbPool,
    pub prepared_by_skill_id: &'a HashMap<String, PreparedSkillUpdate>,
    pub snapshots: &'a HashMap<String, github_import::GitHubRepoSnapshot>,
    pub repo_by_id: &'a HashMap<String, SkillRepository>,
    pub repo_ref_by_id: &'a HashMap<String, github_import::GitHubRepoRef>,
    pub remote_missing_states: &'a mut Vec<SkillUpdateState>,
    pub remote_added_items: &'a mut Vec<CentralRemoteAddedSkill>,
    pub updatable: &'a mut Vec<UpdatableSkill>,
    pub failed_repositories: &'a mut Vec<FailedRepository>,
}

pub(crate) async fn reconcile_relocated_remote_skills(
    ctx: &mut RelocationContext<'_>,
) -> Result<(), CentralUpdatesError> {
    let mut missing_by_key = HashMap::<(String, String), Vec<usize>>::new();
    let mut missing_old_path_by_index = HashMap::<usize, String>::new();
    for (index, state) in ctx.remote_missing_states.iter().enumerate() {
        let Some(repository_id) = repository_id_for_state(ctx.repo_by_id, state) else {
            continue;
        };
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
            .entry((repository_id, state.skill_id.clone()))
            .or_default()
            .push(index);
    }

    let mut added_by_key = HashMap::<(String, String), Vec<usize>>::new();
    let mut added_new_path_by_index = HashMap::<usize, String>::new();
    for (index, item) in ctx.remote_added_items.iter().enumerate() {
        let new_path = normalize_repo_path(&item.preview.source_path)?;
        added_new_path_by_index.insert(index, new_path);
        added_by_key
            .entry((item.repository_id.clone(), item.preview.skill_id.clone()))
            .or_default()
            .push(index);
    }

    let mut resolved_missing = HashSet::<usize>::new();
    let mut resolved_added = HashSet::<usize>::new();
    for (key, missing_indexes) in &missing_by_key {
        let Some(added_indexes) = added_by_key.get(key) else {
            continue;
        };
        if missing_indexes.len() != 1 || added_indexes.len() != 1 {
            continue;
        }
        let missing_index = missing_indexes[0];
        let added_index = added_indexes[0];
        let old_path = &missing_old_path_by_index[&missing_index];
        let new_path = &added_new_path_by_index[&added_index];
        if old_path == new_path {
            continue;
        }

        let state = &ctx.remote_missing_states[missing_index];
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

        let relocated_state =
            match state_from_relocated_source(prepared, repo, new_path, ctx.snapshots) {
                Ok(state) => state,
                Err(error) => {
                    ctx.failed_repositories.push(FailedRepository {
                        repository_id: item.repository_id.clone(),
                        error: format!(
                            "Failed to auto-resolve moved skill '{}': {}",
                            state.skill_id,
                            remote_load_error_message(error)
                        ),
                        diagnostics: None,
                    });
                    continue;
                }
            };

        persist_relocated_skill(
            ctx.pool,
            &item.repository_id,
            &state.skill_id,
            new_path,
            &relocated_state,
        )
        .await?;

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

fn remote_load_error_message(error: RemoteSkillLoadError) -> String {
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
    _state: &SkillUpdateState,
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
