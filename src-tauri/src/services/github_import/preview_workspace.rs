use super::*;
pub(super) fn preview_workspace_registry() -> &'static Mutex<HashMap<String, GitHubPreviewWorkspace>>
{
    GITHUB_PREVIEW_WORKSPACES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn register_preview_workspace(workspace: GitHubPreviewWorkspace) {
    match preview_workspace_registry().lock() {
        Ok(mut registry) => {
            registry.insert(workspace.id.clone(), workspace);
        }
        Err(error) => {
            tracing::warn!(error = %error, "GitHub preview workspace registry lock is poisoned during register");
        }
    }
}

pub(super) fn get_preview_workspace(workspace_id: &str) -> Option<GitHubPreviewWorkspace> {
    match preview_workspace_registry().lock() {
        Ok(registry) => registry.get(workspace_id).cloned(),
        Err(error) => {
            tracing::warn!(workspace_id = %workspace_id, error = %error, "GitHub preview workspace registry lock is poisoned during lookup");
            None
        }
    }
}

pub(super) fn take_preview_workspace(workspace_id: &str) -> Option<GitHubPreviewWorkspace> {
    match preview_workspace_registry().lock() {
        Ok(mut registry) => registry.remove(workspace_id),
        Err(error) => {
            tracing::warn!(workspace_id = %workspace_id, error = %error, "GitHub preview workspace registry lock is poisoned during removal");
            None
        }
    }
}

pub(super) fn prune_expired_preview_workspaces(now: DateTime<Utc>) -> Vec<GitHubPreviewWorkspace> {
    let Ok(mut registry) = preview_workspace_registry().lock() else {
        tracing::warn!("GitHub preview workspace registry lock is poisoned during pruning");
        return Vec::new();
    };
    let expired_ids = registry
        .iter()
        .filter(|(_, workspace)| workspace.is_expired(now))
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    expired_ids
        .into_iter()
        .filter_map(|id| registry.remove(&id))
        .collect()
}
