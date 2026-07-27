//! Session-scoped registry of immutable GitHub preview snapshots.
//!
//! The registry is the only authority that can resolve preview bytes. A
//! registered snapshot is bound to one target, repository, source root, and
//! resolved commit; it expires after [`REMOTE_PREVIEW_WORKSPACE_TTL_MINUTES`]
//! and is never recovered across application restarts.
//!
//! Lifecycle:
//!
//! ```text
//! register -> Ready --(acquire_import_lease)--> Importing
//!   Ready --(consume/discard/expiry)--> removed
//!   Importing --(release, failure)--> Ready (retry allowed)
//!   Importing --(consume, success)--> removed
//!   Importing --(discard)--> discard deferred, applied on release
//! ```

use super::*;

fn registry() -> &'static Mutex<HashMap<String, PreviewSnapshotEntry>> {
    GITHUB_PREVIEW_SNAPSHOTS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Remove expired entries and return them so the caller can release remote
/// storage. Entries with an active import lease are never pruned mid-flight.
pub(super) fn prune_expired_preview_snapshots(
    now: DateTime<Utc>,
) -> Vec<std::sync::Arc<PreviewSnapshot>> {
    let Ok(mut entries) = registry().lock() else {
        tracing::warn!("GitHub preview snapshot registry lock is poisoned during pruning");
        return Vec::new();
    };
    let expired_ids = entries
        .iter()
        .filter(|(_, entry)| !entry.importing && entry.snapshot.is_expired(now))
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    expired_ids
        .into_iter()
        .filter_map(|id| entries.remove(&id).map(|entry| entry.snapshot))
        .collect()
}

pub(super) fn register_preview_snapshot(snapshot: PreviewSnapshot) {
    let entry = PreviewSnapshotEntry {
        snapshot: std::sync::Arc::new(snapshot),
        importing: false,
        discard_pending: false,
    };
    match registry().lock() {
        Ok(mut entries) => {
            entries.insert(entry.snapshot.id.clone(), entry);
        }
        Err(error) => {
            tracing::warn!(
                error = %error,
                "GitHub preview snapshot registry lock is poisoned during register"
            );
        }
    }
}

/// Resolve a snapshot for a read-only operation. Expiry is enforced here so a
/// stale token can never reach storage.
pub(super) fn lookup_preview_snapshot(
    preview_id: &str,
    now: DateTime<Utc>,
) -> Result<std::sync::Arc<PreviewSnapshot>, GithubImportError> {
    let Ok(entries) = registry().lock() else {
        tracing::warn!("GitHub preview snapshot registry lock is poisoned during lookup");
        return Err(GithubImportError::PreviewSnapshotMissing);
    };
    let entry = entries
        .get(preview_id)
        .ok_or(GithubImportError::PreviewSnapshotMissing)?;
    if entry.snapshot.is_expired(now) {
        return Err(GithubImportError::PreviewWorkspaceExpired);
    }
    Ok(entry.snapshot.clone())
}

/// Take the single import lease for a snapshot. A second concurrent import for
/// the same preview fails closed instead of mutating Central twice.
pub(super) fn acquire_import_lease(
    preview_id: &str,
    now: DateTime<Utc>,
) -> Result<std::sync::Arc<PreviewSnapshot>, GithubImportError> {
    let Ok(mut entries) = registry().lock() else {
        tracing::warn!("GitHub preview snapshot registry lock is poisoned during import lease");
        return Err(GithubImportError::PreviewSnapshotMissing);
    };
    let entry = entries
        .get_mut(preview_id)
        .ok_or(GithubImportError::PreviewSnapshotMissing)?;
    if entry.snapshot.is_expired(now) {
        return Err(GithubImportError::PreviewWorkspaceExpired);
    }
    if entry.importing {
        return Err(GithubImportError::PreviewSnapshotBusy);
    }
    entry.importing = true;
    Ok(entry.snapshot.clone())
}

/// Release a failed import lease. Returns the snapshot when a discard was
/// requested while the lease was held, so the caller can clean up storage.
pub(super) fn release_import_lease(preview_id: &str) -> Option<std::sync::Arc<PreviewSnapshot>> {
    let Ok(mut entries) = registry().lock() else {
        tracing::warn!("GitHub preview snapshot registry lock is poisoned during lease release");
        return None;
    };
    let entry = entries.get_mut(preview_id)?;
    entry.importing = false;
    if !entry.discard_pending {
        return None;
    }
    entries.remove(preview_id).map(|entry| entry.snapshot)
}

/// Atomically invalidate a snapshot after a successful import. The token can
/// never be reused for another read or import.
pub(super) fn consume_preview_snapshot(
    preview_id: &str,
) -> Option<std::sync::Arc<PreviewSnapshot>> {
    let Ok(mut entries) = registry().lock() else {
        tracing::warn!("GitHub preview snapshot registry lock is poisoned during consume");
        return None;
    };
    entries.remove(preview_id).map(|entry| entry.snapshot)
}

/// Explicit renderer-driven discard. When an import lease is active the
/// removal is deferred to [`release_import_lease`].
pub(super) fn discard_preview_snapshot(
    preview_id: &str,
) -> Option<std::sync::Arc<PreviewSnapshot>> {
    let Ok(mut entries) = registry().lock() else {
        tracing::warn!("GitHub preview snapshot registry lock is poisoned during discard");
        return None;
    };
    let entry = entries.get_mut(preview_id)?;
    if entry.importing {
        entry.discard_pending = true;
        return None;
    }
    entries.remove(preview_id).map(|entry| entry.snapshot)
}

#[cfg(test)]
pub(super) fn preview_snapshot_is_registered(preview_id: &str) -> bool {
    registry()
        .lock()
        .map(|entries| entries.contains_key(preview_id))
        .unwrap_or(false)
}
