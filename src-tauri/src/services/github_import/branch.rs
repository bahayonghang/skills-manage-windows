use super::{validate_repo_branch, GithubImportError};

pub(super) fn reconcile_selected_branch(
    url_branch: Option<&str>,
    selected_branch: Option<&str>,
) -> Result<Option<String>, GithubImportError> {
    let selected_branch = selected_branch
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(branch) = selected_branch {
        validate_repo_branch(branch).map_err(|_| GithubImportError::InvalidBranchSelection)?;
    }

    match (url_branch, selected_branch) {
        (Some(url_branch), Some(selected_branch)) if url_branch != selected_branch => {
            Err(GithubImportError::BranchSelectionConflict)
        }
        (_, Some(selected_branch)) => Ok(Some(selected_branch.to_string())),
        (Some(url_branch), None) => Ok(Some(url_branch.to_string())),
        (None, None) => Ok(None),
    }
}
