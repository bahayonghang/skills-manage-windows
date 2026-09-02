use std::collections::HashSet;

use crate::db::repos::update_inventory_repo;
use crate::db::{self, DbPool, SkillUpdateInventoryEntry};
use crate::services::central_updates::CentralUpdatesError;

use super::{
    normalize_ids, FailedRepository, RemoteMissingSkill, SkillRefreshCachePolicy, SkillRefreshMode,
    SkillRefreshScope, SkillRefreshScopeKind, SkillUpdateApplyResult, SkillUpdateDiagnostic,
    SkillUpdateInventory, UnsupportedSkill, UpdatableSkill,
};

pub(super) fn inventory_id_for_scope(scope: Option<&SkillRefreshScope>) -> String {
    let Some(scope) = scope else {
        return "all:sync".to_string();
    };
    let kind = match scope.kind {
        SkillRefreshScopeKind::All => "all",
        SkillRefreshScopeKind::Skills => "skills",
        SkillRefreshScopeKind::Repositories => "repositories",
        SkillRefreshScopeKind::Platform => "platform",
    };
    let mode = match scope.mode.unwrap_or(SkillRefreshMode::Sync) {
        SkillRefreshMode::Regular => "regular",
        SkillRefreshMode::Sync => "sync",
    };
    match scope.kind {
        SkillRefreshScopeKind::All => format!("{kind}:{mode}"),
        SkillRefreshScopeKind::Skills => format!(
            "{kind}:{mode}:{}",
            normalize_ids(scope.skill_ids.clone().unwrap_or_default()).join(",")
        ),
        SkillRefreshScopeKind::Repositories => format!(
            "{kind}:{mode}:{}",
            normalize_ids(scope.repository_ids.clone().unwrap_or_default()).join(",")
        ),
        SkillRefreshScopeKind::Platform => format!(
            "{kind}:{mode}:{}",
            normalize_ids(scope.agent_ids.clone().unwrap_or_default()).join(",")
        ),
    }
}

pub(super) async fn persist_refresh_inventory(
    pool: &DbPool,
    scope: &SkillRefreshScope,
    mode: SkillRefreshMode,
    cache_policy: SkillRefreshCachePolicy,
    inventory: &SkillUpdateInventory,
) -> Result<(), CentralUpdatesError> {
    let generated_at = inventory.generated_at.clone();
    let run = db::SkillUpdateInventoryRun {
        inventory_id: inventory_id_for_scope(Some(scope)),
        scope_kind: scope_kind_name(scope.kind).to_string(),
        mode: refresh_mode_name(mode).to_string(),
        skill_ids_json: ids_json(scope.skill_ids.clone())?,
        repository_ids_json: ids_json(scope.repository_ids.clone())?,
        agent_ids_json: ids_json(scope.agent_ids.clone())?,
        cache_policy: cache_policy.as_str().to_string(),
        generated_at: generated_at.clone(),
    };

    let mut entries = Vec::new();
    for item in &inventory.updatable {
        entries.push(entry_from_state_payload(
            &run.inventory_id,
            "updatable",
            &item.state.skill_id,
            item.repository_id.as_deref(),
            cache_policy,
            &generated_at,
            item,
            item.state.error.as_deref(),
            &item.state,
        )?);
    }
    for item in &inventory.remote_missing {
        entries.push(entry_from_state_payload(
            &run.inventory_id,
            "remote_missing",
            &item.state.skill_id,
            item.repository_id.as_deref(),
            cache_policy,
            &generated_at,
            item,
            item.state.error.as_deref(),
            &item.state,
        )?);
    }
    for item in &inventory.unsupported {
        entries.push(entry_from_payload(
            &run.inventory_id,
            "unsupported",
            &item.skill_id,
            Some(&item.skill_id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            cache_policy,
            false,
            &generated_at,
            item,
            None,
        )?);
    }
    for item in &inventory.failed_repositories {
        entries.push(entry_from_payload(
            &run.inventory_id,
            "failed_repository",
            &item.repository_id,
            None,
            Some(&item.repository_id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            cache_policy,
            false,
            &generated_at,
            item,
            Some(&item.error),
        )?);
    }

    validate_inventory_entry_keys(&entries)?;

    update_inventory_repo::replace_skill_update_inventory(pool, &run, &entries).await?;
    Ok(())
}

fn validate_inventory_entry_keys(
    entries: &[SkillUpdateInventoryEntry],
) -> Result<(), CentralUpdatesError> {
    let mut keys = HashSet::new();
    for entry in entries {
        if !keys.insert((
            entry.inventory_id.as_str(),
            entry.bucket.as_str(),
            entry.entity_key.as_str(),
        )) {
            return Err(CentralUpdatesError::InventoryInvariant);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn entry_from_state_payload<T: serde::Serialize>(
    inventory_id: &str,
    bucket: &str,
    entity_key: &str,
    repository_id: Option<&str>,
    cache_policy: SkillRefreshCachePolicy,
    generated_at: &str,
    payload: &T,
    error: Option<&str>,
    state: &db::SkillUpdateState,
) -> Result<SkillUpdateInventoryEntry, CentralUpdatesError> {
    entry_from_payload(
        inventory_id,
        bucket,
        entity_key,
        Some(&state.skill_id),
        repository_id,
        state.source_path.as_deref(),
        state.source_type.as_str().into(),
        state.source_url.as_deref(),
        state.ref_name.as_deref(),
        state.last_remote_hash.as_deref(),
        state.last_remote_hash.as_deref(),
        state.latest_remote_hash.as_deref(),
        cache_policy,
        false,
        generated_at,
        payload,
        error,
    )
}

#[allow(clippy::too_many_arguments)]
fn entry_from_payload<T: serde::Serialize>(
    inventory_id: &str,
    bucket: &str,
    entity_key: &str,
    skill_id: Option<&str>,
    repository_id: Option<&str>,
    source_path: Option<&str>,
    source_type: Option<&str>,
    source_url: Option<&str>,
    ref_name: Option<&str>,
    local_hash: Option<&str>,
    baseline_hash: Option<&str>,
    remote_hash: Option<&str>,
    cache_policy: SkillRefreshCachePolicy,
    cache_hit: bool,
    generated_at: &str,
    payload: &T,
    error: Option<&str>,
) -> Result<SkillUpdateInventoryEntry, CentralUpdatesError> {
    Ok(SkillUpdateInventoryEntry {
        inventory_id: inventory_id.to_string(),
        bucket: bucket.to_string(),
        entity_key: entity_key.to_string(),
        skill_id: skill_id.map(str::to_string),
        skill_name: None,
        repository_id: repository_id.map(str::to_string),
        source_type: source_type.map(str::to_string),
        source_url: source_url.map(str::to_string),
        ref_name: ref_name.map(str::to_string),
        source_path: source_path.map(str::to_string),
        agent_id: None,
        local_hash: local_hash.map(str::to_string),
        baseline_hash: baseline_hash.map(str::to_string),
        remote_hash: remote_hash.map(str::to_string),
        local_version: None,
        remote_version: None,
        cache_policy: cache_policy.as_str().to_string(),
        cache_hit,
        snapshot_fetched_at: None,
        generated_at: generated_at.to_string(),
        payload_json: serde_json::to_string(payload)
            .map_err(|e| CentralUpdatesError::Json(e.to_string()))?,
        error: error.map(str::to_string),
    })
}

type EntryInventory = (
    Vec<UpdatableSkill>,
    Vec<RemoteMissingSkill>,
    Vec<UnsupportedSkill>,
    Vec<FailedRepository>,
    Option<String>,
);

pub(super) fn inventory_from_entries(
    entries: Vec<SkillUpdateInventoryEntry>,
) -> Result<EntryInventory, CentralUpdatesError> {
    let generated_at = entries.first().map(|entry| entry.generated_at.clone());
    let mut updatable = Vec::new();
    let mut remote_missing = Vec::new();
    let mut unsupported = Vec::new();
    let mut failed_repositories = Vec::new();
    for entry in entries {
        match entry.bucket.as_str() {
            "updatable" => updatable.push(
                serde_json::from_str::<UpdatableSkill>(&entry.payload_json)
                    .map_err(|e| CentralUpdatesError::Json(e.to_string()))?,
            ),
            "remote_missing" => remote_missing.push(
                serde_json::from_str::<RemoteMissingSkill>(&entry.payload_json)
                    .map_err(|e| CentralUpdatesError::Json(e.to_string()))?,
            ),
            "unsupported" => unsupported.push(
                serde_json::from_str::<UnsupportedSkill>(&entry.payload_json)
                    .map_err(|e| CentralUpdatesError::Json(e.to_string()))?,
            ),
            "failed_repository" => failed_repositories.push(
                serde_json::from_str::<FailedRepository>(&entry.payload_json)
                    .map_err(|e| CentralUpdatesError::Json(e.to_string()))?,
            ),
            _ => {}
        }
    }
    Ok((
        updatable,
        remote_missing,
        unsupported,
        failed_repositories,
        generated_at,
    ))
}

pub(super) fn diagnostic_from_state(
    state: &db::SkillUpdateState,
    cache_policy: SkillRefreshCachePolicy,
    cache_hit: bool,
) -> SkillUpdateDiagnostic {
    SkillUpdateDiagnostic {
        source_url: state.source_url.clone(),
        ref_name: state.ref_name.clone(),
        source_path: state.source_path.clone(),
        local_hash: state.last_remote_hash.clone(),
        baseline_hash: state.last_remote_hash.clone(),
        remote_hash: state.latest_remote_hash.clone(),
        local_version: None,
        remote_version: None,
        cache_policy,
        cache_hit,
        snapshot_fetched_at: None,
    }
}

fn ids_json(ids: Option<Vec<String>>) -> Result<Option<String>, CentralUpdatesError> {
    let Some(ids) = ids else {
        return Ok(None);
    };
    serde_json::to_string(&normalize_ids(ids))
        .map(Some)
        .map_err(|e| CentralUpdatesError::Json(e.to_string()))
}

fn scope_kind_name(kind: SkillRefreshScopeKind) -> &'static str {
    match kind {
        SkillRefreshScopeKind::All => "all",
        SkillRefreshScopeKind::Skills => "skills",
        SkillRefreshScopeKind::Repositories => "repositories",
        SkillRefreshScopeKind::Platform => "platform",
    }
}

fn refresh_mode_name(mode: SkillRefreshMode) -> &'static str {
    match mode {
        SkillRefreshMode::Regular => "regular",
        SkillRefreshMode::Sync => "sync",
    }
}

pub(super) async fn prune_applied_inventory_entries(
    pool: &DbPool,
    result: &SkillUpdateApplyResult,
) {
    let mut ids = Vec::new();
    ids.extend(result.updated_skill_ids.clone());
    ids.extend(result.kept_missing_skill_ids.clone());
    ids.extend(result.deleted_skill_ids.clone());
    ids.extend(result.imported_skill_ids.clone());
    if !ids.is_empty() {
        let _ = update_inventory_repo::delete_skill_update_inventory_entries_for_skills(
            pool,
            &normalize_ids(ids),
        )
        .await;
    }
}
