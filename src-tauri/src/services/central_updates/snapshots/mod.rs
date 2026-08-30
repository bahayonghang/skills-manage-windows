//! GitHub repository snapshot cache and bulk snapshot preparation.
//!
//! Short-lived snapshots are shared by Central update check and update flows
//! so "check, then update" reuses the archive that was just downloaded
//! without copying credentials into target DBs. `AppState` owns one cache
//! instance for the whole app (re-exported from `lib.rs` as
//! `crate::CentralUpdateSnapshotCache`).

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

use crate::services::github_import::{self, GitHubRepoRef, GitHubRepoSnapshot, GithubImportError};

use super::error::CentralUpdatesError;
use super::types::{PreparedSkillUpdate, SnapshotCachePolicy};

pub(crate) fn snapshot_cache_ttl() -> chrono::Duration {
    chrono::Duration::minutes(10)
}

const SNAPSHOT_DOWNLOAD_CONCURRENCY: usize = 4;
const DEFAULT_SNAPSHOT_CACHE_MAX_ENTRIES: usize = 8;
const DEFAULT_SNAPSHOT_CACHE_MAX_BYTES: u64 = 256 * 1024 * 1024;

pub(crate) type SharedGitHubSnapshots = HashMap<String, Arc<CentralUpdateRepositorySnapshot>>;

/// Immutable repository identity plus the bounded bytes acquired for one
/// Central Update refresh. The display branch remains in the cache key; this
/// value proves which commit and repository digest produced the inventory.
#[derive(Debug)]
pub(crate) struct CentralUpdateRepositorySnapshot {
    pub(crate) resolved_commit_sha: String,
    pub(crate) snapshot_digest: String,
    snapshot: Arc<GitHubRepoSnapshot>,
}

impl CentralUpdateRepositorySnapshot {
    pub(crate) fn new(
        resolved_commit_sha: String,
        snapshot_digest: String,
        snapshot: impl Into<Arc<GitHubRepoSnapshot>>,
    ) -> Self {
        Self {
            resolved_commit_sha,
            snapshot_digest,
            snapshot: snapshot.into(),
        }
    }

    pub(crate) fn matches_identity(&self, commit_sha: &str, snapshot_digest: &str) -> bool {
        self.resolved_commit_sha == commit_sha && self.snapshot_digest == snapshot_digest
    }
}

impl Deref for CentralUpdateRepositorySnapshot {
    type Target = GitHubRepoSnapshot;

    fn deref(&self) -> &Self::Target {
        &self.snapshot
    }
}

#[derive(Debug, Clone, Copy)]
struct SnapshotCacheLimits {
    max_entries: usize,
    max_bytes: u64,
    ttl: chrono::Duration,
}

impl Default for SnapshotCacheLimits {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_SNAPSHOT_CACHE_MAX_ENTRIES,
            max_bytes: DEFAULT_SNAPSHOT_CACHE_MAX_BYTES,
            ttl: snapshot_cache_ttl(),
        }
    }
}

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

pub struct CentralUpdateSnapshotCache {
    limits: SnapshotCacheLimits,
    state: Mutex<SnapshotCacheState>,
}

#[derive(Default)]
struct SnapshotCacheState {
    snapshots: HashMap<String, CachedGitHubSnapshot>,
    retained_bytes: u64,
    next_access_seq: u64,
}

struct CachedGitHubSnapshot {
    snapshot: Arc<CentralUpdateRepositorySnapshot>,
    retained_bytes: u64,
    cached_at: chrono::DateTime<chrono::Utc>,
    last_access_seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotCacheInsertOutcome {
    Cached,
    CurrentUseOnly,
}

impl Default for CentralUpdateSnapshotCache {
    fn default() -> Self {
        Self {
            limits: SnapshotCacheLimits::default(),
            state: Mutex::new(SnapshotCacheState::default()),
        }
    }
}

impl CentralUpdateSnapshotCache {
    pub(crate) fn get_fresh(&self, key: &str) -> Option<Arc<CentralUpdateRepositorySnapshot>> {
        self.get_fresh_at(key, chrono::Utc::now())
    }

    fn get_fresh_at(
        &self,
        key: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Option<Arc<CentralUpdateRepositorySnapshot>> {
        match self.state.lock() {
            Ok(mut state) => {
                prune_expired_cache_entries(&mut state, self.limits.ttl, now);
                let access_seq = next_access_seq(&mut state);
                state.snapshots.get_mut(key).map(|cached| {
                    cached.last_access_seq = access_seq;
                    Arc::clone(&cached.snapshot)
                })
            }
            Err(_error) => {
                tracing::warn!("Central update snapshot cache lock is poisoned during read");
                None
            }
        }
    }

    pub(crate) fn insert(
        &self,
        key: String,
        snapshot: impl Into<Arc<CentralUpdateRepositorySnapshot>>,
    ) -> Result<SnapshotCacheInsertOutcome, GithubImportError> {
        self.insert_at(key, snapshot.into(), chrono::Utc::now())
    }

    fn insert_at(
        &self,
        key: String,
        snapshot: Arc<CentralUpdateRepositorySnapshot>,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<SnapshotCacheInsertOutcome, GithubImportError> {
        let retained_bytes = snapshot.retained_bytes()?;
        if retained_bytes > self.limits.max_bytes || self.limits.max_entries == 0 {
            if let Ok(mut state) = self.state.lock() {
                prune_expired_cache_entries(&mut state, self.limits.ttl, now);
                remove_cache_entry(&mut state, &key);
            }
            tracing::debug!(
                retained_bytes,
                reason = "oversized",
                "Central update snapshot is available for the current request but not cached"
            );
            return Ok(SnapshotCacheInsertOutcome::CurrentUseOnly);
        }

        match self.state.lock() {
            Ok(mut state) => {
                prune_expired_cache_entries(&mut state, self.limits.ttl, now);
                remove_cache_entry(&mut state, &key);

                while state.snapshots.len() >= self.limits.max_entries
                    || state
                        .retained_bytes
                        .checked_add(retained_bytes)
                        .is_none_or(|total| total > self.limits.max_bytes)
                {
                    let Some(victim_key) = state
                        .snapshots
                        .iter()
                        .min_by(|(left_key, left), (right_key, right)| {
                            left.last_access_seq
                                .cmp(&right.last_access_seq)
                                .then_with(|| left_key.cmp(right_key))
                        })
                        .map(|(victim_key, _)| victim_key.clone())
                    else {
                        break;
                    };
                    if let Some(victim) = state.snapshots.remove(&victim_key) {
                        state.retained_bytes =
                            state.retained_bytes.saturating_sub(victim.retained_bytes);
                    }
                }

                let access_seq = next_access_seq(&mut state);
                state.retained_bytes = state
                    .retained_bytes
                    .checked_add(retained_bytes)
                    .ok_or(GithubImportError::SnapshotSizeOverflow)?;
                state.snapshots.insert(
                    key,
                    CachedGitHubSnapshot {
                        snapshot,
                        retained_bytes,
                        cached_at: now,
                        last_access_seq: access_seq,
                    },
                );
                tracing::debug!(
                    entries = state.snapshots.len(),
                    retained_bytes = state.retained_bytes,
                    reason = "insert",
                    "Central update snapshot cache state changed"
                );
                Ok(SnapshotCacheInsertOutcome::Cached)
            }
            Err(_error) => {
                tracing::warn!("Central update snapshot cache lock is poisoned during insert");
                Ok(SnapshotCacheInsertOutcome::CurrentUseOnly)
            }
        }
    }

    pub fn clear(&self) {
        match self.state.lock() {
            Ok(mut state) => *state = SnapshotCacheState::default(),
            Err(_error) => {
                tracing::warn!("Central update snapshot cache lock is poisoned during clear");
            }
        }
    }

    #[cfg(test)]
    fn with_limits(limits: SnapshotCacheLimits) -> Self {
        Self {
            limits,
            state: Mutex::new(SnapshotCacheState::default()),
        }
    }

    #[cfg(test)]
    fn metrics(&self) -> (usize, u64) {
        let state = self.state.lock().expect("snapshot cache state");
        (state.snapshots.len(), state.retained_bytes)
    }
}

fn next_access_seq(state: &mut SnapshotCacheState) -> u64 {
    let next = state.next_access_seq;
    state.next_access_seq = state.next_access_seq.wrapping_add(1);
    next
}

fn prune_expired_cache_entries(
    state: &mut SnapshotCacheState,
    ttl: chrono::Duration,
    now: chrono::DateTime<chrono::Utc>,
) {
    let expired_keys = state
        .snapshots
        .iter()
        .filter(|(_, cached)| now.signed_duration_since(cached.cached_at) > ttl)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    for key in expired_keys {
        remove_cache_entry(state, &key);
    }
}

fn remove_cache_entry(state: &mut SnapshotCacheState, key: &str) {
    if let Some(removed) = state.snapshots.remove(key) {
        state.retained_bytes = state.retained_bytes.saturating_sub(removed.retained_bytes);
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
) -> Result<SharedGitHubSnapshots, CentralUpdatesError> {
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
) -> Result<SharedGitHubSnapshots, CentralUpdatesError> {
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
) -> Result<SharedGitHubSnapshots, CentralUpdatesError> {
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

/// One repository whose snapshot could not be acquired, kept alongside the
/// snapshots that did succeed.
pub(crate) struct SnapshotRepositoryFailure {
    pub(crate) repo: GitHubRepoRef,
    pub(crate) error: CentralUpdatesError,
}

/// Outcome of a snapshot acquisition round: every repository either produced a
/// snapshot or a failure, and neither set discards the other.
pub(crate) struct SnapshotAcquisition {
    pub(crate) snapshots: SharedGitHubSnapshots,
    pub(crate) failures: Vec<SnapshotRepositoryFailure>,
    pub(crate) retry_attempted: usize,
    pub(crate) retry_recovered: usize,
}

/// Fail-fast wrapper kept for callers that treat any repository failure as a
/// failure of the whole batch. Returns the first failure in acquisition order,
/// matching the previous `join_all` + `?` behaviour exactly.
pub(crate) async fn prepare_snapshots_for_repo_refs_with_policy_and_progress(
    client: &reqwest::Client,
    auth_token: Option<&str>,
    repos: &[GitHubRepoRef],
    cache: &CentralUpdateSnapshotCache,
    cache_policy: SnapshotCachePolicy,
    progress: Option<SnapshotProgressReporter>,
) -> Result<SharedGitHubSnapshots, CentralUpdatesError> {
    let acquisition = prepare_snapshots_for_repo_refs_collecting_failures(
        client,
        auth_token,
        repos,
        cache,
        cache_policy,
        progress,
    )
    .await?;
    match acquisition.failures.into_iter().next() {
        Some(failure) => Err(failure.error),
        None => Ok(acquisition.snapshots),
    }
}

/// Acquire every repository snapshot, keeping partial results.
///
/// One unreachable or rejected repository must not discard the snapshots that
/// other repositories already produced: the update check spans every syncable
/// GitHub repository, so aborting on the first failure leaves the whole run
/// with no persisted inventory.
pub(crate) async fn prepare_snapshots_for_repo_refs_collecting_failures(
    client: &reqwest::Client,
    auth_token: Option<&str>,
    repos: &[GitHubRepoRef],
    cache: &CentralUpdateSnapshotCache,
    cache_policy: SnapshotCachePolicy,
    progress: Option<SnapshotProgressReporter>,
) -> Result<SnapshotAcquisition, CentralUpdatesError> {
    let client = client.clone();
    let auth = auth_token.map(str::to_string);
    prepare_snapshots_for_repo_refs_collecting_failures_with_downloader(
        repos,
        cache,
        cache_policy,
        progress,
        SNAPSHOT_DOWNLOAD_CONCURRENCY,
        move |repo| {
            let client = client.clone();
            let auth = auth.clone();
            async move {
                let resolved_commit_sha =
                    github_import::resolve_commit_sha(&client, &repo, auth.as_deref()).await?;
                let pinned_repo = github_import::pinned_repo_ref(&repo, &resolved_commit_sha);
                let snapshot =
                    github_import::download_repo_snapshot(&client, &pinned_repo, auth.as_deref())
                        .await?;
                let snapshot_digest =
                    github_import::repository_snapshot_digest_from_local(&snapshot);
                Ok(CentralUpdateRepositorySnapshot::new(
                    resolved_commit_sha,
                    snapshot_digest,
                    snapshot,
                ))
            }
        },
    )
    .await
}

async fn prepare_snapshots_for_repo_refs_collecting_failures_with_downloader<D, F>(
    repos: &[GitHubRepoRef],
    cache: &CentralUpdateSnapshotCache,
    cache_policy: SnapshotCachePolicy,
    progress: Option<SnapshotProgressReporter>,
    initial_concurrency: usize,
    downloader: D,
) -> Result<SnapshotAcquisition, CentralUpdatesError>
where
    D: Fn(GitHubRepoRef) -> F + Clone,
    F: Future<Output = Result<CentralUpdateRepositorySnapshot, GithubImportError>>,
{
    let mut seen = HashSet::new();
    let mut ordered_repos = Vec::new();
    for repo in repos {
        let key = repo_cache_key(repo);
        if seen.insert(key) {
            ordered_repos.push(repo.clone());
        }
    }
    let total = ordered_repos.len();
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
    for repo in ordered_repos {
        let key = repo_cache_key(&repo);
        if cache_policy == SnapshotCachePolicy::UseFresh {
            if let Some(snapshot) = cache.get_fresh(&key) {
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

    let semaphore = Arc::new(Semaphore::new(initial_concurrency.max(1)));
    let downloads = missing.into_iter().map(|repo| {
        let semaphore = Arc::clone(&semaphore);
        let downloader = downloader.clone();
        let progress = progress.clone();
        let completed = Arc::clone(&completed);
        async move {
            let _permit = match semaphore.acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Err((repo, CentralUpdatesError::SnapshotDownloaderClosed, false));
                }
            };
            report_repository(
                &progress,
                SnapshotProgressStatus::RepositoryStarted,
                total,
                completed.load(Ordering::SeqCst),
                &repo,
            );
            match downloader(repo.clone()).await {
                Ok(snapshot) => Ok((repo, snapshot)),
                Err(error) => {
                    let retryable = error.is_snapshot_retryable();
                    Err((repo, error.into(), retryable))
                }
            }
        }
    });

    let mut failures = Vec::new();
    let mut retryable_failures = Vec::new();
    for result in futures_util::future::join_all(downloads).await {
        match result {
            Ok((repo, snapshot)) => {
                let key = repo_cache_key(&repo);
                let snapshot = Arc::new(snapshot);
                let _ = cache.insert(key.clone(), Arc::clone(&snapshot))?;
                snapshots.insert(key, snapshot);
                report_repository_settled(
                    &progress,
                    &completed,
                    total,
                    &repo,
                    SnapshotProgressStatus::RepositoryCompleted,
                );
            }
            Err((repo, _error, true)) => retryable_failures.push(repo),
            Err((repo, error, false)) => {
                report_repository_settled(
                    &progress,
                    &completed,
                    total,
                    &repo,
                    SnapshotProgressStatus::RepositoryFailed,
                );
                failures.push(SnapshotRepositoryFailure { repo, error });
            }
        }
    }

    let retry_attempted = retryable_failures.len();
    let mut retry_recovered = 0;
    for repo in retryable_failures {
        match downloader(repo.clone()).await {
            Ok(snapshot) => {
                let key = repo_cache_key(&repo);
                let snapshot = Arc::new(snapshot);
                let _ = cache.insert(key.clone(), Arc::clone(&snapshot))?;
                snapshots.insert(key, snapshot);
                retry_recovered += 1;
                report_repository_settled(
                    &progress,
                    &completed,
                    total,
                    &repo,
                    SnapshotProgressStatus::RepositoryCompleted,
                );
            }
            Err(error) => {
                report_repository_settled(
                    &progress,
                    &completed,
                    total,
                    &repo,
                    SnapshotProgressStatus::RepositoryFailed,
                );
                failures.push(SnapshotRepositoryFailure {
                    repo,
                    error: error.into(),
                });
            }
        }
    }

    Ok(SnapshotAcquisition {
        snapshots,
        failures,
        retry_attempted,
        retry_recovered,
    })
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
mod tests;
