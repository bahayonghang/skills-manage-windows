//! Agent source-root discovery: user-managed skill roots plus read-only plugin
//! roots. Claude plugins are discovered from ~/.claude runtime metadata; Codex
//! plugins are discovered from the local ~/.codex/plugins/cache tree.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceKind {
    User,
    Plugin,
}

impl SourceKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Plugin => "plugin",
        }
    }

    pub(super) fn is_read_only(self) -> bool {
        matches!(self, Self::Plugin)
    }
}

#[derive(Debug, Clone)]
pub(super) struct AgentScanRoot {
    pub(super) path: PathBuf,
    pub(super) source_root: Option<PathBuf>,
    pub(super) source_kind: Option<SourceKind>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct ClaudeSettingsFile {
    #[serde(default, rename = "enabledPlugins")]
    enabled_plugins: HashMap<String, bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct ClaudeInstalledPluginsFile {
    #[serde(default)]
    plugins: HashMap<String, Vec<ClaudeInstalledPluginInstall>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct ClaudeInstalledPluginInstall {
    #[serde(default)]
    scope: Option<String>,
    #[serde(rename = "installPath")]
    install_path: String,
    #[serde(default, rename = "installedAt")]
    installed_at: Option<String>,
    #[serde(default, rename = "lastUpdated")]
    last_updated: Option<String>,
}

pub(super) fn read_json_file<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub(super) fn claude_runtime_root(global_skills_dir: &Path) -> Option<PathBuf> {
    global_skills_dir.parent().map(Path::to_path_buf)
}

pub(super) fn claude_enabled_plugin_ids(claude_root: &Path) -> Vec<String> {
    let settings_path = claude_root.join("settings.json");
    let Some(settings) = read_json_file::<ClaudeSettingsFile>(&settings_path) else {
        return Vec::new();
    };

    let mut enabled: Vec<String> = settings
        .enabled_plugins
        .into_iter()
        .filter_map(|(plugin_id, is_enabled)| is_enabled.then_some(plugin_id))
        .collect();
    enabled.sort();
    enabled
}

pub(super) fn claude_select_effective_plugin_installs(
    installs: &[ClaudeInstalledPluginInstall],
) -> Vec<ClaudeInstalledPluginInstall> {
    let preferred_scope = installs
        .iter()
        .any(|install| install.scope.as_deref() == Some("user"));

    installs
        .iter()
        .filter(|install| !preferred_scope || install.scope.as_deref() == Some("user"))
        .max_by(|left, right| {
            let left_key = left
                .last_updated
                .as_deref()
                .or(left.installed_at.as_deref())
                .unwrap_or("");
            let right_key = right
                .last_updated
                .as_deref()
                .or(right.installed_at.as_deref())
                .unwrap_or("");
            left_key
                .cmp(right_key)
                .then_with(|| left.install_path.cmp(&right.install_path))
        })
        .cloned()
        .into_iter()
        .collect()
}

pub(super) fn claude_plugin_roots(global_skills_dir: &Path) -> Vec<AgentScanRoot> {
    let Some(claude_root) = claude_runtime_root(global_skills_dir) else {
        return Vec::new();
    };

    let installed_path = claude_root.join("plugins/installed_plugins.json");
    let installed =
        read_json_file::<ClaudeInstalledPluginsFile>(&installed_path).unwrap_or_default();
    let mut seen_scan_paths = HashSet::new();
    let mut roots = Vec::new();

    for plugin_id in claude_enabled_plugin_ids(&claude_root) {
        let Some(installs) = installed.plugins.get(&plugin_id) else {
            continue;
        };

        for install in claude_select_effective_plugin_installs(installs) {
            let install_root = PathBuf::from(&install.install_path);
            let candidate_paths = [
                install_root.join("skills"),
                install_root.join(".claude").join("skills"),
            ];

            for scan_path in candidate_paths {
                if !scan_path.exists() {
                    continue;
                }

                let scan_path_key = scan_path.to_string_lossy().into_owned();
                if !seen_scan_paths.insert(scan_path_key) {
                    continue;
                }

                roots.push(AgentScanRoot {
                    path: scan_path,
                    source_root: Some(install_root.clone()),
                    source_kind: Some(SourceKind::Plugin),
                });
            }
        }
    }

    roots
}

fn home_from_platform_skills_dir(global_skills_dir: &Path) -> Option<PathBuf> {
    let parent = global_skills_dir.parent()?;
    let parent_name = parent.file_name()?.to_string_lossy();
    if parent_name.eq_ignore_ascii_case(crate::paths::UNIVERSAL_AGENTS_DIR_NAME)
        || parent_name.eq_ignore_ascii_case(".codex")
    {
        return parent.parent().map(Path::to_path_buf);
    }

    None
}

fn plugin_source_root_for_skills_dir(skills_dir: &Path) -> PathBuf {
    let Some(parent) = skills_dir.parent() else {
        return skills_dir.to_path_buf();
    };

    let parent_name = parent
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    if parent_name.eq_ignore_ascii_case(".codex") || parent_name.eq_ignore_ascii_case(".claude") {
        return parent.parent().unwrap_or(parent).to_path_buf();
    }

    parent.to_path_buf()
}

fn collect_codex_skill_roots(dir: &Path, depth: usize, max_depth: usize, roots: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }

        let path = entry.path();
        let is_skills_dir = path
            .file_name()
            .map(|name| name.to_string_lossy().eq_ignore_ascii_case("skills"))
            .unwrap_or(false);
        if is_skills_dir {
            roots.push(path);
            continue;
        }

        collect_codex_skill_roots(&path, depth + 1, max_depth, roots);
    }
}

pub(super) fn codex_plugin_roots(global_skills_dir: &Path) -> Vec<AgentScanRoot> {
    let Some(home) = home_from_platform_skills_dir(global_skills_dir) else {
        return Vec::new();
    };

    let cache_root = home.join(".codex").join("plugins").join("cache");
    if !cache_root.exists() {
        return Vec::new();
    }

    let mut scan_paths = Vec::new();
    collect_codex_skill_roots(&cache_root, 0, 8, &mut scan_paths);
    scan_paths.sort();

    let mut seen_scan_paths = HashSet::new();
    scan_paths
        .into_iter()
        .filter_map(|scan_path| {
            let scan_path_key = scan_path.to_string_lossy().into_owned();
            if !seen_scan_paths.insert(scan_path_key) {
                return None;
            }

            Some(AgentScanRoot {
                source_root: Some(plugin_source_root_for_skills_dir(&scan_path)),
                path: scan_path,
                source_kind: Some(SourceKind::Plugin),
            })
        })
        .collect()
}

pub(super) fn scan_roots_for_agent(agent: &crate::db::Agent) -> Vec<AgentScanRoot> {
    let primary_root = PathBuf::from(&agent.global_skills_dir);
    if agent.id == "claude-code" {
        let mut roots = vec![AgentScanRoot {
            path: primary_root.clone(),
            source_root: Some(primary_root.clone()),
            source_kind: Some(SourceKind::User),
        }];
        roots.extend(claude_plugin_roots(&primary_root));
        return roots;
    }

    let mut roots = vec![AgentScanRoot {
        path: primary_root.clone(),
        source_root: None,
        source_kind: None,
    }];

    if agent.id == "codex" {
        roots.extend(codex_plugin_roots(&primary_root));
    }

    roots
}

pub(super) fn agent_tracks_observations(agent_id: &str) -> bool {
    matches!(agent_id, "claude-code" | "codex")
}

pub(super) fn observation_row_id(agent_id: &str, dir_path: &str) -> String {
    format!("{agent_id}::{dir_path}")
}
