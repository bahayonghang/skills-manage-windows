use super::*;
pub async fn create_ssh_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    request: CreateSshTargetRequest,
) -> Result<TargetSummary, String> {
    let target_id = format!("ssh-{}", Uuid::new_v4());
    let base = request_to_config(request, target_id)?;
    let probe = probe_ssh_target(&base).await?;
    if !is_supported_remote_os(&probe.remote_os) {
        return Err(format!(
            "Remote OS '{}' is not supported in this version. Linux and macOS are supported.",
            probe.remote_os
        ));
    }

    let mut targets = load_remote_targets(local_db).await?;
    let mut target = base;
    target.remote_home = probe.remote_home;
    target.remote_os = probe.remote_os;
    let credential_state = if target.auth_method == SshAuthMethod::Password {
        Some(registry.save_target_password(&mut target)?)
    } else {
        None
    };
    if target.auth_method == SshAuthMethod::Password {
        target.password = None;
    }

    let stored_target = target.clone();
    targets.push(stored_target.clone());
    if let Err(error) = save_remote_targets(local_db, &targets).await {
        let mut cleanup_target = stored_target.clone();
        let _ = registry.delete_target_password(&mut cleanup_target);
        return Err(error);
    }
    registry.remote_db(&stored_target).await?;

    let active_id = active_target_id(local_db).await?;
    let mut summary = registry.target_summary(&stored_target, active_id.as_str());
    if let Some(state) = credential_state {
        summary.credential_status = Some(state.status);
        summary.credential_error = state.error;
        summary.has_stored_password = Some(state.status.is_available());
    }
    Ok(summary)
}

pub async fn update_ssh_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    request: UpdateSshTargetRequest,
) -> Result<TargetSummary, String> {
    let mut targets = load_remote_targets(local_db).await?;
    let index = targets
        .iter()
        .position(|target| target.id == request.id)
        .ok_or_else(|| format!("Target '{}' not found", request.id))?;
    let previous_target = targets[index].clone();
    let mut updated_target = update_request_to_config(request, &previous_target)?;
    let supplied_password = updated_target.auth_method == SshAuthMethod::Password
        && updated_target
            .password
            .as_deref()
            .is_some_and(|password| !password.is_empty());

    if updated_target.auth_method == SshAuthMethod::Password && !supplied_password {
        registry.attach_available_password(&mut updated_target);
    }

    let probe = probe_ssh_target(&updated_target).await?;
    if !is_supported_remote_os(&probe.remote_os) {
        return Err(format!(
            "Remote OS '{}' is not supported in this version. Linux and macOS are supported.",
            probe.remote_os
        ));
    }

    updated_target.remote_home = probe.remote_home;
    updated_target.remote_os = probe.remote_os;
    let credential_state = if updated_target.auth_method == SshAuthMethod::Password {
        if supplied_password {
            Some(registry.save_target_password(&mut updated_target)?)
        } else {
            let state = registry.target_credential_state(&updated_target);
            updated_target.password = None;
            state
        }
    } else {
        updated_target.password = None;
        None
    };

    targets[index] = updated_target.clone();
    save_remote_targets(local_db, &targets).await?;
    if previous_target.auth_method == SshAuthMethod::Password
        && updated_target.auth_method != SshAuthMethod::Password
    {
        let mut cleanup_target = previous_target.clone();
        let _ = registry.delete_target_password(&mut cleanup_target);
    }
    registry.drop_remote_pool(&updated_target.id);
    registry.remote_db(&updated_target).await?;

    let active_id = active_target_id(local_db).await?;
    let mut summary = registry.target_summary(&updated_target, active_id.as_str());
    if let Some(state) = credential_state {
        summary.credential_status = Some(state.status);
        summary.credential_error = state.error;
        summary.has_stored_password = Some(state.status.is_available());
    }
    Ok(summary)
}

pub async fn test_ssh_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    request: TestSshTargetRequest,
) -> Result<SshTargetTestResult, String> {
    let supplied_password = request.password.clone();
    let has_existing_target_id = request
        .id
        .as_deref()
        .is_some_and(|id| !id.trim().is_empty());
    let mut should_store_password = false;
    let mut target = match request.id.as_deref() {
        Some(id) if !id.trim().is_empty() => load_remote_targets(local_db)
            .await?
            .into_iter()
            .find(|target| target.id == id)
            .ok_or_else(|| format!("Target '{}' not found", id))?,
        _ => test_request_to_config(request)?,
    };
    if has_existing_target_id
        && apply_supplied_password_to_existing_target(&mut target, supplied_password.as_deref())?
    {
        should_store_password = true;
    }

    match probe_ssh_target(&target).await {
        Ok(probe) if is_supported_remote_os(&probe.remote_os) => {
            let credential_state = if should_store_password {
                target.remote_home = probe.remote_home.clone();
                target.remote_os = probe.remote_os.clone();
                let credential_state = registry.save_target_password(&mut target)?;
                target.password = None;
                let mut targets = load_remote_targets(local_db).await?;
                if let Some(existing) = targets.iter_mut().find(|item| item.id == target.id) {
                    *existing = target.clone();
                    save_remote_targets(local_db, &targets).await?;
                }
                Some(credential_state)
            } else {
                None
            };
            let message =
                ssh_success_message("SSH connection is available.", credential_state.as_ref());
            Ok(SshTargetTestResult {
                ok: true,
                remote_home: Some(probe.remote_home),
                remote_os: Some(probe.remote_os),
                credential_status: credential_state.as_ref().map(|state| state.status),
                credential_error: credential_state.and_then(|state| state.error),
                message,
            })
        }
        Ok(probe) => Ok(SshTargetTestResult {
            ok: false,
            remote_home: Some(probe.remote_home),
            remote_os: Some(probe.remote_os.clone()),
            credential_status: None,
            credential_error: None,
            message: format!(
                "Remote OS '{}' is not supported in this version. Linux and macOS are supported.",
                probe.remote_os
            ),
        }),
        Err(error) => Ok(SshTargetTestResult {
            ok: false,
            remote_home: None,
            remote_os: None,
            credential_status: None,
            credential_error: None,
            message: error,
        }),
    }
}

pub async fn update_ssh_target_password_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target_id: &str,
    password: &str,
) -> Result<SshTargetTestResult, String> {
    let mut targets = load_remote_targets(local_db).await?;
    let target = targets
        .iter_mut()
        .find(|target| target.id == target_id)
        .ok_or_else(|| format!("Target '{}' not found", target_id))?;

    if target.auth_method != SshAuthMethod::Password {
        return Err("This SSH target does not use password authentication.".to_string());
    }
    apply_supplied_password_to_existing_target(target, Some(password))?;

    match probe_ssh_target(target).await {
        Ok(probe) if is_supported_remote_os(&probe.remote_os) => {
            target.remote_home = probe.remote_home.clone();
            target.remote_os = probe.remote_os.clone();
            let credential_state = registry.save_target_password(target)?;
            target.password = None;
            save_remote_targets(local_db, &targets).await?;

            let message = ssh_success_message(
                "SSH password was verified and saved.",
                Some(&credential_state),
            );
            Ok(SshTargetTestResult {
                ok: true,
                remote_home: Some(probe.remote_home),
                remote_os: Some(probe.remote_os),
                credential_status: Some(credential_state.status),
                credential_error: credential_state.error,
                message,
            })
        }
        Ok(probe) => Ok(SshTargetTestResult {
            ok: false,
            remote_home: Some(probe.remote_home),
            remote_os: Some(probe.remote_os.clone()),
            credential_status: None,
            credential_error: None,
            message: format!(
                "Remote OS '{}' is not supported in this version. Linux and macOS are supported.",
                probe.remote_os
            ),
        }),
        Err(error) => Ok(SshTargetTestResult {
            ok: false,
            remote_home: None,
            remote_os: None,
            credential_status: None,
            credential_error: None,
            message: error,
        }),
    }
}

pub async fn create_wsl_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    request: CreateWslTargetRequest,
) -> Result<TargetSummary, String> {
    let target_id = format!("wsl-{}", Uuid::new_v4());
    let mut target = request_to_wsl_config(request, target_id)?;
    let probe = probe_wsl_target(&target).await?;
    if !is_supported_remote_os(&probe.remote_os) {
        return Err(format!(
            "WSL OS '{}' is not supported in this version. Linux is expected for WSL targets.",
            probe.remote_os
        ));
    }

    target.remote_home = probe.remote_home;
    target.remote_os = probe.remote_os;
    let mut targets = load_wsl_targets(local_db).await?;
    targets.push(target.clone());
    save_wsl_targets(local_db, &targets).await?;
    registry
        .remote_db_for(&target.id, &target.remote_home)
        .await?;

    let active_id = active_target_id(local_db).await?;
    Ok(registry.wsl_target_summary(&target, active_id.as_str()))
}

pub async fn update_wsl_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    request: UpdateWslTargetRequest,
) -> Result<TargetSummary, String> {
    let mut targets = load_wsl_targets(local_db).await?;
    let index = targets
        .iter()
        .position(|target| target.id == request.id)
        .ok_or_else(|| format!("Target '{}' not found", request.id))?;
    let mut updated_target = update_wsl_request_to_config(request, &targets[index])?;

    let probe = probe_wsl_target(&updated_target).await?;
    if !is_supported_remote_os(&probe.remote_os) {
        return Err(format!(
            "WSL OS '{}' is not supported in this version. Linux is expected for WSL targets.",
            probe.remote_os
        ));
    }
    updated_target.remote_home = probe.remote_home;
    updated_target.remote_os = probe.remote_os;

    targets[index] = updated_target.clone();
    save_wsl_targets(local_db, &targets).await?;
    registry.drop_remote_pool(&updated_target.id);
    registry
        .remote_db_for(&updated_target.id, &updated_target.remote_home)
        .await?;

    let active_id = active_target_id(local_db).await?;
    Ok(registry.wsl_target_summary(&updated_target, active_id.as_str()))
}

pub async fn test_wsl_target_impl(
    local_db: &DbPool,
    request: TestWslTargetRequest,
) -> Result<WslTargetTestResult, String> {
    let target = match request.id.as_deref() {
        Some(id) if !id.trim().is_empty() => load_wsl_targets(local_db)
            .await?
            .into_iter()
            .find(|target| target.id == id)
            .ok_or_else(|| format!("Target '{}' not found", id))?,
        _ => test_wsl_request_to_config(request)?,
    };

    match probe_wsl_target(&target).await {
        Ok(probe) if is_supported_remote_os(&probe.remote_os) => Ok(WslTargetTestResult {
            ok: true,
            remote_home: Some(probe.remote_home),
            remote_os: Some(probe.remote_os),
            message: "WSL target is available.".to_string(),
        }),
        Ok(probe) => Ok(WslTargetTestResult {
            ok: false,
            remote_home: Some(probe.remote_home),
            remote_os: Some(probe.remote_os.clone()),
            message: format!(
                "WSL OS '{}' is not supported in this version. Linux is expected for WSL targets.",
                probe.remote_os
            ),
        }),
        Err(error) => Ok(WslTargetTestResult {
            ok: false,
            remote_home: None,
            remote_os: None,
            message: error,
        }),
    }
}

pub async fn delete_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target_id: &str,
) -> Result<(), String> {
    if target_id == LOCAL_TARGET_ID {
        return Err("Local target cannot be deleted.".to_string());
    }

    let mut ssh_targets = load_remote_targets(local_db).await?;
    let mut wsl_targets = load_wsl_targets(local_db).await?;
    let removed_ssh = ssh_targets
        .iter()
        .position(|target| target.id == target_id)
        .map(|index| ssh_targets.remove(index));
    let removed_wsl = wsl_targets
        .iter()
        .position(|target| target.id == target_id)
        .map(|index| wsl_targets.remove(index));
    if removed_ssh.is_none() && removed_wsl.is_none() {
        return Err(format!("Target '{}' not found", target_id));
    }
    if let Some(mut removed) = removed_ssh {
        registry.delete_target_password(&mut removed)?;
    }

    save_remote_targets(local_db, &ssh_targets).await?;
    save_wsl_targets(local_db, &wsl_targets).await?;
    if active_target_id(local_db).await? == target_id {
        db::set_setting(local_db, ACTIVE_TARGET_SETTING_KEY, LOCAL_TARGET_ID).await?;
    }
    registry.drop_remote_pool(target_id);
    Ok(())
}

pub async fn set_active_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target_id: &str,
) -> Result<TargetSummary, String> {
    if target_id != LOCAL_TARGET_ID {
        let ssh_exists = load_remote_targets(local_db)
            .await?
            .iter()
            .any(|target| target.id == target_id);
        let wsl_exists = load_wsl_targets(local_db)
            .await?
            .iter()
            .any(|target| target.id == target_id);
        if !ssh_exists && !wsl_exists {
            return Err(format!("Target '{}' not found", target_id));
        }
    }

    db::set_setting(local_db, ACTIVE_TARGET_SETTING_KEY, target_id).await?;
    let targets = registry.list_targets(local_db).await?;
    targets
        .into_iter()
        .find(|target| target.id == target_id)
        .ok_or_else(|| format!("Target '{}' not found", target_id))
}

pub async fn get_active_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
) -> Result<TargetSummary, String> {
    let active_id = active_target_id(local_db).await?;
    registry
        .list_targets(local_db)
        .await?
        .into_iter()
        .find(|target| target.id == active_id)
        .ok_or_else(|| format!("Target '{}' not found", active_id))
}

pub async fn active_target_id(local_db: &DbPool) -> Result<String, String> {
    Ok(db::get_setting(local_db, ACTIVE_TARGET_SETTING_KEY)
        .await?
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| LOCAL_TARGET_ID.to_string()))
}

pub async fn load_remote_targets(local_db: &DbPool) -> Result<Vec<RemoteTargetConfig>, String> {
    let Some(raw) = db::get_setting(local_db, TARGETS_SETTING_KEY).await? else {
        return Ok(Vec::new());
    };
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&raw).map_err(|e| format!("Failed to parse remote targets: {}", e))
}

pub(super) async fn save_remote_targets(
    local_db: &DbPool,
    targets: &[RemoteTargetConfig],
) -> Result<(), String> {
    let raw = serde_json::to_string(targets).map_err(|e| e.to_string())?;
    db::set_setting(local_db, TARGETS_SETTING_KEY, &raw).await
}

pub async fn load_wsl_targets(local_db: &DbPool) -> Result<Vec<WslTargetConfig>, String> {
    let Some(raw) = db::get_setting(local_db, WSL_TARGETS_SETTING_KEY).await? else {
        return Ok(Vec::new());
    };
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&raw).map_err(|e| format!("Failed to parse WSL targets: {}", e))
}

pub(super) async fn save_wsl_targets(
    local_db: &DbPool,
    targets: &[WslTargetConfig],
) -> Result<(), String> {
    let raw = serde_json::to_string(targets).map_err(|e| e.to_string())?;
    db::set_setting(local_db, WSL_TARGETS_SETTING_KEY, &raw).await
}

pub(super) fn request_to_config(
    request: CreateSshTargetRequest,
    target_id: String,
) -> Result<RemoteTargetConfig, String> {
    let auth_method = request.auth_method.unwrap_or_default();
    if auth_method == SshAuthMethod::Key
        && request
            .passphrase
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    {
        return Err(
            "Passphrase-protected keys are not supported yet. Use ssh-agent or an unencrypted key."
                .to_string(),
        );
    }

    let key_path = match auth_method {
        SshAuthMethod::Key => required_field("keyPath", request.key_path.as_deref().unwrap_or(""))?,
        SshAuthMethod::Password => String::new(),
    };
    let password = match auth_method {
        SshAuthMethod::Key => None,
        SshAuthMethod::Password => Some(required_field(
            "password",
            request.password.as_deref().unwrap_or(""),
        )?),
    };
    let credential_key = match auth_method {
        SshAuthMethod::Key => None,
        SshAuthMethod::Password => Some(credential_key_for_target(&target_id)),
    };

    Ok(RemoteTargetConfig {
        id: target_id,
        label: required_field("label", &request.label)?,
        host: required_field("host", &request.host)?,
        username: required_field("username", &request.username)?,
        port: request.port.unwrap_or(22),
        auth_method,
        key_path,
        credential_key,
        protected_password: None,
        password,
        remote_home: String::new(),
        remote_os: String::new(),
        symlink_enabled: false,
    })
}

pub(super) fn update_request_to_config(
    request: UpdateSshTargetRequest,
    existing: &RemoteTargetConfig,
) -> Result<RemoteTargetConfig, String> {
    let requested_id = required_field("id", &request.id)?;
    if requested_id != existing.id {
        return Err("Target id cannot be changed.".to_string());
    }

    let auth_method = request.auth_method.unwrap_or(existing.auth_method);
    if auth_method == SshAuthMethod::Key
        && request
            .passphrase
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    {
        return Err(
            "Passphrase-protected keys are not supported yet. Use ssh-agent or an unencrypted key."
                .to_string(),
        );
    }

    let key_path = match auth_method {
        SshAuthMethod::Key => required_field("keyPath", request.key_path.as_deref().unwrap_or(""))?,
        SshAuthMethod::Password => String::new(),
    };
    let supplied_password = request
        .password
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let credential_key = match auth_method {
        SshAuthMethod::Key => None,
        SshAuthMethod::Password => Some(
            existing
                .credential_key
                .clone()
                .unwrap_or_else(|| credential_key_for_target(&existing.id)),
        ),
    };
    let protected_password = if auth_method == SshAuthMethod::Password
        && supplied_password.is_none()
        && existing.auth_method == SshAuthMethod::Password
    {
        existing.protected_password.clone()
    } else {
        None
    };
    let password = match auth_method {
        SshAuthMethod::Key => None,
        SshAuthMethod::Password => supplied_password,
    };

    Ok(RemoteTargetConfig {
        id: existing.id.clone(),
        label: required_field("label", &request.label)?,
        host: required_field("host", &request.host)?,
        username: required_field("username", &request.username)?,
        port: request.port.unwrap_or(existing.port),
        auth_method,
        key_path,
        credential_key,
        protected_password,
        password,
        remote_home: existing.remote_home.clone(),
        remote_os: existing.remote_os.clone(),
        symlink_enabled: existing.symlink_enabled,
    })
}

pub(super) fn test_request_to_config(
    request: TestSshTargetRequest,
) -> Result<RemoteTargetConfig, String> {
    let auth_method = request.auth_method.unwrap_or_default();
    if auth_method == SshAuthMethod::Key
        && request
            .passphrase
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    {
        return Err(
            "Passphrase-protected keys are not supported yet. Use ssh-agent or an unencrypted key."
                .to_string(),
        );
    }

    let id = request.id.unwrap_or_else(|| "test".to_string());
    let key_path = match auth_method {
        SshAuthMethod::Key => required_field("keyPath", request.key_path.as_deref().unwrap_or(""))?,
        SshAuthMethod::Password => String::new(),
    };
    let password = match auth_method {
        SshAuthMethod::Key => None,
        SshAuthMethod::Password => Some(required_field(
            "password",
            request.password.as_deref().unwrap_or(""),
        )?),
    };
    let credential_key = match auth_method {
        SshAuthMethod::Key => None,
        SshAuthMethod::Password => Some(credential_key_for_target(&id)),
    };

    Ok(RemoteTargetConfig {
        id,
        label: required_field("label", request.label.as_deref().unwrap_or("SSH target"))?,
        host: required_field("host", request.host.as_deref().unwrap_or(""))?,
        username: required_field("username", request.username.as_deref().unwrap_or(""))?,
        port: request.port.unwrap_or(22),
        auth_method,
        key_path,
        credential_key,
        protected_password: None,
        password,
        remote_home: String::new(),
        remote_os: String::new(),
        symlink_enabled: false,
    })
}

pub(super) fn request_to_wsl_config(
    request: CreateWslTargetRequest,
    target_id: String,
) -> Result<WslTargetConfig, String> {
    Ok(WslTargetConfig {
        id: target_id,
        label: required_field("label", &request.label)?,
        distribution: required_field("distribution", &request.distribution)?,
        remote_home: String::new(),
        remote_os: String::new(),
        symlink_enabled: false,
    })
}

pub(super) fn update_wsl_request_to_config(
    request: UpdateWslTargetRequest,
    existing: &WslTargetConfig,
) -> Result<WslTargetConfig, String> {
    let requested_id = required_field("id", &request.id)?;
    if requested_id != existing.id {
        return Err("Target id cannot be changed.".to_string());
    }

    Ok(WslTargetConfig {
        id: existing.id.clone(),
        label: required_field("label", &request.label)?,
        distribution: required_field("distribution", &request.distribution)?,
        remote_home: existing.remote_home.clone(),
        remote_os: existing.remote_os.clone(),
        symlink_enabled: existing.symlink_enabled,
    })
}

pub(super) fn test_wsl_request_to_config(
    request: TestWslTargetRequest,
) -> Result<WslTargetConfig, String> {
    let id = request.id.unwrap_or_else(|| "wsl-test".to_string());
    Ok(WslTargetConfig {
        id,
        label: required_field("label", request.label.as_deref().unwrap_or("WSL target"))?,
        distribution: required_field(
            "distribution",
            request.distribution.as_deref().unwrap_or(""),
        )?,
        remote_home: String::new(),
        remote_os: String::new(),
        symlink_enabled: false,
    })
}

pub(super) fn required_field(name: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("{} is required.", name))
    } else {
        Ok(trimmed.to_string())
    }
}

pub(super) fn apply_supplied_password_to_existing_target(
    target: &mut RemoteTargetConfig,
    password: Option<&str>,
) -> Result<bool, String> {
    if target.auth_method != SshAuthMethod::Password {
        return Ok(false);
    }

    let Some(password) = password.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(false);
    };
    if target.credential_key.is_none() {
        target.credential_key = Some(credential_key_for_target(&target.id));
    }
    target.password = Some(password.to_string());
    Ok(true)
}

pub fn remote_cache_db_path(target_id: &str) -> Result<PathBuf, String> {
    let target_id = sanitize_target_id(target_id)?;
    Ok(crate::paths::app_data_dir()
        .join("targets")
        .join(target_id)
        .join("db.sqlite"))
}

pub(super) fn sanitize_target_id(target_id: &str) -> Result<String, String> {
    let trimmed = target_id.trim();
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err("Invalid target id.".to_string());
    }
    Ok(trimmed.to_string())
}

pub(super) fn credential_key_for_target(target_id: &str) -> String {
    format!("{}:ssh-password", target_id)
}

pub(super) fn credential_key_for_password_target(target: &RemoteTargetConfig) -> Option<String> {
    if target.auth_method != SshAuthMethod::Password {
        return None;
    }
    Some(
        target
            .credential_key
            .clone()
            .unwrap_or_else(|| credential_key_for_target(&target.id)),
    )
}

pub(super) fn ssh_success_message(
    default_message: &str,
    state: Option<&TargetCredentialState>,
) -> String {
    match state.map(|state| state.status) {
        Some(TargetCredentialStatus::Session) => {
            "SSH password was verified for this session. The system credential store could not be read back, so enter it again after restarting SkillPort.".to_string()
        }
        _ => default_message.to_string(),
    }
}

pub(super) fn load_target_password(target: &RemoteTargetConfig) -> Result<String, String> {
    if let Some(password) = target.password.as_deref().filter(|value| !value.is_empty()) {
        return Ok(password.to_string());
    }
    let Some(credential_key) = credential_key_for_password_target(target) else {
        return Err("Password target is missing its credential key.".to_string());
    };
    match SystemCredentialBackend.get_password(&credential_key) {
        Ok(password) => Ok(password),
        Err(CredentialStoreError::NoEntry) => protected_password_for_target(target)?.ok_or_else(|| {
            format!(
                "SSH password for target '{}' is not available. Open Settings, enter the password for this target, save it, and retry.",
                target.label
            )
        }),
        Err(error) => protected_password_for_target(target)?.ok_or_else(|| error.message()),
    }
}
