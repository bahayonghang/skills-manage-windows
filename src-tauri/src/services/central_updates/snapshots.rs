//! GitHub repository snapshot cache and bulk snapshot preparation.
//!
//! Short-lived snapshots are shared by Central update check and update flows
//! so "check, then update" reuses the archive that was just downloaded
//! without copying credentials into target DBs. `AppState` owns one cache
//! instance for the whole app (re-exported from `lib.rs` as
//! `crate::CentralUpdateSnapshotCache`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

use crate::services::github_import::{self, GitHubRepoRef, GitHubRepoSnapshot};

use super::error::CentralUpdatesError;
use super::types::{PreparedSkillUpdate, SnapshotCachePolicy};

pub(crate) fn snapshot_cache_ttl() -> chrono::Duration {
    chrono::Duration::minutes(10)
}

const SNAPSHOT_DOWNLOAD_CONCURRENCY: usize = 4;

#[derive(Default)]
pub struct CentralUpdateSnapshotCache {
    snapshots: Mutex<HashMap<String, CachedGitHubSnapshot>>,
}

#[derive(Clone)]
struct CachedGitHubSnapshot {
    snapshot: GitHubRepoSnapshot,
    cached_at: chrono::DateTime<chrono::Utc>,
}

impl CentralUpdateSnapshotCache {
    pub(crate) fn get_fresh(
        &self,
        key: &str,
        max_age: chrono::Duration,
    ) -> Option<GitHubRepoSnapshot> {
        let now = chrono::Utc::now();
        match self.snapshots.lock() {
            Ok(snapshots) => snapshots.get(key).and_then(|cached| {
                if now.signed_duration_since(cached.cached_at) <= max_age {
                    Some(cached.snapshot.clone())
                } else {
                    None
                }
            }),
            Err(error) => {
                tracing::warn!(error = %error, "Central update snapshot cache lock is poisoned during read");
                None
            }
        }
    }

    pub(crate) fn insert(&self, key: String, snapshot: GitHubRepoSnapshot) {
        match self.snapshots.lock() {
            Ok(mut snapshots) => {
                snapshots.insert(
                    key,
                    CachedGitHubSnapshot {
                        snapshot,
                        cached_at: chrono::Utc::now(),
                    },
                );
            }
            Err(error) => {
                tracing::warn!(error = %error, "Central update snapshot cache lock is poisoned during insert");
            }
        }
    }

    pub fn clear(&self) {
        match self.snapshots.lock() {
            Ok(mut snapshots) => snapshots.clear(),
            Err(error) => {
                tracing::warn!(error = %error, "Central update snapshot cache lock is poisoned during clear");
            }
        }
    }
}

pub(crate) fn repo_cache_key(repo: &GitHubRepoRef) -> String {
    format!("{}/{}/{}", repo.owner, repo.repo, repo.branch)
}

pub(crate) async fn prepare_snapshots(
    client: &reqwest::Client,
    auth_token: Option<&str>,
    prepared: &[PreparedSkillUpdate],
    cache: &CentralUpdateSnapshotCache,
) -> Result<HashMap<String, GitHubRepoSnapshot>, CentralUpdatesError> {
    let repos = prepared
        .iter()
        .filter_map(|prepared_skill| {
            prepared_skill
                .source
                .as_ref()
                .map(|source| source.repo.clone())
        })
        .collect::<Vec<_>>();
    prepare_snapshots_for_repo_refs(client, auth_token, &repos, cache).await
}

pub(crate) async fn prepare_snapshots_for_repo_refs(
    client: &reqwest::Client,
    auth_token: Option<&str>,
    repos: &[GitHubRepoRef],
    cache: &CentralUpdateSnapshotCache,
) -> Result<HashMap<String, GitHubRepoSnapshot>, CentralUpdatesError> {
    prepare_snapshots_for_repo_refs_with_policy(
        client,
        auth_token,
        repos,
        cache,
        SnapshotCachePolicy::UseFresh,
    )
    .await
}

pub(crate) async fn prepare_snapshots_for_repo_refs_with_policy(
    client: &reqwest::Client,
    auth_token: Option<&str>,
    repos: &[GitHubRepoRef],
    cache: &CentralUpdateSnapshotCache,
    cache_policy: SnapshotCachePolicy,
) -> Result<HashMap<String, GitHubRepoSnapshot>, CentralUpdatesError> {
    let mut repos_by_key = HashMap::<String, GitHubRepoRef>::new();
    for repo in repos {
        repos_by_key
            .entry(repo_cache_key(repo))
            .or_insert_with(|| repo.clone());
    }
    let mut snapshots = HashMap::new();
    let mut missing = Vec::new();
    for (key, repo) in repos_by_key {
        if cache_policy == SnapshotCachePolicy::UseFresh {
            if let Some(snapshot) = cache.get_fresh(&key, snapshot_cache_ttl()) {
                snapshots.insert(key, snapshot);
            } else {
                missing.push(repo);
            }
        } else {
            missing.push(repo);
        }
    }

    let semaphore = Arc::new(Semaphore::new(SNAPSHOT_DOWNLOAD_CONCURRENCY));
    let auth = auth_token.map(str::to_string);
    let downloads = missing.into_iter().map(|repo| {
        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);
        let auth = auth.clone();
        async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .map_err(|_| CentralUpdatesError::SnapshotDownloaderClosed)?;
            let snapshot =
                github_import::download_repo_snapshot(&client, &repo, auth.as_deref()).await?;
            Ok::<_, CentralUpdatesError>((repo_cache_key(&repo), snapshot))
        }
    });

    for result in futures_util::future::join_all(downloads).await {
        let (key, snapshot) = result?;
        cache.insert(key.clone(), snapshot.clone());
        snapshots.insert(key, snapshot);
    }

    Ok(snapshots)
}
