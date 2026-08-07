use super::CentralOperationError;
use crate::targets::ConnectedRemoteTarget;

pub(super) fn normalize_remote_delete_path(path: &str) -> Result<String, CentralOperationError> {
    if !path.starts_with('/') || path.contains(['\0', '\\']) {
        return Err(CentralOperationError::InvalidManifest(
            "invalid remote delete path".to_string(),
        ));
    }

    let mut components = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                return Err(CentralOperationError::InvalidManifest(
                    "invalid remote delete path".to_string(),
                ));
            }
            value => components.push(value),
        }
    }
    if components.is_empty() {
        return Err(CentralOperationError::InvalidManifest(
            "invalid remote delete path".to_string(),
        ));
    }
    Ok(format!("/{}", components.join("/")))
}

pub(super) async fn remote_fingerprint(
    connection: &ConnectedRemoteTarget,
    path: &str,
) -> Result<Option<String>, CentralOperationError> {
    let output = connection
        .run_script(super::fs::REMOTE_FINGERPRINT, &[path])
        .await
        .map_err(|_| CentralOperationError::Remote {
            code: "remote_fingerprint",
        })?;
    let value = output.trim();
    if value == "MISSING" {
        return Ok(None);
    }
    if value.len() != 64 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(CentralOperationError::Remote {
            code: "remote_fingerprint_protocol",
        });
    }
    Ok(Some(value.to_ascii_lowercase()))
}
