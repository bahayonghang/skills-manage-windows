use super::*;
#[allow(dead_code)]
pub(crate) async fn preview_github_repo_import_impl(
    pool: &DbPool,
    secrets: &dyn crate::secrets::SecretStore,
    repo_url: &str,
) -> Result<GitHubRepoPreview, String> {
    let auth = github_direct_auth_from_secret_store(pool, secrets).await?;
    preview_github_repo_import_with_auth(pool, repo_url, auth.as_deref()).await
}

pub(crate) async fn preview_github_repo_import_with_auth(
    pool: &DbPool,
    repo_url: &str,
    auth: Option<&str>,
) -> Result<GitHubRepoPreview, String> {
    let resolved = resolve_repo_source(repo_url, auth).await?;
    let candidates = fetch_repo_skill_candidates_from_source(
        &resolved.repo,
        resolved.source_path.as_deref(),
        auth,
    )
    .await?;
    let skills = build_preview_skills(pool, &candidates).await?;

    if skills.is_empty() {
        return Err(NO_IMPORTABLE_SKILLS_ERROR.to_string());
    }

    Ok(GitHubRepoPreview {
        repo: resolved.repo,
        skills,
        preview_workspace_id: None,
    })
}

pub(crate) async fn preview_github_repo_import_remote_with_auth(
    pool: &DbPool,
    active_target: &ActiveTarget,
    repo_url: &str,
    auth: Option<&str>,
) -> Result<GitHubRepoPreview, String> {
    let resolved = resolve_repo_source(repo_url, auth).await?;
    let connection = connect_remote_target(active_target).await?;
    cleanup_expired_preview_workspaces_for_connection(&connection).await;

    let workspace = create_remote_preview_workspace(&connection, &resolved, auth).await?;
    let preview_result = async {
        let candidates = build_remote_repo_skill_candidates_from_workspace(
            &connection,
            &resolved.repo,
            &workspace.remote_repo_dir,
            resolved.source_path.as_deref(),
        )
        .await?;
        let skills = build_preview_skills(pool, &candidates).await?;
        if skills.is_empty() {
            return Err(NO_IMPORTABLE_SKILLS_ERROR.to_string());
        }
        Ok(skills)
    }
    .await;

    match preview_result {
        Ok(skills) => {
            register_preview_workspace(workspace.clone());
            Ok(GitHubRepoPreview {
                repo: resolved.repo,
                skills,
                preview_workspace_id: Some(workspace.id),
            })
        }
        Err(error) => {
            let _ = connection
                .remove_tree(&workspace.remote_workspace_dir)
                .await;
            Err(error)
        }
    }
}
