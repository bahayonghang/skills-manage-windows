//! Skill scanning service: SKILL.md parsing, agent root discovery, and
//! local/remote scan orchestration. Used by `commands::scanner` (Tauri shell)
//! and by `commands::discover` (skill metadata extraction during project import).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use futures_util::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::db::{self, AgentSkillObservation, DbPool, Skill, SkillInstallation};
use crate::skill_time::filesystem_timestamps_from_metadata;
use crate::targets::{connect_remote_target, ActiveTarget};

mod claude_plugin;
mod error;
mod persistence;
mod ssh_batch;

pub use error::ScannerError;

use claude_plugin::{
    agent_tracks_observations, observation_row_id, scan_roots_for_agent, AgentScanRoot, SourceKind,
};
use persistence::{persist_scan_batch, ScanPersistenceBatch};
use ssh_batch::{
    build_batch_read_script, build_probe_script, build_scanned_skills_from_contents,
    parse_batch_read_output, parse_probe_output, unique_skill_paths, RemoteScanItem,
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
    pub fs_created_at: Option<String>,
    pub fs_updated_at: Option<String>,
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
    pub fs_created_at: Option<String>,
    pub fs_updated_at: Option<String>,
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
    let yaml: serde_norway::Value = serde_norway::from_str(frontmatter_str).ok()?;

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
        fs_created_at: entry.fs_created_at.clone(),
        fs_updated_at: entry.fs_updated_at.clone(),
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
        let file_meta = std::fs::metadata(&skill_md_path).ok();

        let info = match parse_skill_md(&skill_md_path) {
            Some(info) => info,
            None => continue,
        };

        let (is_symlink, symlink_target) = inspect_directory_entry(&entry_path);
        let (fs_created_at, fs_updated_at) =
            filesystem_timestamps_from_metadata(Some(&meta), file_meta.as_ref());
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
            fs_created_at,
            fs_updated_at,
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

fn scan_parallelism_limit() -> usize {
    std::thread::available_parallelism()
        .map(|parallelism| parallelism.get().clamp(1, 8))
        .unwrap_or(4)
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
) -> Result<Vec<ScannedSkill>, ScannerError> {
    crate::fs_util::run_blocking_fs_with(
        "directory scan",
        move || Ok(scan_directory(&dir, is_central)),
        ScannerError::task_join,
    )
    .await
}

async fn scan_directory_with_limit(
    dir: PathBuf,
    is_central: bool,
    semaphore: Arc<Semaphore>,
) -> Result<Vec<ScannedSkill>, ScannerError> {
    let permit = semaphore
        .acquire_owned()
        .await
        .map_err(|_| ScannerError::SemaphoreClosed)?;
    let result = scan_directory_blocking(dir, is_central).await;
    drop(permit);
    result
}

// ─── Tauri Command ────────────────────────────────────────────────────────────

/// Core scanning logic, separated from the Tauri command layer so it can be
/// unit-tested without a running Tauri runtime.
pub async fn scan_all_skills_impl(pool: &DbPool) -> Result<ScanResult, ScannerError> {
    let agents = db::get_all_agents(pool).await?;
    let custom_dirs = db::get_scan_directories(pool).await?;
    let scan_started_at = Utc::now().to_rfc3339();
    let central_root = agents
        .iter()
        .find(|agent| agent.id == "central")
        .map(|agent| PathBuf::from(&agent.global_skills_dir));

    let mut total_skills: usize = 0;
    let mut skills_by_agent: HashMap<String, usize> = HashMap::new();
    let mut scanned_root_cache: HashMap<String, Vec<ScannedSkill>> = HashMap::new();
    let mut counted_scan_roots: HashSet<String> = HashSet::new();
    let scan_semaphore = Arc::new(Semaphore::new(scan_parallelism_limit()));
    let mut persistence = ScanPersistenceBatch::default();

    for agent in &agents {
        let is_central = agent.category == "central";
        let scan_roots = scan_roots_for_agent(agent);
        let existing_roots: Vec<AgentScanRoot> = scan_roots
            .into_iter()
            .filter(|root| root.path.exists())
            .collect();

        if existing_roots.is_empty() {
            persistence.set_agent_detected(&agent.id, false);
            persistence.touch_install_agent(&agent.id);
            if agent_tracks_observations(&agent.id) {
                persistence.touch_observation_agent(&agent.id);
            }
            skills_by_agent.insert(agent.id.clone(), 0);
            continue;
        }

        persistence.set_agent_detected(&agent.id, true);
        persistence.touch_install_agent(&agent.id);
        if agent_tracks_observations(&agent.id) {
            persistence.touch_observation_agent(&agent.id);
        }

        let mut scanned = Vec::new();
        let mut root_results: Vec<Option<Vec<ScannedSkill>>> = vec![None; existing_roots.len()];
        let mut pending_root_scans = Vec::new();

        for (index, root) in existing_roots.iter().enumerate() {
            let root_uses_central_storage = is_central
                || central_root
                    .as_ref()
                    .map(|central| crate::paths::paths_equivalent(&root.path, central))
                    .unwrap_or(false);
            let scan_key = normalize_scan_key(&root.path);
            if let Some(cached) = scanned_root_cache.get(&scan_key) {
                root_results[index] = Some(cached.clone());
            } else {
                pending_root_scans.push((
                    index,
                    scan_key,
                    root.path.clone(),
                    root_uses_central_storage,
                ));
            }
        }

        for (index, scan_key, scanned) in join_all(pending_root_scans.into_iter().map(
            |(index, scan_key, path, is_central_root)| {
                let semaphore = Arc::clone(&scan_semaphore);
                async move {
                    let scanned = scan_directory_with_limit(path, is_central_root, semaphore).await;
                    (index, scan_key, scanned)
                }
            },
        ))
        .await
        {
            let scanned = scanned?;
            scanned_root_cache.insert(scan_key, scanned.clone());
            root_results[index] = Some(scanned);
        }

        for (root, root_scanned) in existing_roots.iter().zip(root_results) {
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
            let root_scanned = root_scanned.unwrap_or_default();
            let scan_key = normalize_scan_key(&root.path);
            if counted_scan_roots.insert(scan_key) {
                total_skills += root_scanned.len();
            }

            for skill in &root_scanned {
                if let Some(source_kind) = root.source_kind {
                    let observation = AgentSkillObservation {
                        row_id: observation_row_id(&agent.id, &skill.dir_path),
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
                        fs_created_at: skill.fs_created_at.clone(),
                        fs_updated_at: skill.fs_updated_at.clone(),
                    };
                    persistence.remember_observation(&agent.id, &observation.row_id);
                    persistence.observations.push(observation);
                }

                let should_persist_manageable_state = root.source_kind != Some(SourceKind::Plugin);
                if should_persist_manageable_state {
                    persistence.global_found_skill_ids.insert(skill.id.clone());
                    persistence.remember_installation(&agent.id, &skill.id);

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
                        fs_created_at: skill.fs_created_at.clone(),
                        fs_updated_at: skill.fs_updated_at.clone(),
                    };
                    persistence.skills.push(db_skill);

                    let installation = SkillInstallation {
                        skill_id: skill.id.clone(),
                        agent_id: agent.id.clone(),
                        installed_path: skill.dir_path.clone(),
                        link_type: skill.link_type.clone(),
                        symlink_target: skill.symlink_target.clone(),
                        created_at: scan_started_at.clone(),
                    };
                    persistence.installations.push(installation);
                }
            }

            scanned.extend(root_scanned);
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
            let scanned =
                scan_directory_with_limit(dir.to_path_buf(), false, Arc::clone(&scan_semaphore))
                    .await?;
            scanned_root_cache.insert(scan_key.clone(), scanned.clone());
            scanned
        };
        for skill in &scanned_skills {
            persistence.global_found_skill_ids.insert(skill.id.clone());

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
                fs_created_at: skill.fs_created_at.clone(),
                fs_updated_at: skill.fs_updated_at.clone(),
            };
            persistence.skills.push(db_skill);
        }
        if counted_scan_roots.insert(scan_key) {
            total_skills += scanned_skills.len();
        }
    }

    persist_scan_batch(pool, persistence).await?;

    Ok(ScanResult {
        total_skills,
        agents_scanned: agents.len(),
        skills_by_agent,
    })
}

pub async fn scan_remote_skills_impl(
    pool: &DbPool,
    active_target: &ActiveTarget,
) -> Result<ScanResult, ScannerError> {
    let agents = db::get_all_agents(pool).await?;
    let scan_started_at = Utc::now().to_rfc3339();
    let connection = connect_remote_target(active_target)
        .await
        .map_err(ScannerError::Remote)?;
    let central_root = agents
        .iter()
        .find(|agent| agent.id == "central")
        .map(|agent| agent.global_skills_dir.clone());
    let scan_roots_by_agent: Vec<(crate::db::Agent, Vec<AgentScanRoot>)> = agents
        .iter()
        .cloned()
        .map(|agent| {
            let scan_roots = if agent.id == "claude-code" {
                scan_roots_for_agent(&agent)
            } else {
                vec![AgentScanRoot {
                    path: PathBuf::from(&agent.global_skills_dir),
                    source_root: None,
                    source_kind: None,
                }]
            };
            (agent, scan_roots)
        })
        .collect();

    let mut unique_roots = Vec::new();
    let mut seen_roots = HashSet::new();
    for (_, roots) in &scan_roots_by_agent {
        for root in roots {
            let root_str = root.path.to_string_lossy().into_owned();
            if seen_roots.insert(root_str.clone()) {
                unique_roots.push(root_str);
            }
        }
    }

    let probe_output = connection
        .run_script(&build_probe_script(&unique_roots), &[])
        .await
        .map_err(ScannerError::Remote)?;
    let probe_items = parse_probe_output(&probe_output);
    let mut root_exists = HashSet::new();
    let mut root_parent_exists = HashSet::new();
    let mut skill_items_by_root: HashMap<String, Vec<RemoteScanItem>> = HashMap::new();
    for item in &probe_items {
        match item {
            RemoteScanItem::RootOk { root } => {
                root_exists.insert(root.clone());
            }
            RemoteScanItem::RootParentOk { root } => {
                root_parent_exists.insert(root.clone());
            }
            RemoteScanItem::RootMiss { .. } => {}
            RemoteScanItem::Skill { root, .. } => {
                skill_items_by_root
                    .entry(root.clone())
                    .or_default()
                    .push(item.clone());
            }
        }
    }

    let unique_skill_paths = unique_skill_paths(&probe_items);
    let content_by_path = if unique_skill_paths.is_empty() {
        HashMap::new()
    } else {
        let read_output = connection
            .run_script(&build_batch_read_script(&unique_skill_paths), &[])
            .await
            .map_err(ScannerError::Remote)?;
        parse_batch_read_output(&read_output)
    };

    let mut total_skills: usize = 0;
    let mut skills_by_agent: HashMap<String, usize> = HashMap::new();
    let mut scanned_root_cache: HashMap<String, Vec<ScannedSkill>> = HashMap::new();
    let mut counted_scan_roots: HashSet<String> = HashSet::new();
    let mut persistence = ScanPersistenceBatch::default();

    for (agent, scan_roots) in &scan_roots_by_agent {
        let existing_roots: Vec<AgentScanRoot> = scan_roots
            .iter()
            .filter(|root| root_exists.contains(&root.path.to_string_lossy().into_owned()))
            .cloned()
            .collect();
        let parent_visible = scan_roots
            .iter()
            .any(|root| root_parent_exists.contains(&root.path.to_string_lossy().into_owned()));

        if existing_roots.is_empty() && !parent_visible {
            persistence.set_agent_detected(&agent.id, false);
            persistence.touch_install_agent(&agent.id);
            if agent_tracks_observations(&agent.id) {
                persistence.touch_observation_agent(&agent.id);
            }
            skills_by_agent.insert(agent.id.clone(), 0);
            continue;
        }

        persistence.set_agent_detected(&agent.id, true);
        persistence.touch_install_agent(&agent.id);
        if agent_tracks_observations(&agent.id) {
            persistence.touch_observation_agent(&agent.id);
        }
        if existing_roots.is_empty() {
            skills_by_agent.insert(agent.id.clone(), 0);
            continue;
        }

        let mut scanned = Vec::new();

        for root in &existing_roots {
            let root_str = root.path.to_string_lossy().into_owned();
            let root_uses_central_storage =
                agent.category == "central" || central_root.as_deref() == Some(root_str.as_str());
            let root_scanned = if let Some(cached) = scanned_root_cache.get(&root_str) {
                cached.clone()
            } else {
                let root_items = skill_items_by_root
                    .get(&root_str)
                    .cloned()
                    .unwrap_or_default();
                let scanned_for_root = build_scanned_skills_from_contents(
                    &root_items,
                    &content_by_path,
                    root_uses_central_storage,
                );
                scanned_root_cache.insert(root_str.clone(), scanned_for_root.clone());
                scanned_for_root
            };

            if counted_scan_roots.insert(root_str.clone()) {
                total_skills += root_scanned.len();
            }

            let source_root = root
                .source_root
                .as_ref()
                .unwrap_or(&root.path)
                .to_string_lossy()
                .into_owned();

            for skill in &root_scanned {
                if let Some(source_kind) = root.source_kind {
                    let observation = AgentSkillObservation {
                        row_id: observation_row_id(&agent.id, &skill.dir_path),
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
                        fs_created_at: skill.fs_created_at.clone(),
                        fs_updated_at: skill.fs_updated_at.clone(),
                    };
                    persistence.remember_observation(&agent.id, &observation.row_id);
                    persistence.observations.push(observation);
                }

                let should_persist_manageable_state = root.source_kind != Some(SourceKind::Plugin);
                if should_persist_manageable_state {
                    persistence.global_found_skill_ids.insert(skill.id.clone());
                    persistence.remember_installation(&agent.id, &skill.id);

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
                        fs_created_at: skill.fs_created_at.clone(),
                        fs_updated_at: skill.fs_updated_at.clone(),
                    };
                    persistence.skills.push(db_skill);

                    let installation = SkillInstallation {
                        skill_id: skill.id.clone(),
                        agent_id: agent.id.clone(),
                        installed_path: skill.dir_path.clone(),
                        link_type: skill.link_type.clone(),
                        symlink_target: skill.symlink_target.clone(),
                        created_at: scan_started_at.clone(),
                    };
                    persistence.installations.push(installation);
                }
            }

            scanned.extend(root_scanned);
        }

        let count = scanned.len();
        skills_by_agent.insert(agent.id.clone(), count);
    }

    persist_scan_batch(pool, persistence).await?;

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
