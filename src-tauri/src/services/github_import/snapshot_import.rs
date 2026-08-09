//! Import driven by an immutable preview snapshot.
//!
//! This is the only import path the GitHub import wizard uses. The registered
//! snapshot is the sole content authority: the submitted repository URL is
//! binding evidence, never a content locator, so a branch that moved after
//! preview cannot change what is written to Central.

use super::*;

/// Import the skills confirmed in a registered preview snapshot.
///
/// The snapshot is the sole content authority: `repo_url` is only compared with
/// the snapshot's binding, and no branch is re-resolved or re-downloaded. A
/// single import lease guards the snapshot, so two concurrent confirmations
/// cannot both mutate Central. Failure releases the lease so the same preview
/// can be retried; success consumes the token atomically.
#[cfg(test)]
pub(crate) async fn import_github_repo_skills_from_preview(
    pool: &DbPool,
    active_target: &ActiveTarget,
    preview_id: &str,
    repo_url: &str,
    selections: Vec<GitHubSkillImportSelection>,
    app: Option<&AppHandle>,
) -> Result<GitHubRepoImportResult, GithubImportError> {
    import_github_repo_skills_from_preview_with_branch(
        pool,
        active_target,
        preview_id,
        repo_url,
        None,
        selections,
        app,
    )
    .await
}

pub(crate) async fn import_github_repo_skills_from_preview_with_branch(
    pool: &DbPool,
    active_target: &ActiveTarget,
    preview_id: &str,
    repo_url: &str,
    branch: Option<&str>,
    selections: Vec<GitHubSkillImportSelection>,
    app: Option<&AppHandle>,
) -> Result<GitHubRepoImportResult, GithubImportError> {
    let snapshot = match acquire_import_lease(preview_id, Utc::now()) {
        Ok(snapshot) => snapshot,
        Err(error @ GithubImportError::PreviewCleanupPending) => {
            retry_pending_preview_cleanup_for_target(active_target).await;
            return Err(error);
        }
        Err(error) => return Err(error),
    };
    let outcome = import_from_preview_snapshot(
        pool,
        active_target,
        &snapshot,
        repo_url,
        branch,
        selections,
        app,
    )
    .await;

    match outcome {
        Ok(result) => {
            if let Some(ticket) = consume_preview_snapshot(preview_id) {
                let _ = cleanup_preview_ticket_for_target(active_target, ticket).await;
            }
            Ok(result)
        }
        Err(error) => {
            if let Some(ticket) = release_import_lease(preview_id) {
                let _ = cleanup_preview_ticket_for_target(active_target, ticket).await;
            }
            Err(error)
        }
    }
}

async fn import_from_preview_snapshot(
    pool: &DbPool,
    active_target: &ActiveTarget,
    snapshot: &PreviewSnapshot,
    repo_url: &str,
    branch: Option<&str>,
    selections: Vec<GitHubSkillImportSelection>,
    app: Option<&AppHandle>,
) -> Result<GitHubRepoImportResult, GithubImportError> {
    if selections.is_empty() {
        return Err(GithubImportError::NoSelections);
    }
    validate_snapshot_binding_with_branch(snapshot, active_target, repo_url, branch)?;
    for selection in &selections {
        if snapshot.candidate(&selection.source_path).is_none() {
            return Err(GithubImportError::SelectionUnavailable(
                selection.source_path.clone(),
            ));
        }
    }
    verify_snapshot_integrity(active_target, snapshot).await?;

    let provenance = ImportProvenance::from_snapshot(snapshot);
    match &snapshot.storage {
        PreviewSnapshotStorage::Local(local) => {
            let candidates = build_repo_skill_candidates_from_snapshot_at_path(
                &snapshot.repo,
                local,
                snapshot.source_path.as_deref(),
            )?;
            if candidates.is_empty() {
                return Err(GithubImportError::NoImportableSkills);
            }
            let central_root = central_skills_root(pool).await?;
            std::fs::create_dir_all(&central_root).map_err(|e| {
                GithubImportError::io("Failed to create central skills directory", e)
            })?;
            import_github_repo_skills_from_snapshot(
                pool,
                &snapshot.repo,
                local,
                &candidates,
                selections,
                &central_root,
                Some(&provenance),
                app,
            )
            .await
        }
        PreviewSnapshotStorage::Remote(workspace) => {
            import_github_repo_skills_remote_from_workspace(
                pool,
                active_target,
                &snapshot.repo,
                snapshot.source_path.as_deref(),
                workspace,
                selections,
                Some(&provenance),
                app,
            )
            .await
        }
    }
}
