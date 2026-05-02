//! Skill scanning service: SKILL.md parsing, agent root discovery, and
//! local/remote scan orchestration. Used by `commands::scanner` (Tauri shell)
//! and by `commands::discover` (skill metadata extraction during project import).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::db::{self, AgentSkillObservation, DbPool, Skill, SkillInstallation};
use crate::targets::{
    connect_ssh_target, remote_file_type_is_dir, remote_join, remote_parent, RemoteTargetConfig,
};

mod claude_plugin;

use claude_plugin::{
    claude_observation_row_id, scan_roots_for_agent, AgentScanRoot, ClaudeSourceKind,
};

// ─── Types ────────────────────────────────────────────────────────────────────

/// Metadata extracted from a SKILL.md frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: Option<String>,
}

/// A single skill discovered during a directory scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedSkill {
    /// Derived from directory name (lowercase, spaces→hyphens).
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Absolute path to the SKILL.md file.
    pub file_path: String,
    /// Absolute path to the skill directory.
    pub dir_path: String,
    /// "symlink", "copy", or "native".
    pub link_type: String,
    /// Symlink target path, if link_type is "symlink".
    pub symlink_target: Option<String>,
    pub is_central: bool,
}

/// Summary returned by `scan_all_skills`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub total_skills: usize,
    pub agents_scanned: usize,
    pub skills_by_agent: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
struct DirectorySkillEntry {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    pub dir_path: String,
    pub is_symlink: bool,
    pub symlink_target: Option<String>,
}

// ─── Core Functions ───────────────────────────────────────────────────────────

/// Read a SKILL.md file and extract the YAML frontmatter fields `name` and
/// `description`. Returns `None` if the file is missing, cannot be read, lacks
/// a frontmatter block, or is missing the required `name` field.
pub fn parse_skill_md(path: &Path) -> Option<SkillInfo> {
    let content = std::fs::read_to_string(path).ok()?;
    parse_skill_md_content(&content)
}

pub fn parse_skill_md_content(content: &str) -> Option<SkillInfo> {
    // Frontmatter must begin on the very first line with "---"
    let after_open = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))?;

    // Locate the closing "---" delimiter
    let close_pos = after_open.find("\n---")?;
    let frontmatter_str = &after_open[..close_pos];

    // Parse the YAML block
    let yaml: serde_yaml::Value = serde_yaml::from_str(frontmatter_str).ok()?;

    // `name` is required
    let name = yaml.get("name")?.as_str()?.to_string();

    // `description` is optional
    let description = yaml
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some(SkillInfo { name, description })
}

/// Determine how a skill directory entry was installed at the given path.
///
/// Uses `symlink_metadata` (lstat) so the check is performed on the entry
/// itself rather than its target:
///
/// * `"symlink"` — the entry is a symbolic link.
/// * `"copy"`    — the entry is a regular directory in a platform skills dir.
/// * `"native"`  — the entry is a regular directory in the central skills dir.
///
/// Also returns the symlink target path when the entry is a symlink.
pub fn detect_link_type(path: &Path, is_central_dir: bool) -> (String, Option<String>) {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            let target = std::fs::read_link(path)
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()));
            ("symlink".to_string(), target)
        }
        _ => {
            let kind = if is_central_dir { "native" } else { "copy" };
            (kind.to_string(), None)
        }
    }
}

fn inspect_directory_entry(path: &Path) -> (bool, Option<String>) {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            let target = std::fs::read_link(path)
                .ok()
                .and_then(|target| target.to_str().map(|value| value.to_string()));
            (true, target)
        }
        _ => (false, None),
    }
}

fn build_scanned_skill(entry: &DirectorySkillEntry, is_central: bool) -> ScannedSkill {
    let link_type = if entry.is_symlink {
        "symlink".to_string()
    } else if is_central {
        "native".to_string()
    } else {
        "copy".to_string()
    };

    ScannedSkill {
        id: entry.id.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        file_path: entry.file_path.clone(),
        dir_path: entry.dir_path.clone(),
        link_type,
        symlink_target: entry.symlink_target.clone(),
        is_central,
    }
}

fn scan_directory_entries(dir: &Path) -> Vec<DirectorySkillEntry> {
    let mut skills = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return skills,
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();

        let meta = match std::fs::metadata(&entry_path) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        if !meta.is_dir() {
            continue;
        }

        let skill_md_path = entry_path.join("SKILL.md");
        if !skill_md_path.exists() {
            continue;
        }

        let info = match parse_skill_md(&skill_md_path) {
            Some(info) => info,
            None => continue,
        };

        let (is_symlink, symlink_target) = inspect_directory_entry(&entry_path);
        let id = entry_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|value| value.to_lowercase().replace(' ', "-"))
            .unwrap_or_else(|| "unknown".to_string());

        skills.push(DirectorySkillEntry {
            id,
            name: info.name,
            description: info.description,
            file_path: skill_md_path.to_string_lossy().into_owned(),
            dir_path: entry_path.to_string_lossy().into_owned(),
            is_symlink,
            symlink_target,
        });
    }

    skills
}

fn normalize_scan_key(dir: &Path) -> String {
    let normalized = dir
        .canonicalize()
        .unwrap_or_else(|_| dir.to_path_buf())
        .to_string_lossy()
        .into_owned();

    #[cfg(target_os = "windows")]
    {
        normalized.to_lowercase()
    }

    #[cfg(not(target_os = "windows"))]
    {
        normalized
    }
}

async fn delete_skills_not_in_scope(
    pool: &DbPool,
    found_skill_ids: &[String],
) -> Result<(), String> {
    if found_skill_ids.is_empty() {
        sqlx::query("DELETE FROM skill_installations")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

        return sqlx::query("DELETE FROM skills")
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string());
    }

    let placeholders = found_skill_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let installation_sql = format!(
        "DELETE FROM skill_installations WHERE skill_id NOT IN ({})",
        placeholders
    );
    let mut installation_query = sqlx::query(&installation_sql);
    for skill_id in found_skill_ids {
        installation_query = installation_query.bind(skill_id.as_str());
    }
    installation_query
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    let skill_sql = format!("DELETE FROM skills WHERE id NOT IN ({})", placeholders);
    let mut skill_query = sqlx::query(&skill_sql);
    for skill_id in found_skill_ids {
        skill_query = skill_query.bind(skill_id.as_str());
    }
    skill_query
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Walk `dir` one level deep, looking for immediate subdirectories that contain
/// a `SKILL.md` file. For each such subdirectory, `parse_skill_md` and
/// `detect_link_type` are called to build a `ScannedSkill`.
///
/// Entries that cannot be read or lack valid frontmatter are silently skipped.
pub fn scan_directory(dir: &Path, is_central: bool) -> Vec<ScannedSkill> {
    scan_directory_entries(dir)
        .into_iter()
        .map(|entry| build_scanned_skill(&entry, is_central))
        .collect()
}

async fn scan_directory_blocking(
    dir: PathBuf,
    is_central: bool,
) -> Result<Vec<ScannedSkill>, String> {
    tauri::async_runtime::spawn_blocking(move || scan_directory(&dir, is_central))
        .await
        .map_err(|e| format!("Failed to join directory scan task: {}", e))
}

// ─── Tauri Command ────────────────────────────────────────────────────────────

/// Core scanning logic, separated from the Tauri command layer so it can be
/// unit-tested without a running Tauri runtime.
pub async fn scan_all_skills_impl(pool: &DbPool) -> Result<ScanResult, String> {
    let agents = db::get_all_agents(pool).await?;
    let custom_dirs = db::get_scan_directories(pool).await?;
    let scan_started_at = Utc::now().to_rfc3339();
    let central_root = agents
        .iter()
        .find(|agent| agent.id == "central")
        .map(|agent| PathBuf::from(&agent.global_skills_dir));

    let mut total_skills: usize = 0;
    let mut skills_by_agent: HashMap<String, usize> = HashMap::new();
    let mut all_found_skill_ids: HashSet<String> = HashSet::new();
    let mut scanned_root_cache: HashMap<String, Vec<ScannedSkill>> = HashMap::new();
    let mut counted_scan_roots: HashSet<String> = HashSet::new();

    for agent in &agents {
        let is_central = agent.category == "central";
        let scan_roots = scan_roots_for_agent(agent);
        let existing_roots: Vec<AgentScanRoot> = scan_roots
            .into_iter()
            .filter(|root| root.path.exists())
            .collect();

        if existing_roots.is_empty() {
            db::update_agent_detected(pool, &agent.id, false).await?;
            skills_by_agent.insert(agent.id.clone(), 0);
            db::delete_stale_skill_installations(pool, &agent.id, &[]).await?;
            if agent.id == "claude-code" {
                db::delete_stale_agent_skill_observations(pool, &agent.id, &[]).await?;
            }
            continue;
        }

        db::update_agent_detected(pool, &agent.id, true).await?;

        let mut scanned = Vec::new();
        let mut found_install_ids = Vec::new();
        let mut found_observation_row_ids = Vec::new();

        for root in &existing_roots {
            let source_root = root
                .source_root
                .as_ref()
                .unwrap_or(&root.path)
                .to_string_lossy()
                .into_owned();
            let root_uses_central_storage = is_central
                || central_root
                    .as_ref()
                    .map(|central| crate::paths::paths_equivalent(&root.path, central))
                    .unwrap_or(false);
            let scan_key = normalize_scan_key(&root.path);
            let root_scanned = if let Some(cached) = scanned_root_cache.get(&scan_key) {
                cached.clone()
            } else {
                let scanned =
                    scan_directory_blocking(root.path.clone(), root_uses_central_storage).await?;
                scanned_root_cache.insert(scan_key.clone(), scanned.clone());
                scanned
            };
            if counted_scan_roots.insert(scan_key) {
                total_skills += root_scanned.len();
            }

            for skill in &root_scanned {
                if let Some(source_kind) = root.claude_source {
                    let observation = AgentSkillObservation {
                        row_id: claude_observation_row_id(&agent.id, &skill.dir_path),
                        agent_id: agent.id.clone(),
                        skill_id: skill.id.clone(),
                        name: skill.name.clone(),
                        description: skill.description.clone(),
                        file_path: skill.file_path.clone(),
                        dir_path: skill.dir_path.clone(),
                        source_kind: source_kind.as_str().to_string(),
                        source_root: source_root.clone(),
                        link_type: skill.link_type.clone(),
                        symlink_target: skill.symlink_target.clone(),
                        is_read_only: source_kind.is_read_only(),
                        scanned_at: scan_started_at.clone(),
                    };
                    db::upsert_agent_skill_observation(pool, &observation).await?;
                    found_observation_row_ids.push(observation.row_id);
                }

                let should_persist_manageable_state =
                    root.claude_source != Some(ClaudeSourceKind::Plugin);
                if should_persist_manageable_state {
                    all_found_skill_ids.insert(skill.id.clone());
                    found_install_ids.push(skill.id.clone());

                    let db_skill = Skill {
                        id: skill.id.clone(),
                        name: skill.name.clone(),
                        description: skill.description.clone(),
                        file_path: skill.file_path.clone(),
                        canonical_path: if root_uses_central_storage {
                            Some(skill.dir_path.clone())
                        } else {
                            None
                        },
                        is_central: root_uses_central_storage,
                        source: Some(skill.link_type.clone()),
                        content: None,
                        scanned_at: scan_started_at.clone(),
                    };
                    db::upsert_skill(pool, &db_skill).await?;

                    let installation = SkillInstallation {
                        skill_id: skill.id.clone(),
                        agent_id: agent.id.clone(),
                        installed_path: skill.dir_path.clone(),
                        link_type: skill.link_type.clone(),
                        symlink_target: skill.symlink_target.clone(),
                        created_at: scan_started_at.clone(),
                    };
                    db::upsert_skill_installation(pool, &installation).await?;
                }
            }

            scanned.extend(root_scanned);
        }

        db::delete_stale_skill_installations(pool, &agent.id, &found_install_ids).await?;
        if agent.id == "claude-code" {
            db::delete_stale_agent_skill_observations(pool, &agent.id, &found_observation_row_ids)
                .await?;
        }

        let count = scanned.len();
        skills_by_agent.insert(agent.id.clone(), count);
    }

    let mut seen_custom_dirs = HashSet::new();
    for scan_dir in custom_dirs.iter().filter(|dir| dir.is_active) {
        let dir = Path::new(&scan_dir.path);
        if !dir.exists() {
            continue;
        }

        let scan_key = normalize_scan_key(dir);
        if !seen_custom_dirs.insert(scan_key.clone()) {
            continue;
        }

        let scanned_skills = if let Some(cached) = scanned_root_cache.get(&scan_key) {
            cached.clone()
        } else {
            let scanned = scan_directory_blocking(dir.to_path_buf(), false).await?;
            scanned_root_cache.insert(scan_key.clone(), scanned.clone());
            scanned
        };
        for skill in &scanned_skills {
            all_found_skill_ids.insert(skill.id.clone());

            let db_skill = Skill {
                id: skill.id.clone(),
                name: skill.name.clone(),
                description: skill.description.clone(),
                file_path: skill.file_path.clone(),
                canonical_path: None,
                is_central: false,
                source: Some(skill.link_type.clone()),
                content: None,
                scanned_at: scan_started_at.clone(),
            };
            db::upsert_skill(pool, &db_skill).await?;
        }
        if counted_scan_roots.insert(scan_key) {
            total_skills += scanned_skills.len();
        }
    }

    let found_ids_vec: Vec<String> = all_found_skill_ids.into_iter().collect();
    delete_skills_not_in_scope(pool, &found_ids_vec).await?;

    Ok(ScanResult {
        total_skills,
        agents_scanned: agents.len(),
        skills_by_agent,
    })
}

async fn scan_ssh_directory(
    connection: &crate::targets::ConnectedSshTarget,
    dir: &str,
    is_central: bool,
) -> Result<Vec<ScannedSkill>, String> {
    if !connection.exists(dir).await? {
        return Ok(Vec::new());
    }

    let entries = connection.list_dir(dir).await?;
    let mut skills = Vec::new();

    for entry in entries {
        let is_symlink = entry.file_type == "symlink";
        if !remote_file_type_is_dir(&entry.file_type) && !is_symlink {
            continue;
        }

        let dir_path = remote_join(dir, &entry.name);
        let skill_md_path = remote_join(&dir_path, "SKILL.md");
        if !connection.exists(&skill_md_path).await? {
            continue;
        }

        let content = connection.read_file(&skill_md_path).await?;
        let content = match String::from_utf8(content) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let Some(info) = parse_skill_md_content(&content) else {
            continue;
        };

        let id = entry.name.to_lowercase().replace(' ', "-");
        let symlink_target = if is_symlink {
            entry.symlink_target.clone()
        } else {
            None
        };
        let link_type = if symlink_target.is_some() {
            "symlink".to_string()
        } else if is_central {
            "native".to_string()
        } else {
            "copy".to_string()
        };

        skills.push(ScannedSkill {
            id,
            name: info.name,
            description: info.description,
            file_path: skill_md_path,
            dir_path,
            link_type,
            symlink_target,
            is_central,
        });
    }

    Ok(skills)
}

async fn remote_agent_parent_detected(
    connection: &crate::targets::ConnectedSshTarget,
    global_skills_dir: &str,
) -> Result<bool, String> {
    let Some(parent) = remote_parent(global_skills_dir) else {
        return Ok(false);
    };
    connection.exists(&parent).await
}

pub async fn scan_ssh_skills_impl(
    pool: &DbPool,
    target: &RemoteTargetConfig,
) -> Result<ScanResult, String> {
    let agents = db::get_all_agents(pool).await?;
    let scan_started_at = Utc::now().to_rfc3339();
    let connection = connect_ssh_target(target).await?;
    let central_root = agents
        .iter()
        .find(|agent| agent.id == "central")
        .map(|agent| agent.global_skills_dir.clone());

    let mut total_skills: usize = 0;
    let mut skills_by_agent: HashMap<String, usize> = HashMap::new();
    let mut all_found_skill_ids: HashSet<String> = HashSet::new();
    let mut scanned_root_cache: HashMap<String, Vec<ScannedSkill>> = HashMap::new();
    let mut counted_scan_roots: HashSet<String> = HashSet::new();

    for agent in &agents {
        let root = agent.global_skills_dir.clone();
        let root_exists = connection.exists(&root).await?;
        if !root_exists && !remote_agent_parent_detected(&connection, &root).await? {
            db::update_agent_detected(pool, &agent.id, false).await?;
            skills_by_agent.insert(agent.id.clone(), 0);
            db::delete_stale_skill_installations(pool, &agent.id, &[]).await?;
            if agent.id == "claude-code" {
                db::delete_stale_agent_skill_observations(pool, &agent.id, &[]).await?;
            }
            continue;
        }

        db::update_agent_detected(pool, &agent.id, true).await?;
        if !root_exists {
            skills_by_agent.insert(agent.id.clone(), 0);
            db::delete_stale_skill_installations(pool, &agent.id, &[]).await?;
            if agent.id == "claude-code" {
                db::delete_stale_agent_skill_observations(pool, &agent.id, &[]).await?;
            }
            continue;
        }

        let root_uses_central_storage =
            agent.category == "central" || central_root.as_deref() == Some(root.as_str());
        let scanned = if let Some(cached) = scanned_root_cache.get(&root) {
            cached.clone()
        } else {
            let scanned = scan_ssh_directory(&connection, &root, root_uses_central_storage).await?;
            scanned_root_cache.insert(root.clone(), scanned.clone());
            scanned
        };

        if counted_scan_roots.insert(root.clone()) {
            total_skills += scanned.len();
        }

        let mut found_install_ids = Vec::new();
        for skill in &scanned {
            all_found_skill_ids.insert(skill.id.clone());
            found_install_ids.push(skill.id.clone());

            let db_skill = Skill {
                id: skill.id.clone(),
                name: skill.name.clone(),
                description: skill.description.clone(),
                file_path: skill.file_path.clone(),
                canonical_path: if root_uses_central_storage {
                    Some(skill.dir_path.clone())
                } else {
                    None
                },
                is_central: root_uses_central_storage,
                source: Some(skill.link_type.clone()),
                content: None,
                scanned_at: scan_started_at.clone(),
            };
            db::upsert_skill(pool, &db_skill).await?;

            let installation = SkillInstallation {
                skill_id: skill.id.clone(),
                agent_id: agent.id.clone(),
                installed_path: skill.dir_path.clone(),
                link_type: skill.link_type.clone(),
                symlink_target: skill.symlink_target.clone(),
                created_at: scan_started_at.clone(),
            };
            db::upsert_skill_installation(pool, &installation).await?;
        }

        db::delete_stale_skill_installations(pool, &agent.id, &found_install_ids).await?;
        if agent.id == "claude-code" {
            db::delete_stale_agent_skill_observations(pool, &agent.id, &[]).await?;
        }
        skills_by_agent.insert(agent.id.clone(), scanned.len());
    }

    let found_ids_vec: Vec<String> = all_found_skill_ids.into_iter().collect();
    delete_skills_not_in_scope(pool, &found_ids_vec).await?;

    Ok(ScanResult {
        total_skills,
        agents_scanned: agents.len(),
        skills_by_agent,
    })
}

// ─── Tests ────────────────────────────────────────────────────────────────────
//
// Integration tests for the scanner orchestration live in scanner/tests.rs;
// keeping them out of this file is what allows the production code here to
// stay under the 800-line cap.

#[cfg(test)]
mod tests;
