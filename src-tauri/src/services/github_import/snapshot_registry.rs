//! Bounded, session-scoped registry of immutable GitHub preview snapshots.
//!
//! Registry transitions are synchronous and never perform remote IO. Remote
//! storage leaves the registry only after an owning-target cleanup ticket is
//! acknowledged with the generation that produced it.

use super::*;

const DEFAULT_MAX_READY_PER_TARGET: usize = 4;
const DEFAULT_MAX_LOCAL_BYTES_PER_TARGET: u64 = 256 * 1024 * 1024;
const DEFAULT_MAX_ENTRIES: usize = 64;

static GITHUB_PREVIEW_REGISTRY: OnceLock<PreviewSnapshotRegistry> = OnceLock::new();

fn registry() -> &'static PreviewSnapshotRegistry {
    GITHUB_PREVIEW_REGISTRY.get_or_init(|| {
        #[cfg(test)]
        {
            PreviewSnapshotRegistry::new(PreviewRegistryPolicy {
                max_ready_per_target: 1_024,
                max_local_bytes_per_target: u64::MAX,
                max_entries: 4_096,
            })
        }
        #[cfg(not(test))]
        PreviewSnapshotRegistry::default()
    })
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PreviewRegistryPolicy {
    max_ready_per_target: usize,
    max_local_bytes_per_target: u64,
    max_entries: usize,
}

impl Default for PreviewRegistryPolicy {
    fn default() -> Self {
        Self {
            max_ready_per_target: DEFAULT_MAX_READY_PER_TARGET,
            max_local_bytes_per_target: DEFAULT_MAX_LOCAL_BYTES_PER_TARGET,
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewEntryState {
    Reserved,
    Ready,
    Importing { discard_pending: bool },
    CleanupPending,
}

#[derive(Debug)]
struct PreviewRegistryEntry {
    generation: u64,
    target_id: String,
    target_kind: TargetKind,
    retained_bytes: u64,
    last_access_seq: u64,
    state: PreviewEntryState,
    snapshot: Option<Arc<PreviewSnapshot>>,
    cleanup_workspace: Option<GitHubPreviewWorkspace>,
}

#[derive(Debug, Default)]
struct PreviewRegistryState {
    entries: HashMap<String, PreviewRegistryEntry>,
    next_generation: u64,
    next_access_seq: u64,
}

#[derive(Debug)]
pub(super) struct PreviewSnapshotRegistry {
    policy: PreviewRegistryPolicy,
    state: Mutex<PreviewRegistryState>,
}

impl Default for PreviewSnapshotRegistry {
    fn default() -> Self {
        Self::new(PreviewRegistryPolicy::default())
    }
}

impl PreviewSnapshotRegistry {
    pub(super) fn new(policy: PreviewRegistryPolicy) -> Self {
        Self {
            policy,
            state: Mutex::new(PreviewRegistryState::default()),
        }
    }

    fn register_local(
        &self,
        snapshot: PreviewSnapshot,
        now: DateTime<Utc>,
    ) -> Result<(), GithubImportError> {
        let retained_bytes = snapshot_retained_bytes(&snapshot)?;
        if retained_bytes > self.policy.max_local_bytes_per_target
            || self.policy.max_ready_per_target == 0
            || self.policy.max_entries == 0
        {
            return Err(GithubImportError::PreviewCapacity);
        }

        let mut state = self.lock_state("local register")?;
        remove_expired_local_entries(&mut state, &snapshot.target_id, now);
        if state.entries.contains_key(&snapshot.id) {
            return Err(GithubImportError::PreviewCapacity);
        }

        while target_ready_admission_count(&state, &snapshot.target_id)
            >= self.policy.max_ready_per_target
            || target_local_retained_bytes(&state, &snapshot.target_id)
                .checked_add(retained_bytes)
                .is_none_or(|total| total > self.policy.max_local_bytes_per_target)
            || state.entries.len() >= self.policy.max_entries
        {
            let Some(victim_id) = oldest_ready_entry_id(&state, &snapshot.target_id) else {
                return Err(GithubImportError::PreviewCapacity);
            };
            state.entries.remove(&victim_id);
        }

        let generation = next_generation(&mut state);
        let last_access_seq = next_access_seq(&mut state);
        state.entries.insert(
            snapshot.id.clone(),
            PreviewRegistryEntry {
                generation,
                target_id: snapshot.target_id.clone(),
                target_kind: snapshot.target_kind,
                retained_bytes,
                last_access_seq,
                state: PreviewEntryState::Ready,
                snapshot: Some(Arc::new(snapshot)),
                cleanup_workspace: None,
            },
        );
        trace_registry_state(&state, TargetKind::Local, retained_bytes, "register");
        Ok(())
    }

    fn reserve_remote(
        &self,
        target_id: &str,
        target_kind: TargetKind,
        now: DateTime<Utc>,
    ) -> Result<RemoteReservationAttempt<'_>, GithubImportError> {
        debug_assert!(target_kind != TargetKind::Local);
        let mut state = self.lock_state("remote reservation")?;

        let tickets = transition_expired_for_target(&mut state, target_id, now);
        if !tickets.is_empty() {
            return Ok(RemoteReservationAttempt::CleanupRequired(tickets));
        }
        let pending = cleanup_tickets_for_target(&state, target_id);
        if !pending.is_empty() {
            return Ok(RemoteReservationAttempt::CleanupRequired(pending));
        }

        let needs_target_slot =
            target_ready_admission_count(&state, target_id) >= self.policy.max_ready_per_target;
        let needs_global_slot = state.entries.len() >= self.policy.max_entries;
        if needs_target_slot || needs_global_slot {
            let Some(victim_id) = oldest_ready_entry_id(&state, target_id) else {
                return Ok(RemoteReservationAttempt::Capacity);
            };
            let ticket = transition_remote_entry_to_cleanup(&mut state, &victim_id)
                .ok_or(GithubImportError::PreviewCapacity)?;
            return Ok(RemoteReservationAttempt::CleanupRequired(vec![ticket]));
        }

        let preview_id = new_preview_id();
        let generation = next_generation(&mut state);
        let last_access_seq = next_access_seq(&mut state);
        state.entries.insert(
            preview_id.clone(),
            PreviewRegistryEntry {
                generation,
                target_id: target_id.to_string(),
                target_kind,
                retained_bytes: 0,
                last_access_seq,
                state: PreviewEntryState::Reserved,
                snapshot: None,
                cleanup_workspace: None,
            },
        );
        trace_registry_state(&state, target_kind, 0, "reserve");
        Ok(RemoteReservationAttempt::Reserved(
            RemotePreviewReservation {
                registry: self,
                preview_id,
                generation,
                target_id: target_id.to_string(),
                target_kind,
                active: true,
            },
        ))
    }

    fn lookup(
        &self,
        preview_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Arc<PreviewSnapshot>, GithubImportError> {
        let mut state = self.lock_state("lookup")?;
        let access_seq = next_access_seq(&mut state);
        let entry = state
            .entries
            .get_mut(preview_id)
            .ok_or(GithubImportError::PreviewSnapshotMissing)?;
        match entry.state {
            PreviewEntryState::CleanupPending => {
                return Err(GithubImportError::PreviewCleanupPending)
            }
            PreviewEntryState::Reserved => return Err(GithubImportError::PreviewSnapshotMissing),
            PreviewEntryState::Ready | PreviewEntryState::Importing { .. } => {}
        }
        let snapshot = entry
            .snapshot
            .as_ref()
            .ok_or(GithubImportError::PreviewSnapshotMissing)?;
        if snapshot.is_expired(now) {
            return Err(GithubImportError::PreviewWorkspaceExpired);
        }
        entry.last_access_seq = access_seq;
        Ok(Arc::clone(snapshot))
    }

    fn acquire_import_lease(
        &self,
        preview_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Arc<PreviewSnapshot>, GithubImportError> {
        let mut state = self.lock_state("import lease")?;
        let access_seq = next_access_seq(&mut state);
        let entry = state
            .entries
            .get_mut(preview_id)
            .ok_or(GithubImportError::PreviewSnapshotMissing)?;
        match entry.state {
            PreviewEntryState::CleanupPending => {
                return Err(GithubImportError::PreviewCleanupPending)
            }
            PreviewEntryState::Reserved => return Err(GithubImportError::PreviewSnapshotMissing),
            PreviewEntryState::Importing { .. } => {
                return Err(GithubImportError::PreviewSnapshotBusy)
            }
            PreviewEntryState::Ready => {}
        }
        let snapshot = entry
            .snapshot
            .as_ref()
            .ok_or(GithubImportError::PreviewSnapshotMissing)?;
        if snapshot.is_expired(now) {
            return Err(GithubImportError::PreviewWorkspaceExpired);
        }
        let snapshot = Arc::clone(snapshot);
        entry.state = PreviewEntryState::Importing {
            discard_pending: false,
        };
        entry.last_access_seq = access_seq;
        Ok(snapshot)
    }

    fn release_import_lease(&self, preview_id: &str) -> Option<CleanupTicket> {
        let Ok(mut state) = self.state.lock() else {
            tracing::warn!(
                "GitHub preview snapshot registry lock is poisoned during lease release"
            );
            return None;
        };
        let discard_pending = matches!(
            state.entries.get(preview_id).map(|entry| entry.state),
            Some(PreviewEntryState::Importing {
                discard_pending: true
            })
        );
        if !discard_pending {
            let access_seq = next_access_seq(&mut state);
            let entry = state.entries.get_mut(preview_id)?;
            if !matches!(entry.state, PreviewEntryState::Importing { .. }) {
                return None;
            }
            entry.state = PreviewEntryState::Ready;
            entry.last_access_seq = access_seq;
            return None;
        }
        transition_entry_out_of_registry(&mut state, preview_id)
    }

    fn consume(&self, preview_id: &str) -> Option<CleanupTicket> {
        let Ok(mut state) = self.state.lock() else {
            tracing::warn!("GitHub preview snapshot registry lock is poisoned during consume");
            return None;
        };
        if !matches!(
            state.entries.get(preview_id).map(|entry| entry.state),
            Some(PreviewEntryState::Importing { .. })
        ) {
            return None;
        }
        transition_entry_out_of_registry(&mut state, preview_id)
    }

    fn discard_for_target(&self, target_id: &str, preview_id: &str) -> Option<CleanupTicket> {
        let Ok(mut state) = self.state.lock() else {
            tracing::warn!("GitHub preview snapshot registry lock is poisoned during discard");
            return None;
        };
        let entry = state.entries.get_mut(preview_id)?;
        if entry.target_id != target_id {
            return None;
        }
        match entry.state {
            PreviewEntryState::Importing { .. } => {
                entry.state = PreviewEntryState::Importing {
                    discard_pending: true,
                };
                None
            }
            PreviewEntryState::Ready => transition_entry_out_of_registry(&mut state, preview_id),
            PreviewEntryState::CleanupPending => cleanup_ticket(entry, preview_id),
            PreviewEntryState::Reserved => None,
        }
    }

    fn sweep_target(&self, target_id: &str, now: DateTime<Utc>) -> Vec<CleanupTicket> {
        let Ok(mut state) = self.state.lock() else {
            tracing::warn!("GitHub preview snapshot registry lock is poisoned during target sweep");
            return Vec::new();
        };
        let mut tickets = transition_expired_for_target(&mut state, target_id, now);
        let existing = cleanup_tickets_for_target(&state, target_id);
        for ticket in existing {
            if !tickets.iter().any(|current| {
                current.preview_id == ticket.preview_id && current.generation == ticket.generation
            }) {
                tickets.push(ticket);
            }
        }
        tickets
    }

    fn ack_cleanup(&self, ticket: &CleanupTicket) -> bool {
        let Ok(mut state) = self.state.lock() else {
            tracing::warn!(
                "GitHub preview snapshot registry lock is poisoned during cleanup acknowledgement"
            );
            return false;
        };
        let matches = state.entries.get(&ticket.preview_id).is_some_and(|entry| {
            entry.generation == ticket.generation
                && entry.state == PreviewEntryState::CleanupPending
        });
        if matches {
            state.entries.remove(&ticket.preview_id);
            trace_registry_state(&state, ticket.target_kind, 0, "cleanup_ack");
        }
        matches
    }

    fn cancel_reservation(&self, preview_id: &str, generation: u64) -> bool {
        let Ok(mut state) = self.state.lock() else {
            tracing::warn!(
                "GitHub preview snapshot registry lock is poisoned during reservation cancellation"
            );
            return false;
        };
        let matches = state.entries.get(preview_id).is_some_and(|entry| {
            entry.generation == generation && entry.state == PreviewEntryState::Reserved
        });
        if matches {
            state.entries.remove(preview_id);
        }
        matches
    }

    fn abandon_reservation(&self, preview_id: &str, generation: u64) {
        let Ok(mut state) = self.state.lock() else {
            tracing::warn!(
                "GitHub preview snapshot registry lock is poisoned during reservation abandonment"
            );
            return;
        };
        let Some(entry) = state.entries.get_mut(preview_id).filter(|entry| {
            entry.generation == generation && entry.state == PreviewEntryState::Reserved
        }) else {
            return;
        };
        if entry.cleanup_workspace.is_some() {
            entry.state = PreviewEntryState::CleanupPending;
        } else {
            state.entries.remove(preview_id);
        }
    }

    fn lock_state(
        &self,
        operation: &'static str,
    ) -> Result<std::sync::MutexGuard<'_, PreviewRegistryState>, GithubImportError> {
        self.state.lock().map_err(|_error| {
            tracing::warn!(
                operation,
                "GitHub preview snapshot registry lock is poisoned"
            );
            GithubImportError::PreviewSnapshotMissing
        })
    }

    #[cfg(test)]
    fn contains(&self, preview_id: &str) -> bool {
        self.state
            .lock()
            .map(|state| state.entries.contains_key(preview_id))
            .unwrap_or(false)
    }

    #[cfg(test)]
    fn metrics(&self, target_id: &str) -> (usize, usize, u64) {
        let state = self.state.lock().expect("preview registry state");
        (
            state.entries.len(),
            target_ready_admission_count(&state, target_id),
            target_local_retained_bytes(&state, target_id),
        )
    }
}

mod reservation;
pub(super) use reservation::*;

pub(super) fn register_preview_snapshot(
    snapshot: PreviewSnapshot,
) -> Result<(), GithubImportError> {
    registry().register_local(snapshot, Utc::now())
}

pub(super) fn reserve_remote_preview_snapshot(
    target_id: &str,
    target_kind: TargetKind,
    now: DateTime<Utc>,
) -> Result<RemoteReservationAttempt<'static>, GithubImportError> {
    registry().reserve_remote(target_id, target_kind, now)
}

pub(super) fn lookup_preview_snapshot(
    preview_id: &str,
    now: DateTime<Utc>,
) -> Result<Arc<PreviewSnapshot>, GithubImportError> {
    registry().lookup(preview_id, now)
}

pub(super) fn acquire_import_lease(
    preview_id: &str,
    now: DateTime<Utc>,
) -> Result<Arc<PreviewSnapshot>, GithubImportError> {
    registry().acquire_import_lease(preview_id, now)
}

pub(super) fn release_import_lease(preview_id: &str) -> Option<CleanupTicket> {
    registry().release_import_lease(preview_id)
}

pub(super) fn consume_preview_snapshot(preview_id: &str) -> Option<CleanupTicket> {
    registry().consume(preview_id)
}

pub(super) fn discard_preview_snapshot_for_target_transition(
    target_id: &str,
    preview_id: &str,
) -> Option<CleanupTicket> {
    registry().discard_for_target(target_id, preview_id)
}

pub(super) fn sweep_preview_snapshots_for_target(
    target_id: &str,
    now: DateTime<Utc>,
) -> Vec<CleanupTicket> {
    registry().sweep_target(target_id, now)
}

pub(super) fn ack_preview_snapshot_cleanup(ticket: &CleanupTicket) -> bool {
    registry().ack_cleanup(ticket)
}

fn snapshot_retained_bytes(snapshot: &PreviewSnapshot) -> Result<u64, GithubImportError> {
    match &snapshot.storage {
        PreviewSnapshotStorage::Local(local) => local.retained_bytes(),
        PreviewSnapshotStorage::Remote(_) => Ok(0),
    }
}

fn remove_expired_local_entries(
    state: &mut PreviewRegistryState,
    target_id: &str,
    now: DateTime<Utc>,
) {
    let expired = state
        .entries
        .iter()
        .filter(|(_, entry)| {
            entry.target_id == target_id
                && entry.target_kind == TargetKind::Local
                && entry.state == PreviewEntryState::Ready
                && entry
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.is_expired(now))
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    for id in expired {
        state.entries.remove(&id);
    }
}

fn transition_expired_for_target(
    state: &mut PreviewRegistryState,
    target_id: &str,
    now: DateTime<Utc>,
) -> Vec<CleanupTicket> {
    let expired = state
        .entries
        .iter()
        .filter(|(_, entry)| {
            entry.target_id == target_id
                && entry.state == PreviewEntryState::Ready
                && entry
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.is_expired(now))
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let mut tickets = Vec::new();
    for id in expired {
        if state
            .entries
            .get(&id)
            .is_some_and(|entry| entry.target_kind == TargetKind::Local)
        {
            state.entries.remove(&id);
        } else if let Some(ticket) = transition_remote_entry_to_cleanup(state, &id) {
            tickets.push(ticket);
        }
    }
    tickets
}

fn transition_entry_out_of_registry(
    state: &mut PreviewRegistryState,
    preview_id: &str,
) -> Option<CleanupTicket> {
    if state
        .entries
        .get(preview_id)
        .is_some_and(|entry| entry.target_kind == TargetKind::Local)
    {
        state.entries.remove(preview_id);
        return None;
    }
    transition_remote_entry_to_cleanup(state, preview_id)
}

fn transition_remote_entry_to_cleanup(
    state: &mut PreviewRegistryState,
    preview_id: &str,
) -> Option<CleanupTicket> {
    let entry = state.entries.get_mut(preview_id)?;
    let workspace = entry
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.remote_workspace().cloned())
        .or_else(|| entry.cleanup_workspace.clone())?;
    entry.state = PreviewEntryState::CleanupPending;
    entry.cleanup_workspace = Some(workspace);
    cleanup_ticket(entry, preview_id)
}

fn cleanup_ticket(entry: &PreviewRegistryEntry, preview_id: &str) -> Option<CleanupTicket> {
    let workspace = entry
        .cleanup_workspace
        .clone()
        .or_else(|| entry.snapshot.as_ref()?.remote_workspace().cloned())?;
    Some(CleanupTicket {
        preview_id: preview_id.to_string(),
        generation: entry.generation,
        target_id: entry.target_id.clone(),
        target_kind: entry.target_kind,
        workspace,
    })
}

fn cleanup_tickets_for_target(state: &PreviewRegistryState, target_id: &str) -> Vec<CleanupTicket> {
    state
        .entries
        .iter()
        .filter(|(_, entry)| {
            entry.target_id == target_id && entry.state == PreviewEntryState::CleanupPending
        })
        .filter_map(|(id, entry)| cleanup_ticket(entry, id))
        .collect()
}

fn oldest_ready_entry_id(state: &PreviewRegistryState, target_id: &str) -> Option<String> {
    state
        .entries
        .iter()
        .filter(|(_, entry)| {
            entry.target_id == target_id && entry.state == PreviewEntryState::Ready
        })
        .min_by(|(left_id, left), (right_id, right)| {
            left.last_access_seq
                .cmp(&right.last_access_seq)
                .then_with(|| left_id.cmp(right_id))
        })
        .map(|(id, _)| id.clone())
}

fn target_ready_admission_count(state: &PreviewRegistryState, target_id: &str) -> usize {
    state
        .entries
        .values()
        .filter(|entry| {
            entry.target_id == target_id
                && matches!(
                    entry.state,
                    PreviewEntryState::Reserved | PreviewEntryState::Ready
                )
        })
        .count()
}

fn target_local_retained_bytes(state: &PreviewRegistryState, target_id: &str) -> u64 {
    state
        .entries
        .values()
        .filter(|entry| {
            entry.target_id == target_id
                && entry.target_kind == TargetKind::Local
                && matches!(
                    entry.state,
                    PreviewEntryState::Ready | PreviewEntryState::Importing { .. }
                )
        })
        .fold(0_u64, |total, entry| {
            total.saturating_add(entry.retained_bytes)
        })
}

fn next_generation(state: &mut PreviewRegistryState) -> u64 {
    let generation = state.next_generation;
    state.next_generation = state.next_generation.wrapping_add(1);
    generation
}

fn next_access_seq(state: &mut PreviewRegistryState) -> u64 {
    let access_seq = state.next_access_seq;
    state.next_access_seq = state.next_access_seq.wrapping_add(1);
    access_seq
}

fn trace_registry_state(
    state: &PreviewRegistryState,
    target_kind: TargetKind,
    retained_bytes: u64,
    reason: &'static str,
) {
    tracing::debug!(
        entries = state.entries.len(),
        retained_bytes,
        reason,
        target_kind = ?target_kind,
        "GitHub preview snapshot registry state changed"
    );
}

#[cfg(test)]
pub(super) fn preview_snapshot_is_registered(preview_id: &str) -> bool {
    registry().contains(preview_id)
}

#[cfg(test)]
pub(super) fn discard_preview_snapshot(preview_id: &str) -> Option<()> {
    let (target_id, importing) = {
        let state = registry().state.lock().ok()?;
        let entry = state.entries.get(preview_id)?;
        (
            entry.target_id.clone(),
            matches!(entry.state, PreviewEntryState::Importing { .. }),
        )
    };
    let existed = registry().contains(preview_id);
    let _ = registry().discard_for_target(&target_id, preview_id);
    if existed && !importing {
        Some(())
    } else {
        None
    }
}

#[cfg(test)]
pub(super) fn prune_expired_preview_snapshots_for_target(
    target_id: &str,
    now: DateTime<Utc>,
) -> Vec<Arc<PreviewSnapshot>> {
    let Ok(mut state) = registry().state.lock() else {
        return Vec::new();
    };
    let expired = state
        .entries
        .iter()
        .filter(|(_, entry)| {
            entry.target_id == target_id
                && entry.state == PreviewEntryState::Ready
                && entry
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.is_expired(now))
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    expired
        .into_iter()
        .filter_map(|id| state.entries.remove(&id)?.snapshot)
        .collect()
}

#[cfg(test)]
#[path = "snapshot_registry/tests.rs"]
mod tests;
