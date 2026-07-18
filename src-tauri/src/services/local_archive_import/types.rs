//! DTOs for the local archive import pipeline.
//!
//! Preview and result DTOs never expose absolute user-directory paths. They
//! carry only the archive display name, post-strip relative paths, the
//! archive fingerprint, and the conflict summary.

use serde::{Deserialize, Serialize};

use crate::services::local_archive_import::inventory::ArchiveFingerprint;

/// Conflict resolution strategy. Mirrors the GitHub import semantics so the
/// frontend wizard can reuse the same interaction vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalArchiveImportResolution {
    Overwrite,
    Skip,
    Rename,
}

/// A regular file in the preview tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalArchivePreviewFile {
    pub path: String,
    pub byte_len: u64,
}

/// A discovered skill candidate inside the archive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalArchivePreviewSkill {
    /// Archive-relative root directory of the skill (empty string when the
    /// skill lives at the archive root). Does not expose absolute paths.
    pub root_directory: String,
    /// The resolved skill id (sanitized, lowercase, dash-separated).
    pub skill_id: String,
    pub skill_name: String,
    pub description: Option<String>,
    /// The relative path to `SKILL.md` after wrapper stripping.
    pub skill_md_path: String,
    /// The full file tree under the skill root (relative paths).
    pub files: Vec<LocalArchivePreviewFile>,
    pub file_count: usize,
    pub total_expanded_bytes: u64,
    /// Central conflict for this candidate, if any.
    pub conflict: Option<LocalSkillConflict>,
}

/// Conflict info mirroring `GitHubSkillConflict` so the frontend can reuse
/// the existing conflict UI patterns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSkillConflict {
    pub existing_skill_id: String,
    pub existing_name: String,
    pub existing_canonical_path: Option<String>,
    pub proposed_skill_id: String,
    pub proposed_name: String,
}

/// Read-only preview DTO returned by `preview_local_skill_archive`.
///
/// Carries the archive fingerprint so the import step can prove the archive
/// on disk is byte-identical to the one the user confirmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalArchivePreview {
    /// Display name of the archive file (basename only, no absolute path).
    pub archive_display_name: String,
    pub fingerprint: ArchiveFingerprint,
    pub skills: Vec<LocalArchivePreviewSkill>,
    pub total_files: usize,
    pub total_expanded_bytes: u64,
    pub total_compressed_bytes: u64,
    pub archive_byte_len: u64,
}

/// Result of a successful import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalArchiveImportResult {
    /// The final skill id written to Central. May differ from the preview
    /// `skill_id` only when the user chose `rename`.
    pub imported_skill_id: String,
    pub skill_name: String,
    /// Archive-relative root directory of the imported skill.
    pub root_directory: String,
    /// The resolution that was applied.
    pub resolution: LocalArchiveImportResolution,
    pub file_count: usize,
    pub total_expanded_bytes: u64,
    /// Whether an existing Central skill was backed up and replaced.
    pub replaced_existing: bool,
}
