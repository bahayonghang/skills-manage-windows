//! Lock + filesystem projection for Skills CLI global inventory.
//!
//! Membership is lock v3 names only. Display path prefers the Universal
//! canonical directory; otherwise a mapped detected agent copy. Platform
//! attribution uses five-state placements; `agents` is a compatibility
//! projection from managed_link and direct_copy display names.

use std::path::{Path, PathBuf};

use super::lock::CliLockOwnership;
use super::placement::{classify_placements, compatible_agents, PlacementPlatform};
use super::{SkillsCliGlobalSkill, SkillsCliInstallKind, SkillsCliSourceTypeBucket};

/// One mapped builtin platform used while classifying placements.
#[derive(Debug, Clone)]
pub(crate) struct InventoryPlatform {
    pub agent_id: String,
    pub display_name: String,
    pub global_skills_dir: PathBuf,
    pub is_enabled: bool,
    pub is_detected: bool,
    pub supports_local_placement: bool,
}

impl InventoryPlatform {
    #[cfg(test)]
    pub(crate) fn for_test(agent_id: &str, display_name: &str, dir: PathBuf) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            display_name: display_name.to_string(),
            global_skills_dir: dir,
            is_enabled: true,
            is_detected: true,
            supports_local_placement: cfg!(any(unix, windows)),
        }
    }

    pub(crate) fn as_placement_platform(&self) -> PlacementPlatform {
        PlacementPlatform {
            agent_id: self.agent_id.clone(),
            display_name: self.display_name.clone(),
            global_skills_dir: self.global_skills_dir.clone(),
            is_enabled: self.is_enabled,
            is_detected: self.is_detected,
            supports_local_placement: self.supports_local_placement,
        }
    }
}

pub(crate) fn source_type_bucket(raw: Option<&str>) -> SkillsCliSourceTypeBucket {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        Some("github") => SkillsCliSourceTypeBucket::Github,
        Some("gitlab") => SkillsCliSourceTypeBucket::Gitlab,
        Some("git") => SkillsCliSourceTypeBucket::Git,
        Some("mintlify") => SkillsCliSourceTypeBucket::Mintlify,
        Some("huggingface") => SkillsCliSourceTypeBucket::Huggingface,
        Some("local") => SkillsCliSourceTypeBucket::Local,
        Some("well-known") => SkillsCliSourceTypeBucket::WellKnown,
        _ => SkillsCliSourceTypeBucket::Unknown,
    }
}

fn is_owned_directory(path: &Path) -> bool {
    super::placement::canonical_is_owned_directory(path)
}

pub(crate) fn project_global_inventory(
    ownership: &CliLockOwnership,
    canonical_root: &Path,
    platforms: &[InventoryPlatform],
) -> Vec<SkillsCliGlobalSkill> {
    let placement_platforms: Vec<PlacementPlatform> = platforms
        .iter()
        .map(InventoryPlatform::as_placement_platform)
        .collect();
    let mut skills = Vec::new();
    for (name, entry) in ownership.iter() {
        let canonical = canonical_root.join(name);
        let placements = classify_placements(ownership, name, canonical_root, &placement_platforms);
        let agents = compatible_agents(&placements);
        let first_copy = placements.iter().find(|placement| {
            matches!(
                placement.state,
                super::SkillsCliPlacementState::DirectCopy
                    | super::SkillsCliPlacementState::ManagedLink
            )
        });
        let (path, install_kind) = if is_owned_directory(&canonical) {
            (
                Some(canonical.to_string_lossy().into_owned()),
                SkillsCliInstallKind::Canonical,
            )
        } else if let Some(copy) = first_copy {
            (Some(copy.target_path.clone()), SkillsCliInstallKind::Copy)
        } else {
            (None, SkillsCliInstallKind::Missing)
        };
        skills.push(SkillsCliGlobalSkill {
            name: name.to_string(),
            path,
            install_kind,
            scope: Some("global".to_string()),
            agents,
            source: entry.source.clone(),
            source_url: entry.source_url.clone(),
            source_type: entry.source_type.clone(),
            source_type_bucket: source_type_bucket(entry.source_type.as_deref()),
            canonical_path: entry.skill_path.clone(),
            folder_hash: entry.skill_folder_hash.clone(),
            installed_at: entry.installed_at.clone(),
            updated_at: entry.updated_at.clone(),
            placements,
        });
    }
    skills
}
