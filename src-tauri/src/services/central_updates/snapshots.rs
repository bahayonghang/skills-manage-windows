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

use crate::services::github_import::{self, GitHubRepoRef, GitHubRepoSnapshot, GithubImportError};

use super::error::CentralUpdatesError;
use super::types::{PreparedSkillUpdate, SnapshotCachePolicy};

pub(crate) fn snapshot_cache_ttl() -> chrono::Duration {
    chrono::Duration::minutes(10)
}

const SNAPSHOT_DOWNLOAD_CONCURRENCY: usize = 4;
const DEFAULT_SNAPSHOT_CACHE_MAX_ENTRIES: usize = 8;
const DEFAULT_SNAPSHOT_CACHE_MAX_BYTES: u64 = 256 * 1024 * 1024;

pub(crate) type SharedGitHubSnapshots = HashMap<String, Arc<GitHubRepoSnapshot>>;

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
    snapshot: Arc<GitHubRepoSnapshot>,
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
    pub(crate) fn get_fresh(&self, key: &str) -> Option<Arc<GitHubRepoSnapshot>> {
        self.get_fresh_at(key, chrono::Utc::now())
    }

    fn get_fresh_at(
        &self,
        key: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Option<Arc<GitHubRepoSnapshot>> {
        match self.state.lock() {
            Ok(mut state) => {
                prune_expired_cache_entries(&mut state, self.limits.ttl, now);
                let access_seq = next_access_seq(&mut state);
                state.snapshots.get_mut(key).map(|cached| {
                    cached.last_access_seq = access_seq;
                    Arc::clone(&cached.snapshot)
                })
            }
            Err(error) => {
                tracing::warn!(error = %error, "Central update snapshot cache lock is poisoned during read");
                None
            }
        }
    }

    pub(crate) fn insert(
        &self,
        key: String,
        snapshot: impl Into<Arc<GitHubRepoSnapshot>>,
    ) -> Result<SnapshotCacheInsertOutcome, GithubImportError> {
        self.insert_at(key, snapshot.into(), chrono::Utc::now())
    }

    fn insert_at(
        &self,
        key: String,
        snapshot: Arc<GitHubRepoSnapshot>,
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
            Err(error) => {
                tracing::warn!(error = %error, "Central update snapshot cache lock is poisoned during insert");
                Ok(SnapshotCacheInsertOutcome::CurrentUseOnly)
            }
        }
    }

    pub fn clear(&self) {
        match self.state.lock() {
            Ok(mut state) => *state = SnapshotCacheState::default(),
            Err(error) => {
                tracing::warn!(error = %error, "Central update snapshot cache lock is poisoned during clear");
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

pub(crate) async fn prepare_snapshots_for_repo_refs_with_policy_and_progress(
    client: &reqwest::Client,
    auth_token: Option<&str>,
    repos: &[GitHubRepoRef],
    cache: &CentralUpdateSnapshotCache,
    cache_policy: SnapshotCachePolicy,
    progress: Option<SnapshotProgressReporter>,
) -> Result<SharedGitHubSnapshots, CentralUpdatesError> {
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
        let snapshot = Arc::new(snapshot);
        let _ = cache.insert(key.clone(), Arc::clone(&snapshot))?;
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

    fn snapshot_with_bytes(bytes: &[u8]) -> Arc<GitHubRepoSnapshot> {
        Arc::new(GitHubRepoSnapshot {
            files: HashMap::from([("SKILL.md".to_string(), bytes.to_vec())]),
        })
    }

    fn test_cache(
        max_entries: usize,
        max_bytes: u64,
        ttl_seconds: i64,
    ) -> CentralUpdateSnapshotCache {
        CentralUpdateSnapshotCache::with_limits(SnapshotCacheLimits {
            max_entries,
            max_bytes,
            ttl: chrono::Duration::seconds(ttl_seconds),
        })
    }

    #[test]
    fn cache_hit_preserves_arc_identity() {
        let cache = test_cache(2, 32, 60);
        let snapshot = snapshot_with_bytes(b"shared bytes");
        cache
            .insert("repo".to_string(), Arc::clone(&snapshot))
            .expect("cache insert");

        let hit = cache.get_fresh("repo").expect("cache hit");
        assert!(Arc::ptr_eq(&snapshot, &hit));
        assert!(std::ptr::eq(
            snapshot.files["SKILL.md"].as_ptr(),
            hit.files["SKILL.md"].as_ptr()
        ));
    }

    #[test]
    fn cache_enforces_entry_limit_with_deterministic_lru() {
        let cache = test_cache(2, 64, 60);
        let now = chrono::Utc::now();
        cache
            .insert_at("a".to_string(), snapshot_with_bytes(b"a"), now)
            .unwrap();
        cache
            .insert_at("b".to_string(), snapshot_with_bytes(b"b"), now)
            .unwrap();
        assert!(cache.get_fresh_at("a", now).is_some());
        cache
            .insert_at("c".to_string(), snapshot_with_bytes(b"c"), now)
            .unwrap();

        assert!(cache.get_fresh_at("a", now).is_some());
        assert!(cache.get_fresh_at("b", now).is_none());
        assert!(cache.get_fresh_at("c", now).is_some());
        assert_eq!(cache.metrics(), (2, 2));
    }

    #[test]
    fn cache_enforces_aggregate_byte_limit() {
        let cache = test_cache(4, 5, 60);
        let now = chrono::Utc::now();
        cache
            .insert_at("a".to_string(), snapshot_with_bytes(b"abc"), now)
            .unwrap();
        cache
            .insert_at("b".to_string(), snapshot_with_bytes(b"def"), now)
            .unwrap();

        assert!(cache.get_fresh_at("a", now).is_none());
        assert!(cache.get_fresh_at("b", now).is_some());
        assert_eq!(cache.metrics(), (1, 3));
    }

    #[test]
    fn expired_entries_are_reclaimed_on_read_and_insert() {
        let cache = test_cache(2, 16, 10);
        let inserted_at = chrono::Utc::now();
        cache
            .insert_at(
                "expired".to_string(),
                snapshot_with_bytes(b"old"),
                inserted_at,
            )
            .unwrap();

        let after_ttl = inserted_at + chrono::Duration::seconds(11);
        assert!(cache.get_fresh_at("expired", after_ttl).is_none());
        assert_eq!(cache.metrics(), (0, 0));

        cache
            .insert_at("fresh".to_string(), snapshot_with_bytes(b"new"), after_ttl)
            .unwrap();
        assert_eq!(cache.metrics(), (1, 3));
    }

    #[test]
    fn oversized_snapshot_remains_current_use_only() {
        let cache = test_cache(2, 2, 60);
        let snapshot = snapshot_with_bytes(b"abc");
        assert_eq!(
            cache
                .insert("large".to_string(), Arc::clone(&snapshot))
                .unwrap(),
            SnapshotCacheInsertOutcome::CurrentUseOnly
        );
        assert_eq!(cache.metrics(), (0, 0));
        assert!(cache.get_fresh("large").is_none());
        assert_eq!(snapshot.files["SKILL.md"], b"abc");
    }

    #[test]
    fn oversized_refresh_invalidates_older_snapshot_for_the_same_key() {
        let cache = test_cache(2, 2, 60);
        cache
            .insert("repo".to_string(), snapshot_with_bytes(b"ok"))
            .unwrap();

        assert_eq!(
            cache
                .insert("repo".to_string(), snapshot_with_bytes(b"oversized"))
                .unwrap(),
            SnapshotCacheInsertOutcome::CurrentUseOnly
        );
        assert_eq!(cache.metrics(), (0, 0));
        assert!(cache.get_fresh("repo").is_none());
    }

    #[test]
    fn replacing_a_key_updates_retained_byte_accounting() {
        let cache = test_cache(2, 8, 60);
        cache
            .insert("repo".to_string(), snapshot_with_bytes(b"old"))
            .unwrap();
        cache
            .insert("repo".to_string(), snapshot_with_bytes(b"newer"))
            .unwrap();
        assert_eq!(cache.metrics(), (1, 5));
    }

    #[tokio::test]
    async fn cached_progress_counts_deduplicated_repositories() {
        let first = repo("openai", "skills");
        let second = repo("anthropics", "skills");
        let cache = CentralUpdateSnapshotCache::default();
        cache
            .insert(repo_cache_key(&first), snapshot())
            .expect("cache first snapshot");
        cache
            .insert(repo_cache_key(&second), snapshot())
            .expect("cache second snapshot");
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
