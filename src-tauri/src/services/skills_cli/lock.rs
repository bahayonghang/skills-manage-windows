//! Skills CLI lock ownership evidence.
//!
//! The official CLI records every global install in
//! `$XDG_STATE_HOME/skills/.skill-lock.json` (falling back to
//! `~/.agents/.skill-lock.json`), keyed by sanitized skill name. A path is
//! CLI-owned only when the lock proves it — never merely because it lives
//! under `~/.agents/skills/`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::error::SkillsCliError;
use super::cli_agent_for_skillport_id;

/// Lock schema version the PIN package writes. Older versions are treated as
/// "no lock" exactly like the CLI itself does.
const LOCK_SCHEMA_VERSION: u64 = 3;

/// One v3 lock row. Source fields are optional; missing values stay `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliLockEntry {
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub source_type: Option<String>,
}

/// Parsed ownership evidence for one machine.
#[derive(Debug, Clone, Default)]
pub struct CliLockOwnership {
    entries: BTreeMap<String, CliLockEntry>,
}

impl CliLockOwnership {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Canonical directory the CLI owns for this entry:
    /// `~/.agents/skills/<sanitized-name>`.
    pub fn canonical_dir(&self, universal_skills_dir: &Path, name: &str) -> PathBuf {
        universal_skills_dir.join(name)
    }

    pub fn contains_name(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn entry(&self, name: &str) -> Option<&CliLockEntry> {
        self.entries.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &CliLockEntry)> {
        self.entries.iter().map(|(name, entry)| (name.as_str(), entry))
    }
}

/// The lock file location: `$XDG_STATE_HOME/skills/.skill-lock.json`, else
/// `~/.agents/.skill-lock.json`.
pub fn skills_cli_lock_path_from_env(xdg_state_home: Option<&str>, home_dir: &Path) -> PathBuf {
    match xdg_state_home.filter(|value| !value.trim().is_empty()) {
        Some(state_home) => Path::new(state_home)
            .join("skills")
            .join(".skill-lock.json"),
        None => home_dir
            .join(crate::paths::UNIVERSAL_AGENTS_DIR_NAME)
            .join(".skill-lock.json"),
    }
}

pub fn skills_cli_lock_path(home_dir: &Path) -> PathBuf {
    skills_cli_lock_path_from_env(std::env::var("XDG_STATE_HOME").ok().as_deref(), home_dir)
}

/// Read and parse the lock file. A missing file yields empty ownership; an
/// unparsable or older-version lock also yields empty ownership (matching the
/// CLI's own "incompatible version = empty lock" behavior).
pub fn load_cli_lock_ownership(lock_path: &Path) -> Result<CliLockOwnership, SkillsCliError> {
    let content = match std::fs::read_to_string(lock_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CliLockOwnership::default());
        }
        Err(error) => {
            return Err(SkillsCliError::Io {
                context: "read Skills CLI lock",
                source: error,
            });
        }
    };
    Ok(parse_lock_content(&content))
}

/// Tolerant lock parser: accepts `{ "version": 3, "skills": { name: {...} } }`
/// and flat `{ name: {...}, "version": 3 }` layouts.
pub(crate) fn parse_lock_content(content: &str) -> CliLockOwnership {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(content) else {
        return CliLockOwnership::default();
    };
    if value.get("version").and_then(|v| v.as_u64()) != Some(LOCK_SCHEMA_VERSION) {
        return CliLockOwnership::default();
    }

    let entries = match value.get("skills") {
        Some(serde_json::Value::Object(map)) => map,
        _ => match &value {
            serde_json::Value::Object(map) => map,
            _ => return CliLockOwnership::default(),
        },
    };

    let mut parsed = BTreeMap::new();
    for (key, value) in entries {
        if key == "version" {
            continue;
        }
        parsed.insert(key.clone(), lock_entry_from_value(value));
    }
    CliLockOwnership { entries: parsed }
}

fn json_string(value: &serde_json::Value, camel: &str, snake: &str) -> Option<String> {
    value
        .get(camel)
        .or_else(|| value.get(snake))
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
}

fn lock_entry_from_value(value: &serde_json::Value) -> CliLockEntry {
    CliLockEntry {
        source: json_string(value, "source", "source"),
        source_url: json_string(value, "sourceUrl", "source_url"),
        source_type: json_string(value, "sourceType", "source_type"),
    }
}

/// Resolve a symlink/junction target to an absolute normalized path relative
/// to the link's parent directory.
///
/// Junctions report `is_symlink()` through `symlink_metadata` on Windows and
/// resolve through `read_link`, so both link kinds share this helper.
pub fn resolved_link_target(link_path: &Path) -> Option<PathBuf> {
    let metadata = std::fs::symlink_metadata(link_path).ok()?;
    if !metadata.file_type().is_symlink() {
        return None;
    }
    let raw = std::fs::read_link(link_path).ok()?;
    let absolute = if raw.is_absolute() {
        raw
    } else {
        link_path.parent()?.join(raw)
    };
    Some(std::path::PathBuf::from(
        crate::paths::normalize_stored_path(&absolute.to_string_lossy()),
    ))
}

fn normalized(path: &Path) -> String {
    crate::paths::normalize_stored_path(&path.to_string_lossy())
}

fn is_candidate_inside_owned(
    candidate_norm: &str,
    universal_skills_dir: &Path,
    name: &str,
) -> bool {
    let canonical = normalized(&universal_skills_dir.join(name));
    candidate_norm == canonical || candidate_norm.starts_with(&format!("{canonical}/"))
}

/// Whether `candidate` is inside the CLI-owned canonical dir for `name`
/// (`universal_skills_dir/<name>[/...]`). Never true for unrelated siblings.
pub fn is_path_inside_owned_canonical(
    candidate: &Path,
    universal_skills_dir: &Path,
    name: &str,
) -> bool {
    is_candidate_inside_owned(&normalized(candidate), universal_skills_dir, name)
}

/// Ownership decision for one platform-side path (Local only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOrigin {
    /// Symlink/junction whose target resolves into a lock-owned canonical dir.
    SkillsCli,
    /// Everything else (Central installs, manual copies, unknown links).
    Other,
}

/// Classify one platform path against lock ownership.
///
/// Priority order per design §8:
/// 1. Path inside a lock-owned canonical dir → the CLI's real source.
/// 2. Symlink/junction resolving into a lock-owned canonical dir → platform link.
/// 3. Anything else → not CLI-owned.
pub fn classify_local_path_origin(
    path: &Path,
    universal_skills_dir: &Path,
    ownership: &CliLockOwnership,
) -> LinkOrigin {
    if ownership.is_empty() {
        return LinkOrigin::Other;
    }
    let candidate = normalized(path);
    // Junctions report `is_symlink()` via symlink_metadata on Windows and
    // resolve through read_link, so both link kinds share this resolution.
    let resolved = resolved_link_target(path).map(|target| normalized(&target));
    for name in ownership.names() {
        if is_candidate_inside_owned(&candidate, universal_skills_dir, name) {
            return LinkOrigin::SkillsCli;
        }
        if let Some(target) = &resolved {
            if is_candidate_inside_owned(target, universal_skills_dir, name) {
                return LinkOrigin::SkillsCli;
            }
        }
    }
    LinkOrigin::Other
}

/// Fill `install_origin` on platform skill rows from lock evidence.
///
/// Local-only: callers must not invoke this for SSH/WSL listings. A missing
/// or unreadable lock yields the link_type fallback (symlink → central).
pub fn annotate_platform_install_origins(skills: &mut [crate::db::SkillForAgent]) {
    let home = crate::paths::resolve_home_dir();
    let ownership = load_cli_lock_ownership(&skills_cli_lock_path(&home)).unwrap_or_default();
    annotate_platform_install_origins_with(
        &ownership,
        &crate::paths::universal_skills_dir(),
        skills,
    );
}

pub fn annotate_platform_install_origins_with(
    ownership: &CliLockOwnership,
    universal_skills_dir: &Path,
    skills: &mut [crate::db::SkillForAgent],
) {
    for skill in skills {
        skill.install_origin = match classify_local_path_origin(
            Path::new(&skill.dir_path),
            universal_skills_dir,
            ownership,
        ) {
            LinkOrigin::SkillsCli => "skills_cli".to_string(),
            LinkOrigin::Other if skill.link_type == "symlink" => "central".to_string(),
            LinkOrigin::Other => "standalone".to_string(),
        };
    }
}

/// Local leftover protection for PIN copy installs: lock contains `name` and
/// `path` is `{mapped_agent.global_skills_dir}/<name>`. Does not use
/// [`classify_local_path_origin`] (copy directories are `Other`).
pub fn is_mapped_agent_lock_copy(
    path: &Path,
    agent_global_skills_dir: &Path,
    agent_id: &str,
    ownership: &CliLockOwnership,
) -> bool {
    if cli_agent_for_skillport_id(agent_id).is_none() {
        return false;
    }
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if !ownership.contains_name(name) {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    normalized(parent) == normalized(agent_global_skills_dir)
}
