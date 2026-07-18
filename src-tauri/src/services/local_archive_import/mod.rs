//! Local skill archive (ZIP) import service.
//!
//! Deep module owning the safe import pipeline for local `.zip` skill
//! archives:
//! - [`inventory`] reads archive bytes once, enforces the safety matrix
//!   (absolute, traversal, Windows drive/UNC, symlink, encrypted,
//!   unsupported method, case/prefix collision), applies the archive budget
//!   and zip-bomb ratio guard, and produces a deterministic inventory.
//! - [`candidate`] strips the wrapper directory and resolves the single
//!   skill manifest (`SKILL.md`), failing closed on ambiguity.
//! - [`preview`] builds the read-only preview DTO (including the archive
//!   fingerprint and Central conflict) without touching the filesystem,
//!   database, or Operation Log.
//! - [`import`] re-verifies the fingerprint, stages the archive under a
//!   unique work directory, acquires the Central mutation guard, backs up
//!   the existing skill on overwrite, atomically swaps the target
//!   directory, updates the database, and records a redacted Operation Log.
//!
//! Invariants enforced across the module:
//! - No Central, staging, database, or Operation Log mutation occurs
//!   before fingerprint verification succeeds.
//! - Any failure after staging starts must restore the backup and clean
//!   up staging directories; no partial skill is left behind.
//! - Archive sources are never assigned a GitHub repository; the update
//!   center treats them as unsupported/unknown rather than remote-missing.
//! - The frontend never receives absolute user-directory paths; preview and
//!   result DTOs expose only archive display name, relative paths, and the
//!   fingerprint.

pub mod candidate;
pub mod error;
pub mod import;
pub mod inventory;
pub mod preview;
#[cfg(test)]
mod tests;
pub mod types;

pub use error::LocalArchiveImportError;
#[allow(unused_imports)]
pub(crate) use import::{import_local_skill_archive_impl, ImportOutcome};
pub use inventory::{ArchiveFingerprint, ZipInventory, ZipInventoryEntry};
pub(crate) use preview::preview_local_skill_archive_impl;
pub use types::{
    LocalArchiveImportResolution, LocalArchiveImportResult, LocalArchivePreview,
    LocalArchivePreviewFile, LocalArchivePreviewSkill, LocalSkillConflict,
};
