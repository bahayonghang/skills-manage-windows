use std::path::Path;

use crate::services::central_updates::CentralUpdatesError;

pub(super) fn ensure_local_child_path(
    root: &Path,
    child: &Path,
    label: &str,
) -> Result<(), CentralUpdatesError> {
    if crate::paths::paths_equivalent(root, child) {
        return Err(CentralUpdatesError::PlatformRootDeletion(label.to_string()));
    }

    let root_cmp = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let child_parent = child
        .parent()
        .ok_or_else(|| CentralUpdatesError::PathNoParent(child.display().to_string()))?;
    let child_parent_cmp = child_parent
        .canonicalize()
        .unwrap_or_else(|_| child_parent.to_path_buf());
    if !child_parent_cmp.starts_with(&root_cmp) {
        return Err(CentralUpdatesError::OutsidePlatformRoot {
            child: child.display().to_string(),
            root: root.display().to_string(),
        });
    }
    Ok(())
}

pub(super) fn ensure_remote_child_path(
    root: &str,
    child: &str,
    label: &str,
) -> Result<String, CentralUpdatesError> {
    let root_cmp = normalize_remote_path(root)?;
    let child_cmp = normalize_remote_path(child)?;
    if root_cmp == "/" {
        return Err(CentralUpdatesError::RemoteRootDeletionScope(
            label.to_string(),
        ));
    }
    if root_cmp == child_cmp {
        return Err(CentralUpdatesError::RemoteRootDeletion {
            root: root_cmp,
            label: label.to_string(),
        });
    }
    let prefix = format!("{}/", root_cmp.trim_end_matches('/'));
    if !child_cmp.starts_with(&prefix) {
        return Err(CentralUpdatesError::OutsideRemoteRoot {
            child: child.to_string(),
            root: root.to_string(),
        });
    }
    Ok(child_cmp)
}

fn normalize_remote_path(path: &str) -> Result<String, CentralUpdatesError> {
    let trimmed = path.trim();
    if trimmed.is_empty() || !trimmed.starts_with('/') || trimmed.contains('\0') {
        return Err(CentralUpdatesError::InvalidRemotePath(path.to_string()));
    }

    let mut segments = Vec::new();
    for segment in trimmed.split('/') {
        match segment {
            "" | "." => {}
            ".." => return Err(CentralUpdatesError::RemotePathTraversal(path.to_string())),
            value => segments.push(value),
        }
    }
    if segments.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", segments.join("/")))
    }
}

pub(super) fn paths_equivalent_str(left: &str, right: &str) -> bool {
    paths_equivalent_path(Path::new(left), Path::new(right))
}

pub(super) fn paths_equivalent_path(left: &Path, right: &Path) -> bool {
    crate::paths::paths_equivalent(left, right)
}
