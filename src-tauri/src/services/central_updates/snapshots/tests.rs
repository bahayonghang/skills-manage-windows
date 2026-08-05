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

fn snapshot() -> GitHubRepoSnapshot {
    GitHubRepoSnapshot::default()
}

fn snapshot_with_bytes(bytes: &[u8]) -> Arc<GitHubRepoSnapshot> {
    Arc::new(GitHubRepoSnapshot {
        files: HashMap::from([("SKILL.md".to_string(), bytes.to_vec())]),
    })
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
