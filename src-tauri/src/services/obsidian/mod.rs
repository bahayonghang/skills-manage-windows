//! Obsidian vault discovery + source-import. Salvaged from the old discovery
//! module so the Obsidian view keeps working after the legacy Discover purge.
//!
//! Vault detection scans known Obsidian registry locations plus a few common
//! roots; imports go through the same FS primitives as the rest of the app.

mod import;
mod query;
mod types;

pub use import::{import_obsidian_skill_to_central_impl, import_obsidian_skill_to_platform_impl};
pub use query::{get_obsidian_vault_skills_impl, get_obsidian_vaults_impl};
pub use types::{
    ObsidianImportResult, ObsidianSkill, ObsidianVault, OBSIDIAN_PLATFORM_ID,
    OBSIDIAN_PLATFORM_NAME,
};
