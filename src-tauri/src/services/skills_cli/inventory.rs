//! Lock + filesystem projection for Skills CLI global inventory.
//!
//! Membership is lock v3 names only. Display path prefers the Universal
//! canonical directory; otherwise a mapped detected agent copy. Platform
//! attribution does not require [`super::classify_local_path_origin`] == SkillsCli.

use std::path::{Path, PathBuf};

use super::lock::CliLockOwnership;
use super::{SkillsCliGlobalSkill, SkillsCliInstallKind, SkillsCliSourceTypeBucket};

/// One mapped ∩ detected platform used while attributing copy directories.
#[derive(Debug, Clone)]
pub(crate) struct InventoryPlatform {
    pub display_name: String,
    pub global_skills_dir: PathBuf,
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

fn is_dir(path: &Path) -> bool {
    path.is_dir()
}

pub(crate) fn project_global_inventory(
    ownership: &CliLockOwnership,
    canonical_root: &Path,
    platforms: &[InventoryPlatform],
) -> Vec<SkillsCliGlobalSkill> {
    let mut skills = Vec::new();
    for (name, entry) in ownership.iter() {
        let canonical = canonical_root.join(name);
        let mut agents = Vec::new();
        let mut copy_paths = Vec::new();
        for platform in platforms {
            let copy = platform.global_skills_dir.join(name);
            if is_dir(&copy) {
                agents.push(platform.display_name.clone());
                copy_paths.push(copy);
            }
        }
        let (path, install_kind) = if is_dir(&canonical) {
            (
                Some(canonical.to_string_lossy().into_owned()),
                SkillsCliInstallKind::Canonical,
            )
        } else if let Some(first) = copy_paths.first() {
            (
                Some(first.to_string_lossy().into_owned()),
                SkillsCliInstallKind::Copy,
            )
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
        });
    }
    skills
}
