//! Batch link/unlink mutations (local loop + remote chunked).

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;

use crate::db::DbPool;
use crate::services::central_mutation::{
    acquire_target_mutation_guard, DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};

use super::super::error::SkillsCliError;
use super::super::placement::{classify_one_observed, ObservedSlot, PlacementPlatform};
use super::super::probe;
use super::super::remote_scripts::{
    VerifiedLinkRemoveStatus, SKILLS_CLI_REMOTE_MUTATION_CHUNK_SIZE,
};
use super::super::{
    check_cancel, is_valid_skill_token, map_guard_error, mapped_inventory_platforms_via_transport,
    SkillsCliPlacementMutationFailure, SkillsCliPlacementMutationItem,
    SkillsCliPlacementMutationOutcome, SkillsCliTransport,
};
use super::{
    decide_link, decide_unlink, link_platform, unlink_platform, LinkOp, PlacementAction, UnlinkOp,
    LINK_LOCK_OPERATION, UNLINK_LOCK_OPERATION,
};

pub(crate) async fn link_platforms_batch(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    items: &[(String, String)],
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliPlacementMutationOutcome, SkillsCliError> {
    mutate_platforms_batch(tx, pool, items, cancel, PlacementAction::Link).await
}

pub(crate) async fn unlink_platforms_batch(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    items: &[(String, String)],
    force: bool,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliPlacementMutationOutcome, SkillsCliError> {
    mutate_platforms_batch(tx, pool, items, cancel, PlacementAction::Unlink { force }).await
}

struct BatchJob {
    skill_name: String,
    agent_id: String,
    slot: String,
    canonical: String,
    failed_code: Option<String>,
}

fn batch_item(job: &BatchJob) -> SkillsCliPlacementMutationItem {
    SkillsCliPlacementMutationItem {
        skill_name: job.skill_name.clone(),
        agent_id: job.agent_id.clone(),
    }
}

fn push_failed(outcome: &mut SkillsCliPlacementMutationOutcome, job: &BatchJob, code: String) {
    outcome.failed.push(SkillsCliPlacementMutationFailure {
        skill_name: job.skill_name.clone(),
        agent_id: job.agent_id.clone(),
        error_code: code,
    });
}

async fn mutate_platforms_batch(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    items: &[(String, String)],
    cancel: Option<&AtomicBool>,
    action: PlacementAction,
) -> Result<SkillsCliPlacementMutationOutcome, SkillsCliError> {
    if !tx.is_remote() {
        return mutate_platforms_batch_local(tx, pool, items, cancel, action).await;
    }
    check_cancel(cancel)?;
    let operation = match action {
        PlacementAction::Link => LINK_LOCK_OPERATION,
        PlacementAction::Unlink { .. } => UNLINK_LOCK_OPERATION,
    };
    let _guard = acquire_target_mutation_guard(
        &tx.mutation_target(),
        operation,
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
    .map_err(map_guard_error)?;
    check_cancel(cancel)?;

    let platforms = mapped_inventory_platforms_via_transport(tx, pool).await?;
    let placement_platforms: Vec<_> = platforms
        .iter()
        .map(|item| item.as_placement_platform())
        .collect();
    let by_agent: HashMap<&str, &PlacementPlatform> = placement_platforms
        .iter()
        .map(|platform| (platform.agent_id.as_str(), platform))
        .collect();
    let paths = tx.paths();
    let mut probe_paths = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut jobs = Vec::new();
    for (skill_name, agent_id) in items {
        if !is_valid_skill_token(skill_name) {
            jobs.push(BatchJob {
                skill_name: skill_name.clone(),
                agent_id: agent_id.clone(),
                slot: String::new(),
                canonical: String::new(),
                failed_code: Some(SkillsCliError::SkillNotOwned.ipc_code().to_string()),
            });
            continue;
        }
        let Some(platform) = by_agent.get(agent_id.as_str()) else {
            jobs.push(BatchJob {
                skill_name: skill_name.clone(),
                agent_id: agent_id.clone(),
                slot: String::new(),
                canonical: String::new(),
                failed_code: Some(
                    SkillsCliError::AgentUnmapped(agent_id.clone())
                        .ipc_code()
                        .to_string(),
                ),
            });
            continue;
        };
        let canonical = paths.join_child(paths.canonical_root(), skill_name);
        let slot = paths.join_child(&platform.global_skills_dir.to_string_lossy(), skill_name);
        if seen.insert(canonical.clone()) {
            probe_paths.push(canonical.clone());
        }
        if seen.insert(slot.clone()) {
            probe_paths.push(slot.clone());
        }
        jobs.push(BatchJob {
            skill_name: skill_name.clone(),
            agent_id: agent_id.clone(),
            slot,
            canonical,
            failed_code: None,
        });
    }

    check_cancel(cancel)?;
    let probes = if probe_paths.is_empty() {
        Vec::new()
    } else {
        tx.fs().probe_paths(&probe_paths).await?
    };
    let probe_map = probe::index_probes(&probes);
    let link_kind = tx.managed_link_kind();
    let posix = paths.uses_posix();

    let mut outcome = SkillsCliPlacementMutationOutcome::default();
    let mut mutate_indexes: Vec<usize> = Vec::new();
    for (index, job) in jobs.iter().enumerate() {
        if let Some(code) = &job.failed_code {
            push_failed(&mut outcome, job, code.clone());
            continue;
        }
        let Some(platform) = by_agent.get(job.agent_id.as_str()) else {
            push_failed(
                &mut outcome,
                job,
                SkillsCliError::AgentUnmapped(job.agent_id.clone())
                    .ipc_code()
                    .to_string(),
            );
            continue;
        };
        let canonical_owned = probe::canonical_owned_from_probe(probe_map.get(&job.canonical));
        let observed = probe_map
            .get(&job.slot)
            .map(|item| probe::observed_slot_from_probe(item, &job.canonical, link_kind, posix))
            .unwrap_or(ObservedSlot::Absent);
        let current = classify_one_observed(canonical_owned, observed, platform, job.slot.clone());
        match action {
            PlacementAction::Link => match decide_link(current.state) {
                Ok(LinkOp::Noop) => outcome.skipped.push(batch_item(job)),
                Ok(LinkOp::Create) => mutate_indexes.push(index),
                Err(error) => push_failed(&mut outcome, job, error.ipc_code().to_string()),
            },
            PlacementAction::Unlink { force } => match decide_unlink(current.state, force) {
                Ok(UnlinkOp::Noop) => outcome.skipped.push(batch_item(job)),
                Ok(UnlinkOp::Remove) => mutate_indexes.push(index),
                Err(SkillsCliError::DirectCopyNotToggleable) => {
                    outcome.skipped.push(batch_item(job));
                }
                Err(error) => push_failed(&mut outcome, job, error.ipc_code().to_string()),
            },
        }
    }

    let mut offset = 0usize;
    while offset < mutate_indexes.len() {
        check_cancel(cancel)?;
        let end = (offset + SKILLS_CLI_REMOTE_MUTATION_CHUNK_SIZE).min(mutate_indexes.len());
        let chunk = &mutate_indexes[offset..end];
        let fail_rest = |outcome: &mut SkillsCliPlacementMutationOutcome,
                         jobs: &[BatchJob],
                         from: usize,
                         code: String| {
            for index in &mutate_indexes[from..] {
                push_failed(outcome, &jobs[*index], code.clone());
            }
        };
        match action {
            PlacementAction::Link => {
                let pairs: Vec<(String, String)> = chunk
                    .iter()
                    .map(|index| (jobs[*index].canonical.clone(), jobs[*index].slot.clone()))
                    .collect();
                match tx.fs().create_managed_links(&pairs).await {
                    Ok(()) => {
                        for index in chunk {
                            outcome.succeeded.push(batch_item(&jobs[*index]));
                        }
                    }
                    Err(error) => {
                        fail_rest(&mut outcome, &jobs, offset, error.ipc_code().to_string());
                        break;
                    }
                }
            }
            PlacementAction::Unlink { .. } => {
                let links: Vec<String> = chunk
                    .iter()
                    .map(|index| jobs[*index].slot.clone())
                    .collect();
                match tx.fs().remove_verified_links(&links).await {
                    Ok(results) => {
                        for (index, (_, status)) in chunk.iter().zip(results) {
                            match status {
                                VerifiedLinkRemoveStatus::Removed => {
                                    outcome.succeeded.push(batch_item(&jobs[*index]));
                                }
                                VerifiedLinkRemoveStatus::SkippedNotLink
                                | VerifiedLinkRemoveStatus::Absent => {
                                    outcome.skipped.push(batch_item(&jobs[*index]));
                                }
                            }
                        }
                    }
                    Err(error) => {
                        fail_rest(&mut outcome, &jobs, offset, error.ipc_code().to_string());
                        break;
                    }
                }
            }
        }
        offset = end;
    }
    Ok(outcome)
}

async fn mutate_platforms_batch_local(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    items: &[(String, String)],
    cancel: Option<&AtomicBool>,
    action: PlacementAction,
) -> Result<SkillsCliPlacementMutationOutcome, SkillsCliError> {
    let mut outcome = SkillsCliPlacementMutationOutcome::default();
    for (skill_name, agent_id) in items {
        check_cancel(cancel)?;
        let result = match action {
            PlacementAction::Link => link_platform(tx, pool, skill_name, agent_id, cancel).await,
            PlacementAction::Unlink { force } => {
                unlink_platform(tx, pool, skill_name, agent_id, force, cancel).await
            }
        };
        match result {
            Ok(_) => outcome.succeeded.push(SkillsCliPlacementMutationItem {
                skill_name: skill_name.clone(),
                agent_id: agent_id.clone(),
            }),
            Err(SkillsCliError::DirectCopyNotToggleable)
                if matches!(action, PlacementAction::Unlink { .. }) =>
            {
                outcome.skipped.push(SkillsCliPlacementMutationItem {
                    skill_name: skill_name.clone(),
                    agent_id: agent_id.clone(),
                });
            }
            Err(error) => outcome.failed.push(SkillsCliPlacementMutationFailure {
                skill_name: skill_name.clone(),
                agent_id: agent_id.clone(),
                error_code: error.ipc_code().to_string(),
            }),
        }
    }
    Ok(outcome)
}
