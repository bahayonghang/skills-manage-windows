//! Discover service layer.
//!
//! Owns local project skill discovery, discovered-skill cache reconciliation,
//! persisted scan-root settings, and import orchestration. Tauri IPC shells live
//! in `crate::commands::discover`.

mod import;
mod query;
mod roots;
mod scan;
mod types;

#[cfg(test)]
mod tests;

pub use import::{
    import_discovered_skill_to_central_at, import_discovered_skill_to_central_impl,
    import_discovered_skill_to_platform_at, import_discovered_skill_to_platform_impl,
    import_discovered_skill_to_platform_with_method_at,
    import_discovered_skill_to_platform_with_method_impl,
    import_source_skill_to_central_at, import_source_skill_to_central_impl,
    import_source_skill_to_platform_with_method_impl,
};
pub use query::{
    clear_discovered_skills_impl, get_discovered_skills_impl, get_discovered_summary_impl,
    get_obsidian_vault_skills_impl, get_obsidian_vaults_impl,
};
pub use roots::{
    default_scan_roots, get_scan_roots_impl, platform_skill_patterns, set_scan_root_enabled_impl,
};
pub use scan::{
    reconcile_discovered_skills, scan_root_for_projects, should_skip_dir, start_project_scan_impl,
    stop_project_scan_impl,
};
pub use types::{
    CompletePayload, DiscoverResult, DiscoveredProject, DiscoveredSkill, DiscoveredSummary,
    FoundPayload, ImportResult, ImportTarget, ObsidianVault, ProgressPayload, ScanRoot,
    OBSIDIAN_PLATFORM_ID, OBSIDIAN_PLATFORM_NAME,
};
