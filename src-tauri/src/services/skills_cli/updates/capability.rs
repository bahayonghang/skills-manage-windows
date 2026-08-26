//! Fail-closed Skills CLI update capability plan from the isolated probe ledger.

use serde::{Deserialize, Serialize};

/// Probe ledger status. Unsupported and unverified both fail closed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySupport {
    VerifiedSupported,
    VerifiedUnsupported,
    Unverified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillsCliUpdateCapabilityPlan {
    pub npm_spec: String,
    pub force_flag: CapabilitySupport,
    pub keep_links_flag: CapabilitySupport,
    pub pinned_full_sha_source: CapabilitySupport,
    pub direct_copy_refresh: CapabilitySupport,
    pub apply_method: String,
}

pub fn update_capability_plan() -> SkillsCliUpdateCapabilityPlan {
    SkillsCliUpdateCapabilityPlan {
        npm_spec: super::super::SKILLS_CLI_NPM_SPEC.to_string(),
        force_flag: CapabilitySupport::VerifiedUnsupported,
        keep_links_flag: CapabilitySupport::VerifiedUnsupported,
        pinned_full_sha_source: CapabilitySupport::Unverified,
        direct_copy_refresh: CapabilitySupport::Unverified,
        apply_method: "pinned_snapshot_canonical_refresh".to_string(),
    }
}

/// Preview tokens shown in the update drawer. Never includes `--force`,
/// `--keep-links`, or an unverified `@<full-sha>` source.
pub fn apply_argv_preview(skill_names: &[String]) -> Vec<String> {
    let mut preview = vec![
        "refresh".to_string(),
        "owned-canonical".to_string(),
        "from-pinned-github-snapshot".to_string(),
    ];
    for name in skill_names {
        preview.push(name.clone());
    }
    debug_assert!(
        !preview.iter().any(|item| item == "--force" || item == "--keep-links"),
        "capability plan must never advertise unsupported flags"
    );
    preview
}

pub fn argv_contains_forbidden_flags(argv: &[String]) -> bool {
    argv.iter().any(|item| {
        item == "--force"
            || item == "--keep-links"
            || item.contains("@") && item.chars().filter(|ch| ch.is_ascii_hexdigit()).count() >= 40
    })
}
