//! Skills CLI argv construction, source whitelist, and Node launcher
//! resolution.
//!
//! The GUI never executes `npx.cmd`: the program is always a resolved
//! `node`/`node.exe` binary and npm's `npx-cli.js` is passed as `argv[1]`.
//! npx-level flags (`--yes`, `--package=`) sit before the `--` separator;
//! skills-level flags (`-g`, `-y`, `-a`, `-s`) come after it.

use std::path::{Path, PathBuf};

/// Frozen npm package spec for the official Skills CLI.
pub const SKILLS_CLI_NPM_SPEC: &str = "skills@1.5.23";

/// Minimum Node version declared by the PIN package (`engines.node`).
pub const SKILLS_CLI_MIN_NODE: (u32, u32, u32) = (22, 20, 0);
pub const SKILLS_CLI_MIN_NODE_DISPLAY: &str = "22.20.0";

/// Characters that must never reach an argv slot derived from user input.
const FORBIDDEN_SOURCE_CHARS: [char; 9] = ['&', '|', '^', '%', '!', '<', '>', '"', '\''];

// ─── Source grammar ──────────────────────────────────────────────────────────

fn is_owner_segment(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// A validated skill source accepted by the install flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    /// `owner/repo` or `owner/repo@skill`
    Shorthand { repo: String, skill: Option<String> },
    /// `https://github.com/...` or `https://gitlab.com/...` (no query string)
    WebUrl { url: String },
    /// `git@github.com:owner/repo.git`
    SshUrl { url: String },
}

impl SkillSource {
    /// The literal text passed to the CLI as `<source>`.
    pub fn as_argv_value(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::Shorthand {
                repo,
                skill: Some(skill),
            } => std::borrow::Cow::Owned(format!("{repo}@{skill}")),
            Self::Shorthand { repo, skill: None } => std::borrow::Cow::Borrowed(repo),
            Self::WebUrl { url } | Self::SshUrl { url } => std::borrow::Cow::Borrowed(url),
        }
    }

    /// Redacted summary safe for operation-log details (host + path only,
    /// already enforced by the whitelist).
    pub fn as_log_label(&self) -> std::borrow::Cow<'_, str> {
        self.as_argv_value()
    }
}

/// Validate a user-supplied skill source against the reviewed whitelist.
///
/// Accepted shapes:
/// - `owner/repo` and `owner/repo@skill-name`
/// - `https://github.com/<path>` and `https://gitlab.com/<path>` without query
/// - `git@github.com:owner/repo.git`
pub fn parse_skill_source(raw: &str) -> Result<SkillSource, super::SkillsCliError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 512 {
        return Err(super::SkillsCliError::SourceInvalid);
    }
    if trimmed.chars().any(|c| {
        FORBIDDEN_SOURCE_CHARS.contains(&c)
            || matches!(c, '\n' | '\r' | '\t' | '\0')
            || c.is_control()
    }) {
        return Err(super::SkillsCliError::SourceInvalid);
    }
    // Shell-shaped input such as leading `-c` or embedded spaces is rejected:
    // every accepted shape is a single token with no whitespace.
    if trimmed.split_whitespace().count() != 1 {
        return Err(super::SkillsCliError::SourceInvalid);
    }

    if let Some(rest) = trimmed.strip_prefix("https://") {
        let host = rest.split('/').next().unwrap_or("");
        if host != "github.com" && host != "gitlab.com" {
            return Err(super::SkillsCliError::SourceInvalid);
        }
        if trimmed.contains('?') || trimmed.contains('#') {
            return Err(super::SkillsCliError::SourceInvalid);
        }
        return Ok(SkillSource::WebUrl {
            url: trimmed.to_string(),
        });
    }

    if let Some(rest) = trimmed.strip_prefix("git@github.com:") {
        // Exactly owner/repo.git, no extra segments or spaces.
        let ok = rest.ends_with(".git")
            && rest[..rest.len() - 4].split('/').collect::<Vec<_>>().len() == 2
            && rest[..rest.len() - 4].split('/').all(is_owner_segment);
        if !ok {
            return Err(super::SkillsCliError::SourceInvalid);
        }
        return Ok(SkillSource::SshUrl {
            url: trimmed.to_string(),
        });
    }

    parse_shorthand_source(trimmed)
}

fn is_valid_skill_name(skill: &str) -> bool {
    !skill.is_empty()
        && skill.len() <= 128
        && skill
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-' | ' '))
}

fn is_owner_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None)
            if is_owner_segment(owner) && is_owner_segment(name)
    )
}

/// Shorthand: `owner/repo` or `owner/repo@skill`.
fn parse_shorthand_source(trimmed: &str) -> Result<SkillSource, super::SkillsCliError> {
    let (repo, skill) = match trimmed.split_once('@') {
        Some((repo, skill)) => (repo, Some(skill)),
        None => (trimmed, None),
    };
    if !is_owner_repo(repo) {
        return Err(super::SkillsCliError::SourceInvalid);
    }
    if let Some(skill) = skill {
        if !is_valid_skill_name(skill) {
            return Err(super::SkillsCliError::SourceInvalid);
        }
        return Ok(SkillSource::Shorthand {
            repo: repo.to_string(),
            skill: Some(skill.to_string()),
        });
    }
    Ok(SkillSource::Shorthand {
        repo: trimmed.to_string(),
        skill: None,
    })
}

// ─── Node launcher resolution ────────────────────────────────────────────────

/// Resolved local launch inputs for the pinned CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeLauncher {
    /// Absolute path to the `node` executable (never `npx.cmd`).
    pub program: PathBuf,
    /// Absolute path to npm's `npx-cli.js` passed as `argv[1]`.
    pub npx_js: PathBuf,
}

impl NodeLauncher {
    /// Arguments passed to `node`/`node.exe`: `<npx-js> --yes --package=<spec> -- skills …`.
    /// The program itself is `self.program` and is never duplicated into argv.
    pub fn npx_argv_prefix(&self) -> Vec<String> {
        vec![
            self.npx_js.to_string_lossy().into_owned(),
            "--yes".to_string(),
            format!("--package={SKILLS_CLI_NPM_SPEC}"),
            "--".to_string(),
            "skills".to_string(),
        ]
    }
}

/// Candidate npm layouts relative to the resolved `node` directory, plus
/// well-known global roots. Ordered by likelihood.
fn npx_js_candidates(node_dir: &Path) -> Vec<PathBuf> {
    vec![
        node_dir.join("node_modules/npm/bin/npx-cli.js"),
        node_dir.join("lib/node_modules/npm/bin/npx-cli.js"),
        PathBuf::from("/usr/lib/node_modules/npm/bin/npx-cli.js"),
        PathBuf::from("/usr/local/lib/node_modules/npm/bin/npx-cli.js"),
        PathBuf::from("/opt/homebrew/lib/node_modules/npm/bin/npx-cli.js"),
    ]
}

/// Locate `node` on the given PATH-style search path without invoking a
/// shell. Returns `None` when no candidate binary exists.
pub fn find_node_in_paths(search_dirs: &[PathBuf]) -> Option<PathBuf> {
    const NODE_BIN: &str = if cfg!(windows) { "node.exe" } else { "node" };
    search_dirs
        .iter()
        .map(|dir| dir.join(NODE_BIN))
        .find(|candidate| candidate.is_file())
}

/// Resolve the launcher from a PATH string and the user's home directory.
///
/// `path_var` mirrors `$PATH`; tests inject synthetic directories.
pub fn resolve_node_launcher(path_var: &str) -> Result<NodeLauncher, super::SkillsCliError> {
    let search_dirs: Vec<PathBuf> = std::env::split_paths(path_var).collect();
    resolve_node_launcher_from_dirs(&search_dirs)
}

pub(crate) fn resolve_node_launcher_from_dirs(
    search_dirs: &[PathBuf],
) -> Result<NodeLauncher, super::SkillsCliError> {
    let program = find_node_in_paths(search_dirs)
        .ok_or(super::SkillsCliError::NodeMissing)?
        .canonicalize()
        .map_err(|_| super::SkillsCliError::NodeMissing)?;
    let node_dir = program
        .parent()
        .ok_or(super::SkillsCliError::NodeMissing)?
        .to_path_buf();

    npx_js_candidates(&node_dir)
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(|npx_js| NodeLauncher { program, npx_js })
        .ok_or(super::SkillsCliError::CliUnavailable)
}

/// Parse `vMAJOR.MINOR.PATCH` from `node --version` output.
pub fn parse_node_version(output: &str) -> Option<(u32, u32, u32)> {
    let trimmed = output.trim().trim_start_matches('v');
    let mut parts = trimmed.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next()?.parse::<u32>().ok()?;
    let patch = parts.next()?.parse::<u32>().ok()?;
    Some((major, minor, patch))
}

pub fn is_node_version_supported(version: (u32, u32, u32)) -> bool {
    version >= SKILLS_CLI_MIN_NODE
}

// ─── Argv builders ───────────────────────────────────────────────────────────

fn skills_subcommand(launcher: &NodeLauncher, subcommand: &str) -> Vec<String> {
    let mut argv = launcher.npx_argv_prefix();
    argv.push(subcommand.to_string());
    argv
}

/// `skills ls -g --json`
pub fn build_list_global_argv(launcher: &NodeLauncher) -> Vec<String> {
    let mut argv = skills_subcommand(launcher, "ls");
    argv.push("-g".to_string());
    argv.push("--json".to_string());
    argv
}

/// `node --version` (launcher probe; no npx involvement). Program is
/// `launcher.program`; argv is only `--version`.
pub fn build_node_version_argv(_launcher: &NodeLauncher) -> Vec<String> {
    vec!["--version".to_string()]
}

/// `skills --help` — proves the pinned package can execute without touching
/// user state.
pub fn build_probe_argv(launcher: &NodeLauncher) -> Vec<String> {
    skills_subcommand(launcher, "--help")
}

/// `skills add <source> --list`
pub fn build_preview_argv(launcher: &NodeLauncher, source: &SkillSource) -> Vec<String> {
    let mut argv = skills_subcommand(launcher, "add");
    argv.push(source.as_argv_value().to_string());
    argv.push("--list".to_string());
    argv
}

/// `skills add <source> -s <name>... -g -a <agent>... -y`
///
/// `--all` and `--agent '*'` are deliberately unreachable: callers must pass a
/// non-empty skill list and concrete mapped agent ids.
pub fn build_add_global_argv(
    launcher: &NodeLauncher,
    source: &SkillSource,
    skill_names: &[String],
    cli_agents: &[String],
) -> Vec<String> {
    debug_assert!(!skill_names.is_empty(), "add requires selected skills");
    debug_assert!(!cli_agents.is_empty(), "add requires mapped agents");
    let mut argv = skills_subcommand(launcher, "add");
    argv.push(source.as_argv_value().to_string());
    for name in skill_names {
        argv.push("-s".to_string());
        argv.push(name.clone());
    }
    argv.push("-g".to_string());
    for agent in cli_agents {
        argv.push("-a".to_string());
        argv.push(agent.clone());
    }
    argv.push("-y".to_string());
    argv
}

/// `skills remove --global <name> -y`
pub fn build_remove_global_argv(launcher: &NodeLauncher, skill_name: &str) -> Vec<String> {
    let mut argv = skills_subcommand(launcher, "remove");
    argv.push("--global".to_string());
    argv.push(skill_name.to_string());
    argv.push("-y".to_string());
    argv
}
