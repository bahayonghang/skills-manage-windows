pub struct ConnectedSshTarget {
    target: RemoteTargetConfig,
    password: Option<String>,
    askpass_helper: Option<AskpassHelper>,
}

#[derive(Debug, Clone)]
struct AskpassHelper {
    path: PathBuf,
    remove_on_drop: bool,
    use_app_helper_env: bool,
}

pub async fn connect_ssh_target(target: &RemoteTargetConfig) -> Result<ConnectedSshTarget, String> {
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
    };
    connection
        .run_command("printf '%s' connected >/dev/null")
        .await?;
    Ok(connection)
}

#[cfg(windows)]
fn create_askpass_helper() -> Result<AskpassHelper, String> {
    let path = env::current_exe()
        .map_err(|e| format!("Failed to resolve SkillPort SSH askpass helper path: {}", e))?;
    Ok(AskpassHelper {
        path,
        remove_on_drop: false,
        use_app_helper_env: true,
    })
}

#[cfg(not(windows))]
fn create_askpass_helper() -> Result<AskpassHelper, String> {
    let extension = "sh";
    let path = env::temp_dir().join(format!(
        "skillport-ssh-askpass-{}.{}",
        Uuid::new_v4(),
        extension
    ));
    let content = format!("#!/bin/sh\nprintf '%s' \"${}\"\n", SSH_PASSWORD_ENV);
    fs::write(&path, content).map_err(|e| {
        format!(
            "Failed to create SSH askpass helper '{}': {}",
            path.display(),
            e
        )
    })?;
    set_askpass_permissions(&path)?;
    Ok(AskpassHelper {
        path,
        remove_on_drop: true,
        use_app_helper_env: false,
    })
}

#[cfg(unix)]
fn set_askpass_permissions(path: &PathBuf) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|e| {
            format!(
                "Failed to inspect askpass helper '{}': {}",
                path.display(),
                e
            )
        })?
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions).map_err(|e| {
        format!(
            "Failed to set askpass helper permissions '{}': {}",
            path.display(),
            e
        )
    })
}

impl Drop for ConnectedSshTarget {
    fn drop(&mut self) {
        if let Some(helper) = &self.askpass_helper {
            if helper.remove_on_drop {
                let _ = fs::remove_file(&helper.path);
            }
        }
    }
}
