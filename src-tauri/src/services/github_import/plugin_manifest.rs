use crate::services::resource_budget::ResourceBudget;

use super::source::{join_repo_path, normalize_repo_path};
use super::*;

#[derive(Debug, Clone, Default)]
pub(super) struct PluginManifestDiscovery {
    pub(super) explicit_skill_paths: Vec<String>,
    pub(super) plugin_by_source_path: HashMap<String, String>,
}

pub(super) fn plugin_manifest_discovery_from_snapshot(
    snapshot: &GitHubRepoSnapshot,
    source_path: Option<&str>,
) -> Result<PluginManifestDiscovery, GithubImportError> {
    let base_path = effective_source_root(source_path)?;
    let plugin_json_path = join_repo_path(&base_path, ".claude-plugin/plugin.json")?;
    let marketplace_json_path = join_repo_path(&base_path, ".claude-plugin/marketplace.json")?;

    Ok(plugin_manifest_discovery_from_manifest_bytes(
        &base_path,
        snapshot.files.get(&plugin_json_path).map(Vec::as_slice),
        snapshot
            .files
            .get(&marketplace_json_path)
            .map(Vec::as_slice),
    ))
}

pub(super) async fn plugin_manifest_discovery_from_remote_workspace(
    connection: &ConnectedRemoteTarget,
    remote_repo_dir: &str,
    source_path: Option<&str>,
) -> Result<PluginManifestDiscovery, GithubImportError> {
    let base_path = effective_source_root(source_path)?;
    let plugin_json_path = join_repo_path(&base_path, ".claude-plugin/plugin.json")?;
    let marketplace_json_path = join_repo_path(&base_path, ".claude-plugin/marketplace.json")?;
    let plugin_json =
        read_remote_optional_manifest_file(connection, remote_repo_dir, &plugin_json_path).await;
    let marketplace_json =
        read_remote_optional_manifest_file(connection, remote_repo_dir, &marketplace_json_path)
            .await;

    Ok(plugin_manifest_discovery_from_manifest_bytes(
        &base_path,
        plugin_json.as_deref(),
        marketplace_json.as_deref(),
    ))
}

pub(super) fn skill_md_path_from_source_path(
    source_path: &str,
) -> Result<String, GithubImportError> {
    let normalized = normalize_repo_path(source_path)?;
    if normalized.is_empty() {
        Ok("SKILL.md".to_string())
    } else {
        join_repo_path(&normalized, "SKILL.md")
    }
}

async fn read_remote_optional_manifest_file(
    connection: &ConnectedRemoteTarget,
    remote_repo_dir: &str,
    manifest_path: &str,
) -> Option<Vec<u8>> {
    let remote_path = remote_join(remote_repo_dir, manifest_path);
    let raw = connection.read_file(&remote_path).await.ok()?;
    ResourceBudget::default_skill()
        .reject_file_read_size(&remote_path, raw.len() as u64)
        .ok()?;
    Some(raw)
}

pub(super) fn plugin_manifest_discovery_from_manifest_bytes(
    base_path: &str,
    plugin_json: Option<&[u8]>,
    marketplace_json: Option<&[u8]>,
) -> PluginManifestDiscovery {
    let mut discovery = PluginManifestDiscovery::default();
    let mut seen_explicit_paths = HashSet::new();

    if let Some(raw) = plugin_json {
        collect_plugin_json_manifest_discovery(
            base_path,
            raw,
            &mut discovery,
            &mut seen_explicit_paths,
        );
    }

    if let Some(raw) = marketplace_json {
        collect_marketplace_json_manifest_discovery(
            base_path,
            raw,
            &mut discovery,
            &mut seen_explicit_paths,
        );
    }

    discovery
}

#[derive(Debug, Deserialize)]
struct ClaudePluginManifest {
    name: Option<String>,
    #[serde(default)]
    skills: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeMarketplaceMetadata {
    plugin_root: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMarketplacePlugin {
    name: Option<String>,
    source: Option<serde_json::Value>,
    #[serde(default)]
    skills: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMarketplaceManifest {
    metadata: Option<ClaudeMarketplaceMetadata>,
    #[serde(default)]
    plugins: Vec<ClaudeMarketplacePlugin>,
}

fn collect_plugin_json_manifest_discovery(
    base_path: &str,
    raw: &[u8],
    discovery: &mut PluginManifestDiscovery,
    seen_explicit_paths: &mut HashSet<String>,
) {
    let Ok(manifest) = serde_json::from_slice::<ClaudePluginManifest>(raw) else {
        return;
    };
    let plugin_name = clean_plugin_name(manifest.name.as_deref());

    for skill in manifest.skills {
        let Some(skill_path) = manifest_local_path_from_value(&skill) else {
            continue;
        };
        let Ok(source_path) = manifest_source_path(base_path, &skill_path) else {
            continue;
        };
        add_plugin_manifest_skill_path(
            discovery,
            seen_explicit_paths,
            source_path,
            plugin_name.as_deref(),
        );
    }
}

fn collect_marketplace_json_manifest_discovery(
    base_path: &str,
    raw: &[u8],
    discovery: &mut PluginManifestDiscovery,
    seen_explicit_paths: &mut HashSet<String>,
) {
    let Ok(manifest) = serde_json::from_slice::<ClaudeMarketplaceManifest>(raw) else {
        return;
    };

    let plugin_root = match manifest
        .metadata
        .and_then(|metadata| metadata.plugin_root)
        .as_ref()
        .map(manifest_local_path_from_value)
    {
        Some(Some(path)) => path,
        Some(None) => return,
        None => String::new(),
    };
    let Ok(plugin_root_path) = manifest_source_path(base_path, &plugin_root) else {
        return;
    };

    for plugin in manifest.plugins {
        let Some(source) = plugin
            .source
            .as_ref()
            .and_then(manifest_local_path_from_value)
        else {
            continue;
        };
        let Ok(plugin_dir) = manifest_source_path(&plugin_root_path, &source) else {
            continue;
        };
        let plugin_name = clean_plugin_name(plugin.name.as_deref());

        for skill in plugin.skills {
            let Some(skill_path) = manifest_local_path_from_value(&skill) else {
                continue;
            };
            let Ok(source_path) = manifest_source_path(&plugin_dir, &skill_path) else {
                continue;
            };
            add_plugin_manifest_skill_path(
                discovery,
                seen_explicit_paths,
                source_path,
                plugin_name.as_deref(),
            );
        }
    }
}

fn add_plugin_manifest_skill_path(
    discovery: &mut PluginManifestDiscovery,
    seen_explicit_paths: &mut HashSet<String>,
    source_path: String,
    plugin_name: Option<&str>,
) {
    if seen_explicit_paths.insert(source_path.clone()) {
        discovery.explicit_skill_paths.push(source_path.clone());
    }
    if let Some(plugin_name) = plugin_name {
        discovery
            .plugin_by_source_path
            .entry(source_path)
            .or_insert_with(|| plugin_name.to_string());
    }
}

fn clean_plugin_name(name: Option<&str>) -> Option<String> {
    name.map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
}

fn manifest_local_path_from_value(value: &serde_json::Value) -> Option<String> {
    let raw = value.as_str()?.trim();
    if raw.is_empty()
        || raw.starts_with('/')
        || raw.starts_with('\\')
        || raw.contains('\\')
        || raw.contains(':')
        || raw.contains("://")
    {
        return None;
    }

    join_repo_path("", raw).ok()
}

fn manifest_source_path(base_path: &str, local_path: &str) -> Result<String, GithubImportError> {
    let path = join_repo_path(base_path, local_path)?;
    Ok(if path.is_empty() {
        ".".to_string()
    } else {
        path
    })
}

pub(super) fn effective_source_root(source_path: Option<&str>) -> Result<String, GithubImportError> {
    source_path
        .map(normalize_repo_path)
        .transpose()
        .map(|path| path.unwrap_or_default())
}
