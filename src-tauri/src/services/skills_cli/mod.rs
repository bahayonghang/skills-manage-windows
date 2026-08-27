//! Skills CLI global management service.
//!
//! Wraps the official `skills` npm package (PIN: [`SKILLS_CLI_NPM_SPEC`]) for
//! the `-g` lifecycle. Local and Remote share one transport seam; this task
//! opens doctor on Remote and keeps other capabilities gated.

mod agent_map;
mod argv;
mod error;
mod export;
mod files;
mod inventory;
mod link;
mod lock;
mod placement;
mod remove;
mod runner;
mod transport;
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
pub(crate) use link::{link_platform, unlink_platform};
pub use lock::{
    annotate_platform_install_origins, annotate_platform_install_origins_with,
    classify_local_path_origin, is_mapped_agent_lock_copy, is_path_inside_owned_canonical,
    load_cli_lock_ownership, resolved_link_target, skills_cli_lock_path,
    skills_cli_lock_path_from_env, CliLockEntry, CliLockOwnership, LinkOrigin,
};
pub(crate) use remove::{preview_remove_global, remove_global};
pub(crate) use runner::{bulk_transfer_policy, standard_policy, SkillsCliRunner};
pub use transport::{SkillsCliCapability, SkillsCliTransport};
pub use updates::{
    SkillsCliApplyRecoveryResult, SkillsCliApplyResult, SkillsCliApplySelection,
    SkillsCliApplyUpdateRequest, SkillsCliUpdateInventory, SkillsCliUpdateProgress,
    SkillsCliUpdateStatus,
};

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::services::central_mutation::{
    acquire_target_mutation_guard, CentralMutationError, DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};
use crate::targets::{ActiveTarget, ProcessPolicy};

#[cfg(test)]
use crate::services::central_mutation::{acquire_central_mutation_guard_at, CentralMutationGuard};

use runner::{CliOutput, RunnerRequest};

// ─── IPC payload types ───────────────────────────────────────────────────────

/// Result of `skills_cli_doctor`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliDoctorReport {
    pub node_version: String,
    pub npm_spec: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum SkillsCliInstallKind {
    Canonical,
    Copy,
    Missing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum SkillsCliSourceTypeBucket {
    Github,
    Gitlab,
    Git,
    Mintlify,
    Huggingface,
    Local,
    WellKnown,
    Unknown,
}

/// One global skill projected from lock v3 + filesystem (no CLI spawn).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliGlobalSkill {
    pub name: String,
    pub path: Option<String>,
    pub install_kind: SkillsCliInstallKind,
    pub scope: Option<String>,
    pub agents: Vec<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub source_type: Option<String>,
    pub source_type_bucket: SkillsCliSourceTypeBucket,
    pub canonical_path: Option<String>,
    pub folder_hash: Option<String>,
    pub installed_at: Option<String>,
    pub updated_at: Option<String>,
    pub placements: Vec<SkillsCliPlacement>,
}

/// Lock + filesystem snapshot returned by `skills_cli_list_global`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliGlobalSnapshot {
    pub skills: Vec<SkillsCliGlobalSkill>,
    pub canonical_root: String,
    pub lock_path: String,
}

/// One detected, mappable Local platform offered by the install flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliInstallTarget {
    pub id: String,
    pub display_name: String,
    pub icon_name: Option<String>,
    /// CLI `--agent` id this platform maps to.
    pub cli_agent: String,
    /// SkillPort enablement state; drives the default selection.
    pub is_enabled: bool,
    pub default_selected: bool,
}

/// Parsed result of `skills add <source> --list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliSourcePreview {
    pub source: String,
    pub skills: Vec<String>,
}

/// Summary of a completed global install.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliAddResult {
    pub installed_skills: u32,
    pub targeted_platforms: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum SkillsCliPlacementState {
    ManagedLink,
    DirectCopy,
    Missing,
    Conflict,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum SkillsCliManagedLinkKind {
    WindowsJunction,
    Symlink,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliPlacement {
    pub agent_id: String,
    pub display_name: String,
    pub target_path: String,
    pub state: SkillsCliPlacementState,
    pub managed_link_kind: Option<SkillsCliManagedLinkKind>,
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliSkillDoc {
    pub skill_name: String,
    pub content: String,
    pub byte_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliRemovePlacementSummary {
    pub agent_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliPlacementConflict {
    pub agent_id: String,
    pub display_name: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliRemovePlan {
    pub skill_name: String,
    pub owned_canonical: bool,
    pub managed_placements: Vec<SkillsCliRemovePlacementSummary>,
    pub retained_direct_copies: Vec<SkillsCliRemovePlacementSummary>,
    pub conflicts: Vec<SkillsCliPlacementConflict>,
    pub confirmable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliRemoveResult {
    pub removed_canonical: bool,
    pub removed_managed_agent_ids: Vec<String>,
    pub retained_direct_copy_agent_ids: Vec<String>,
}

// ─── Launcher helpers ────────────────────────────────────────────────────────

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

/// List global Skills CLI skills from lock v3 + filesystem. Does not spawn.
pub(crate) async fn list_global(
    tx: &SkillsCliTransport,
    pool: &DbPool,
) -> Result<SkillsCliGlobalSnapshot, SkillsCliError> {
    let paths = tx.paths();
    list_global_at(pool, &paths.canonical_root_path(), &paths.lock_path_buf()).await
}

pub(crate) async fn list_global_at(
    pool: &DbPool,
    canonical_root: &std::path::Path,
    lock_path: &std::path::Path,
) -> Result<SkillsCliGlobalSnapshot, SkillsCliError> {
    let ownership = load_cli_lock_ownership(lock_path)?;
    let agents = crate::db::get_all_agents(pool)
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
    _tx: &SkillsCliTransport,
    pool: &DbPool,
) -> Result<Vec<SkillsCliInstallTarget>, SkillsCliError> {
    let agents = crate::db::get_all_agents(pool)
        .await
        .map_err(|error| SkillsCliError::Io {
            context: "read platforms",
            source: std::io::Error::other(error.to_string()),
        })?;

    let mut targets = Vec::new();
    for agent in agents {
        if !agent.is_builtin {
            continue;
        }
        let Some(cli_agent) = cli_agent_for_skillport_id(&agent.id) else {
            continue;
        };
        if !is_platform_detected(&agent.global_skills_dir) {
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
    preview_source_with_launcher(tx.runner(), &resolve_launcher()?, raw_source).await
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
/// (cancellation/progress only); this function owns the Local target mutation
/// guard across the whole child process lifetime.
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
    let launcher = resolve_launcher()?;

    let _guard = acquire_target_mutation_guard(
        &ActiveTarget::Local,
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
    )
    .await
}

#[cfg(test)]
mod tests;
