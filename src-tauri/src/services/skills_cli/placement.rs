//! Five-state platform placement classifier for Skills CLI inventory.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::services::installation::fs_util::{
    observe_directory_slot, DirectorySlotObservation, ManagedDirectoryLinkKind,
    REASON_WRONG_LINK_TARGET,
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

/// Raw slot observation. Five-state classification stays in Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ObservedSlot {
    Absent,
    ManagedLink {
        kind: SkillsCliManagedLinkKind,
        resolves_to_canonical: bool,
    },
    PlainDirectory,
    Conflict {
        reason_code: String,
    },
}

pub(crate) fn observe_slot_from_fs(slot: &Path, canonical: &Path) -> ObservedSlot {
    match observe_directory_slot(slot, canonical) {
        DirectorySlotObservation::Absent => ObservedSlot::Absent,
        DirectorySlotObservation::Managed { kind } => ObservedSlot::ManagedLink {
            kind: to_ipc_kind(kind),
            resolves_to_canonical: true,
        },
        DirectorySlotObservation::OrdinaryDirectory => ObservedSlot::PlainDirectory,
        DirectorySlotObservation::Conflict { reason_code } => ObservedSlot::Conflict {
            reason_code: reason_code.to_string(),
        },
    }
}

pub(crate) fn classify_placements(
    ownership: &CliLockOwnership,
    skill_name: &str,
    canonical_root: &Path,
    platforms: &[PlacementPlatform],
) -> Vec<SkillsCliPlacement> {
    let canonical = ownership.canonical_dir(canonical_root, skill_name);
    let canonical_owned = canonical_is_owned_directory(&canonical);
    let mut slots = HashMap::new();
    for platform in platforms {
        let slot_path = platform.global_skills_dir.join(skill_name);
        slots.insert(
            platform.agent_id.clone(),
            (
                slot_path.to_string_lossy().into_owned(),
                observe_slot_from_fs(&slot_path, &canonical),
            ),
        );
    }
    classify_placements_observed(skill_name, canonical_owned, platforms, &slots)
}

pub(crate) fn classify_placements_observed(
    skill_name: &str,
    canonical_owned: bool,
    platforms: &[PlacementPlatform],
    slots: &HashMap<String, (String, ObservedSlot)>,
) -> Vec<SkillsCliPlacement> {
    platforms
        .iter()
        .map(|platform| {
            let (target_path, slot) = slots.get(&platform.agent_id).cloned().unwrap_or_else(|| {
                (
                    platform
                        .global_skills_dir
                        .join(skill_name)
                        .to_string_lossy()
                        .into_owned(),
                    ObservedSlot::Absent,
                )
            });
            classify_one_observed(canonical_owned, slot, platform, target_path)
        })
        .collect()
}

pub(crate) fn classify_one(
    skill_name: &str,
    canonical: &Path,
    platform: &PlacementPlatform,
) -> SkillsCliPlacement {
    let slot = platform.global_skills_dir.join(skill_name);
    let observed = observe_slot_from_fs(&slot, canonical);
    classify_one_observed(
        canonical_is_owned_directory(canonical),
        observed,
        platform,
        slot.to_string_lossy().into_owned(),
    )
}

pub(crate) fn classify_one_observed(
    canonical_owned: bool,
    slot: ObservedSlot,
    platform: &PlacementPlatform,
    target_path: String,
) -> SkillsCliPlacement {
    let (state, managed_link_kind, reason_code) = match slot {
        ObservedSlot::ManagedLink {
            kind,
            resolves_to_canonical,
        } => {
            if resolves_to_canonical {
                (SkillsCliPlacementState::ManagedLink, Some(kind), None)
            } else {
                (
                    SkillsCliPlacementState::Conflict,
                    None,
                    Some(REASON_WRONG_LINK_TARGET.to_string()),
                )
            }
        }
        ObservedSlot::PlainDirectory => (SkillsCliPlacementState::DirectCopy, None, None),
        ObservedSlot::Conflict { reason_code } => {
            (SkillsCliPlacementState::Conflict, None, Some(reason_code))
        }
        ObservedSlot::Absent => classify_absent(canonical_owned, platform),
    };
    SkillsCliPlacement {
        agent_id: platform.agent_id.clone(),
        display_name: platform.display_name.clone(),
        target_path,
        state,
        managed_link_kind,
        reason_code,
        install_origin: None,
    }
}

fn classify_absent(
    canonical_owned: bool,
    platform: &PlacementPlatform,
) -> (
    SkillsCliPlacementState,
    Option<SkillsCliManagedLinkKind>,
    Option<String>,
) {
    if !canonical_owned {
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

    fn observed_platform(
        id: &str,
        detected: bool,
        enabled: bool,
        supported: bool,
    ) -> PlacementPlatform {
        PlacementPlatform {
            agent_id: id.to_string(),
            display_name: id.to_string(),
            global_skills_dir: PathBuf::from(format!("/remote/{id}/skills")),
            is_enabled: enabled,
            is_detected: detected,
            supports_local_placement: supported,
        }
    }

    fn observed_equals_classify(
        canonical_owned: bool,
        slot: ObservedSlot,
        platform: &PlacementPlatform,
    ) {
        let via_observed = classify_one_observed(
            canonical_owned,
            slot.clone(),
            platform,
            "/remote/slot/demo".to_string(),
        );
        let via_again = classify_one_observed(
            canonical_owned,
            slot,
            platform,
            "/remote/slot/demo".to_string(),
        );
        assert_eq!(via_observed, via_again);
    }

    #[test]
    fn observed_five_states_and_four_reason_codes() {
        let ready = observed_platform("cursor", true, true, true);
        let managed = classify_one_observed(
            true,
            ObservedSlot::ManagedLink {
                kind: SkillsCliManagedLinkKind::Symlink,
                resolves_to_canonical: true,
            },
            &ready,
            "/remote/cursor/skills/demo".to_string(),
        );
        assert_eq!(managed.state, SkillsCliPlacementState::ManagedLink);
        let copy = classify_one_observed(
            true,
            ObservedSlot::PlainDirectory,
            &ready,
            "/remote/cursor/skills/demo".to_string(),
        );
        assert_eq!(copy.state, SkillsCliPlacementState::DirectCopy);
        let missing =
            classify_one_observed(true, ObservedSlot::Absent, &ready, "/slot".to_string());
        assert_eq!(missing.state, SkillsCliPlacementState::Missing);
        let conflict = classify_one_observed(
            true,
            ObservedSlot::Conflict {
                reason_code: "not_a_directory".to_string(),
            },
            &ready,
            "/slot".to_string(),
        );
        assert_eq!(conflict.state, SkillsCliPlacementState::Conflict);

        let missing_canonical =
            classify_one_observed(false, ObservedSlot::Absent, &ready, "/slot".to_string());
        assert_eq!(
            missing_canonical.reason_code.as_deref(),
            Some(REASON_CANONICAL_MISSING)
        );
        let unsupported = classify_one_observed(
            true,
            ObservedSlot::Absent,
            &observed_platform("x", true, true, false),
            "/slot".to_string(),
        );
        assert_eq!(
            unsupported.reason_code.as_deref(),
            Some(REASON_PLATFORM_UNSUPPORTED)
        );
        let undetected = classify_one_observed(
            true,
            ObservedSlot::Absent,
            &observed_platform("x", false, true, true),
            "/slot".to_string(),
        );
        assert_eq!(
            undetected.reason_code.as_deref(),
            Some(REASON_PLATFORM_NOT_DETECTED)
        );
        let disabled = classify_one_observed(
            true,
            ObservedSlot::Absent,
            &observed_platform("x", true, false, true),
            "/slot".to_string(),
        );
        assert_eq!(
            disabled.reason_code.as_deref(),
            Some(REASON_PLATFORM_DISABLED)
        );
        observed_equals_classify(true, ObservedSlot::PlainDirectory, &ready);
    }

    #[test]
    fn remote_windows_directory_is_not_guessed_as_managed_link() {
        let platform = observed_platform("cursor", true, true, true);
        let placement = classify_one_observed(
            true,
            ObservedSlot::PlainDirectory,
            &platform,
            "/c/Users/me/.cursor/skills/demo".to_string(),
        );
        assert_eq!(placement.state, SkillsCliPlacementState::DirectCopy);
        assert!(placement.managed_link_kind.is_none());
        let linked = classify_one_observed(
            true,
            ObservedSlot::ManagedLink {
                kind: SkillsCliManagedLinkKind::WindowsJunction,
                resolves_to_canonical: true,
            },
            &platform,
            "/c/Users/me/.cursor/skills/demo".to_string(),
        );
        assert_eq!(linked.state, SkillsCliPlacementState::ManagedLink);
        assert_eq!(
            linked.managed_link_kind,
            Some(SkillsCliManagedLinkKind::WindowsJunction)
        );
    }
}
