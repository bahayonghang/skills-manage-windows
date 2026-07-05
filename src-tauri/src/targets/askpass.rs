use super::*;
pub struct ConnectedSshTarget {
    pub(super) target: RemoteTargetConfig,
    pub(super) password: Option<String>,
    pub(super) askpass_helper: Option<AskpassHelper>,
    pub(super) runner: Arc<dyn CommandRunner>,
}

#[cfg(test)]
impl ConnectedSshTarget {
    /// Test-only constructor: no live connection probe, injectable runner.
    pub(crate) fn for_tests_with_runner(
        target: RemoteTargetConfig,
        runner: Arc<dyn CommandRunner>,
    ) -> Self {
        Self {
            target,
            password: None,
            askpass_helper: None,
            runner,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct AskpassHelper {
    pub(super) path: PathBuf,
    pub(super) remove_on_drop: bool,
    pub(super) use_app_helper_env: bool,
}

#[cfg(not(windows))]
pub(super) const ASKPASS_HELPER_FILE_PREFIX: &str = "skillport-ssh-askpass-";
#[cfg(not(windows))]
pub(super) const ASKPASS_HELPER_MAX_AGE_SECS: u64 = 60 * 60;

pub async fn connect_ssh_target(
    target: &RemoteTargetConfig,
) -> Result<ConnectedSshTarget, TargetsError> {
    let password = match target.auth_method {
        SshAuthMethod::Key => None,
        SshAuthMethod::Password => Some(load_target_password(target)?),
    };
    let askpass_helper = match password.as_deref() {
        Some(_) => Some(create_askpass_helper()?),
        None => None,
    };
    let connection = ConnectedSshTarget {
        target: target.clone(),
        password,
        askpass_helper,
        runner: Arc::new(ProcessRunner),
    };
    connection
        .run_command("printf '%s' connected >/dev/null")
        .await?;
    Ok(connection)
}

#[cfg(windows)]
pub(super) fn create_askpass_helper() -> Result<AskpassHelper, TargetsError> {
    let path = env::current_exe()
        .map_err(|e| TargetsError::io("Failed to resolve SkillPort SSH askpass helper path", e))?;
    Ok(AskpassHelper {
        path,
        remove_on_drop: false,
        use_app_helper_env: true,
    })
}

#[cfg(not(windows))]
pub(super) fn create_askpass_helper() -> Result<AskpassHelper, TargetsError> {
    sweep_stale_askpass_helpers()?;
    let extension = "sh";
    let pid = std::process::id();
    let timestamp = current_unix_timestamp_secs();
    let path = env::temp_dir().join(format!(
        "{}{}-{}-{}.{}",
        ASKPASS_HELPER_FILE_PREFIX,
        pid,
        timestamp,
        Uuid::new_v4(),
        extension
    ));
    let content = format!("#!/bin/sh\nprintf '%s' \"${}\"\n", SSH_PASSWORD_ENV);
    fs::write(&path, content).map_err(|e| {
        TargetsError::io(
            format!("Failed to create SSH askpass helper '{}'", path.display()),
            e,
        )
    })?;
    set_askpass_permissions(&path)?;
    Ok(AskpassHelper {
        path,
        remove_on_drop: true,
        use_app_helper_env: false,
    })
}

#[cfg(not(windows))]
pub(super) fn current_unix_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(not(windows))]
pub(super) fn sweep_stale_askpass_helpers() -> Result<(), TargetsError> {
    sweep_stale_askpass_helpers_in(&env::temp_dir(), std::time::SystemTime::now())
}

#[cfg(not(windows))]
pub(super) fn sweep_stale_askpass_helpers_in(
    temp_dir: &PathBuf,
    now: std::time::SystemTime,
) -> Result<(), TargetsError> {
    let entries = match fs::read_dir(temp_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(TargetsError::io(
                format!(
                    "Failed to inspect SSH askpass temp directory '{}'",
                    temp_dir.display()
                ),
                error,
            ))
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !should_remove_stale_askpass_helper(&path, now) {
            continue;
        }
        let _ = fs::remove_file(path);
    }

    Ok(())
}

#[cfg(not(windows))]
pub(super) fn should_remove_stale_askpass_helper(
    path: &std::path::Path,
    now: std::time::SystemTime,
) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if !file_name.starts_with(ASKPASS_HELPER_FILE_PREFIX) {
        return false;
    }

    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified_at) = metadata.modified() else {
        return false;
    };
    let Ok(age) = now.duration_since(modified_at) else {
        return false;
    };
    age.as_secs() > ASKPASS_HELPER_MAX_AGE_SECS
}

#[cfg(unix)]
pub(super) fn set_askpass_permissions(path: &PathBuf) -> Result<(), TargetsError> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|e| {
            TargetsError::io(
                format!("Failed to inspect askpass helper '{}'", path.display()),
                e,
            )
        })?
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions).map_err(|e| {
        TargetsError::io(
            format!(
                "Failed to set askpass helper permissions '{}'",
                path.display()
            ),
            e,
        )
    })
}

impl Drop for AskpassHelper {
    fn drop(&mut self) {
        if self.remove_on_drop {
            let _ = fs::remove_file(&self.path);
        }
    }
}
