//! Portable SkillPort state import/export service layer.
//!
//! Tauri IPC shells live in `crate::commands::portable_state`; this module owns
//! manifest types, JSON validation, preview classification, GitHub-backed import
//! orchestration, and progress helpers.

mod error;
mod export;
mod import;
mod preview;
mod progress;
mod types;

#[cfg(test)]
mod tests;

use sqlx::Row;
use std::collections::HashSet;

use crate::db::DbPool;

pub use error::PortableStateError;
pub(crate) use export::export_skillport_state_impl;
pub(crate) use import::import_skillport_state_for_target;
#[cfg(test)]
use import::{
    build_import_groups, ensure_github_sources, import_skillport_state_impl,
    portable_import_target_kind, restore_skill_tags, PortableImportTargetKind,
};
pub(crate) use preview::{
    build_remote_catalog, parse_manifest, preview_skillport_state_import_impl,
};
pub(crate) use progress::emit_portability_progress;
use types::RepoKey;
pub use types::{
    ExportedFrom, ExportedTarget, PortableCentralSkill, PortableCentralSkillSource,
    PortableGithubSource, PortableSkillTag, PortableUnrestorableSkill, SkillPreviewStatus,
    SkillportStateExportOptions, SkillportStateImportFailure, SkillportStateImportPreview,
    SkillportStateImportPreviewSummary, SkillportStateImportPreviewWarning,
    SkillportStateImportResolution, SkillportStateImportResult, SkillportStateImportedSkill,
    SkillportStateManifest, SkillportStatePortabilityPhase,
    SkillportStatePortabilityProgressPayload, SkillportStatePortabilityStatus,
    SkillportStateSkillPreview, SkillportStateSourcePreview, SourcePreviewStatus,
};
pub(crate) use types::{PortabilityProgressUpdate, PortableStateTargetContext};
#[cfg(test)]
use types::{RemoteCatalogEntry, RemoteCatalogInvalidCandidate, EXPORT_KIND, EXPORT_VERSION};

async fn existing_registry_identities(
    pool: &DbPool,
) -> Result<HashSet<String>, PortableStateError> {
    let rows = sqlx::query("SELECT url FROM skill_registries WHERE source_type = 'github'")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .map(|row| normalize_registry_identity(row.get::<String, _>("url").as_str()))
        .collect())
}

fn repo_key(source: &PortableCentralSkillSource) -> RepoKey {
    RepoKey {
        owner: source.owner.to_ascii_lowercase(),
        repo: source.repo.to_ascii_lowercase(),
        branch: source.branch.to_ascii_lowercase(),
    }
}

fn repo_url_for_source(source: &PortableCentralSkillSource) -> String {
    if !source.url.trim().is_empty() && source.url.contains("/tree/") {
        source.url.clone()
    } else {
        format!(
            "https://github.com/{}/{}/tree/{}",
            source.owner, source.repo, source.branch
        )
    }
}

fn normalize_registry_identity(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/').trim_end_matches(".git");
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let without_www = without_scheme
        .strip_prefix("www.")
        .unwrap_or(without_scheme);
    let lower = without_www.to_ascii_lowercase();
    if let Some(path) = lower.strip_prefix("github.com/") {
        let mut parts = path.split('/');
        if let (Some(owner), Some(repo)) = (parts.next(), parts.next()) {
            return format!("github:{owner}/{repo}");
        }
    }
    lower
}

fn export_source_path(source_path: &str) -> String {
    let normalized = import_source_path(source_path);
    if normalized == "." {
        "SKILL.md".to_string()
    } else if normalized.to_ascii_lowercase().ends_with("/skill.md") {
        normalized
    } else {
        format!("{normalized}/SKILL.md")
    }
}

fn import_source_path(source_path: &str) -> String {
    let normalized = source_path.trim().trim_matches('/').replace('\\', "/");
    if normalized.is_empty() || normalized.eq_ignore_ascii_case("SKILL.md") {
        return ".".to_string();
    }
    let lower = normalized.to_ascii_lowercase();
    if lower.ends_with("/skill.md") {
        normalized[..normalized.len() - "/SKILL.md".len()].to_string()
    } else {
        normalized
    }
}
