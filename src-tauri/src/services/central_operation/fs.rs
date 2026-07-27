use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::fs_util::run_blocking_fs_with;
use crate::targets::ConnectedRemoteTarget;

use super::{CentralOperationError, DeleteManifest, ManagedPath, MANIFEST_VERSION};

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

const REMOTE_FINGERPRINT: &str = r#"
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
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path).map_err(|error| CentralOperationError::io("cleanup_file", error))
    } else {
        fs::remove_dir_all(path)
            .map_err(|error| CentralOperationError::io("cleanup_directory", error))
    }
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
        entries.sort_by_key(|entry| entry.path().to_path_buf());
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
    Ok(Some(format!("{:x}", hasher.finalize())))
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
    let digest = format!("{:x}", Sha256::digest(path.as_bytes()));
    digest[..16].to_string()
}

async fn remote_fingerprint(
    connection: &ConnectedRemoteTarget,
    path: &str,
) -> Result<Option<String>, CentralOperationError> {
    let output = connection
        .run_script(REMOTE_FINGERPRINT, &[path])
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::targets::{
        ConnectedRemoteTarget, ConnectedSshTarget, ConnectedWslTarget, RemoteTargetConfig,
        SshAuthMethod, WslTargetConfig,
    };
    use crate::test_support::FakeRunner;
    use std::sync::Arc;

    fn fake_connections() -> Vec<(Arc<FakeRunner>, ConnectedRemoteTarget)> {
        let ssh_runner = Arc::new(FakeRunner::new());
        let ssh = ConnectedSshTarget::for_tests_with_runner(
            RemoteTargetConfig {
                id: "ssh-operation-test".to_string(),
                label: "SSH operation test".to_string(),
                host: "example.invalid".to_string(),
                username: "tester".to_string(),
                port: 22,
                auth_method: SshAuthMethod::Key,
                key_path: "~/.ssh/id_ed25519".to_string(),
                credential_key: None,
                protected_password: None,
                password: None,
                remote_home: "/home/tester".to_string(),
                remote_os: "linux".to_string(),
                symlink_enabled: true,
            },
            ssh_runner.clone(),
        );
        let wsl_runner = Arc::new(FakeRunner::new());
        let wsl = ConnectedWslTarget::for_tests_with_runner(
            WslTargetConfig {
                id: "wsl-operation-test".to_string(),
                label: "WSL operation test".to_string(),
                distribution: "TestDistro".to_string(),
                remote_home: "/home/tester".to_string(),
                remote_os: "linux".to_string(),
                symlink_enabled: true,
            },
            wsl_runner.clone(),
        );
        vec![
            (ssh_runner, ConnectedRemoteTarget::Ssh(ssh)),
            (wsl_runner, ConnectedRemoteTarget::Wsl(wsl)),
        ]
    }

    #[tokio::test]
    async fn local_delete_stage_restore_and_finalize_are_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("skill-a");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("SKILL.md"), "before").unwrap();
        let manifest = build_local_delete_manifest("op-a", vec![target.clone()])
            .await
            .unwrap();
        stage_delete_local(&manifest).await.unwrap();
        assert!(!target.exists());
        restore_delete_local(&manifest).await.unwrap();
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "before"
        );

        let manifest = build_local_delete_manifest("op-b", vec![target.clone()])
            .await
            .unwrap();
        stage_delete_local(&manifest).await.unwrap();
        finalize_delete_local(&manifest).await.unwrap();
        finalize_delete_local(&manifest).await.unwrap();
        assert!(!target.exists());
    }

    #[tokio::test]
    async fn local_delete_restore_preserves_collision_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("skill-a");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("SKILL.md"), "before").unwrap();
        let manifest = build_local_delete_manifest("op-a", vec![target.clone()])
            .await
            .unwrap();
        stage_delete_local(&manifest).await.unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(target.join("SKILL.md"), "new-user-data").unwrap();
        let error = restore_delete_local(&manifest).await.unwrap_err();
        assert!(matches!(
            error,
            CentralOperationError::RecoveryCollision { .. }
        ));
        assert!(Path::new(&manifest.paths[0].backup).exists());
    }

    #[tokio::test]
    async fn ssh_and_wsl_fake_runners_cover_delete_stage_and_phase_loss_restore() {
        let digest = "a".repeat(64);
        for (runner, connection) in fake_connections() {
            runner.push_success("");
            runner.push_success(&digest);
            runner.push_success("STAGED\n");
            runner.push_success(&digest);
            runner.push_success("RESTORED\n");
            let manifest = build_remote_delete_manifest(
                &connection,
                "remote-op",
                vec!["/home/tester/.skillsmanage/skills/demo".to_string()],
            )
            .await
            .unwrap();
            stage_delete_remote(&connection, &manifest).await.unwrap();
            restore_delete_remote(&connection, &manifest).await.unwrap();
            let calls = runner.calls();
            assert_eq!(calls.len(), 5);
            assert!(calls.iter().any(|call| {
                call.args
                    .iter()
                    .any(|argument| argument.contains("skillport-delete-backup"))
            }));
        }
    }

    #[tokio::test]
    async fn ssh_and_wsl_delete_finalize_is_idempotent_after_cleanup() {
        let digest = "a".repeat(64);
        for (runner, connection) in fake_connections() {
            let manifest = DeleteManifest {
                version: MANIFEST_VERSION,
                operation_id: "remote-finalize-op".to_string(),
                paths: vec![ManagedPath {
                    original: "/home/tester/.skillsmanage/skills/demo".to_string(),
                    backup: "/home/tester/.skillsmanage/skills/.skillport-delete-backup"
                        .to_string(),
                    marker: "/home/tester/.skillsmanage/skills/.skillport-operation.marker"
                        .to_string(),
                    expected_present: true,
                    fingerprint: Some(digest.clone()),
                }],
            };
            for _ in 0..2 {
                runner.push_success("MISSING\n");
                runner.push_success("FINALIZED\n");
                finalize_delete_remote(&connection, &manifest)
                    .await
                    .unwrap();
            }
            assert_eq!(runner.calls().len(), 4);
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "requires SKILLPORT_TEST_WSL_DISTRO and writes only under WSL /tmp"]
    async fn live_wsl_delete_stage_and_restore_smoke() {
        let distribution = std::env::var("SKILLPORT_TEST_WSL_DISTRO")
            .expect("set SKILLPORT_TEST_WSL_DISTRO to an installed distribution");
        let target = WslTargetConfig {
            id: "operation-wsl-smoke".to_string(),
            label: "Operation WSL smoke".to_string(),
            distribution,
            remote_home: "/tmp".to_string(),
            remote_os: "linux".to_string(),
            symlink_enabled: true,
        };
        let connection = ConnectedRemoteTarget::Wsl(
            crate::targets::open_wsl_target(&target).expect("open WSL target"),
        );
        let root = format!("/tmp/skillport-operation-smoke-{}", uuid::Uuid::new_v4());
        let skill = format!("{root}/demo");
        connection
            .run_script(
                "set -eu; mkdir -p -- \"$1\"; printf before > \"$1/SKILL.md\"",
                &[&skill],
            )
            .await
            .unwrap();
        let manifest =
            build_remote_delete_manifest(&connection, "wsl-smoke-op", vec![skill.clone()])
                .await
                .unwrap();
        stage_delete_remote(&connection, &manifest).await.unwrap();
        restore_delete_remote(&connection, &manifest).await.unwrap();
        assert_eq!(
            connection
                .run_script("cat -- \"$1/SKILL.md\"", &[&skill])
                .await
                .unwrap(),
            "before"
        );
        connection
            .run_script("rm -rf -- \"$1\"", &[&root])
            .await
            .unwrap();
    }
}
