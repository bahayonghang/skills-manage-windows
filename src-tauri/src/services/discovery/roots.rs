use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::db::{self, DbPool};
use crate::paths;

use super::types::ScanRoot;

const SCAN_ROOTS_CONFIG_KEY: &str = "discover_scan_roots_config";

/// Returns a list of candidate scan roots, checking which ones exist on disk.
pub fn default_scan_roots() -> Vec<ScanRoot> {
    let home = paths::resolve_home_dir();
    let candidates = vec![
        (paths::path_to_string(&home.join("projects")), "projects"),
        (paths::path_to_string(&home.join("Documents")), "Documents"),
        (paths::path_to_string(&home.join("Developer")), "Developer"),
        (paths::path_to_string(&home.join("work")), "work"),
        (paths::path_to_string(&home.join("src")), "src"),
        (paths::path_to_string(&home.join("code")), "code"),
        (paths::path_to_string(&home.join("repos")), "repos"),
        (paths::path_to_string(&home.join("Desktop")), "Desktop"),
        // macOS: scan /Applications for apps with built-in skills (e.g. OpenClaw)
        ("/Applications".to_string(), "Applications"),
    ];

    candidates
        .into_iter()
        .map(|(path, label)| {
            let exists = Path::new(&path).exists();
            ScanRoot {
                path,
                label: label.to_string(),
                exists,
                enabled: exists,
            }
        })
        .collect()
}

/// Build the list of platform skill directory patterns to look for.
///
/// For each enabled agent, its `global_skills_dir` is split to derive a
/// relative pattern like `.claude/skills` from `/home/user/.claude/skills`.
pub fn platform_skill_patterns(_pool: &DbPool) -> Vec<(String, String, PathBuf)> {
    let home = paths::resolve_home_dir();
    let mut seen = std::collections::HashSet::new();

    db::builtin_agents()
        .iter()
        .filter(|a| a.id != "central")
        .filter_map(|a| {
            let rel = match a.project_skills_dir.as_deref() {
                Some(project_skills_dir) if !project_skills_dir.trim().is_empty() => {
                    let trimmed = project_skills_dir.trim();
                    let relative = trimmed
                        .strip_prefix("~/")
                        .or_else(|| trimmed.strip_prefix("~\\"))
                        .unwrap_or(trimmed);
                    let path = PathBuf::from(relative);
                    if path.is_absolute() {
                        return None;
                    }
                    path
                }
                _ => {
                    let full = PathBuf::from(&a.global_skills_dir);
                    full.strip_prefix(&home).ok()?.to_path_buf()
                }
            };
            if !seen.insert(rel.clone()) {
                return None;
            }
            Some((a.id.clone(), a.display_name.clone(), rel))
        })
        .collect()
}

/// Get scan roots with persisted enabled state from DB.
pub async fn get_scan_roots_impl(pool: &DbPool) -> Result<Vec<ScanRoot>, String> {
    let mut roots = default_scan_roots();

    if let Some(json) = db::get_setting(pool, SCAN_ROOTS_CONFIG_KEY).await? {
        let config: HashMap<String, bool> =
            serde_json::from_str(&json).map_err(|e| format!("Invalid scan roots config: {}", e))?;
        for root in &mut roots {
            if let Some(&enabled) = config.get(&root.path) {
                root.enabled = enabled;
            }
        }
    }

    Ok(roots)
}

/// Persist the enabled/disabled state of a scan root.
pub async fn set_scan_root_enabled_impl(
    pool: &DbPool,
    path: String,
    enabled: bool,
) -> Result<(), String> {
    let mut config: HashMap<String, bool> =
        match db::get_setting(pool, SCAN_ROOTS_CONFIG_KEY).await? {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| format!("Invalid scan roots config: {}", e))?,
            None => HashMap::new(),
        };

    config.insert(path, enabled);

    let json = serde_json::to_string(&config)
        .map_err(|e| format!("Failed to serialize scan roots config: {}", e))?;
    db::set_setting(pool, SCAN_ROOTS_CONFIG_KEY, &json).await
}
