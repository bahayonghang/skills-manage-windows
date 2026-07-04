use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::db::DbPool;
use crate::paths;
use crate::services::scanner::scan_directory;

use super::error::ObsidianError;
use super::types::{ObsidianSkill, ObsidianVault, OBSIDIAN_PLATFORM_ID, OBSIDIAN_PLATFORM_NAME};

#[derive(Debug, Default, serde::Deserialize)]
struct ObsidianRegistryFile {
    #[serde(default)]
    vaults: std::collections::HashMap<String, ObsidianRegistryVault>,
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

fn normalize_obsidian_path(path: &str) -> String {
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

fn normalize_obsidian_pathbuf(path: &Path) -> String {
    normalize_obsidian_path(&path.to_string_lossy())
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
            .map(|path| PathBuf::from(normalize_obsidian_path(&path)))
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

fn scan_obsidian_vault(vault_dir: &Path, central_dir: &Path) -> Vec<ObsidianSkill> {
    if !is_obsidian_vault_dir(vault_dir) {
        return Vec::new();
    }

    let project_path = normalize_obsidian_pathbuf(vault_dir);
    let project_name = vault_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut selected_by_skill_id = BTreeMap::<String, ObsidianSkill>::new();

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
                ObsidianSkill {
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

    selected_by_skill_id.into_values().collect()
}

pub async fn get_obsidian_vaults_impl(_pool: &DbPool) -> Result<Vec<ObsidianVault>, ObsidianError> {
    let central_dir = paths::central_skills_dir();
    let mut vaults = crate::fs_util::run_blocking_fs_with(
        "Obsidian vault scan",
        move || {
            Ok(obsidian_source_vault_paths()
                .into_iter()
                .filter_map(|path| {
                    let normalized_path = normalize_obsidian_pathbuf(&path);
                    let skills = scan_obsidian_vault(&path, &central_dir);
                    if skills.is_empty() {
                        return None;
                    }
                    let name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    Some(ObsidianVault {
                        id: obsidian_vault_id(&normalized_path),
                        name,
                        path: normalized_path,
                        skill_count: skills.len(),
                    })
                })
                .collect::<Vec<_>>())
        },
        ObsidianError::task_join,
    )
    .await?;

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
) -> Result<Vec<ObsidianSkill>, ObsidianError> {
    let vault_id = vault_id.to_string();
    crate::fs_util::run_blocking_fs_with(
        "Obsidian vault skill scan",
        move || {
            let vault_path = obsidian_source_vault_paths()
                .into_iter()
                .find(|path| {
                    let normalized = normalize_obsidian_pathbuf(path);
                    obsidian_vault_id(&normalized) == vault_id || normalized == vault_id
                })
                .ok_or_else(|| ObsidianError::VaultNotFound(vault_id.clone()))?;

            Ok(scan_obsidian_vault(
                &vault_path,
                &paths::central_skills_dir(),
            ))
        },
        ObsidianError::task_join,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::write_skill_md;
    use tempfile::TempDir;

    fn make_vault(root: &Path) -> PathBuf {
        let vault = root.join("vault");
        std::fs::create_dir_all(vault.join(".obsidian")).unwrap();
        vault
    }

    /// 同一 skill id 同现 `.skills` 与 `.claude/skills` 时取 `.skills`，且不重复。
    #[test]
    fn scan_prefers_dot_skills_over_platform_sources() {
        let tmp = TempDir::new().unwrap();
        let vault = make_vault(tmp.path());
        let central = tmp.path().join("central");
        std::fs::create_dir_all(&central).unwrap();

        write_skill_md(&vault.join(".skills/alpha"), "alpha", Some("primary"));
        write_skill_md(&vault.join(".claude/skills/alpha"), "alpha", Some("shadowed"));
        write_skill_md(&vault.join(".claude/skills/beta"), "beta", Some("claude only"));

        let skills = scan_obsidian_vault(&vault, &central);
        assert_eq!(skills.len(), 2);
        let alpha = skills.iter().find(|s| s.name == "alpha").unwrap();
        assert_eq!(
            alpha.description.as_deref(),
            Some("primary"),
            ".skills 源应优先于 .claude/skills"
        );
        assert!(skills.iter().any(|s| s.name == "beta"));
    }

    #[test]
    fn scan_marks_skills_already_in_central() {
        let tmp = TempDir::new().unwrap();
        let vault = make_vault(tmp.path());
        let central = tmp.path().join("central");
        std::fs::create_dir_all(central.join("alpha")).unwrap();

        write_skill_md(&vault.join(".skills/alpha"), "alpha", None);
        write_skill_md(&vault.join(".skills/beta"), "beta", None);

        let skills = scan_obsidian_vault(&vault, &central);
        let alpha = skills.iter().find(|s| s.name == "alpha").unwrap();
        let beta = skills.iter().find(|s| s.name == "beta").unwrap();
        assert!(alpha.is_already_central);
        assert!(!beta.is_already_central);
    }

    /// 没有 `.obsidian` 标记的目录不是 vault，即便有 skill 也返回空。
    #[test]
    fn scan_returns_empty_for_non_vault_dir() {
        let tmp = TempDir::new().unwrap();
        let not_vault = tmp.path().join("plain");
        write_skill_md(&not_vault.join(".skills/alpha"), "alpha", None);
        let central = tmp.path().join("central");
        std::fs::create_dir_all(&central).unwrap();

        assert!(scan_obsidian_vault(&not_vault, &central).is_empty());
    }
}
