fn preview_workspace_registry() -> &'static Mutex<HashMap<String, GitHubPreviewWorkspace>> {
    GITHUB_PREVIEW_WORKSPACES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_preview_workspace(workspace: GitHubPreviewWorkspace) {
    if let Ok(mut registry) = preview_workspace_registry().lock() {
        registry.insert(workspace.id.clone(), workspace);
    }
}

fn get_preview_workspace(workspace_id: &str) -> Option<GitHubPreviewWorkspace> {
    preview_workspace_registry()
        .lock()
        .ok()
        .and_then(|registry| registry.get(workspace_id).cloned())
}

fn take_preview_workspace(workspace_id: &str) -> Option<GitHubPreviewWorkspace> {
    preview_workspace_registry()
        .lock()
        .ok()
        .and_then(|mut registry| registry.remove(workspace_id))
}

fn prune_expired_preview_workspaces(now: DateTime<Utc>) -> Vec<GitHubPreviewWorkspace> {
    let Ok(mut registry) = preview_workspace_registry().lock() else {
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
