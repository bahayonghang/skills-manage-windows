//! Post-probe persist, credential, and cache mutation for SSH/WSL create/update.
//! Probe/transport stay in `commands.rs`; this module owns the all-or-nothing
//! settings + SecretStore + remote-pool outcome.

use std::collections::HashMap;

use super::config::{load_target_config_snapshot, restore_target_settings};
use super::*;

pub(super) enum RemoteCacheInit {
    Open,
    #[cfg(test)]
    Fail,
    #[cfg(test)]
    Prefill(DbPool),
}

pub(super) async fn persist_new_ssh_target(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target: RemoteTargetConfig,
) -> Result<TargetSummary, TargetsError> {
    persist_new_ssh_target_with_cache(registry, local_db, target, RemoteCacheInit::Open).await
}

pub(super) async fn persist_new_ssh_target_with_cache(
    registry: &TargetRegistry,
    local_db: &DbPool,
    mut target: RemoteTargetConfig,
    cache_init: RemoteCacheInit,
) -> Result<TargetSummary, TargetsError> {
    let credential_state = if target.auth_method == SshAuthMethod::Password {
        Some(registry.save_target_password(&mut target)?)
    } else {
        None
    };
    if target.auth_method == SshAuthMethod::Password {
        target.password = None;
    }

    let settings_before = target_settings_snapshot(local_db).await?;
    let mut targets = load_remote_targets(local_db).await?;
    let stored_target = target;
    targets.push(stored_target.clone());
    if let Err(error) = save_remote_targets(local_db, &targets).await {
        rollback_new_ssh_credential(registry, &stored_target);
        return Err(error);
    }

    if let Err(error) = run_remote_cache_init(
        registry,
        &stored_target.id,
        &stored_target.remote_home,
        cache_init,
    )
    .await
    {
        rollback_created_ssh(registry, local_db, &settings_before, &stored_target).await?;
        return Err(error);
    }

    ssh_target_summary(registry, local_db, &stored_target, credential_state).await
}

pub(super) async fn persist_updated_ssh_target(
    registry: &TargetRegistry,
    local_db: &DbPool,
    previous_target: RemoteTargetConfig,
    updated_target: RemoteTargetConfig,
    supplied_password: bool,
) -> Result<TargetSummary, TargetsError> {
    persist_updated_ssh_target_with_cache(
        registry,
        local_db,
        previous_target,
        updated_target,
        supplied_password,
        RemoteCacheInit::Open,
    )
    .await
}

pub(super) async fn persist_updated_ssh_target_with_cache(
    registry: &TargetRegistry,
    local_db: &DbPool,
    previous_target: RemoteTargetConfig,
    mut updated_target: RemoteTargetConfig,
    supplied_password: bool,
    cache_init: RemoteCacheInit,
) -> Result<TargetSummary, TargetsError> {
    let mut previous_with_secret = previous_target.clone();
    registry.attach_available_password(&mut previous_with_secret);
    let previous_password = previous_with_secret.password.clone();

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

    let settings_before = target_settings_snapshot(local_db).await?;
    let mut targets = load_remote_targets(local_db).await?;
    let index = targets
        .iter()
        .position(|target| target.id == updated_target.id)
        .ok_or_else(|| TargetsError::TargetNotFound(updated_target.id.clone()))?;
    targets[index] = updated_target.clone();
    if let Err(error) = save_remote_targets(local_db, &targets).await {
        restore_ssh_password(registry, &previous_target, previous_password.as_deref());
        return Err(error);
    }

    if previous_target.auth_method == SshAuthMethod::Password
        && updated_target.auth_method != SshAuthMethod::Password
    {
        let mut cleanup_target = previous_target.clone();
        if let Err(credential_error) = registry.delete_target_password(&mut cleanup_target) {
            rollback_updated_ssh(
                registry,
                local_db,
                &settings_before,
                &previous_target,
                previous_password.as_deref(),
                &updated_target.id,
            )
            .await?;
            return Err(credential_error);
        }
    }

    registry.drop_remote_pool(&updated_target.id);
    if let Err(error) = run_remote_cache_init(
        registry,
        &updated_target.id,
        &updated_target.remote_home,
        cache_init,
    )
    .await
    {
        rollback_updated_ssh(
            registry,
            local_db,
            &settings_before,
            &previous_target,
            previous_password.as_deref(),
            &updated_target.id,
        )
        .await?;
        return Err(error);
    }

    ssh_target_summary(registry, local_db, &updated_target, credential_state).await
}

pub(super) async fn persist_new_wsl_target(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target: WslTargetConfig,
) -> Result<TargetSummary, TargetsError> {
    persist_new_wsl_target_with_cache(registry, local_db, target, RemoteCacheInit::Open).await
}

pub(super) async fn persist_new_wsl_target_with_cache(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target: WslTargetConfig,
    cache_init: RemoteCacheInit,
) -> Result<TargetSummary, TargetsError> {
    let settings_before = target_settings_snapshot(local_db).await?;
    let mut targets = load_wsl_targets(local_db).await?;
    targets.push(target.clone());
    save_wsl_targets(local_db, &targets).await?;
    if let Err(error) =
        run_remote_cache_init(registry, &target.id, &target.remote_home, cache_init).await
    {
        rollback_wsl_mutation(registry, local_db, &settings_before, &target.id).await?;
        return Err(error);
    }

    wsl_target_summary(registry, local_db, &target).await
}

pub(super) async fn persist_updated_wsl_target(
    registry: &TargetRegistry,
    local_db: &DbPool,
    updated_target: WslTargetConfig,
) -> Result<TargetSummary, TargetsError> {
    persist_updated_wsl_target_with_cache(registry, local_db, updated_target, RemoteCacheInit::Open)
        .await
}

pub(super) async fn persist_updated_wsl_target_with_cache(
    registry: &TargetRegistry,
    local_db: &DbPool,
    updated_target: WslTargetConfig,
    cache_init: RemoteCacheInit,
) -> Result<TargetSummary, TargetsError> {
    let settings_before = target_settings_snapshot(local_db).await?;
    let mut targets = load_wsl_targets(local_db).await?;
    let index = targets
        .iter()
        .position(|target| target.id == updated_target.id)
        .ok_or_else(|| TargetsError::TargetNotFound(updated_target.id.clone()))?;
    targets[index] = updated_target.clone();
    save_wsl_targets(local_db, &targets).await?;
    registry.drop_remote_pool(&updated_target.id);
    if let Err(error) = run_remote_cache_init(
        registry,
        &updated_target.id,
        &updated_target.remote_home,
        cache_init,
    )
    .await
    {
        rollback_wsl_mutation(registry, local_db, &settings_before, &updated_target.id).await?;
        return Err(error);
    }

    wsl_target_summary(registry, local_db, &updated_target).await
}

async fn run_remote_cache_init(
    registry: &TargetRegistry,
    target_id: &str,
    remote_home: &str,
    cache_init: RemoteCacheInit,
) -> Result<(), TargetsError> {
    match cache_init {
        RemoteCacheInit::Open => registry
            .remote_db_for(target_id, remote_home)
            .await
            .map(|_| ()),
        #[cfg(test)]
        RemoteCacheInit::Fail => Err(TargetsError::io(
            "injected remote cache init failure",
            std::io::Error::other("injected remote cache init failure"),
        )),
        #[cfg(test)]
        RemoteCacheInit::Prefill(pool) => {
            registry.insert_test_pool(target_id, pool);
            Ok(())
        }
    }
}

async fn target_settings_snapshot(
    local_db: &DbPool,
) -> Result<HashMap<String, Option<String>>, TargetsError> {
    Ok(db::get_settings(
        local_db,
        &[
            TARGETS_SETTING_KEY.to_string(),
            WSL_TARGETS_SETTING_KEY.to_string(),
            ACTIVE_TARGET_SETTING_KEY.to_string(),
        ],
    )
    .await?)
}

async fn rollback_created_ssh(
    registry: &TargetRegistry,
    local_db: &DbPool,
    settings_before: &HashMap<String, Option<String>>,
    target: &RemoteTargetConfig,
) -> Result<(), TargetsError> {
    if let Err(rollback_error) = restore_target_settings(local_db, settings_before).await {
        rollback_new_ssh_credential(registry, target);
        registry.drop_remote_pool(&target.id);
        return Err(rollback_error.into());
    }
    rollback_new_ssh_credential(registry, target);
    registry.drop_remote_pool(&target.id);
    Ok(())
}

async fn rollback_updated_ssh(
    registry: &TargetRegistry,
    local_db: &DbPool,
    settings_before: &HashMap<String, Option<String>>,
    previous_target: &RemoteTargetConfig,
    previous_password: Option<&str>,
    target_id: &str,
) -> Result<(), TargetsError> {
    if let Err(rollback_error) = restore_target_settings(local_db, settings_before).await {
        restore_ssh_password(registry, previous_target, previous_password);
        registry.drop_remote_pool(target_id);
        return Err(rollback_error.into());
    }
    restore_ssh_password(registry, previous_target, previous_password);
    registry.drop_remote_pool(target_id);
    Ok(())
}

async fn rollback_wsl_mutation(
    registry: &TargetRegistry,
    local_db: &DbPool,
    settings_before: &HashMap<String, Option<String>>,
    target_id: &str,
) -> Result<(), TargetsError> {
    restore_target_settings(local_db, settings_before).await?;
    registry.drop_remote_pool(target_id);
    Ok(())
}

fn rollback_new_ssh_credential(registry: &TargetRegistry, target: &RemoteTargetConfig) {
    let mut cleanup_target = target.clone();
    let _ = registry.delete_target_password(&mut cleanup_target);
}

fn restore_ssh_password(
    registry: &TargetRegistry,
    previous_target: &RemoteTargetConfig,
    previous_password: Option<&str>,
) {
    let mut previous_target = previous_target.clone();
    if previous_target.auth_method == SshAuthMethod::Password {
        if let Some(password) = previous_password.filter(|password| !password.is_empty()) {
            previous_target.password = Some(password.to_string());
            let _ = registry.save_target_password(&mut previous_target);
            return;
        }
    }
    let _ = registry.delete_target_password(&mut previous_target);
}

async fn ssh_target_summary(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target: &RemoteTargetConfig,
    credential_state: Option<TargetCredentialState>,
) -> Result<TargetSummary, TargetsError> {
    let active_id = active_target_id(local_db).await?;
    let mut summary = registry.target_summary(target, active_id.as_str());
    if let Some(state) = credential_state {
        summary.credential_status = Some(state.status);
        summary.credential_error = state.error;
        summary.has_stored_password = Some(state.status.is_available());
    }
    Ok(summary)
}

async fn wsl_target_summary(
    registry: &TargetRegistry,
    local_db: &DbPool,
    target: &WslTargetConfig,
) -> Result<TargetSummary, TargetsError> {
    let active_id = active_target_id(local_db).await?;
    Ok(registry.wsl_target_summary(target, active_id.as_str()))
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
