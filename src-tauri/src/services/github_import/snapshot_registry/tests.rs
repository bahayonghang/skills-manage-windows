use super::*;
use std::sync::{Arc as Shared, Barrier};

fn policy(max_ready: usize, max_local_bytes: u64, max_entries: usize) -> PreviewRegistryPolicy {
    PreviewRegistryPolicy {
        max_ready_per_target: max_ready,
        max_local_bytes_per_target: max_local_bytes,
        max_entries,
    }
}

fn repo() -> GitHubRepoRef {
    GitHubRepoRef {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        branch: "main".to_string(),
        normalized_url: "https://github.com/owner/repo".to_string(),
    }
}

fn local_snapshot(
    id: &str,
    target_id: &str,
    bytes: &[u8],
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> PreviewSnapshot {
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([("SKILL.md".to_string(), bytes.to_vec())]),
    };
    PreviewSnapshot {
        id: id.to_string(),
        target_id: target_id.to_string(),
        target_kind: TargetKind::Local,
        repo: repo(),
        source_path: None,
        resolved_commit_sha: "a".repeat(40),
        snapshot_digest: "sha256-v1:repository".to_string(),
        files: Vec::new(),
        candidates: Vec::new(),
        created_at,
        expires_at,
        storage: PreviewSnapshotStorage::Local(Arc::new(snapshot)),
    }
}

fn fill_remote(
    registry: &PreviewSnapshotRegistry,
    target_id: &str,
    target_kind: TargetKind,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> String {
    let mut reservation = match registry
        .reserve_remote(target_id, target_kind, created_at)
        .expect("remote reservation")
    {
        RemoteReservationAttempt::Reserved(reservation) => reservation,
        other => panic!("expected reservation, got {other:?}"),
    };
    let id = reservation.preview_id().to_string();
    let workspace = GitHubPreviewWorkspace {
        remote_workspace_dir: format!("/tmp/{target_id}"),
        remote_repo_dir: format!("/tmp/{target_id}/repo"),
    };
    reservation
        .fill(PreviewSnapshot {
            id: id.clone(),
            target_id: target_id.to_string(),
            target_kind,
            repo: repo(),
            source_path: None,
            resolved_commit_sha: "b".repeat(40),
            snapshot_digest: "sha256-v1:remote".to_string(),
            files: Vec::new(),
            candidates: Vec::new(),
            created_at,
            expires_at,
            storage: PreviewSnapshotStorage::Remote(workspace),
        })
        .expect("fill remote reservation");
    id
}

#[test]
fn per_target_ready_cap_uses_deterministic_lru() {
    let registry = PreviewSnapshotRegistry::new(policy(2, 100, 10));
    let now = Utc::now();
    for id in ["a", "b"] {
        registry
            .register_local(
                local_snapshot(id, "local", id.as_bytes(), now, now + Duration::minutes(1)),
                now,
            )
            .unwrap();
    }
    assert!(registry.lookup("a", now).is_ok());
    registry
        .register_local(
            local_snapshot("c", "local", b"c", now, now + Duration::minutes(1)),
            now,
        )
        .unwrap();

    assert!(registry.contains("a"));
    assert!(!registry.contains("b"));
    assert!(registry.contains("c"));
    assert_eq!(registry.metrics("local"), (2, 2, 2));
}

#[test]
fn local_retained_byte_cap_evicts_ready_entries() {
    let registry = PreviewSnapshotRegistry::new(policy(4, 5, 10));
    let now = Utc::now();
    registry
        .register_local(
            local_snapshot("a", "local", b"abc", now, now + Duration::minutes(1)),
            now,
        )
        .unwrap();
    registry
        .register_local(
            local_snapshot("b", "local", b"def", now, now + Duration::minutes(1)),
            now,
        )
        .unwrap();
    assert!(!registry.contains("a"));
    assert_eq!(registry.metrics("local"), (1, 1, 3));
}

#[test]
fn global_cap_rejects_when_only_foreign_target_entries_exist() {
    let registry = PreviewSnapshotRegistry::new(policy(4, 100, 2));
    let now = Utc::now();
    for (id, target) in [("a", "target-a"), ("b", "target-b")] {
        registry
            .register_local(
                local_snapshot(id, target, b"x", now, now + Duration::minutes(1)),
                now,
            )
            .unwrap();
    }
    assert!(matches!(
        registry.register_local(
            local_snapshot("c", "target-c", b"x", now, now + Duration::minutes(1)),
            now,
        ),
        Err(GithubImportError::PreviewCapacity)
    ));
    assert!(registry.contains("a"));
    assert!(registry.contains("b"));
}

#[test]
fn active_import_lease_is_never_an_eviction_victim() {
    let registry = PreviewSnapshotRegistry::new(policy(1, 100, 1));
    let now = Utc::now();
    registry
        .register_local(
            local_snapshot("leased", "local", b"x", now, now + Duration::minutes(1)),
            now,
        )
        .unwrap();
    registry.acquire_import_lease("leased", now).unwrap();
    assert!(matches!(
        registry.register_local(
            local_snapshot("new", "local", b"y", now, now + Duration::minutes(1)),
            now,
        ),
        Err(GithubImportError::PreviewCapacity)
    ));
    assert!(registry.contains("leased"));
}

#[test]
fn discard_during_import_is_deferred_until_release() {
    let registry = PreviewSnapshotRegistry::new(policy(2, 100, 2));
    let now = Utc::now();
    registry
        .register_local(
            local_snapshot("leased", "local", b"x", now, now + Duration::minutes(1)),
            now,
        )
        .unwrap();
    registry.acquire_import_lease("leased", now).unwrap();
    assert!(registry.discard_for_target("local", "leased").is_none());
    assert!(registry.contains("leased"));
    assert!(registry.release_import_lease("leased").is_none());
    assert!(!registry.contains("leased"));
}

#[test]
fn target_scoped_sweep_keeps_foreign_remote_ownership() {
    let registry = PreviewSnapshotRegistry::new(policy(4, 100, 4));
    let now = Utc::now();
    let target_a = fill_remote(
        &registry,
        "target-a",
        TargetKind::Ssh,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    let target_b = fill_remote(
        &registry,
        "target-b",
        TargetKind::Wsl,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );

    let tickets = registry.sweep_target("target-a", now);
    assert_eq!(tickets.len(), 1);
    assert_eq!(tickets[0].target_id(), "target-a");
    assert!(matches!(
        registry.lookup(&target_a, now),
        Err(GithubImportError::PreviewCleanupPending)
    ));
    assert!(registry.contains(&target_b));
    assert!(matches!(
        registry.lookup(&target_b, now),
        Err(GithubImportError::PreviewWorkspaceExpired)
    ));
}

#[test]
fn cleanup_failure_stays_pending_until_retry_ack() {
    let registry = PreviewSnapshotRegistry::new(policy(4, 100, 4));
    let now = Utc::now();
    let id = fill_remote(
        &registry,
        "target-a",
        TargetKind::Ssh,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    let first = registry.sweep_target("target-a", now);
    assert_eq!(first.len(), 1);
    assert!(matches!(
        registry.acquire_import_lease(&id, now),
        Err(GithubImportError::PreviewCleanupPending)
    ));

    let retry = registry.sweep_target("target-a", now);
    assert_eq!(retry, first);
    assert!(registry.ack_cleanup(&retry[0]));
    assert!(!registry.contains(&id));
}

#[test]
fn stale_cleanup_ack_cannot_delete_replacement() {
    let registry = PreviewSnapshotRegistry::new(policy(4, 100, 4));
    let now = Utc::now();
    let id = fill_remote(
        &registry,
        "target-a",
        TargetKind::Ssh,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    let ticket = registry.sweep_target("target-a", now).remove(0);
    assert!(registry.ack_cleanup(&ticket));
    registry
        .register_local(
            local_snapshot(
                &id,
                "local",
                b"replacement",
                now,
                now + Duration::minutes(1),
            ),
            now,
        )
        .unwrap();
    assert!(!registry.ack_cleanup(&ticket));
    assert!(registry.contains(&id));
}

#[test]
fn concurrent_remote_reservations_respect_global_admission() {
    let registry = Shared::new(PreviewSnapshotRegistry::new(policy(4, 100, 1)));
    let start = Shared::new(Barrier::new(3));
    let finish = Shared::new(Barrier::new(3));
    let outcomes = std::thread::scope(|scope| {
        let handles = (0..2)
            .map(|_| {
                let registry = Shared::clone(&registry);
                let start = Shared::clone(&start);
                let finish = Shared::clone(&finish);
                scope.spawn(move || {
                    start.wait();
                    let attempt = registry
                        .reserve_remote("target", TargetKind::Ssh, Utc::now())
                        .unwrap();
                    let reserved = matches!(&attempt, RemoteReservationAttempt::Reserved(_));
                    finish.wait();
                    drop(attempt);
                    reserved
                })
            })
            .collect::<Vec<_>>();
        start.wait();
        finish.wait();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(outcomes.iter().filter(|reserved| **reserved).count(), 1);
}

#[test]
fn failed_new_workspace_cleanup_stays_within_reserved_global_slot() {
    let registry = PreviewSnapshotRegistry::new(policy(1, 100, 1));
    let now = Utc::now();
    let mut reservation = match registry
        .reserve_remote("target", TargetKind::Ssh, now)
        .unwrap()
    {
        RemoteReservationAttempt::Reserved(reservation) => reservation,
        other => panic!("expected reservation, got {other:?}"),
    };
    let workspace = GitHubPreviewWorkspace {
        remote_workspace_dir: "/tmp/failed-new-workspace".to_string(),
        remote_repo_dir: "/tmp/failed-new-workspace/repo".to_string(),
    };
    let ticket = reservation
        .retain_cleanup_pending(workspace)
        .expect("retain failed workspace ownership");

    assert_eq!(registry.metrics("target"), (1, 0, 0));
    assert!(matches!(
        registry.reserve_remote("target", TargetKind::Ssh, now),
        Ok(RemoteReservationAttempt::CleanupRequired(_))
    ));
    assert!(registry.ack_cleanup(&ticket));
    assert!(matches!(
        registry.reserve_remote("target", TargetKind::Ssh, now),
        Ok(RemoteReservationAttempt::Reserved(_))
    ));
}

#[test]
fn cancelled_after_workspace_claim_retains_cleanup_ownership_in_reserved_slot() {
    let registry = PreviewSnapshotRegistry::new(policy(1, 100, 1));
    let now = Utc::now();
    let mut reservation = match registry
        .reserve_remote("target", TargetKind::Ssh, now)
        .unwrap()
    {
        RemoteReservationAttempt::Reserved(reservation) => reservation,
        other => panic!("expected reservation, got {other:?}"),
    };
    let preview_id = reservation.preview_id().to_string();
    let workspace = GitHubPreviewWorkspace {
        remote_workspace_dir: "/tmp/cancelled-workspace".to_string(),
        remote_repo_dir: "/tmp/cancelled-workspace/repo".to_string(),
    };
    reservation
        .claim_workspace(&workspace)
        .expect("claim remote workspace");

    drop(reservation);

    assert_eq!(registry.metrics("target"), (1, 0, 0));
    assert!(matches!(
        registry.lookup(&preview_id, now),
        Err(GithubImportError::PreviewCleanupPending)
    ));
    let tickets = match registry
        .reserve_remote("target", TargetKind::Ssh, now)
        .unwrap()
    {
        RemoteReservationAttempt::CleanupRequired(tickets) => tickets,
        other => panic!("expected cleanup ticket, got {other:?}"),
    };
    assert_eq!(tickets.len(), 1);
    assert_eq!(tickets[0].workspace_dir(), workspace.remote_workspace_dir);
}

#[test]
fn retained_byte_accounting_detects_checked_overflow() {
    assert!(matches!(
        checked_retained_bytes([u64::MAX, 1]),
        Err(GithubImportError::SnapshotSizeOverflow)
    ));
}
