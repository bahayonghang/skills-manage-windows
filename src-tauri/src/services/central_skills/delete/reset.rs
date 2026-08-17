//! Target-scoped reset of Central skills that have no repository membership.
//!
//! Candidate ids are recomputed from the frozen target DB under the mutation
//! lock. Delete reuses the journaled batch-delete path without acquiring the
//! lock again. Inventory clear matches
//! `clear_skill_update_inventory_impl(pool, None)` on that pool only.

use std::collections::HashSet;
use std::sync::Arc;

use crate::db::{self, DbPool};
use crate::services::central_mutation::{
    acquire_target_mutation_guard, DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};
use crate::targets::{connect_remote_target, ActiveTarget, ConnectedRemoteTarget};

use super::super::error::CentralSkillsError;
use super::super::types::{
    BatchDeleteCentralSkillRequest, BatchDeleteCentralSkillResult, ResetUnknownSourceSkillsPreview,
};
use super::batch::{delete_central_skills_under_guard, public_delete_batch_result};
use super::{preview_delete_central_skills_impl, preview_delete_central_skills_ssh_impl};

pub async fn list_unknown_source_central_skill_ids(
    pool: &DbPool,
) -> Result<Vec<String>, CentralSkillsError> {
    let ids = sqlx::query_scalar::<_, String>(
        "SELECT s.id
         FROM skills s
         WHERE s.is_central = 1
           AND NOT EXISTS (
             SELECT 1 FROM skill_repository_members m WHERE m.skill_id = s.id
           )
         ORDER BY s.id",
    )
    .fetch_all(pool)
    .await?;
    Ok(ids)
}

pub async fn preview_reset_unknown_source_skills_impl(
    pool: &DbPool,
    active_target: &ActiveTarget,
) -> Result<ResetUnknownSourceSkillsPreview, CentralSkillsError> {
    let skill_ids = list_unknown_source_central_skill_ids(pool).await?;
    let preview = match active_target {
        ActiveTarget::Local => preview_delete_central_skills_impl(pool, &skill_ids).await?,
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            preview_delete_central_skills_ssh_impl(pool, active_target, &skill_ids).await?
        }
    };
    Ok(ResetUnknownSourceSkillsPreview { skill_ids, preview })
}

pub async fn reset_unknown_source_skills_impl(
    pool: &DbPool,
    active_target: &ActiveTarget,
    confirmed_skill_ids: &[String],
    remove_copy_agent_ids: &[String],
) -> Result<BatchDeleteCentralSkillResult, CentralSkillsError> {
    reset_unknown_source_skills_for_target(
        pool,
        active_target,
        None,
        confirmed_skill_ids,
        remove_copy_agent_ids,
    )
    .await
}

#[cfg(test)]
pub(crate) async fn reset_unknown_source_skills_for_target_with_connection_for_tests(
    pool: &DbPool,
    active_target: &ActiveTarget,
    connection: Arc<ConnectedRemoteTarget>,
    confirmed_skill_ids: &[String],
    remove_copy_agent_ids: &[String],
) -> Result<BatchDeleteCentralSkillResult, CentralSkillsError> {
    reset_unknown_source_skills_for_target(
        pool,
        active_target,
        Some(connection),
        confirmed_skill_ids,
        remove_copy_agent_ids,
    )
    .await
}

async fn reset_unknown_source_skills_for_target(
    pool: &DbPool,
    active_target: &ActiveTarget,
    injected_remote: Option<Arc<ConnectedRemoteTarget>>,
    confirmed_skill_ids: &[String],
    remove_copy_agent_ids: &[String],
) -> Result<BatchDeleteCentralSkillResult, CentralSkillsError> {
    if let Some(connection) = injected_remote.as_ref() {
        if connection.target_id() != active_target.id()
            || connection.active_target().kind() != active_target.kind()
        {
            return Err(CentralSkillsError::Remote(
                "Selected target connection identity mismatch.".to_string(),
            ));
        }
    }

    let _mutation_guard = acquire_target_mutation_guard(
        active_target,
        "reset unknown-source Central skills",
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await?;

    let listed = list_unknown_source_central_skill_ids(pool).await?;
    let deletable_ids =
        confirmed_unknown_source_preview_ids(pool, active_target, &listed, confirmed_skill_ids)
            .await?;
    let result = if deletable_ids.is_empty() {
        BatchDeleteCentralSkillResult {
            succeeded: Vec::new(),
            failed: Vec::new(),
        }
    } else {
        let requests = reset_delete_requests(pool, &deletable_ids, remove_copy_agent_ids).await?;
        let remote = if let Some(connection) = injected_remote {
            Some(connection)
        } else if active_target.is_remote_like() {
            Some(Arc::new(
                connect_remote_target(active_target).await.map_err(|_| {
                    CentralSkillsError::Remote(
                        "Failed to connect to the selected target.".to_string(),
                    )
                })?,
            ))
        } else {
            None
        };
        let batch_id = uuid::Uuid::new_v4().to_string();
        let outcomes = delete_central_skills_under_guard(
            pool,
            active_target,
            requests,
            Some(&batch_id),
            remote.as_ref(),
        )
        .await?;
        public_delete_batch_result(outcomes)
    };
    maybe_clear_target_inventory(pool, &result, listed.is_empty()).await?;
    Ok(result)
}

async fn confirmed_unknown_source_preview_ids(
    pool: &DbPool,
    active_target: &ActiveTarget,
    listed: &[String],
    confirmed_skill_ids: &[String],
) -> Result<Vec<String>, CentralSkillsError> {
    if confirmed_skill_ids.is_empty() {
        return Ok(Vec::new());
    }
    let confirmed: HashSet<&str> = confirmed_skill_ids.iter().map(String::as_str).collect();
    let candidates: Vec<String> = listed
        .iter()
        .filter(|id| confirmed.contains(id.as_str()))
        .cloned()
        .collect();
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    let preview = match active_target {
        ActiveTarget::Local => preview_delete_central_skills_impl(pool, &candidates).await?,
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            preview_delete_central_skills_ssh_impl(pool, active_target, &candidates).await?
        }
    };
    Ok(preview
        .previews
        .into_iter()
        .map(|item| item.skill_id)
        .collect())
}

async fn reset_delete_requests(
    pool: &DbPool,
    skill_ids: &[String],
    remove_copy_agent_ids: &[String],
) -> Result<Vec<BatchDeleteCentralSkillRequest>, CentralSkillsError> {
    let selected: HashSet<&str> = remove_copy_agent_ids.iter().map(String::as_str).collect();
    let installations = db::get_skill_installations_for_skills(pool, skill_ids).await?;
    Ok(skill_ids
        .iter()
        .map(|skill_id| {
            let mut remove_agent_ids = Vec::new();
            let mut seen = HashSet::new();
            if let Some(items) = installations.get(skill_id) {
                for installation in items {
                    if installation.link_type == "copy"
                        && selected.contains(installation.agent_id.as_str())
                        && seen.insert(installation.agent_id.as_str())
                    {
                        remove_agent_ids.push(installation.agent_id.clone());
                    }
                }
            }
            BatchDeleteCentralSkillRequest {
                skill_id: skill_id.clone(),
                remove_agent_ids,
                force: false,
            }
        })
        .collect())
}

async fn maybe_clear_target_inventory(
    pool: &DbPool,
    result: &BatchDeleteCentralSkillResult,
    listed_unknown_source_was_empty: bool,
) -> Result<(), CentralSkillsError> {
    // Same as clear_skill_update_inventory_impl(pool, None) on this pool only.
    // Clear after any successful delete so stale Unsupported rows cannot
    // resurrect deleted skill ids. Empty candidate sets still clear leftover
    // inventory. All-fail and "confirmed ids were not deletable" keep inventory.
    if !result.succeeded.is_empty() || (result.failed.is_empty() && listed_unknown_source_was_empty)
    {
        db::clear_all_skill_update_inventory(pool).await?;
        db::clear_pending_additions(pool).await?;
    }
    Ok(())
}
