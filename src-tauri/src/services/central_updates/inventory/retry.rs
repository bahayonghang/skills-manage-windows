//! Retry a subset of repositories without discarding the rest of the panel.
//!
//! A refresh persists and returns one inventory per scope, so re-running a
//! narrow scope would replace what the user is currently looking at. Retry
//! instead computes only the requested repositories and merges the result into
//! the inventory stored for the panel's own scope.

use std::collections::HashSet;

use crate::db::{self, DbPool};

use super::super::error::CentralUpdatesError;
use super::super::fs::CentralFs;
use super::super::snapshots::{CentralUpdateSnapshotCache, SnapshotProgressReporter};
use super::{
    compute_skill_update_inventory, get_skill_update_inventory_impl_scoped, normalize_ids,
    persist_refresh_inventory, SkillRefreshCachePolicy, SkillRefreshMode, SkillRefreshScope,
    SkillRefreshScopeKind, SkillUpdateInventory,
};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn retry_failed_repositories_impl(
    pool: &DbPool,
    fs: &CentralFs,
    auth_token: Option<&str>,
    client: &reqwest::Client,
    snapshots_cache: &CentralUpdateSnapshotCache,
    base_scope: SkillRefreshScope,
    repository_ids: Vec<String>,
    mode_override: Option<SkillRefreshMode>,
    progress: Option<SnapshotProgressReporter>,
) -> Result<SkillUpdateInventory, CentralUpdatesError> {
    let repository_ids = normalize_ids(repository_ids);
    let base = get_skill_update_inventory_impl_scoped(pool, Some(base_scope.clone())).await?;
    if repository_ids.is_empty() {
        return Ok(base);
    }

    let mut legacy_member_skill_ids = HashSet::new();
    for repository_id in &repository_ids {
        legacy_member_skill_ids
            .extend(db::get_central_skill_ids_by_repository(pool, repository_id).await?);
    }
    let targets = RepositoryRetryTargets {
        repository_ids: repository_ids.iter().cloned().collect(),
        legacy_member_skill_ids,
    };

    let base_mode = base_scope.mode.unwrap_or(SkillRefreshMode::Sync);
    let base_cache_policy = base_scope
        .cache_policy
        .unwrap_or(SkillRefreshCachePolicy::Bypass);

    // The slice inherits the panel's cache policy so a retry sees exactly what
    // another refresh of that panel would have seen.
    let slice_scope = SkillRefreshScope {
        kind: SkillRefreshScopeKind::Repositories,
        mode: Some(mode_override.unwrap_or(base_mode)),
        cache_policy: Some(base_cache_policy),
        skill_ids: None,
        repository_ids: Some(repository_ids.clone()),
        agent_ids: None,
    };
    let slice = compute_skill_update_inventory(
        pool,
        fs,
        auth_token,
        client,
        snapshots_cache,
        &slice_scope,
        progress,
    )
    .await?;

    let merged = merge_inventory_for_repositories(base, slice, &targets);
    // The stored run keeps the panel's own scope and mode: a mode override only
    // changes what this slice looked for, not which inventory it belongs to.
    persist_refresh_inventory(pool, &base_scope, base_mode, base_cache_policy, &merged).await?;
    Ok(merged)
}

/// Replace only what belongs to `repository_ids`.
///
/// Buckets that no repository owns (`unsupported`) and buckets that only a full
/// scan produces (platform duplicates, deleted platform copies, orphans) are
/// carried over from the baseline, because a per-repository slice cannot
/// re-derive them.
struct RepositoryRetryTargets {
    repository_ids: HashSet<String>,
    legacy_member_skill_ids: HashSet<String>,
}

impl RepositoryRetryTargets {
    /// New inventories carry an explicit repository id. The skill-id fallback
    /// is deliberately restricted to legacy null rows persisted by affected
    /// releases, so an explicit assignment to another repository always wins.
    fn owns_actionable(&self, repository_id: Option<&str>, skill_id: &str) -> bool {
        match repository_id {
            Some(repository_id) => self.repository_ids.contains(repository_id),
            None => self.legacy_member_skill_ids.contains(skill_id),
        }
    }
}

fn merge_inventory_for_repositories(
    base: SkillUpdateInventory,
    slice: SkillUpdateInventory,
    targets: &RepositoryRetryTargets,
) -> SkillUpdateInventory {
    let snapshot_retry_attempted = slice.snapshot_retry_attempted;
    let snapshot_retry_recovered = slice.snapshot_retry_recovered;
    let mut updatable = base
        .updatable
        .into_iter()
        .filter(|item| {
            !targets.owns_actionable(item.repository_id.as_deref(), &item.state.skill_id)
        })
        .collect::<Vec<_>>();
    updatable.extend(slice.updatable);

    let mut remote_missing = base
        .remote_missing
        .into_iter()
        .filter(|item| {
            !targets.owns_actionable(item.repository_id.as_deref(), &item.state.skill_id)
        })
        .collect::<Vec<_>>();
    remote_missing.extend(slice.remote_missing);

    let mut remote_added = base
        .remote_added
        .into_iter()
        .filter(|item| !targets.repository_ids.contains(&item.repository_id))
        .collect::<Vec<_>>();
    remote_added.extend(slice.remote_added);

    let mut failed_repositories = base
        .failed_repositories
        .into_iter()
        .filter(|item| !targets.repository_ids.contains(&item.repository_id))
        .collect::<Vec<_>>();
    failed_repositories.extend(slice.failed_repositories);

    SkillUpdateInventory {
        updatable,
        remote_added,
        remote_missing,
        unsupported: base.unsupported,
        platform_duplicates: base.platform_duplicates,
        deleted_platform_copies: base.deleted_platform_copies,
        orphans: base.orphans,
        failed_repositories,
        snapshot_retry_attempted,
        snapshot_retry_recovered,
        generated_at: chrono::Utc::now().to_rfc3339(),
    }
}
