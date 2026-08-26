//! Five-state platform placement classifier for Skills CLI inventory.

use std::path::{Path, PathBuf};

use crate::services::installation::fs_util::{
    observe_directory_slot, DirectorySlotObservation, ManagedDirectoryLinkKind,
};

use super::lock::CliLockOwnership;
use super::{SkillsCliManagedLinkKind, SkillsCliPlacement, SkillsCliPlacementState};

pub(crate) const REASON_CANONICAL_MISSING: &str = "canonical_missing";
pub(crate) const REASON_PLATFORM_DISABLED: &str = "platform_disabled";
pub(crate) const REASON_PLATFORM_NOT_DETECTED: &str = "platform_not_detected";
pub(crate) const REASON_PLATFORM_UNSUPPORTED: &str = "platform_unsupported";

#[derive(Debug, Clone)]
pub(crate) struct PlacementPlatform {
    pub agent_id: String,
    pub display_name: String,
    pub global_skills_dir: PathBuf,
    pub is_enabled: bool,
    pub is_detected: bool,
    pub supports_local_placement: bool,
}

pub(crate) fn classify_placements(
    ownership: &CliLockOwnership,
    skill_name: &str,
    canonical_root: &Path,
    platforms: &[PlacementPlatform],
) -> Vec<SkillsCliPlacement> {
    let canonical = ownership.canonical_dir(canonical_root, skill_name);
    platforms
        .iter()
        .map(|platform| classify_one(skill_name, &canonical, platform))
        .collect()
}

pub(crate) fn classify_one(
    skill_name: &str,
    canonical: &Path,
    platform: &PlacementPlatform,
) -> SkillsCliPlacement {
    let slot = platform.global_skills_dir.join(skill_name);
    let observation = observe_directory_slot(&slot, canonical);
    let (state, managed_link_kind, reason_code) = match observation {
        DirectorySlotObservation::Managed { kind } => (
            SkillsCliPlacementState::ManagedLink,
            Some(to_ipc_kind(kind)),
            None,
        ),
        DirectorySlotObservation::OrdinaryDirectory => {
            (SkillsCliPlacementState::DirectCopy, None, None)
        }
        DirectorySlotObservation::Conflict { reason_code } => (
            SkillsCliPlacementState::Conflict,
            None,
            Some(reason_code.to_string()),
        ),
        DirectorySlotObservation::Absent => classify_absent(canonical, platform),
    };
    SkillsCliPlacement {
        agent_id: platform.agent_id.clone(),
        display_name: platform.display_name.clone(),
        target_path: slot.to_string_lossy().into_owned(),
        state,
        managed_link_kind,
        reason_code,
    }
}

fn classify_absent(
    canonical: &Path,
    platform: &PlacementPlatform,
) -> (
    SkillsCliPlacementState,
    Option<SkillsCliManagedLinkKind>,
    Option<String>,
) {
    if !canonical_is_owned_directory(canonical) {
        return (
            SkillsCliPlacementState::Unavailable,
            None,
            Some(REASON_CANONICAL_MISSING.to_string()),
        );
    }
    if !platform.supports_local_placement {
        return (
            SkillsCliPlacementState::Unavailable,
            None,
            Some(REASON_PLATFORM_UNSUPPORTED.to_string()),
        );
    }
    if !platform.is_detected {
        return (
            SkillsCliPlacementState::Unavailable,
            None,
            Some(REASON_PLATFORM_NOT_DETECTED.to_string()),
        );
    }
    if !platform.is_enabled {
        return (
            SkillsCliPlacementState::Unavailable,
            None,
            Some(REASON_PLATFORM_DISABLED.to_string()),
        );
    }
    (SkillsCliPlacementState::Missing, None, None)
}

pub(crate) fn canonical_is_owned_directory(canonical: &Path) -> bool {
    std::fs::symlink_metadata(canonical).is_ok_and(|metadata| {
        metadata.is_dir()
            && !crate::services::installation::fs_util::is_reparse_or_symlink(&metadata)
    })
}

fn to_ipc_kind(kind: ManagedDirectoryLinkKind) -> SkillsCliManagedLinkKind {
    match kind {
        ManagedDirectoryLinkKind::WindowsJunction => SkillsCliManagedLinkKind::WindowsJunction,
        ManagedDirectoryLinkKind::Symlink => SkillsCliManagedLinkKind::Symlink,
    }
}

pub(crate) fn compatible_agents(placements: &[SkillsCliPlacement]) -> Vec<String> {
    placements
        .iter()
        .filter(|placement| {
            matches!(
                placement.state,
                SkillsCliPlacementState::ManagedLink | SkillsCliPlacementState::DirectCopy
            )
        })
        .map(|placement| placement.display_name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::installation::fs_util::create_skills_cli_directory_link;
    use crate::services::skills_cli::lock::parse_lock_content;
    use tempfile::TempDir;

    fn platform(id: &str, name: &str, dir: PathBuf) -> PlacementPlatform {
        PlacementPlatform {
            agent_id: id.to_string(),
            display_name: name.to_string(),
            global_skills_dir: dir,
            is_enabled: true,
            is_detected: true,
            supports_local_placement: true,
        }
    }

    #[test]
    fn stable_order_and_compatible_agents_ignore_missing_conflict() {
        let ownership = parse_lock_content(r#"{"version":3,"skills":{"demo":{}}}"#);
        let temp = TempDir::new().unwrap();
        let canonical_root = temp.path().join("universal");
        let canonical = canonical_root.join("demo");
        std::fs::create_dir_all(&canonical).unwrap();
        let cursor_dir = temp.path().join("cursor");
        let amp_dir = temp.path().join("amp");
        std::fs::create_dir_all(cursor_dir.join("demo")).unwrap();
        std::fs::create_dir_all(&amp_dir).unwrap();
        let platforms = [
            platform("cursor", "Cursor", cursor_dir),
            platform("amp", "Amp", amp_dir),
        ];
        let placements = classify_placements(&ownership, "demo", &canonical_root, &platforms);
        assert_eq!(placements[0].agent_id, "cursor");
        assert_eq!(placements[0].state, SkillsCliPlacementState::DirectCopy);
        assert_eq!(placements[1].agent_id, "amp");
        assert_eq!(placements[1].state, SkillsCliPlacementState::Missing);
        assert_eq!(compatible_agents(&placements), vec!["Cursor"]);
    }

    #[test]
    fn canonical_missing_absent_slot_is_unavailable() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("missing-canonical");
        let amp_dir = temp.path().join("amp");
        std::fs::create_dir_all(&amp_dir).unwrap();
        let placement = classify_one("demo", &canonical, &platform("amp", "Amp", amp_dir));
        assert_eq!(placement.state, SkillsCliPlacementState::Unavailable);
        assert_eq!(
            placement.reason_code.as_deref(),
            Some(REASON_CANONICAL_MISSING)
        );
    }

    #[test]
    fn disabled_and_undetected_reason_codes() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("canonical");
        std::fs::create_dir_all(&canonical).unwrap();
        let disabled_dir = temp.path().join("disabled");
        std::fs::create_dir_all(&disabled_dir).unwrap();
        let mut disabled = platform("amp", "Amp", disabled_dir);
        disabled.is_enabled = false;
        let placement = classify_one("demo", &canonical, &disabled);
        assert_eq!(placement.state, SkillsCliPlacementState::Unavailable);
        assert_eq!(
            placement.reason_code.as_deref(),
            Some(REASON_PLATFORM_DISABLED)
        );

        let missing_dir = temp.path().join("undetected");
        let mut undetected = platform("zed", "Zed", missing_dir);
        undetected.is_detected = false;
        let placement = classify_one("demo", &canonical, &undetected);
        assert_eq!(
            placement.reason_code.as_deref(),
            Some(REASON_PLATFORM_NOT_DETECTED)
        );
    }

    #[test]
    fn file_slot_is_conflict() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("canonical");
        std::fs::create_dir_all(&canonical).unwrap();
        let agent_dir = temp.path().join("agent");
        std::fs::create_dir_all(&agent_dir).unwrap();
        std::fs::write(agent_dir.join("demo"), b"file").unwrap();
        let placement = classify_one("demo", &canonical, &platform("cursor", "Cursor", agent_dir));
        assert_eq!(placement.state, SkillsCliPlacementState::Conflict);
        assert_eq!(placement.reason_code.as_deref(), Some("not_a_directory"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_junction_is_managed_link() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("canonical");
        let agent_dir = temp.path().join("agent");
        std::fs::create_dir_all(&canonical).unwrap();
        std::fs::create_dir_all(&agent_dir).unwrap();
        create_skills_cli_directory_link(&canonical, &agent_dir.join("demo")).unwrap();
        let placement = classify_one("demo", &canonical, &platform("cursor", "Cursor", agent_dir));
        assert_eq!(placement.state, SkillsCliPlacementState::ManagedLink);
        assert_eq!(
            placement.managed_link_kind,
            Some(SkillsCliManagedLinkKind::WindowsJunction)
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_symlink_is_managed_link() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("canonical");
        let agent_dir = temp.path().join("agent");
        std::fs::create_dir_all(&canonical).unwrap();
        std::fs::create_dir_all(&agent_dir).unwrap();
        create_skills_cli_directory_link(&canonical, &agent_dir.join("demo")).unwrap();
        let placement = classify_one("demo", &canonical, &platform("cursor", "Cursor", agent_dir));
        assert_eq!(placement.state, SkillsCliPlacementState::ManagedLink);
        assert_eq!(
            placement.managed_link_kind,
            Some(SkillsCliManagedLinkKind::Symlink)
        );
    }
}
