use super::*;

#[derive(Debug)]
pub(in crate::services::github_import) enum RemoteReservationAttempt<'a> {
    Reserved(RemotePreviewReservation<'a>),
    CleanupRequired(Vec<CleanupTicket>),
    Capacity,
}

#[derive(Debug)]
pub(in crate::services::github_import) struct RemotePreviewReservation<'a> {
    pub(super) registry: &'a PreviewSnapshotRegistry,
    pub(super) preview_id: String,
    pub(super) generation: u64,
    pub(super) target_id: String,
    pub(super) target_kind: TargetKind,
    pub(super) active: bool,
}

impl RemotePreviewReservation<'_> {
    pub(in crate::services::github_import) fn preview_id(&self) -> &str {
        &self.preview_id
    }

    pub(in crate::services::github_import) fn claim_workspace(
        &mut self,
        workspace: &GitHubPreviewWorkspace,
    ) -> Result<(), GithubImportError> {
        let mut state = self.registry.lock_state("reservation workspace claim")?;
        let entry = state
            .entries
            .get_mut(&self.preview_id)
            .filter(|entry| {
                entry.generation == self.generation && entry.state == PreviewEntryState::Reserved
            })
            .ok_or(GithubImportError::PreviewCapacity)?;
        entry.cleanup_workspace = Some(workspace.clone());
        Ok(())
    }

    pub(in crate::services::github_import) fn fill(
        &mut self,
        snapshot: PreviewSnapshot,
    ) -> Result<(), GithubImportError> {
        if snapshot.id != self.preview_id
            || snapshot.target_id != self.target_id
            || snapshot.target_kind != self.target_kind
            || snapshot.remote_workspace().is_none()
        {
            return Err(GithubImportError::PreviewWorkspaceMismatch);
        }
        let mut state = self.registry.lock_state("reservation fill")?;
        let access_seq = next_access_seq(&mut state);
        let entry = state
            .entries
            .get_mut(&self.preview_id)
            .filter(|entry| {
                entry.generation == self.generation && entry.state == PreviewEntryState::Reserved
            })
            .ok_or(GithubImportError::PreviewCapacity)?;
        entry.snapshot = Some(Arc::new(snapshot));
        entry.cleanup_workspace = None;
        entry.state = PreviewEntryState::Ready;
        entry.last_access_seq = access_seq;
        self.active = false;
        Ok(())
    }

    pub(in crate::services::github_import) fn retain_cleanup_pending(
        &mut self,
        workspace: GitHubPreviewWorkspace,
    ) -> Result<CleanupTicket, GithubImportError> {
        let mut state = self.registry.lock_state("reservation cleanup retention")?;
        let entry = state
            .entries
            .get_mut(&self.preview_id)
            .filter(|entry| {
                entry.generation == self.generation && entry.state == PreviewEntryState::Reserved
            })
            .ok_or(GithubImportError::PreviewCapacity)?;
        entry.cleanup_workspace = Some(workspace);
        entry.state = PreviewEntryState::CleanupPending;
        let ticket = cleanup_ticket(entry, &self.preview_id)
            .ok_or(GithubImportError::PreviewCleanupPending)?;
        self.active = false;
        Ok(ticket)
    }

    pub(in crate::services::github_import) fn release_after_cleanup(&mut self) {
        if self
            .registry
            .cancel_reservation(&self.preview_id, self.generation)
        {
            self.active = false;
        }
    }
}

impl Drop for RemotePreviewReservation<'_> {
    fn drop(&mut self) {
        if self.active {
            self.registry
                .abandon_reservation(&self.preview_id, self.generation);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::services::github_import) struct CleanupTicket {
    pub(super) preview_id: String,
    pub(super) generation: u64,
    pub(super) target_id: String,
    pub(super) target_kind: TargetKind,
    pub(super) workspace: GitHubPreviewWorkspace,
}

impl CleanupTicket {
    pub(in crate::services::github_import) fn target_id(&self) -> &str {
        &self.target_id
    }

    pub(in crate::services::github_import) fn workspace_dir(&self) -> &str {
        &self.workspace.remote_workspace_dir
    }

    pub(in crate::services::github_import) fn target_kind(&self) -> TargetKind {
        self.target_kind
    }
}
