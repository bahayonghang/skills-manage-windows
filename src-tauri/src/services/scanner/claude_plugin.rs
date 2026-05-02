//! Claude plugin discovery: parses ~/.claude/settings.json and
//! ~/.claude/plugins/installed_plugins.json to enumerate enabled-plugin skill
//! roots, plus a few small helpers (read_json_file, claude_observation_row_id,
//! scan_roots_for_agent) that are only relevant to Claude-aware scanning.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClaudeSourceKind {
    User,
    Plugin,
}

impl ClaudeSourceKind {
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
    pub(super) claude_source: Option<ClaudeSourceKind>,
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
                    claude_source: Some(ClaudeSourceKind::Plugin),
                });
            }
        }
    }

    roots
}

pub(super) fn scan_roots_for_agent(agent: &crate::db::Agent) -> Vec<AgentScanRoot> {
    let primary_root = PathBuf::from(&agent.global_skills_dir);
    if agent.id != "claude-code" {
        return vec![AgentScanRoot {
            path: primary_root,
            source_root: None,
            claude_source: None,
        }];
    }

    let mut roots = vec![AgentScanRoot {
        path: primary_root.clone(),
        source_root: Some(primary_root.clone()),
        claude_source: Some(ClaudeSourceKind::User),
    }];
    roots.extend(claude_plugin_roots(&primary_root));
    roots
}

pub(super) fn claude_observation_row_id(agent_id: &str, dir_path: &str) -> String {
    format!("{agent_id}::{dir_path}")
}
