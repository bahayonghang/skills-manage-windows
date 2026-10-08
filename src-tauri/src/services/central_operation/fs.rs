use std::fs;
use std::io::Read;
#[cfg(windows)]
use std::os::windows::fs::FileTypeExt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::fs_util::run_blocking_fs_with;
use crate::targets::ConnectedRemoteTarget;

use super::path::{normalize_remote_delete_path, remote_fingerprint};
use super::{CentralOperationError, DeleteManifest, ManagedPath, MANIFEST_VERSION};

#[cfg(test)]
#[path = "fs_dedupe_tests.rs"]
mod dedupe_tests;

#[cfg(test)]
#[path = "fs_hash_tests.rs"]
mod hash_tests;

const REMOTE_STAGE_DELETE: &str = r#"
set -eu
operation_id=$1
original=$2
backup=$3
marker=$4
[ ! -e "$backup" ] && [ ! -L "$backup" ] || exit 41
[ ! -e "$marker" ] || exit 42
if [ ! -e "$original" ] && [ ! -L "$original" ]; then
  printf 'MISSING\n'
  exit 0
fi
printf '%s\n' "$operation_id" > "$marker" || exit 43
if mv -- "$original" "$backup"; then
  printf 'STAGED\n'
else
  rm -f -- "$marker" || exit 45
  exit 44
fi
"#;

const REMOTE_RESTORE_DELETE: &str = r#"
set -eu
operation_id=$1
original=$2
backup=$3
marker=$4
if [ -e "$backup" ] || [ -L "$backup" ]; then
  [ ! -e "$original" ] && [ ! -L "$original" ] || exit 51
  [ -f "$marker" ] && [ "$(cat "$marker")" = "$operation_id" ] || exit 52
  mv -- "$backup" "$original" || exit 53
  rm -f -- "$marker" || exit 54
elif [ -e "$original" ] || [ -L "$original" ]; then
  if [ -e "$marker" ]; then
    [ -f "$marker" ] && [ "$(cat "$marker")" = "$operation_id" ] || exit 52
    rm -f -- "$marker" || exit 54
  fi
else
  exit 55
fi
printf 'RESTORED\n'
"#;

const REMOTE_FINALIZE_DELETE: &str = r#"
set -eu
operation_id=$1
original=$2
backup=$3
marker=$4
[ ! -e "$original" ] && [ ! -L "$original" ] || exit 61
if [ -e "$backup" ] || [ -L "$backup" ]; then
  [ -f "$marker" ] && [ "$(cat "$marker")" = "$operation_id" ] || exit 62
  rm -rf -- "$backup" || exit 63
fi
if [ -e "$marker" ]; then
  [ -f "$marker" ] && [ "$(cat "$marker")" = "$operation_id" ] || exit 62
  rm -f -- "$marker" || exit 64
fi
printf 'FINALIZED\n'
"#;

pub(super) const REMOTE_FINGERPRINT: &str = r#"
set -eu
path=$1
if [ ! -e "$path" ] && [ ! -L "$path" ]; then
  printf 'MISSING\n'
  exit 0
fi
hash_stream() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 | sed 's/^.*= //'
  else
    exit 86
  fi
}
if [ -L "$path" ]; then
  { printf 'symlink\000'; readlink -- "$path"; } | hash_stream
elif [ -f "$path" ]; then
  { printf 'file\000'; cat -- "$path"; } | hash_stream
elif [ -d "$path" ]; then
  { printf 'dir\000'; tar -cf - -C "$path" .; } | hash_stream
else
  exit 87
fi
"#;

pub(crate) async fn fingerprint_local_path(
    path: &Path,
) -> Result<Option<String>, CentralOperationError> {
    let path = path.to_path_buf();
    run_blocking_fs_with(
        "Central operation fingerprint",
        move || fingerprint_path_blocking(&path),
        CentralOperationError::task_join,
    )
    .await
}

pub(crate) async fn build_local_delete_manifest(
    operation_id: &str,
    paths: Vec<PathBuf>,
) -> Result<DeleteManifest, CentralOperationError> {
    let mut managed = Vec::with_capacity(paths.len());
    for path in paths {
        if managed.iter().any(|entry: &ManagedPath| {
            crate::paths::paths_equivalent(Path::new(&entry.original), &path)
        }) {
            continue;
        }
        let parent = path.parent().ok_or_else(|| {
            CentralOperationError::InvalidManifest("delete path has no parent".to_string())
        })?;
        let token = path_token(&path.to_string_lossy());
        let backup = parent.join(format!(".skillport-delete-backup-{operation_id}-{token}"));
        let marker = parent.join(format!(
            ".skillport-operation-{operation_id}-{token}.marker"
        ));
        let fingerprint = fingerprint_local_path(&path).await?;
        managed.push(ManagedPath {
            original: path.to_string_lossy().into_owned(),
            backup: backup.to_string_lossy().into_owned(),
            marker: marker.to_string_lossy().into_owned(),
            expected_present: fingerprint.is_some(),
            fingerprint,
        });
    }
    Ok(DeleteManifest {
        version: MANIFEST_VERSION,
        operation_id: operation_id.to_string(),
        paths: managed,
    })
}

pub(crate) async fn build_remote_delete_manifest(
    connection: &ConnectedRemoteTarget,
    operation_id: &str,
    paths: Vec<String>,
) -> Result<DeleteManifest, CentralOperationError> {
    let mut managed = Vec::with_capacity(paths.len());
    for original in paths {
        let original = normalize_remote_delete_path(&original)?;
        if managed
            .iter()
            .any(|entry: &ManagedPath| entry.original == original)
        {
            continue;
        }
        let (parent, _) = original.rsplit_once('/').ok_or_else(|| {
            CentralOperationError::InvalidManifest("remote delete path has no parent".to_string())
        })?;
        if parent.is_empty() || original.contains('\0') {
            return Err(CentralOperationError::InvalidManifest(
                "invalid remote delete path".to_string(),
            ));
        }
        let token = path_token(&original);
        let backup = format!("{parent}/.skillport-delete-backup-{operation_id}-{token}");
        let marker = format!("{parent}/.skillport-operation-{operation_id}-{token}.marker");
        let expected_present =
            connection
                .exists(&original)
                .await
                .map_err(|_| CentralOperationError::Remote {
                    code: "remote_inspect",
                })?;
        let fingerprint = if expected_present {
            remote_fingerprint(connection, &original).await?
        } else {
            None
        };
        managed.push(ManagedPath {
            original,
            backup,
            marker,
            expected_present,
            fingerprint,
        });
    }
    Ok(DeleteManifest {
        version: MANIFEST_VERSION,
        operation_id: operation_id.to_string(),
        paths: managed,
    })
}

pub(crate) async fn stage_delete_local(
    manifest: &DeleteManifest,
) -> Result<(), CentralOperationError> {
    let manifest = manifest.clone();
    run_blocking_fs_with(
        "Central delete staging",
        move || stage_delete_local_blocking(&manifest),
        CentralOperationError::task_join,
    )
    .await
}

pub(crate) async fn restore_delete_local(
    manifest: &DeleteManifest,
) -> Result<(), CentralOperationError> {
    let manifest = manifest.clone();
    run_blocking_fs_with(
        "Central delete restore",
        move || restore_delete_local_blocking(&manifest),
        CentralOperationError::task_join,
    )
    .await
}

pub(crate) async fn finalize_delete_local(
    manifest: &DeleteManifest,
) -> Result<(), CentralOperationError> {
    let manifest = manifest.clone();
    run_blocking_fs_with(
        "Central delete finalize",
        move || finalize_delete_local_blocking(&manifest),
        CentralOperationError::task_join,
    )
    .await
}

pub(crate) async fn stage_delete_remote(
    connection: &ConnectedRemoteTarget,
    manifest: &DeleteManifest,
) -> Result<(), CentralOperationError> {
    let mut staged = Vec::new();
    for path in &manifest.paths {
        if !path.expected_present {
            continue;
        }
        let output = connection
            .run_script(
                REMOTE_STAGE_DELETE,
                &[
                    &manifest.operation_id,
                    &path.original,
                    &path.backup,
                    &path.marker,
                ],
            )
            .await
            .map_err(|_| CentralOperationError::Remote {
                code: "remote_stage",
            });
        match output {
            Ok(value) if value.trim() == "STAGED" => staged.push(path.clone()),
            _ => {
                let rollback = DeleteManifest {
                    version: manifest.version,
                    operation_id: manifest.operation_id.clone(),
                    paths: staged,
                };
                restore_delete_remote(connection, &rollback).await?;
                return Err(CentralOperationError::Remote {
                    code: "remote_stage",
                });
            }
        }
    }
    Ok(())
}

pub(crate) async fn restore_delete_remote(
    connection: &ConnectedRemoteTarget,
    manifest: &DeleteManifest,
) -> Result<(), CentralOperationError> {
    for path in manifest
        .paths
        .iter()
        .rev()
        .filter(|path| path.expected_present)
    {
        let backup_fingerprint = remote_fingerprint(connection, &path.backup).await?;
        let actual = if backup_fingerprint.is_some() {
            backup_fingerprint
        } else {
            remote_fingerprint(connection, &path.original).await?
        };
        if actual.as_deref() != path.fingerprint.as_deref() {
            return Err(CentralOperationError::RecoveryCollision {
                code: "remote_delete_fingerprint",
            });
        }
        let output = connection
            .run_script(
                REMOTE_RESTORE_DELETE,
                &[
                    &manifest.operation_id,
                    &path.original,
                    &path.backup,
                    &path.marker,
                ],
            )
            .await
            .map_err(|_| CentralOperationError::Remote {
                code: "remote_restore_collision",
            })?;
        if output.trim() != "RESTORED" {
            return Err(CentralOperationError::Remote {
                code: "remote_restore_protocol",
            });
        }
    }
    Ok(())
}

pub(crate) async fn finalize_delete_remote(
    connection: &ConnectedRemoteTarget,
    manifest: &DeleteManifest,
) -> Result<(), CentralOperationError> {
    for path in manifest.paths.iter().filter(|path| path.expected_present) {
        if let Some(actual) = remote_fingerprint(connection, &path.backup).await? {
            if Some(actual.as_str()) != path.fingerprint.as_deref() {
                return Err(CentralOperationError::RecoveryCollision {
                    code: "remote_delete_fingerprint",
                });
            }
        }
        let output = connection
            .run_script(
                REMOTE_FINALIZE_DELETE,
                &[
                    &manifest.operation_id,
                    &path.original,
                    &path.backup,
                    &path.marker,
                ],
            )
            .await
            .map_err(|_| CentralOperationError::Remote {
                code: "remote_finalize_collision",
            })?;
        if output.trim() != "FINALIZED" {
            return Err(CentralOperationError::Remote {
                code: "remote_finalize_protocol",
            });
        }
    }
    Ok(())
}

fn stage_delete_local_blocking(manifest: &DeleteManifest) -> Result<(), CentralOperationError> {
    let mut staged = Vec::new();
    for path in &manifest.paths {
        let original = Path::new(&path.original);
        let backup = Path::new(&path.backup);
        let marker = Path::new(&path.marker);
        let present = fs::symlink_metadata(original).is_ok();
        if present != path.expected_present
            || fs::symlink_metadata(backup).is_ok()
            || marker.exists()
        {
            let rollback = DeleteManifest {
                version: manifest.version,
                operation_id: manifest.operation_id.clone(),
                paths: staged,
            };
            restore_delete_local_blocking(&rollback)?;
            return Err(CentralOperationError::RecoveryCollision {
                code: "delete_stage_collision",
            });
        }
        if !present {
            continue;
        }
        fs::write(marker, manifest.operation_id.as_bytes())
            .map_err(|error| CentralOperationError::io("marker_write", error))?;
        if let Err(error) = fs::rename(original, backup) {
            let marker_cleanup_error = fs::remove_file(marker).err();
            let rollback = DeleteManifest {
                version: manifest.version,
                operation_id: manifest.operation_id.clone(),
                paths: staged,
            };
            restore_delete_local_blocking(&rollback)?;
            if let Some(cleanup_error) = marker_cleanup_error {
                return Err(CentralOperationError::io(
                    "marker_cleanup_after_stage_failure",
                    cleanup_error,
                ));
            }
            return Err(CentralOperationError::io("delete_stage_rename", error));
        }
        staged.push(path.clone());
    }
    Ok(())
}

fn restore_delete_local_blocking(manifest: &DeleteManifest) -> Result<(), CentralOperationError> {
    for path in manifest
        .paths
        .iter()
        .rev()
        .filter(|path| path.expected_present)
    {
        let original = Path::new(&path.original);
        let backup = Path::new(&path.backup);
        let marker = Path::new(&path.marker);
        let original_exists = fs::symlink_metadata(original).is_ok();
        let backup_exists = fs::symlink_metadata(backup).is_ok();
        match (original_exists, backup_exists) {
            (false, true) => {
                verify_marker(marker, &manifest.operation_id)?;
                verify_fingerprint(backup, path.fingerprint.as_deref())?;
                fs::rename(backup, original)
                    .map_err(|error| CentralOperationError::io("delete_restore_rename", error))?;
                remove_marker(marker, &manifest.operation_id)?;
            }
            (true, false) => {
                verify_fingerprint(original, path.fingerprint.as_deref())?;
                if marker.exists() {
                    remove_marker(marker, &manifest.operation_id)?;
                }
            }
            _ => {
                return Err(CentralOperationError::RecoveryCollision {
                    code: "delete_restore_collision",
                })
            }
        }
    }
    Ok(())
}

fn finalize_delete_local_blocking(manifest: &DeleteManifest) -> Result<(), CentralOperationError> {
    for path in manifest.paths.iter().filter(|path| path.expected_present) {
        let original = Path::new(&path.original);
        let backup = Path::new(&path.backup);
        let marker = Path::new(&path.marker);
        if fs::symlink_metadata(original).is_ok() {
            return Err(CentralOperationError::RecoveryCollision {
                code: "delete_finalize_collision",
            });
        }
        if fs::symlink_metadata(backup).is_ok() {
            verify_marker(marker, &manifest.operation_id)?;
            verify_fingerprint(backup, path.fingerprint.as_deref())?;
            remove_any_path(backup)?;
        }
        if marker.exists() {
            remove_marker(marker, &manifest.operation_id)?;
        }
    }
    Ok(())
}

fn remove_any_path(path: &Path) -> Result<(), CentralOperationError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| CentralOperationError::io("cleanup_inspect", error))?;
    if is_directory_symlink(&metadata) {
        // RemoveDirectoryW deletes the reparse point and does not follow it.
        fs::remove_dir(path).map_err(|error| CentralOperationError::io("cleanup_file", error))
    } else if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path).map_err(|error| CentralOperationError::io("cleanup_file", error))
    } else {
        fs::remove_dir_all(path)
            .map_err(|error| CentralOperationError::io("cleanup_directory", error))
    }
}

/// Windows directory symlinks keep `FILE_ATTRIBUTE_DIRECTORY`, but
/// `Metadata::is_dir` is false for reparse points.
/// `FileTypeExt::is_symlink_dir` is the check that selects `remove_dir`.
/// Unix `symlink_metadata` never reports a symlink as a directory, so those
/// links stay on `remove_file` and are not walked with `remove_dir_all`.
#[cfg(windows)]
fn is_directory_symlink(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink_dir()
}

#[cfg(not(windows))]
fn is_directory_symlink(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink() && metadata.is_dir()
}

fn verify_marker(marker: &Path, operation_id: &str) -> Result<(), CentralOperationError> {
    let value =
        fs::read_to_string(marker).map_err(|_| CentralOperationError::RecoveryCollision {
            code: "marker_missing",
        })?;
    if value != operation_id {
        return Err(CentralOperationError::RecoveryCollision {
            code: "marker_mismatch",
        });
    }
    Ok(())
}

fn remove_marker(marker: &Path, operation_id: &str) -> Result<(), CentralOperationError> {
    verify_marker(marker, operation_id)?;
    fs::remove_file(marker).map_err(|error| CentralOperationError::io("marker_cleanup", error))
}

fn verify_fingerprint(path: &Path, expected: Option<&str>) -> Result<(), CentralOperationError> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let actual =
        fingerprint_path_blocking(path)?.ok_or(CentralOperationError::RecoveryCollision {
            code: "fingerprint_missing",
        })?;
    if actual != expected {
        return Err(CentralOperationError::RecoveryCollision {
            code: "fingerprint_mismatch",
        });
    }
    Ok(())
}

fn fingerprint_path_blocking(path: &Path) -> Result<Option<String>, CentralOperationError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(CentralOperationError::io("fingerprint_inspect", error)),
    };
    let mut hasher = Sha256::new();
    if metadata.file_type().is_symlink() {
        hasher.update(b"symlink\0");
        let target = fs::read_link(path)
            .map_err(|error| CentralOperationError::io("fingerprint_symlink", error))?;
        hasher.update(target.to_string_lossy().as_bytes());
    } else if metadata.is_file() {
        hasher.update(b"file\0");
        hash_file(path, &mut hasher)?;
    } else if metadata.is_dir() {
        hasher.update(b"dir\0");
        let mut entries = WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                CentralOperationError::io(
                    "fingerprint_walk",
                    std::io::Error::other(error.to_string()),
                )
            })?;
        entries.sort_by(|left, right| left.path().cmp(right.path()));
        for entry in entries.into_iter().skip(1) {
            let relative = entry.path().strip_prefix(path).map_err(|_| {
                CentralOperationError::InvalidManifest("fingerprint path escaped root".to_string())
            })?;
            hasher.update(relative.to_string_lossy().as_bytes());
            hasher.update([0]);
            let metadata = entry.metadata().map_err(|error| {
                CentralOperationError::io(
                    "fingerprint_metadata",
                    std::io::Error::other(error.to_string()),
                )
            })?;
            if metadata.file_type().is_symlink() {
                hasher.update(b"symlink\0");
                let target = fs::read_link(entry.path())
                    .map_err(|error| CentralOperationError::io("fingerprint_symlink", error))?;
                hasher.update(target.to_string_lossy().as_bytes());
            } else if metadata.is_file() {
                hasher.update(b"file\0");
                hash_file(entry.path(), &mut hasher)?;
            } else {
                hasher.update(b"dir\0");
            }
        }
    } else {
        return Err(CentralOperationError::RecoveryCollision {
            code: "unsupported_path_type",
        });
    }
    Ok(Some(crate::hashing::encode_lower_hex(
        hasher.finalize().as_ref(),
    )))
}

fn hash_file(path: &Path, hasher: &mut Sha256) -> Result<(), CentralOperationError> {
    let mut file = fs::File::open(path)
        .map_err(|error| CentralOperationError::io("fingerprint_open", error))?;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| CentralOperationError::io("fingerprint_read", error))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(())
}

fn path_token(path: &str) -> String {
    let digest = crate::hashing::encode_lower_hex(Sha256::digest(path.as_bytes()).as_ref());
    digest[..16].to_string()
}

#[cfg(test)]
mod tests;
