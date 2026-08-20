//! Snapshot cache, acquisition, and progress-reporting tests.
//!
//! Split out of `snapshots/mod.rs` to keep both files inside the module
//! size budget; the test bodies are unchanged.

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

fn pinned_snapshot(snapshot: GitHubRepoSnapshot) -> CentralUpdateRepositorySnapshot {
    let digest = github_import::repository_snapshot_digest_from_local(&snapshot);
    CentralUpdateRepositorySnapshot::new("a".repeat(40), digest, snapshot)
}

fn snapshot() -> CentralUpdateRepositorySnapshot {
    pinned_snapshot(GitHubRepoSnapshot::default())
}

fn snapshot_with_bytes(bytes: &[u8]) -> Arc<CentralUpdateRepositorySnapshot> {
    Arc::new(pinned_snapshot(GitHubRepoSnapshot {
        files: HashMap::from([("SKILL.md".to_string(), bytes.to_vec())]),
    }))
}

fn test_cache(max_entries: usize, max_bytes: u64, ttl_seconds: i64) -> CentralUpdateSnapshotCache {
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

/// One repository failing must not discard the snapshots the others already
/// produced. The failing ref is rejected by local validation, so this stays
/// deterministic and offline.
#[tokio::test]
async fn one_failed_repository_does_not_discard_the_others() {
    let healthy = repo("openai", "skills");
    let broken = GitHubRepoRef {
        branch: "unsafe/branch".to_string(),
        ..repo("anthropics", "skills")
    };
    let cache = CentralUpdateSnapshotCache::default();
    cache
        .insert(repo_cache_key(&healthy), snapshot())
        .expect("cache healthy snapshot");

    let acquisition = prepare_snapshots_for_repo_refs_collecting_failures(
        &reqwest::Client::new(),
        None,
        &[healthy.clone(), broken.clone()],
        &cache,
        SnapshotCachePolicy::UseFresh,
        None,
    )
    .await
    .expect("acquisition must succeed with partial results");

    assert_eq!(acquisition.snapshots.len(), 1);
    assert!(acquisition
        .snapshots
        .contains_key(&repo_cache_key(&healthy)));
    assert_eq!(acquisition.failures.len(), 1);
    assert_eq!(acquisition.failures[0].repo.repo, broken.repo);

    // The fail-fast wrapper keeps its old contract for callers that still
    // treat any repository failure as a failure of the whole batch.
    assert!(prepare_snapshots_for_repo_refs_with_policy_and_progress(
        &reqwest::Client::new(),
        None,
        &[healthy, broken],
        &cache,
        SnapshotCachePolicy::UseFresh,
        None,
    )
    .await
    .is_err());
}

#[tokio::test]
async fn transient_snapshot_failures_retry_once_serially_after_initial_batch_settles() {
    let repos = (0..5)
        .map(|index| repo("owner", &format!("repo-{index}")))
        .collect::<Vec<_>>();
    let calls = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
    let retry_order = Arc::new(Mutex::new(Vec::new()));
    let initial_active = Arc::new(AtomicUsize::new(0));
    let initial_peak = Arc::new(AtomicUsize::new(0));
    let retry_active = Arc::new(AtomicUsize::new(0));
    let retry_peak = Arc::new(AtomicUsize::new(0));
    let events = Arc::new(Mutex::new(Vec::new()));
    let recorded_events = Arc::clone(&events);
    let progress: SnapshotProgressReporter = Arc::new(move |event| {
        recorded_events.lock().unwrap().push(event);
    });
    let downloader = {
        let calls = Arc::clone(&calls);
        let retry_order = Arc::clone(&retry_order);
        let initial_active = Arc::clone(&initial_active);
        let initial_peak = Arc::clone(&initial_peak);
        let retry_active = Arc::clone(&retry_active);
        let retry_peak = Arc::clone(&retry_peak);
        move |repo: GitHubRepoRef| {
            let calls = Arc::clone(&calls);
            let retry_order = Arc::clone(&retry_order);
            let initial_active = Arc::clone(&initial_active);
            let initial_peak = Arc::clone(&initial_peak);
            let retry_active = Arc::clone(&retry_active);
            let retry_peak = Arc::clone(&retry_peak);
            async move {
                let attempt = {
                    let mut calls = calls.lock().unwrap();
                    let attempt = calls.entry(repo.repo.clone()).or_default();
                    *attempt += 1;
                    *attempt
                };
                let (active, peak) = if attempt == 1 {
                    (&initial_active, &initial_peak)
                } else {
                    retry_order.lock().unwrap().push(repo.repo.clone());
                    (&retry_active, &retry_peak)
                };
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(current, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                active.fetch_sub(1, Ordering::SeqCst);
                if attempt == 1 {
                    Err(GithubImportError::ArchiveTimeout)
                } else {
                    Ok(snapshot())
                }
            }
        }
    };
    let cache = CentralUpdateSnapshotCache::default();

    let acquisition = prepare_snapshots_for_repo_refs_collecting_failures_with_downloader(
        &repos,
        &cache,
        SnapshotCachePolicy::Bypass,
        Some(progress),
        SNAPSHOT_DOWNLOAD_CONCURRENCY,
        downloader,
    )
    .await
    .unwrap();

    assert_eq!(acquisition.snapshots.len(), 5);
    assert!(acquisition.failures.is_empty());
    assert_eq!(acquisition.retry_attempted, 5);
    assert_eq!(acquisition.retry_recovered, 5);
    assert_eq!(initial_peak.load(Ordering::SeqCst), 4);
    assert_eq!(retry_peak.load(Ordering::SeqCst), 1);
    assert_eq!(
        retry_order.lock().unwrap().as_slice(),
        ["repo-0", "repo-1", "repo-2", "repo-3", "repo-4"]
    );
    assert!(calls.lock().unwrap().values().all(|count| *count == 2));
    for repo in &repos {
        assert!(cache.get_fresh(&repo_cache_key(repo)).is_some());
    }
    let events = events.lock().unwrap();
    let settled = events
        .iter()
        .filter(|event| {
            matches!(
                event.status,
                SnapshotProgressStatus::RepositoryCompleted
                    | SnapshotProgressStatus::RepositoryFailed
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(settled.len(), 5);
    assert!(settled
        .iter()
        .all(|event| event.status == SnapshotProgressStatus::RepositoryCompleted));
    assert_eq!(settled.last().unwrap().completed, 5);
    assert!(events.iter().all(|event| event.completed <= event.total));
}

#[tokio::test]
async fn every_retryable_snapshot_family_is_attempted_exactly_twice() {
    let repos = ["timeout", "request", "body", "status"]
        .into_iter()
        .map(|name| repo("owner", name))
        .collect::<Vec<_>>();
    let calls = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
    let downloader = {
        let calls = Arc::clone(&calls);
        move |repo: GitHubRepoRef| {
            let calls = Arc::clone(&calls);
            async move {
                *calls.lock().unwrap().entry(repo.repo.clone()).or_default() += 1;
                Err(match repo.repo.as_str() {
                    "timeout" => GithubImportError::ArchiveTimeout,
                    "request" => GithubImportError::ArchiveRequest,
                    "body" => GithubImportError::ArchiveResponseBody,
                    "status" => GithubImportError::ArchiveStatusExhausted,
                    _ => unreachable!(),
                })
            }
        }
    };

    let acquisition = prepare_snapshots_for_repo_refs_collecting_failures_with_downloader(
        &repos,
        &CentralUpdateSnapshotCache::default(),
        SnapshotCachePolicy::Bypass,
        None,
        SNAPSHOT_DOWNLOAD_CONCURRENCY,
        downloader,
    )
    .await
    .unwrap();

    assert_eq!(acquisition.retry_attempted, 4);
    assert_eq!(acquisition.retry_recovered, 0);
    assert_eq!(acquisition.failures.len(), 4);
    assert!(calls.lock().unwrap().values().all(|count| *count == 2));
}

#[tokio::test]
async fn terminal_snapshot_families_are_never_retried() {
    let repos = [
        "invalid-ref",
        "redirect",
        "denied",
        "not-found",
        "parse",
        "budget",
        "integrity",
    ]
    .into_iter()
    .map(|name| repo("owner", name))
    .collect::<Vec<_>>();
    let calls = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
    let downloader = {
        let calls = Arc::clone(&calls);
        move |repo: GitHubRepoRef| {
            let calls = Arc::clone(&calls);
            async move {
                *calls.lock().unwrap().entry(repo.repo.clone()).or_default() += 1;
                Err(match repo.repo.as_str() {
                    "invalid-ref" => GithubImportError::InvalidBranchSelection,
                    "redirect" => GithubImportError::ArchiveRedirectRejected,
                    "denied" => GithubImportError::AccessDenied("secret".to_string()),
                    "not-found" => GithubImportError::RepoNotFound,
                    "parse" => GithubImportError::Parse("response body".to_string()),
                    "budget" => GithubImportError::Budget(
                        crate::services::resource_budget::BudgetExceeded::new("archive", 2, 1),
                    ),
                    "integrity" => GithubImportError::PreviewSnapshotIntegrity,
                    _ => unreachable!(),
                })
            }
        }
    };

    let acquisition = prepare_snapshots_for_repo_refs_collecting_failures_with_downloader(
        &repos,
        &CentralUpdateSnapshotCache::default(),
        SnapshotCachePolicy::Bypass,
        None,
        SNAPSHOT_DOWNLOAD_CONCURRENCY,
        downloader,
    )
    .await
    .unwrap();

    assert_eq!(acquisition.retry_attempted, 0);
    assert_eq!(acquisition.retry_recovered, 0);
    assert_eq!(acquisition.failures.len(), 7);
    assert!(calls.lock().unwrap().values().all(|count| *count == 1));
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
