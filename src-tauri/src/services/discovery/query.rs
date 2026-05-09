use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::db::{self, DbPool};
use crate::paths;
use crate::services::scanner::scan_directory;

use super::types::{
    DiscoveredProject, DiscoveredSkill, DiscoveredSummary, ObsidianVault, OBSIDIAN_PLATFORM_ID,
    OBSIDIAN_PLATFORM_NAME,
};

pub async fn get_discovered_summary_impl(pool: &DbPool) -> Result<DiscoveredSummary, String> {
    let total_skills_found = db::get_discovered_skill_count(pool).await?;
    let total_projects_found = db::get_discovered_project_count(pool).await?.max(0) as usize;

    Ok(DiscoveredSummary {
        total_skills_found,
        total_projects_found,
    })
}

/// Load previously discovered skills from the database, grouped by project.
pub async fn get_discovered_skills_impl(pool: &DbPool) -> Result<Vec<DiscoveredProject>, String> {
    let rows = db::get_all_discovered_skills(pool).await?;

    let central_dir = paths::central_skills_dir();
    let platform_names: HashMap<String, String> = db::builtin_agents()
        .into_iter()
        .map(|agent| (agent.id, agent.display_name))
        .collect();

    let skills: Vec<DiscoveredSkill> = rows
        .into_iter()
        .map(|row| {
            let skill_dir_name = Path::new(&row.dir_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            let is_already_central = central_dir.join(skill_dir_name).exists();
            let platform_id = row.platform_id.clone();

            DiscoveredSkill {
                id: row.id,
                name: row.name,
                description: row.description,
                file_path: row.file_path,
                dir_path: row.dir_path,
                platform_id: platform_id.clone(),
                platform_name: platform_names
                    .get(&platform_id)
                    .cloned()
                    .unwrap_or_else(|| platform_id.clone()),
                project_path: row.project_path,
                project_name: row.project_name,
                is_already_central,
            }
        })
        .collect();

    let mut by_project: HashMap<String, Vec<DiscoveredSkill>> = HashMap::new();
    let mut project_names: HashMap<String, String> = HashMap::new();

    for skill in skills {
        project_names.insert(skill.project_path.clone(), skill.project_name.clone());
        by_project
            .entry(skill.project_path.clone())
            .or_default()
            .push(skill);
    }

    let mut projects: Vec<DiscoveredProject> = by_project
        .into_iter()
        .map(|(path, skills)| DiscoveredProject {
            project_path: path.clone(),
            project_name: project_names.get(&path).cloned().unwrap_or_default(),
            skills,
        })
        .collect();

    projects.sort_by(|a, b| a.project_name.cmp(&b.project_name));

    Ok(projects)
}

/// Clear all discovered skills from the database.
pub async fn clear_discovered_skills_impl(pool: &DbPool) -> Result<(), String> {
    db::clear_all_discovered_skills(pool).await
}

#[derive(Debug, Default, serde::Deserialize)]
struct ObsidianRegistryFile {
    #[serde(default)]
    vaults: HashMap<String, ObsidianRegistryVault>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct ObsidianRegistryVault {
    #[serde(default)]
    path: Option<String>,
}

fn stable_path_hash(path: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in path.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn obsidian_vault_id(vault_path: &str) -> String {
    stable_path_hash(vault_path)
}

fn obsidian_qualified_id(vault_path: &str, skill_id: &str) -> String {
    format!(
        "{}__{}__{}",
        OBSIDIAN_PLATFORM_ID,
        obsidian_vault_id(vault_path),
        skill_id
    )
}

fn is_obsidian_vault_dir(path: &Path) -> bool {
    path.join(".obsidian").is_dir()
}

fn normalize_discovery_path(path: &str) -> String {
    let normalized = path.trim().replace('\\', "/");
    let normalized = if normalized == "~" {
        normalized
    } else {
        normalized
            .strip_prefix("~/")
            .map(|suffix| {
                paths::resolve_home_dir()
                    .join(suffix)
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .unwrap_or(normalized)
    };
    normalized.trim_end_matches('/').to_string()
}

fn normalize_discovery_pathbuf(path: &Path) -> String {
    normalize_discovery_path(&path.to_string_lossy())
}

fn obsidian_registry_candidates() -> Vec<PathBuf> {
    let home = paths::resolve_home_dir();
    let mut candidates = vec![
        home.join("Library")
            .join("Application Support")
            .join("obsidian")
            .join("obsidian.json"),
        home.join("AppData")
            .join("Roaming")
            .join("Obsidian")
            .join("obsidian.json"),
        home.join(".config").join("obsidian").join("obsidian.json"),
    ];
    candidates.dedup();
    candidates
}

fn obsidian_fallback_roots() -> Vec<PathBuf> {
    let home = paths::resolve_home_dir();
    let mut candidates = vec![
        home.join("Documents"),
        home.join("Desktop"),
        home.join("projects"),
        home.join("work"),
        home.join("Developer"),
        home.join("code"),
        home.join("repos"),
    ];
    candidates.retain(|path| path.exists());
    candidates
}

fn read_obsidian_registry_vault_paths() -> Vec<PathBuf> {
    for registry_path in obsidian_registry_candidates() {
        let content = match std::fs::read_to_string(&registry_path) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let registry: ObsidianRegistryFile = match serde_json::from_str(&content) {
            Ok(registry) => registry,
            Err(_) => continue,
        };
        let mut paths: Vec<PathBuf> = registry
            .vaults
            .into_values()
            .filter_map(|vault| vault.path)
            .map(|path| PathBuf::from(normalize_discovery_path(&path)))
            .filter(|path| path.is_dir() && is_obsidian_vault_dir(path))
            .collect();
        paths.sort();
        paths.dedup();
        if !paths.is_empty() {
            return paths;
        }
    }
    Vec::new()
}

fn direct_obsidian_vault_children(root: &Path) -> Vec<PathBuf> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut vaults = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && is_obsidian_vault_dir(&path) {
            vaults.push(path);
        }
    }
    vaults.sort();
    vaults.dedup();
    vaults
}

fn obsidian_source_vault_paths() -> Vec<PathBuf> {
    let registry_paths = read_obsidian_registry_vault_paths();
    if !registry_paths.is_empty() {
        return registry_paths;
    }

    let mut results = Vec::new();
    for root in obsidian_fallback_roots() {
        if is_obsidian_vault_dir(&root) {
            results.push(root.clone());
        }
        results.extend(direct_obsidian_vault_children(&root));
    }
    results.sort();
    results.dedup();
    results
}

fn selected_skill_dir_name(dir_path: &str) -> String {
    Path::new(dir_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn scan_obsidian_vault(vault_dir: &Path, central_dir: &Path) -> Option<DiscoveredProject> {
    if !is_obsidian_vault_dir(vault_dir) {
        return None;
    }

    let project_path = normalize_discovery_pathbuf(vault_dir);
    let project_name = vault_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut selected_by_skill_id = std::collections::BTreeMap::<String, DiscoveredSkill>::new();

    for rel_source in [
        PathBuf::from(".skills"),
        PathBuf::from(".agents/skills"),
        PathBuf::from(".claude/skills"),
    ] {
        let source_dir = vault_dir.join(rel_source);
        let mut scanned = scan_directory(&source_dir, false);
        scanned.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.dir_path.cmp(&b.dir_path)));

        for skill in scanned {
            if selected_by_skill_id.contains_key(&skill.id) {
                continue;
            }

            let skill_dir_name = selected_skill_dir_name(&skill.dir_path);
            let is_already_central = central_dir.join(skill_dir_name).exists();
            selected_by_skill_id.insert(
                skill.id.clone(),
                DiscoveredSkill {
                    id: obsidian_qualified_id(&project_path, &skill.id),
                    name: skill.name,
                    description: skill.description,
                    file_path: skill.file_path,
                    dir_path: skill.dir_path,
                    platform_id: OBSIDIAN_PLATFORM_ID.to_string(),
                    platform_name: OBSIDIAN_PLATFORM_NAME.to_string(),
                    project_path: project_path.clone(),
                    project_name: project_name.clone(),
                    is_already_central,
                },
            );
        }
    }

    if selected_by_skill_id.is_empty() {
        None
    } else {
        Some(DiscoveredProject {
            project_path,
            project_name,
            skills: selected_by_skill_id.into_values().collect(),
        })
    }
}

pub async fn get_obsidian_vaults_impl(_pool: &DbPool) -> Result<Vec<ObsidianVault>, String> {
    let central_dir = paths::central_skills_dir();
    let mut vaults = obsidian_source_vault_paths()
        .into_iter()
        .filter_map(|path| {
            let normalized_path = normalize_discovery_pathbuf(&path);
            let project = scan_obsidian_vault(&path, &central_dir)?;
            Some(ObsidianVault {
                id: obsidian_vault_id(&normalized_path),
                name: project.project_name,
                path: normalized_path,
                skill_count: project.skills.len(),
            })
        })
        .collect::<Vec<_>>();

    vaults.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(vaults)
}

pub async fn get_obsidian_vault_skills_impl(
    _pool: &DbPool,
    vault_id: &str,
) -> Result<Vec<DiscoveredSkill>, String> {
    let vault_path = obsidian_source_vault_paths()
        .into_iter()
        .find(|path| {
            let normalized = normalize_discovery_pathbuf(path);
            obsidian_vault_id(&normalized) == vault_id || normalized == vault_id
        })
        .ok_or_else(|| format!("Obsidian vault '{}' not found", vault_id))?;

    Ok(
        scan_obsidian_vault(&vault_path, &paths::central_skills_dir())
            .map(|project| project.skills)
            .unwrap_or_default(),
    )
}
