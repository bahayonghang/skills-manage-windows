//! GitHub repository snapshot cache and bulk snapshot preparation.
//!
//! Short-lived snapshots are shared by Central update check and update flows
//! so "check, then update" reuses the archive that was just downloaded
//! without copying credentials into target DBs. `AppState` owns one cache
//! instance for the whole app (re-exported from `lib.rs` as
//! `crate::CentralUpdateSnapshotCache`).

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

use crate::services::github_import::{self, GitHubRepoRef, GitHubRepoSnapshot};

use super::error::CentralUpdatesError;
use super::types::{PreparedSkillUpdate, SnapshotCachePolicy};

pub(crate) fn snapshot_cache_ttl() -> chrono::Duration {
    chrono::Duration::minutes(10)
}

const SNAPSHOT_DOWNLOAD_CONCURRENCY: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotProgressStatus {
    Started,
    RepositoryStarted,
    RepositoryCompleted,
    RepositoryFailed,
    Finalizing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotProgressEvent {
    pub status: SnapshotProgressStatus,
    pub total: usize,
    pub completed: usize,
    pub repository_key: Option<String>,
    pub repository_name: Option<String>,
}

impl SnapshotProgressEvent {
    pub(crate) fn finalizing(total: usize, completed: usize) -> Self {
        Self {
            status: SnapshotProgressStatus::Finalizing,
            total,
            completed,
            repository_key: None,
            repository_name: None,
        }
    }
}

pub(crate) type SnapshotProgressReporter =
    Arc<dyn Fn(SnapshotProgressEvent) + Send + Sync + 'static>;

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

#[tracing::instrument(
    skip_all,
    fields(phase = "snapshot_download", repositories = repos.len())
)]
pub(crate) async fn prepare_snapshots_for_repo_refs_with_policy(
    client: &reqwest::Client,
    auth_token: Option<&str>,
    repos: &[GitHubRepoRef],
    cache: &CentralUpdateSnapshotCache,
    cache_policy: SnapshotCachePolicy,
) -> Result<HashMap<String, GitHubRepoSnapshot>, CentralUpdatesError> {
    prepare_snapshots_for_repo_refs_with_policy_and_progress(
        client,
        auth_token,
        repos,
        cache,
        cache_policy,
        None,
    )
    .await
}

pub(crate) async fn prepare_snapshots_for_repo_refs_with_policy_and_progress(
    client: &reqwest::Client,
    auth_token: Option<&str>,
    repos: &[GitHubRepoRef],
    cache: &CentralUpdateSnapshotCache,
    cache_policy: SnapshotCachePolicy,
    progress: Option<SnapshotProgressReporter>,
) -> Result<HashMap<String, GitHubRepoSnapshot>, CentralUpdatesError> {
    let mut repos_by_key = HashMap::<String, GitHubRepoRef>::new();
    for repo in repos {
        repos_by_key
            .entry(repo_cache_key(repo))
            .or_insert_with(|| repo.clone());
    }
    let total = repos_by_key.len();
    let completed = Arc::new(AtomicUsize::new(0));
    report_progress(
        &progress,
        SnapshotProgressEvent {
            status: SnapshotProgressStatus::Started,
            total,
            completed: 0,
            repository_key: None,
            repository_name: None,
        },
    );

    let mut snapshots = HashMap::new();
    let mut missing = Vec::new();
    for (key, repo) in repos_by_key {
        if cache_policy == SnapshotCachePolicy::UseFresh {
            if let Some(snapshot) = cache.get_fresh(&key, snapshot_cache_ttl()) {
                snapshots.insert(key, snapshot);
                report_repository_settled(
                    &progress,
                    &completed,
                    total,
                    &repo,
                    SnapshotProgressStatus::RepositoryCompleted,
                );
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
        let progress = progress.clone();
        let completed = Arc::clone(&completed);
        async move {
            let _permit = match semaphore.acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    report_repository_settled(
                        &progress,
                        &completed,
                        total,
                        &repo,
                        SnapshotProgressStatus::RepositoryFailed,
                    );
                    return Err(CentralUpdatesError::SnapshotDownloaderClosed);
                }
            };
            report_repository(
                &progress,
                SnapshotProgressStatus::RepositoryStarted,
                total,
                completed.load(Ordering::SeqCst),
                &repo,
            );
            match github_import::download_repo_snapshot(&client, &repo, auth.as_deref()).await {
                Ok(snapshot) => {
                    report_repository_settled(
                        &progress,
                        &completed,
                        total,
                        &repo,
                        SnapshotProgressStatus::RepositoryCompleted,
                    );
                    Ok((repo_cache_key(&repo), snapshot))
                }
                Err(error) => {
                    report_repository_settled(
                        &progress,
                        &completed,
                        total,
                        &repo,
                        SnapshotProgressStatus::RepositoryFailed,
                    );
                    Err(error.into())
                }
            }
        }
    });

    for result in futures_util::future::join_all(downloads).await {
        let (key, snapshot) = result?;
        cache.insert(key.clone(), snapshot.clone());
        snapshots.insert(key, snapshot);
    }

    Ok(snapshots)
}

fn report_repository_settled(
    progress: &Option<SnapshotProgressReporter>,
    completed: &AtomicUsize,
    total: usize,
    repo: &GitHubRepoRef,
    status: SnapshotProgressStatus,
) {
    let completed = completed.fetch_add(1, Ordering::SeqCst) + 1;
    report_repository(progress, status, total, completed, repo);
}

fn report_repository(
    progress: &Option<SnapshotProgressReporter>,
    status: SnapshotProgressStatus,
    total: usize,
    completed: usize,
    repo: &GitHubRepoRef,
) {
    report_progress(
        progress,
        SnapshotProgressEvent {
            status,
            total,
            completed,
            repository_key: Some(repo_cache_key(repo)),
            repository_name: Some(format!("{}/{}", repo.owner, repo.repo)),
        },
    );
}

fn report_progress(progress: &Option<SnapshotProgressReporter>, event: SnapshotProgressEvent) {
    if let Some(progress) = progress {
        progress(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn repo(owner: &str, name: &str) -> GitHubRepoRef {
        GitHubRepoRef {
            owner: owner.to_string(),
            repo: name.to_string(),
            branch: "main".to_string(),
            normalized_url: format!("https://github.com/{owner}/{name}"),
        }
    }

    fn snapshot() -> GitHubRepoSnapshot {
        GitHubRepoSnapshot::default()
    }

    #[tokio::test]
    async fn cached_progress_counts_deduplicated_repositories() {
        let first = repo("openai", "skills");
        let second = repo("anthropics", "skills");
        let cache = CentralUpdateSnapshotCache::default();
        cache.insert(repo_cache_key(&first), snapshot());
        cache.insert(repo_cache_key(&second), snapshot());
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let progress: SnapshotProgressReporter = Arc::new(move |event| {
            recorded.lock().unwrap().push(event);
        });

        let snapshots = prepare_snapshots_for_repo_refs_with_policy_and_progress(
            &reqwest::Client::new(),
            None,
            &[first.clone(), second.clone(), first],
            &cache,
            SnapshotCachePolicy::UseFresh,
            Some(progress),
        )
        .await
        .unwrap();

        assert_eq!(snapshots.len(), 2);
        let events = events.lock().unwrap();
        assert_eq!(events[0].status, SnapshotProgressStatus::Started);
        assert_eq!(events[0].total, 2);
        assert_eq!(events[0].completed, 0);
        let settled = events
            .iter()
            .filter(|event| event.status == SnapshotProgressStatus::RepositoryCompleted)
            .collect::<Vec<_>>();
        assert_eq!(settled.len(), 2);
        assert_eq!(settled.last().unwrap().completed, 2);
        assert!(events
            .iter()
            .all(|event| event.status != SnapshotProgressStatus::RepositoryStarted));
        let names = settled
            .iter()
            .filter_map(|event| event.repository_name.as_deref())
            .collect::<HashSet<_>>();
        assert_eq!(names, HashSet::from(["openai/skills", "anthropics/skills"]));
    }

    #[tokio::test]
    async fn empty_progress_reports_zero_total_without_downloads() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let progress: SnapshotProgressReporter = Arc::new(move |event| {
            recorded.lock().unwrap().push(event);
        });

        let snapshots = prepare_snapshots_for_repo_refs_with_policy_and_progress(
            &reqwest::Client::new(),
            None,
            &[],
            &CentralUpdateSnapshotCache::default(),
            SnapshotCachePolicy::Bypass,
            Some(progress),
        )
        .await
        .unwrap();

        assert!(snapshots.is_empty());
        assert_eq!(
            events.lock().unwrap().as_slice(),
            &[SnapshotProgressEvent {
                status: SnapshotProgressStatus::Started,
                total: 0,
                completed: 0,
                repository_key: None,
                repository_name: None,
            }]
        );
    }
}
