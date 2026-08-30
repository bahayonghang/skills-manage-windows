use super::config::{
    load_target_config_snapshot, persist_target_deletion_settings, restore_target_settings,
};
use super::*;
pub async fn create_ssh_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    request: CreateSshTargetRequest,
) -> Result<TargetSummary, TargetsError> {
    let target_id = format!("ssh-{}", Uuid::new_v4());
    let base = request_to_config(request, target_id)?;
    let probe = probe_ssh_target(&base).await?;
    if !is_supported_remote_os(&probe.remote_os) {
        return Err(TargetsError::UnsupportedRemoteOs(probe.remote_os));
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
) -> Result<TargetSummary, TargetsError> {
    let mut targets = load_remote_targets(local_db).await?;
    let index = targets
        .iter()
        .position(|target| target.id == request.id)
        .ok_or_else(|| TargetsError::TargetNotFound(request.id.clone()))?;
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
        return Err(TargetsError::UnsupportedRemoteOs(probe.remote_os));
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
) -> Result<SshTargetTestResult, TargetsError> {
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
            .ok_or_else(|| TargetsError::TargetNotFound(id.to_string()))?,
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
            message: error.to_string(),
        }),
    }
}

pub async fn update_ssh_target_password_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target_id: &str,
    password: &str,
) -> Result<SshTargetTestResult, TargetsError> {
    let mut targets = load_remote_targets(local_db).await?;
    let target = targets
        .iter_mut()
        .find(|target| target.id == target_id)
        .ok_or_else(|| TargetsError::TargetNotFound(target_id.to_string()))?;

    if target.auth_method != SshAuthMethod::Password {
        return Err(TargetsError::NotPasswordAuth);
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
            message: error.to_string(),
        }),
    }
}

pub async fn create_wsl_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    request: CreateWslTargetRequest,
) -> Result<TargetSummary, TargetsError> {
    let target_id = format!("wsl-{}", Uuid::new_v4());
    let mut target = request_to_wsl_config(request, target_id)?;
    let probe = probe_wsl_target(&target).await?;
    if !is_supported_remote_os(&probe.remote_os) {
        return Err(TargetsError::UnsupportedWslOs(probe.remote_os));
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
) -> Result<TargetSummary, TargetsError> {
    let mut targets = load_wsl_targets(local_db).await?;
    let index = targets
        .iter()
        .position(|target| target.id == request.id)
        .ok_or_else(|| TargetsError::TargetNotFound(request.id.clone()))?;
    let mut updated_target = update_wsl_request_to_config(request, &targets[index])?;

    let probe = probe_wsl_target(&updated_target).await?;
    if !is_supported_remote_os(&probe.remote_os) {
        return Err(TargetsError::UnsupportedWslOs(probe.remote_os));
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
) -> Result<WslTargetTestResult, TargetsError> {
    let target = match request.id.as_deref() {
        Some(id) if !id.trim().is_empty() => load_wsl_targets(local_db)
            .await?
            .into_iter()
            .find(|target| target.id == id)
            .ok_or_else(|| TargetsError::TargetNotFound(id.to_string()))?,
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
            message: error.to_string(),
        }),
    }
}

pub async fn delete_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target_id: &str,
) -> Result<(), TargetsError> {
    if target_id == LOCAL_TARGET_ID {
        return Err(TargetsError::LocalTargetUndeletable);
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
        return Err(TargetsError::TargetNotFound(target_id.to_string()));
    }
    let session_credential = removed_ssh
        .as_ref()
        .and_then(credential_key_for_password_target)
        .and_then(|credential_key| {
            registry
                .get_session_password(&credential_key)
                .map(|password| (credential_key, password))
        });

    let reset_active = active_target_id(local_db).await? == target_id;
    let original_settings =
        persist_target_deletion_settings(local_db, &ssh_targets, &wsl_targets, reset_active)
            .await?;

    if let Some(mut removed) = removed_ssh {
        if let Err(credential_error) = registry.delete_target_password(&mut removed) {
            if let Err(rollback_error) = restore_target_settings(local_db, &original_settings).await
            {
                tracing::error!(
                    target_id,
                    "Failed to restore target settings after credential deletion failure"
                );
                return Err(rollback_error.into());
            }
            if let Some((credential_key, password)) = session_credential {
                registry.set_session_password(&credential_key, &password)?;
            }
            return Err(credential_error);
        }
    }
    registry.drop_remote_pool(target_id);
    Ok(())
}

pub async fn set_active_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target_id: &str,
) -> Result<TargetSummary, TargetsError> {
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
            return Err(TargetsError::TargetNotFound(target_id.to_string()));
        }
    }

    db::set_setting(local_db, ACTIVE_TARGET_SETTING_KEY, target_id).await?;
    let targets = registry.list_targets(local_db).await?;
    targets
        .into_iter()
        .find(|target| target.id == target_id)
        .ok_or_else(|| TargetsError::TargetNotFound(target_id.to_string()))
}

pub async fn get_active_target_impl(
    registry: &TargetRegistry,
    local_db: &DbPool,
) -> Result<TargetSummary, TargetsError> {
    let active_id = active_target_id(local_db).await?;
    registry
        .list_targets(local_db)
        .await?
        .into_iter()
        .find(|target| target.id == active_id)
        .ok_or_else(|| TargetsError::TargetNotFound(active_id))
}

pub async fn active_target_id(local_db: &DbPool) -> Result<String, TargetsError> {
    Ok(load_target_config_snapshot(local_db)
        .await?
        .active_target_id)
}

pub async fn load_remote_targets(
    local_db: &DbPool,
) -> Result<Vec<RemoteTargetConfig>, TargetsError> {
    Ok(load_target_config_snapshot(local_db).await?.ssh_targets)
}

pub(super) async fn save_remote_targets(
    local_db: &DbPool,
    targets: &[RemoteTargetConfig],
) -> Result<(), TargetsError> {
    let raw = serde_json::to_string(targets)?;
    Ok(db::set_setting(local_db, TARGETS_SETTING_KEY, &raw).await?)
}

pub async fn load_wsl_targets(local_db: &DbPool) -> Result<Vec<WslTargetConfig>, TargetsError> {
    Ok(load_target_config_snapshot(local_db).await?.wsl_targets)
}

pub(super) async fn save_wsl_targets(
    local_db: &DbPool,
    targets: &[WslTargetConfig],
) -> Result<(), TargetsError> {
    let raw = serde_json::to_string(targets)?;
    Ok(db::set_setting(local_db, WSL_TARGETS_SETTING_KEY, &raw).await?)
}

pub(super) fn request_to_config(
    request: CreateSshTargetRequest,
    target_id: String,
) -> Result<RemoteTargetConfig, TargetsError> {
    let auth_method = request.auth_method.unwrap_or_default();
    if auth_method == SshAuthMethod::Key
        && request
            .passphrase
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    {
        return Err(TargetsError::PassphraseUnsupported);
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
) -> Result<RemoteTargetConfig, TargetsError> {
    let requested_id = required_field("id", &request.id)?;
    if requested_id != existing.id {
        return Err(TargetsError::TargetIdImmutable);
    }

    let auth_method = request.auth_method.unwrap_or(existing.auth_method);
    if auth_method == SshAuthMethod::Key
        && request
            .passphrase
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    {
        return Err(TargetsError::PassphraseUnsupported);
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
) -> Result<RemoteTargetConfig, TargetsError> {
    let auth_method = request.auth_method.unwrap_or_default();
    if auth_method == SshAuthMethod::Key
        && request
            .passphrase
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    {
        return Err(TargetsError::PassphraseUnsupported);
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
) -> Result<WslTargetConfig, TargetsError> {
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
) -> Result<WslTargetConfig, TargetsError> {
    let requested_id = required_field("id", &request.id)?;
    if requested_id != existing.id {
        return Err(TargetsError::TargetIdImmutable);
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
) -> Result<WslTargetConfig, TargetsError> {
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

pub(super) fn required_field(name: &str, value: &str) -> Result<String, TargetsError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(TargetsError::RequiredField(name.to_string()))
    } else {
        Ok(trimmed.to_string())
    }
}

pub(super) fn apply_supplied_password_to_existing_target(
    target: &mut RemoteTargetConfig,
    password: Option<&str>,
) -> Result<bool, TargetsError> {
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

pub fn remote_cache_db_path(target_id: &str) -> Result<PathBuf, TargetsError> {
    let target_id = sanitize_target_id(target_id)?;
    Ok(crate::paths::app_data_dir()
        .join("targets")
        .join(target_id)
        .join("db.sqlite"))
}

pub(super) fn sanitize_target_id(target_id: &str) -> Result<String, TargetsError> {
    let trimmed = target_id.trim();
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(TargetsError::InvalidTargetId);
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

pub(super) fn load_target_password(target: &RemoteTargetConfig) -> Result<String, TargetsError> {
    if let Some(password) = target.password.as_deref().filter(|value| !value.is_empty()) {
        return Ok(password.to_string());
    }
    let Some(credential_key) = credential_key_for_password_target(target) else {
        return Err(TargetsError::MissingCredentialKey);
    };
    match SystemCredentialBackend.get_password(&credential_key) {
        Ok(password) => Ok(password),
        Err(CredentialStoreError::NoEntry) => protected_password_for_target(target)?
            .ok_or_else(|| TargetsError::PasswordUnavailable(target.label.clone())),
        Err(error) => protected_password_for_target(target)?
            .ok_or_else(|| TargetsError::CredentialStore(error.message())),
    }
}
