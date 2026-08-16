//! Single business implementation of skill install / uninstall against the
//! active target. Transport differences (Local vs SSH/WSL) live entirely in
//! the [`InstallTransport`] hooks; adding a new operation means writing this
//! orchestration once, not once per transport.

use crate::db::{self, DbPool};

use super::error::InstallationError;
use super::native;
use super::skip::record_installation;
use super::transport::InstallTransport;
use super::types::{InstallOutcome, InstallResult};

/// Install a skill to an agent over the given transport.
///
/// Skeleton (shared once): central-target guard → agent/central lookup →
/// source validation → shared-root native shortcut → method resolution →
/// target preparation → skip detection → placement → DB record.
pub async fn install_skill(
    pool: &DbPool,
    transport: &InstallTransport,
    skill_id: &str,
    agent_id: &str,
    method: &str,
) -> Result<InstallOutcome, InstallationError> {
    let target = transport.active_target();
    let _guard = crate::services::central_mutation::acquire_target_mutation_guard(
        &target,
        "install skill",
        crate::services::central_mutation::DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await?;
    reject_pending_recovery(pool, &target, skill_id).await?;
    install_skill_under_guard(pool, transport, skill_id, agent_id, method).await
}

pub(crate) async fn install_skill_under_guard(
    pool: &DbPool,
    transport: &InstallTransport,
    skill_id: &str,
    agent_id: &str,
    method: &str,
) -> Result<InstallOutcome, InstallationError> {
    // Guard: cannot install to the central agent itself.
    if agent_id == "central" {
        return Err(InstallationError::CentralAgentTarget);
    }

    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| InstallationError::AgentNotFound(agent_id.to_string()))?;
    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or(InstallationError::CentralAgentMissing)?;

    let source = transport.validate_source(pool, skill_id).await?;

    // Shared canonical root: centralizing is the whole install; only a
    // native DB record is needed.
    if transport.shares_central_root(&agent, &central) {
        let canonical_dir = transport
            .centralize_shared_root(pool, skill_id, &central)
            .await?;
        record_installation(
            pool,
            skill_id,
            agent_id,
            std::path::Path::new(&canonical_dir),
            "native",
            None,
        )
        .await?;
        return Ok(InstallOutcome::Installed(InstallResult {
            symlink_path: canonical_dir,
        }));
    }

    let resolved = transport.resolve_method(method)?;
    transport
        .prepare_target(pool, skill_id, &agent, &central)
        .await?;

    if let Some(skipped) = transport
        .detect_existing(pool, skill_id, agent_id, &agent, &central)
        .await?
    {
        return Ok(InstallOutcome::Skipped(skipped));
    }

    let placement = transport
        .place_install(pool, source, &agent, &central, skill_id, resolved)
        .await?;
    record_installation(
        pool,
        skill_id,
        agent_id,
        std::path::Path::new(&placement.installed_path),
        placement.link_type,
        placement.symlink_target,
    )
    .await?;

    Ok(InstallOutcome::Installed(InstallResult {
        symlink_path: placement.installed_path,
    }))
}

/// Uninstall a skill from an agent over the given transport.
pub async fn uninstall_skill(
    pool: &DbPool,
    transport: &InstallTransport,
    skill_id: &str,
    agent_id: &str,
    row_id: Option<&str>,
) -> Result<(), InstallationError> {
    let target = transport.active_target();
    let _guard = crate::services::central_mutation::acquire_target_mutation_guard(
        &target,
        "uninstall skill",
        crate::services::central_mutation::DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await?;
    reject_pending_recovery(pool, &target, skill_id).await?;
    uninstall_skill_under_guard(pool, transport, skill_id, agent_id, row_id).await
}

pub(crate) async fn uninstall_skill_under_guard(
    pool: &DbPool,
    transport: &InstallTransport,
    skill_id: &str,
    agent_id: &str,
    row_id: Option<&str>,
) -> Result<(), InstallationError> {
    // Observation-row uninstall is a local-only surface; the remote command
    // path has always ignored row_id.
    if let (InstallTransport::Local, Some(row_id)) = (transport, row_id) {
        return native::uninstall_observation_from_agent_impl(pool, skill_id, agent_id, row_id)
            .await;
    }

    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| InstallationError::AgentNotFound(agent_id.to_string()))?;
    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or(InstallationError::CentralAgentMissing)?;

    if agent_id == "central" || transport.shares_central_root(&agent, &central) {
        return Err(InstallationError::SharedCentralUninstall {
            display_name: agent.display_name,
        });
    }

    // Capture the recorded install path up front: after a successful removal
    // the matching scanner observation row is deleted too, so the
    // unused-skills report does not keep showing a stale platform entry
    // until the next scan (R4/D2).
    let installed_path = db::get_skill_installations(pool, skill_id)
        .await?
        .into_iter()
        .find(|record| record.agent_id == agent_id)
        .map(|record| record.installed_path);

    let observation_row_ids = if let Some(installed_path) = installed_path.as_deref() {
        db::get_agent_skill_observations(pool, agent_id)
            .await?
            .into_iter()
            .filter(|observation| {
                crate::paths::paths_equivalent(
                    std::path::Path::new(&observation.dir_path),
                    std::path::Path::new(installed_path),
                )
            })
            .map(|observation| observation.row_id)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    transport.remove_install(pool, &agent, skill_id).await?;
    db::delete_skill_installation_with_observations(pool, skill_id, agent_id, &observation_row_ids)
        .await?;
    Ok(())
}

pub(crate) async fn reject_pending_recovery(
    pool: &DbPool,
    target: &crate::targets::ActiveTarget,
    skill_id: &str,
) -> Result<(), InstallationError> {
    let pending = db::list_pending_fs_db_operations(pool, target.id()).await?;
    if pending
        .iter()
        .any(|operation| operation.skill_id == skill_id)
    {
        return Err(InstallationError::PendingCentralRecovery);
    }
    Ok(())
}
