//! Skills CLI global management service.
//!
//! Wraps the official `skills` npm package ([`SKILLS_CLI_NPM_SPEC`], npm `latest`) for
//! the `-g` lifecycle. Local and Remote share one transport seam. Install and
//! update capabilities are open on Remote except RevealFolder and
//! `install_origin` guessing.

mod agent_map;
mod argv;
mod error;
mod export;
mod files;
mod inventory;
mod link;
mod lock;
mod placement;
mod probe;
mod remote_scripts;
mod remove;
mod runner;
mod transport;
mod types;
pub mod updates;

pub use agent_map::{
    cli_agent_for_skillport_id, is_explicitly_unsupported, map_skillport_ids_to_cli_agents,
    SKILLS_CLI_AGENT_MAP, SKILLS_CLI_UNSUPPORTED,
};
pub use argv::{
    build_add_global_argv, build_list_global_argv, build_node_version_argv, build_preview_argv,
    build_probe_argv, build_remove_global_argv, is_node_version_supported, parse_node_version,
    parse_skill_source, resolve_node_launcher, resolve_node_program, NodeLauncher, SkillSource,
    SKILLS_CLI_MIN_NODE_DISPLAY, SKILLS_CLI_NPM_SPEC,
};
pub use error::SkillsCliError;
pub(crate) use export::export_inventory;
pub(crate) use files::{read_skill_md, reveal_skill_folder};
pub(crate) use link::{
    link_platform, link_platforms_batch, unlink_platform, unlink_platforms_batch,
};
pub use lock::{
    annotate_platform_install_origins, annotate_platform_install_origins_with,
    classify_local_path_origin, is_mapped_agent_lock_copy, is_path_inside_owned_canonical,
    load_cli_lock_ownership, resolved_link_target, skills_cli_lock_path,
    skills_cli_lock_path_from_env, CliLockEntry, CliLockOwnership, LinkOrigin,
};
pub(crate) use remove::{preview_remove_global, remove_global};
pub(crate) use runner::{bulk_transfer_policy, standard_policy, SkillsCliRunner};
pub use transport::{SkillsCliCapability, SkillsCliTransport};
pub use types::{
    SkillsCliAddResult, SkillsCliDoctorReport, SkillsCliGlobalSkill, SkillsCliGlobalSnapshot,
    SkillsCliInstallKind, SkillsCliInstallTarget, SkillsCliManagedLinkKind, SkillsCliPlacement,
    SkillsCliPlacementBatchItem, SkillsCliPlacementConflict, SkillsCliPlacementMutationFailure,
    SkillsCliPlacementMutationItem, SkillsCliPlacementMutationOutcome, SkillsCliPlacementState,
    SkillsCliRemovePlacementSummary, SkillsCliRemovePlan, SkillsCliRemoveResult, SkillsCliSkillDoc,
    SkillsCliSourcePreview, SkillsCliSourceTypeBucket,
};
pub use updates::{
    SkillsCliApplyRecoveryResult, SkillsCliApplyResult, SkillsCliApplySelection,
    SkillsCliApplyUpdateRequest, SkillsCliUpdateInventory, SkillsCliUpdateProgress,
    SkillsCliUpdateStatus,
};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::db::DbPool;
use crate::services::central_mutation::{
    acquire_target_mutation_guard, CentralMutationError, DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};
use crate::targets::ProcessPolicy;

#[cfg(test)]
use crate::services::central_mutation::{acquire_central_mutation_guard_at, CentralMutationGuard};

use runner::{CliOutput, RunnerRequest};

// ─── Launcher helpers ────────────────────────────────────────────────────────

#[allow(dead_code)]
fn resolve_launcher() -> Result<NodeLauncher, SkillsCliError> {
    let path = std::env::var("PATH").unwrap_or_default();
    resolve_node_launcher(&path)
}

pub(crate) fn resolve_node_program_from_env() -> Result<PathBuf, SkillsCliError> {
    let path = std::env::var("PATH").unwrap_or_default();
    resolve_node_program(&path)
}

async fn run_node_program(
    runner: &dyn SkillsCliRunner,
    program: &Path,
    args: Vec<String>,
    policy: ProcessPolicy,
    cancel: Option<&AtomicBool>,
) -> Result<CliOutput, SkillsCliError> {
    runner
        .run(RunnerRequest {
            program: program.to_path_buf(),
            args,
            policy,
            cancel,
        })
        .await
}

async fn run_cli(
    runner: &dyn SkillsCliRunner,
    launcher: &NodeLauncher,
    args: Vec<String>,
    policy: ProcessPolicy,
    cancel: Option<&AtomicBool>,
) -> Result<CliOutput, SkillsCliError> {
    run_node_program(runner, &launcher.program, args, policy, cancel).await
}

pub(crate) fn map_guard_error(error: CentralMutationError) -> SkillsCliError {
    match error {
        CentralMutationError::Busy { .. } | CentralMutationError::Timeout { .. } => {
            SkillsCliError::Busy
        }
        other => SkillsCliError::Io {
            context: "acquire target mutation lock",
            source: std::io::Error::other(other.to_string()),
        },
    }
}

// ─── Doctor ──────────────────────────────────────────────────────────────────

/// Probe node on the frozen transport. Local resolves PATH; Remote uses one
/// `run_script` round-trip and never probes `skills --help`.
pub(crate) async fn doctor(
    tx: &SkillsCliTransport,
) -> Result<SkillsCliDoctorReport, SkillsCliError> {
    tx.doctor().await
}

pub(crate) async fn doctor_with_program(
    runner: &dyn SkillsCliRunner,
    program: &Path,
) -> Result<SkillsCliDoctorReport, SkillsCliError> {
    let version_output = match run_node_program(
        runner,
        program,
        vec!["--version".to_string()],
        standard_policy(),
        None,
    )
    .await
    {
        Ok(output) => output,
        Err(SkillsCliError::CliUnavailable) => return Err(SkillsCliError::NodeMissing),
        Err(error) => return Err(error),
    };
    if !version_output.status_success {
        return Err(SkillsCliError::NodeMissing);
    }
    let version_text = String::from_utf8_lossy(&version_output.stdout).into_owned();
    let found_version = version_text.trim().to_string();
    let parsed = parse_node_version(&version_text).ok_or_else(|| SkillsCliError::NodeTooOld {
        required: SKILLS_CLI_MIN_NODE_DISPLAY,
        found: found_version.clone(),
    })?;
    if !is_node_version_supported(parsed) {
        return Err(SkillsCliError::NodeTooOld {
            required: SKILLS_CLI_MIN_NODE_DISPLAY,
            found: found_version,
        });
    }

    Ok(SkillsCliDoctorReport {
        node_version: version_text.trim().to_string(),
        npm_spec: SKILLS_CLI_NPM_SPEC.to_string(),
    })
}

// ─── List ────────────────────────────────────────────────────────────────────

const LOCK_READ_LIMIT: u64 = 1_048_576;

fn is_missing_path(error: &SkillsCliError) -> bool {
    matches!(error, SkillsCliError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound)
}

pub(crate) async fn load_lock_from_transport(
    tx: &SkillsCliTransport,
) -> Result<lock::CliLockOwnership, SkillsCliError> {
    match tx
        .fs()
        .read_file_bounded(tx.paths().lock_path(), LOCK_READ_LIMIT)
        .await
    {
        Ok(bytes) => Ok(lock::parse_lock_content(&String::from_utf8_lossy(&bytes))),
        Err(error) if is_missing_path(&error) => Ok(lock::CliLockOwnership::default()),
        Err(error) => Err(error),
    }
}

/// List global Skills CLI skills from lock v3 + filesystem. Does not spawn.
///
/// Local keeps `observe_directory_slot` via `list_global_at`. Remote round-trips
/// are RT1 lock read + RT2 one `probe_paths`. Those two are the entire remote
/// command budget; a per-skill remote call here is a regression against
/// constant round-trips.
pub(crate) async fn list_global(
    tx: &SkillsCliTransport,
    pool: &DbPool,
) -> Result<SkillsCliGlobalSnapshot, SkillsCliError> {
    let paths = tx.paths();
    if !tx.is_remote() {
        return list_global_at(pool, &paths.canonical_root_path(), &paths.lock_path_buf()).await;
    }
    let ownership = load_lock_from_transport(tx).await?;
    let agents = crate::db::repos::agents_repo::get_all_agents(pool)
        .await
        .map_err(|error| SkillsCliError::Io {
            context: "read platforms",
            source: std::io::Error::other(error.to_string()),
        })?;
    let mapped: Vec<&crate::db::Agent> = SKILLS_CLI_AGENT_MAP
        .iter()
        .filter_map(|(id, _)| agents.iter().find(|agent| agent.id == *id))
        .collect();
    let platform_dirs: Vec<String> = mapped
        .iter()
        .map(|agent| agent.global_skills_dir.clone())
        .collect();
    let probe_path_list = probe::collect_inventory_probe_paths(
        ownership.names().map(str::to_string),
        paths.canonical_root(),
        &platform_dirs,
        |parent, child| paths.join_child(parent, child),
        |path| paths.parent_of(path),
    );
    let probes = if ownership.is_empty() {
        Vec::new()
    } else {
        tx.fs().probe_paths(&probe_path_list).await?
    };
    let probe_map = probe::index_probes(&probes);
    let platforms: Vec<inventory::InventoryPlatform> = mapped
        .iter()
        .map(|agent| {
            let dir = agent.global_skills_dir.as_str();
            let detected = probe::probe_exists(&probe_map, dir)
                || paths
                    .parent_of(dir)
                    .is_some_and(|parent| probe::probe_exists(&probe_map, &parent));
            inventory::InventoryPlatform {
                agent_id: agent.id.clone(),
                display_name: agent.display_name.clone(),
                global_skills_dir: PathBuf::from(&agent.global_skills_dir),
                is_enabled: agent.is_enabled,
                is_detected: detected,
                supports_local_placement: cfg!(any(unix, windows)),
            }
        })
        .collect();
    let placement_platforms: Vec<placement::PlacementPlatform> = platforms
        .iter()
        .map(inventory::InventoryPlatform::as_placement_platform)
        .collect();
    let link_kind = tx.managed_link_kind();
    let posix = paths.uses_posix();
    let mut skills = Vec::new();
    for (name, entry) in ownership.iter() {
        let canonical = paths.join_child(paths.canonical_root(), name);
        let canonical_owned = probe::canonical_owned_from_probe(probe_map.get(&canonical));
        let mut slots = HashMap::new();
        for (agent, platform) in mapped.iter().zip(placement_platforms.iter()) {
            let target_path = paths.join_child(&agent.global_skills_dir, name);
            let slot = probe_map
                .get(&target_path)
                .map(|item| probe::observed_slot_from_probe(item, &canonical, link_kind, posix))
                .unwrap_or(placement::ObservedSlot::Absent);
            slots.insert(platform.agent_id.clone(), (target_path, slot));
        }
        let placements = placement::classify_placements_observed(
            name,
            canonical_owned,
            &placement_platforms,
            &slots,
        );
        skills.push(inventory::skill_from_lock_entry(
            name,
            entry,
            &canonical,
            canonical_owned,
            placements,
        ));
    }
    Ok(SkillsCliGlobalSnapshot {
        skills,
        canonical_root: paths.canonical_root().to_string(),
        lock_path: paths.lock_path().to_string(),
    })
}

pub(crate) async fn list_global_at(
    pool: &DbPool,
    canonical_root: &std::path::Path,
    lock_path: &std::path::Path,
) -> Result<SkillsCliGlobalSnapshot, SkillsCliError> {
    let ownership = load_cli_lock_ownership(lock_path)?;
    let agents = crate::db::repos::agents_repo::get_all_agents(pool)
        .await
        .map_err(|error| SkillsCliError::Io {
            context: "read platforms",
            source: std::io::Error::other(error.to_string()),
        })?;
    let platforms = mapped_inventory_platforms(&agents);
    Ok(SkillsCliGlobalSnapshot {
        skills: inventory::project_global_inventory(&ownership, canonical_root, &platforms),
        canonical_root: canonical_root.to_string_lossy().into_owned(),
        lock_path: lock_path.to_string_lossy().into_owned(),
    })
}

pub(crate) fn mapped_inventory_platforms(
    agents: &[crate::db::Agent],
) -> Vec<inventory::InventoryPlatform> {
    let mut platforms = Vec::new();
    for (id, _) in SKILLS_CLI_AGENT_MAP {
        let Some(agent) = agents.iter().find(|agent| agent.id == *id) else {
            continue;
        };
        platforms.push(inventory::InventoryPlatform {
            agent_id: agent.id.clone(),
            display_name: agent.display_name.clone(),
            global_skills_dir: std::path::PathBuf::from(&agent.global_skills_dir),
            is_enabled: agent.is_enabled,
            is_detected: is_platform_detected(&agent.global_skills_dir),
            supports_local_placement: cfg!(any(unix, windows)),
        });
    }
    platforms
}

pub(crate) async fn mapped_inventory_platforms_via_transport(
    tx: &SkillsCliTransport,
    pool: &DbPool,
) -> Result<Vec<inventory::InventoryPlatform>, SkillsCliError> {
    let agents = crate::db::repos::agents_repo::get_all_agents(pool)
        .await
        .map_err(|error| SkillsCliError::Io {
            context: "read platforms",
            source: std::io::Error::other(error.to_string()),
        })?;
    if !tx.is_remote() {
        return Ok(mapped_inventory_platforms(&agents));
    }
    Ok(SKILLS_CLI_AGENT_MAP
        .iter()
        .filter_map(|(id, _)| agents.iter().find(|agent| agent.id == *id))
        .map(|agent| inventory::InventoryPlatform {
            agent_id: agent.id.clone(),
            display_name: agent.display_name.clone(),
            global_skills_dir: PathBuf::from(&agent.global_skills_dir),
            is_enabled: agent.is_enabled,
            is_detected: true,
            supports_local_placement: true,
        })
        .collect())
}

pub(crate) fn remove_recovery_dir_for_transport(tx: &SkillsCliTransport) -> PathBuf {
    match tx.recovery_target_id() {
        Some(target_id) if cfg!(test) => std::env::temp_dir()
            .join("skillport-skills-cli-remove-recovery")
            .join(target_id),
        Some(target_id) => crate::paths::skills_cli_remove_recovery_dir_for_target(target_id),
        None => crate::paths::skills_cli_remove_recovery_dir(),
    }
}

// ─── Install targets ─────────────────────────────────────────────────────────

/// Live platform detection using the same rule as
/// `commands::agents::is_agent_detected`: the directory itself or its parent
/// exists on the local machine.
fn is_platform_detected(global_skills_dir: &str) -> bool {
    let dir = std::path::Path::new(global_skills_dir);
    dir.exists() || dir.parent().is_some_and(|parent| parent.exists())
}

/// Detected ∩ mapped platforms with default selection = enabled ones.
///
/// Unsupported builtins are hidden from the selector entirely; custom agents
/// never appear because they have no reviewed mapping.
pub async fn install_targets(
    tx: &SkillsCliTransport,
    pool: &DbPool,
) -> Result<Vec<SkillsCliInstallTarget>, SkillsCliError> {
    let agents = crate::db::repos::agents_repo::get_all_agents(pool)
        .await
        .map_err(|error| SkillsCliError::Io {
            context: "read platforms",
            source: std::io::Error::other(error.to_string()),
        })?;
    let paths = tx.paths();
    let mut probe_dirs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for agent in &agents {
        if !agent.is_builtin || cli_agent_for_skillport_id(&agent.id).is_none() {
            continue;
        }
        if seen.insert(agent.global_skills_dir.clone()) {
            probe_dirs.push(agent.global_skills_dir.clone());
        }
        if let Some(parent) = paths.parent_of(&agent.global_skills_dir) {
            if seen.insert(parent.clone()) {
                probe_dirs.push(parent);
            }
        }
    }
    let probes = tx.fs().probe_paths(&probe_dirs).await?;
    let probe_map = probe::index_probes(&probes);

    let mut targets = Vec::new();
    for agent in agents {
        if !agent.is_builtin {
            continue;
        }
        let Some(cli_agent) = cli_agent_for_skillport_id(&agent.id) else {
            continue;
        };
        let detected = probe::probe_exists(&probe_map, &agent.global_skills_dir)
            || paths
                .parent_of(&agent.global_skills_dir)
                .is_some_and(|parent| probe::probe_exists(&probe_map, &parent));
        if !detected {
            continue;
        }
        targets.push(SkillsCliInstallTarget {
            default_selected: agent.is_enabled,
            cli_agent: cli_agent.to_string(),
            id: agent.id,
            display_name: agent.display_name,
            icon_name: agent.icon_name,
            is_enabled: agent.is_enabled,
        });
    }
    Ok(targets)
}

// ─── Preview ─────────────────────────────────────────────────────────────────

/// Parse bullet-style skill names from the human `--list` output of the PIN
/// version. Returns an empty vec when nothing recognizable is present — the
/// caller converts that to [`SkillsCliError::PreviewUnparsed`].
pub fn parse_preview_skill_names(stdout: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in stdout.lines() {
        // Strip common Clack list decorations (│ ├ └ ╭ ╰ ─) then bullets.
        let stripped = line.trim_start_matches(['│', '├', '└', '┬', '╭', '╰', '─', ' ']);
        let Some(after_marker) = stripped
            .strip_prefix("- ")
            .or_else(|| stripped.strip_prefix("* "))
            .or_else(|| stripped.strip_prefix("• "))
            .or_else(|| stripped.strip_prefix("· "))
        else {
            continue;
        };
        let token = after_marker.split_whitespace().next().unwrap_or("");
        let token = token.trim_end_matches([',', ';']);
        let valid = !token.is_empty()
            && token.len() <= 128
            && token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-'));
        if valid && !names.iter().any(|existing| existing == token) {
            names.push(token.to_string());
        }
    }
    names
}

/// Preview installable skills for a whitelisted source without installing.
pub(crate) async fn preview_source(
    tx: &SkillsCliTransport,
    raw_source: &str,
) -> Result<SkillsCliSourcePreview, SkillsCliError> {
    let _ = parse_skill_source(raw_source)?;
    let launcher = tx.resolve_launcher().await?;
    preview_source_with_launcher(tx.runner(), &launcher, raw_source).await
}

pub(crate) async fn preview_source_with_launcher(
    runner: &dyn SkillsCliRunner,
    launcher: &NodeLauncher,
    raw_source: &str,
) -> Result<SkillsCliSourcePreview, SkillsCliError> {
    let source = parse_skill_source(raw_source)?;
    let output = run_cli(
        runner,
        launcher,
        build_preview_argv(launcher, &source),
        standard_policy(),
        None,
    )
    .await?;
    if !output.status_success {
        return Err(SkillsCliError::PreviewUnparsed);
    }
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let skills = parse_preview_skill_names(&text);
    if skills.is_empty() {
        return Err(SkillsCliError::PreviewUnparsed);
    }
    Ok(SkillsCliSourcePreview {
        source: source.as_argv_value().to_string(),
        skills,
    })
}

// ─── Add / remove ────────────────────────────────────────────────────────────

const INSTALL_LOCK_OPERATION: &str = "Skills CLI global install";

pub(crate) fn check_cancel(cancel: Option<&AtomicBool>) -> Result<(), SkillsCliError> {
    if cancel.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed)) {
        Err(SkillsCliError::Cancelled)
    } else {
        Ok(())
    }
}

pub(crate) fn is_valid_skill_token(name: &str) -> bool {
    !name.trim().is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-' | ' '))
}

fn validate_selection(
    skill_names: &[String],
    skillport_agent_ids: &[String],
) -> Result<(), SkillsCliError> {
    if skill_names.is_empty() || skillport_agent_ids.is_empty() {
        return Err(SkillsCliError::SelectionEmpty);
    }
    if !skill_names.iter().all(|name| is_valid_skill_token(name)) {
        return Err(SkillsCliError::SourceInvalid);
    }
    Ok(())
}

async fn add_global_locked(
    runner: &dyn SkillsCliRunner,
    launcher: &NodeLauncher,
    source: &SkillSource,
    skill_names: Vec<String>,
    cli_agents: Vec<String>,
    cancel: Option<&AtomicBool>,
    target_kind: &'static str,
) -> Result<SkillsCliAddResult, SkillsCliError> {
    let output = run_cli(
        runner,
        launcher,
        build_add_global_argv(launcher, source, &skill_names, &cli_agents),
        bulk_transfer_policy(),
        cancel,
    )
    .await?;
    if !output.status_success {
        tracing::warn!(
            operation = "skills_cli.add_global",
            exit_code = ?output.exit_code,
            stderr_bytes = output.stderr.len(),
            stdout_bytes = output.stdout.len(),
            skill_count = skill_names.len(),
            agent_count = cli_agents.len(),
            source_kind = source.source_kind(),
            target_kind,
            "Skills CLI add command failed"
        );
        return Err(SkillsCliError::CliFailed);
    }
    Ok(SkillsCliAddResult {
        installed_skills: skill_names.len() as u32,
        targeted_platforms: cli_agents.len() as u32,
    })
}

/// Install selected skills globally onto mapped platforms.
///
/// Lock order per design §4: the caller holds the exclusive job lease
/// (cancellation/progress only); this function owns the target mutation
/// guard across the whole child process lifetime. Source whitelist runs
/// before any remote command.
pub(crate) async fn add_global(
    tx: &SkillsCliTransport,
    raw_source: &str,
    skill_names: Vec<String>,
    skillport_agent_ids: Vec<String>,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliAddResult, SkillsCliError> {
    validate_selection(&skill_names, &skillport_agent_ids)?;
    let cli_agents = map_skillport_ids_to_cli_agents(&skillport_agent_ids)?;
    let source = parse_skill_source(raw_source)?;
    let launcher = tx.resolve_launcher().await?;
    let target_kind = tx.warn_target_kind();

    let _guard = acquire_target_mutation_guard(
        &tx.mutation_target(),
        INSTALL_LOCK_OPERATION,
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
    .map_err(map_guard_error)?;

    add_global_locked(
        tx.runner(),
        &launcher,
        &source,
        skill_names,
        cli_agents,
        cancel,
        target_kind,
    )
    .await
}

/// Isolated-lock inputs for [`add_global_with_lock_at`].
#[cfg(test)]
pub(crate) struct AddGlobalLockRequest<'a> {
    pub lock_path: PathBuf,
    pub runner: &'a dyn SkillsCliRunner,
    pub launcher: &'a NodeLauncher,
    pub source: &'a str,
    pub skill_names: Vec<String>,
    pub skillport_agent_ids: Vec<String>,
    pub cancel: Option<&'a AtomicBool>,
    pub timeout: std::time::Duration,
}

/// Test seam mirroring [`add_global`] against an isolated lock file so
/// contention tests never touch the shared default path.
#[cfg(test)]
pub(crate) async fn add_global_with_lock_at(
    request: AddGlobalLockRequest<'_>,
) -> Result<SkillsCliAddResult, SkillsCliError> {
    validate_selection(&request.skill_names, &request.skillport_agent_ids)?;
    let cli_agents = map_skillport_ids_to_cli_agents(&request.skillport_agent_ids)?;
    let source = parse_skill_source(request.source)?;

    let _guard: CentralMutationGuard = acquire_central_mutation_guard_at(
        request.lock_path,
        INSTALL_LOCK_OPERATION,
        request.timeout,
    )
    .await
    .map_err(map_guard_error)?;

    add_global_locked(
        request.runner,
        request.launcher,
        &source,
        request.skill_names,
        cli_agents,
        request.cancel,
        "local",
    )
    .await
}

#[cfg(test)]
mod force_mutate_tests;
#[cfg(test)]
mod install_update_tests;
#[cfg(test)]
mod mutate_tests;
#[cfg(test)]
mod tests;
