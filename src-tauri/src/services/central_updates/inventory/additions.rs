//! Immutable repository authority for Update Center remote additions.

use std::collections::HashMap;
use std::sync::Arc;

use tauri::AppHandle;

use crate::db::{self, DbPool};
use crate::services::github_import;
use crate::targets::ActiveTarget;

use super::super::error::CentralUpdatesError;
use super::super::fs::normalize_repo_path;
use super::super::snapshots::{
    repo_cache_key, CentralUpdateRepositorySnapshot, CentralUpdateSnapshotCache,
};
use super::super::CentralRepositoryAddedSkillSelection;

pub(super) async fn load_repository_for_import_addition(
    pool: &DbPool,
    repository_id: &str,
) -> Result<Option<db::SkillRepository>, CentralUpdatesError> {
    match db::get_skill_repository_by_id(pool, repository_id).await? {
        Some(repository) => Ok(Some(repository)),
        None => {
            db::clear_pending_additions_for_repos(pool, &[repository_id.to_string()]).await?;
            Ok(None)
        }
    }
}

pub(super) fn group_repository_import_additions(
    additions: Vec<CentralRepositoryAddedSkillSelection>,
) -> Vec<CentralRepositoryAddedSkillSelection> {
    let mut grouped = Vec::<CentralRepositoryAddedSkillSelection>::new();
    let mut index_by_repository = HashMap::<String, usize>::new();
    for addition in additions {
        if let Some(index) = index_by_repository.get(&addition.repository_id).copied() {
            grouped[index].selections.extend(addition.selections);
            continue;
        }
        index_by_repository.insert(addition.repository_id.clone(), grouped.len());
        grouped.push(addition);
    }
    grouped
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PendingAdditionSnapshotIdentity {
    pub(super) resolved_commit_sha: String,
    pub(super) snapshot_digest: String,
}

pub(super) async fn load_pending_addition_snapshot_identity(
    pool: &DbPool,
    repository_id: &str,
    selections: &[github_import::GitHubSkillImportSelection],
) -> Result<PendingAdditionSnapshotIdentity, CentralUpdatesError> {
    let repository_ids = [repository_id.to_string()];
    let pending = db::list_pending_additions_for_repos(pool, &repository_ids).await?;
    let pending_by_source_path = pending
        .into_iter()
        .map(|item| (item.source_path.clone(), item))
        .collect::<HashMap<_, _>>();
    let mut identity: Option<PendingAdditionSnapshotIdentity> = None;

    for selection in selections {
        let source_path = normalize_repo_path(&selection.source_path)?;
        let item = pending_by_source_path
            .get(&source_path)
            .ok_or(CentralUpdatesError::InventoryRefreshRequired)?;
        let resolved_commit_sha = item
            .resolved_commit_sha
            .as_deref()
            .filter(|value| github_import::validate_commit_sha(value).is_ok())
            .ok_or(CentralUpdatesError::InventoryRefreshRequired)?;
        let snapshot_digest = item
            .snapshot_digest
            .as_deref()
            .filter(|value| is_snapshot_digest(value))
            .ok_or(CentralUpdatesError::InventoryRefreshRequired)?;
        let item_identity = PendingAdditionSnapshotIdentity {
            resolved_commit_sha: resolved_commit_sha.to_string(),
            snapshot_digest: snapshot_digest.to_string(),
        };
        if identity
            .as_ref()
            .is_some_and(|current| current != &item_identity)
        {
            return Err(CentralUpdatesError::InventoryRefreshRequired);
        }
        identity = Some(item_identity);
    }

    identity.ok_or(CentralUpdatesError::InventoryRefreshRequired)
}

fn is_snapshot_digest(value: &str) -> bool {
    value.strip_prefix("sha256-v1:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn apply_pinned_repository_additions(
    app: Option<&AppHandle>,
    pool: &DbPool,
    active_target: &ActiveTarget,
    auth_token: Option<&str>,
    client: &reqwest::Client,
    snapshots_cache: &CentralUpdateSnapshotCache,
    repo: &github_import::GitHubRepoRef,
    identity: &PendingAdditionSnapshotIdentity,
    selections: Vec<github_import::GitHubSkillImportSelection>,
) -> Result<github_import::GitHubRepoImportResult, CentralUpdatesError> {
    match active_target {
        ActiveTarget::Local => {
            let snapshot = load_verified_local_addition_snapshot(
                client,
                auth_token,
                snapshots_cache,
                repo,
                identity,
            )
            .await?;
            github_import::import_github_repo_skills_from_pinned_snapshot(
                pool,
                repo,
                &identity.resolved_commit_sha,
                &snapshot,
                selections,
                app,
            )
            .await
            .map_err(CentralUpdatesError::from)
        }
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            github_import::import_github_repo_skills_remote_from_pinned_snapshot(
                pool,
                active_target,
                repo,
                &identity.resolved_commit_sha,
                &identity.snapshot_digest,
                selections,
                app,
                auth_token,
            )
            .await
            .map_err(|error| match error {
                github_import::GithubImportError::PreviewSnapshotIntegrity => {
                    CentralUpdatesError::SnapshotChanged
                }
                error => CentralUpdatesError::GithubImport(error),
            })
        }
    }
}

async fn load_verified_local_addition_snapshot(
    client: &reqwest::Client,
    auth_token: Option<&str>,
    snapshots_cache: &CentralUpdateSnapshotCache,
    repo: &github_import::GitHubRepoRef,
    identity: &PendingAdditionSnapshotIdentity,
) -> Result<Arc<CentralUpdateRepositorySnapshot>, CentralUpdatesError> {
    load_verified_local_addition_snapshot_with(
        snapshots_cache,
        repo,
        identity,
        |pinned_repo| async move {
            github_import::download_repo_snapshot(client, &pinned_repo, auth_token)
                .await
                .map_err(CentralUpdatesError::from)
        },
    )
    .await
}

pub(super) async fn load_verified_local_addition_snapshot_with<F, Fut>(
    snapshots_cache: &CentralUpdateSnapshotCache,
    repo: &github_import::GitHubRepoRef,
    identity: &PendingAdditionSnapshotIdentity,
    downloader: F,
) -> Result<Arc<CentralUpdateRepositorySnapshot>, CentralUpdatesError>
where
    F: FnOnce(github_import::GitHubRepoRef) -> Fut,
    Fut: std::future::Future<
        Output = Result<github_import::GitHubRepoSnapshot, CentralUpdatesError>,
    >,
{
    let cache_key = repo_cache_key(repo);
    if let Some(snapshot) = snapshots_cache.get_fresh(&cache_key) {
        if snapshot.matches_identity(&identity.resolved_commit_sha, &identity.snapshot_digest) {
            if github_import::repository_snapshot_digest_from_local(&snapshot)
                != identity.snapshot_digest
            {
                return Err(CentralUpdatesError::SnapshotChanged);
            }
            return Ok(snapshot);
        }
    }

    let pinned_repo = github_import::pinned_repo_ref(repo, &identity.resolved_commit_sha);
    let snapshot = downloader(pinned_repo).await?;
    let snapshot_digest = github_import::repository_snapshot_digest_from_local(&snapshot);
    if snapshot_digest != identity.snapshot_digest {
        return Err(CentralUpdatesError::SnapshotChanged);
    }
    let snapshot = Arc::new(CentralUpdateRepositorySnapshot::new(
        identity.resolved_commit_sha.clone(),
        snapshot_digest,
        snapshot,
    ));
    snapshots_cache.insert(cache_key, Arc::clone(&snapshot))?;
    Ok(snapshot)
}
