//! Skills CLI global management service.
//!
//! Wraps the official `skills` npm package (PIN: [`SKILLS_CLI_NPM_SPEC`]) for
//! the `-g` lifecycle on the Local target only. The CLI owns add/remove/lock
//! writes; this service validates input, supervises the process, and derives
//! lock-based ownership evidence for leftover protection.

mod agent_map;
mod argv;
mod error;
mod lock;
mod runner;

pub use agent_map::{
    cli_agent_for_skillport_id, is_explicitly_unsupported, map_skillport_ids_to_cli_agents,
    SKILLS_CLI_AGENT_MAP, SKILLS_CLI_UNSUPPORTED,
};
pub use argv::{
    build_add_global_argv, build_list_global_argv, build_node_version_argv, build_preview_argv,
    build_probe_argv, build_remove_global_argv, is_node_version_supported, parse_node_version,
    parse_skill_source, resolve_node_launcher, NodeLauncher, SkillSource,
    SKILLS_CLI_MIN_NODE_DISPLAY, SKILLS_CLI_NPM_SPEC,
};
pub use error::SkillsCliError;
pub use lock::{
    annotate_platform_install_origins, annotate_platform_install_origins_with,
    classify_local_path_origin, is_path_inside_owned_canonical, load_cli_lock_ownership,
    resolved_link_target, skills_cli_lock_path, skills_cli_lock_path_from_env, CliLockOwnership,
    LinkOrigin,
};
pub(crate) use runner::{
    bulk_transfer_policy, standard_policy, NodeProcessRunner, SkillsCliRunner,
};

use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::services::central_mutation::{
    acquire_target_mutation_guard, CentralMutationError, DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};
use crate::targets::{ActiveTarget, ProcessPolicy};

#[cfg(test)]
use crate::services::central_mutation::{acquire_central_mutation_guard_at, CentralMutationGuard};
#[cfg(test)]
use std::path::PathBuf;

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

/// One global skill as reported by `skills ls -g --json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliGlobalSkill {
    pub name: String,
    pub path: Option<String>,
    pub scope: Option<String>,
    pub agents: Vec<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub source_type: Option<String>,
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

// ─── Target gate ─────────────────────────────────────────────────────────────

/// Every `skills_cli_*` command rejects non-Local targets before any spawn or
/// local lock read.
pub fn ensure_local_target(target: &ActiveTarget) -> Result<(), SkillsCliError> {
    match target {
        ActiveTarget::Local => Ok(()),
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => Err(SkillsCliError::LocalTargetOnly),
    }
}

/// True when the given explicit target is Local.
pub fn is_local_target(target: &ActiveTarget) -> bool {
    matches!(target, ActiveTarget::Local)
}

// ─── Launcher helpers ────────────────────────────────────────────────────────

fn resolve_launcher() -> Result<NodeLauncher, SkillsCliError> {
    let path = std::env::var("PATH").unwrap_or_default();
    resolve_node_launcher(&path)
}

async fn run_cli(
    runner: &dyn SkillsCliRunner,
    launcher: &NodeLauncher,
    args: Vec<String>,
    policy: ProcessPolicy,
    cancel: Option<&AtomicBool>,
) -> Result<CliOutput, SkillsCliError> {
    let program = launcher.program.clone();
    runner
        .run(RunnerRequest {
            program,
            args,
            policy,
            cancel,
        })
        .await
}

fn map_guard_error(error: CentralMutationError) -> SkillsCliError {
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

/// Probe the local runtime: node present, version >= PIN requirement, and the
/// pinned package executable via npx.
pub(crate) async fn doctor(
    runner: &dyn SkillsCliRunner,
) -> Result<SkillsCliDoctorReport, SkillsCliError> {
    doctor_with_launcher(runner, &resolve_launcher()?).await
}

pub(crate) async fn doctor_with_launcher(
    runner: &dyn SkillsCliRunner,
    launcher: &NodeLauncher,
) -> Result<SkillsCliDoctorReport, SkillsCliError> {
    let version_output = run_cli(
        runner,
        launcher,
        build_node_version_argv(launcher),
        standard_policy(),
        None,
    )
    .await?;
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

    // Prove the pinned package can execute without touching user state.
    let probe = run_cli(
        runner,
        launcher,
        build_probe_argv(launcher),
        standard_policy(),
        None,
    )
    .await?;
    if !probe.status_success {
        return Err(SkillsCliError::CliUnavailable);
    }

    Ok(SkillsCliDoctorReport {
        node_version: version_text.trim().to_string(),
        npm_spec: SKILLS_CLI_NPM_SPEC.to_string(),
    })
}

// ─── List ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LsJsonEntry {
    #[serde(default)]
    name: Option<serde_json::Value>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    agents: Option<Vec<String>>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default)]
    source_type: Option<String>,
}

fn parse_ls_json(stdout: &[u8]) -> Result<Vec<SkillsCliGlobalSkill>, SkillsCliError> {
    let text = String::from_utf8_lossy(stdout);
    let trimmed = text.trim();
    // Tolerate status lines printed before the JSON array.
    let json_start = trimmed.find('[').ok_or(SkillsCliError::ListUnparsed)?;
    let entries: Vec<LsJsonEntry> =
        serde_json::from_str(&trimmed[json_start..]).map_err(|_| SkillsCliError::ListUnparsed)?;

    let mut skills = Vec::with_capacity(entries.len());
    for entry in entries {
        let name = match entry.name {
            Some(serde_json::Value::String(name)) if !name.trim().is_empty() => name,
            _ => return Err(SkillsCliError::ListUnparsed),
        };
        skills.push(SkillsCliGlobalSkill {
            name,
            path: entry.path,
            scope: entry.scope,
            agents: entry.agents.unwrap_or_default(),
            source: entry.source,
            source_url: entry.source_url,
            source_type: entry.source_type,
        });
    }
    Ok(skills)
}

/// List global Skills CLI skills through the frozen-version CLI.
pub(crate) async fn list_global(
    runner: &dyn SkillsCliRunner,
) -> Result<Vec<SkillsCliGlobalSkill>, SkillsCliError> {
    list_global_with_launcher(runner, &resolve_launcher()?).await
}

pub(crate) async fn list_global_with_launcher(
    runner: &dyn SkillsCliRunner,
    launcher: &NodeLauncher,
) -> Result<Vec<SkillsCliGlobalSkill>, SkillsCliError> {
    let output = run_cli(
        runner,
        launcher,
        build_list_global_argv(launcher),
        standard_policy(),
        None,
    )
    .await?;
    if !output.status_success {
        return Err(SkillsCliError::CliUnavailable);
    }
    parse_ls_json(&output.stdout)
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
pub async fn install_targets(pool: &DbPool) -> Result<Vec<SkillsCliInstallTarget>, SkillsCliError> {
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
    runner: &dyn SkillsCliRunner,
    raw_source: &str,
) -> Result<SkillsCliSourcePreview, SkillsCliError> {
    preview_source_with_launcher(runner, &resolve_launcher()?, raw_source).await
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
const REMOVE_LOCK_OPERATION: &str = "Skills CLI global remove";

fn is_valid_skill_token(name: &str) -> bool {
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
    runner: &dyn SkillsCliRunner,
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

    add_global_locked(runner, &launcher, &source, skill_names, cli_agents, cancel).await
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

async fn remove_global_locked(
    runner: &dyn SkillsCliRunner,
    launcher: &NodeLauncher,
    skill_name: &str,
    cancel: Option<&AtomicBool>,
) -> Result<(), SkillsCliError> {
    let output = run_cli(
        runner,
        launcher,
        build_remove_global_argv(launcher, skill_name),
        bulk_transfer_policy(),
        cancel,
    )
    .await?;
    if !output.status_success {
        return Err(SkillsCliError::CliFailed);
    }
    Ok(())
}

/// Fully uninstall one global skill (canonical + platform links + lock row).
pub(crate) async fn remove_global(
    runner: &dyn SkillsCliRunner,
    skill_name: &str,
    cancel: Option<&AtomicBool>,
) -> Result<(), SkillsCliError> {
    if !is_valid_skill_token(skill_name) {
        return Err(SkillsCliError::SourceInvalid);
    }
    let launcher = resolve_launcher()?;

    let _guard = acquire_target_mutation_guard(
        &ActiveTarget::Local,
        REMOVE_LOCK_OPERATION,
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
    .map_err(map_guard_error)?;

    remove_global_locked(runner, &launcher, skill_name, cancel).await
}

/// Test seam mirroring [`remove_global`] against an isolated lock file.
#[cfg(test)]
#[allow(dead_code)]
pub(crate) async fn remove_global_with_lock_at(
    lock_path: PathBuf,
    runner: &dyn SkillsCliRunner,
    launcher: &NodeLauncher,
    skill_name: &str,
    cancel: Option<&AtomicBool>,
    timeout: std::time::Duration,
) -> Result<(), SkillsCliError> {
    if !is_valid_skill_token(skill_name) {
        return Err(SkillsCliError::SourceInvalid);
    }

    let _guard: CentralMutationGuard =
        acquire_central_mutation_guard_at(lock_path, REMOVE_LOCK_OPERATION, timeout)
            .await
            .map_err(map_guard_error)?;

    remove_global_locked(runner, launcher, skill_name, cancel).await
}

#[cfg(test)]
mod tests;
