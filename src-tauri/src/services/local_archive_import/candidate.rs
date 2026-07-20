//! Skill candidate discovery for a local ZIP archive.
//!
//! Given a validated [`ZipInventory`], determine whether the archive contains
//! a single importable skill and, if so, produce a [`ResolvedSkillCandidate`]
//! with the archive-relative skill root, the `SKILL.md` relative path, and the
//! sanitized skill id.
//!
//! Rules (fail closed on ambiguity):
//! 1. If the archive contains a root `SKILL.md` (path `SKILL.md`), the skill
//!    root is the archive root. All safe files belong to the skill.
//! 2. Otherwise, collect all `*/SKILL.md` (single-segment wrapper). Only
//!    when every candidate shares the same single top-level wrapper directory
//!    and there is no second candidate at a different depth do we strip that
//!    wrapper. Otherwise return `ambiguous_archive_layout`.
//! 3. If no `SKILL.md` exists, return `no_skill_manifest`.
//! 4. After stripping, re-check path duplication, prefix conflicts, and budget
//!    against the post-strip relative paths.

use std::collections::HashSet;

use crate::services::local_archive_import::error::LocalArchiveImportError;
use crate::services::local_archive_import::inventory::{ZipInventory, ZipInventoryEntry};
use crate::services::resource_budget::ResourceBudget;

/// A resolved skill candidate inside the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSkillCandidate {
    /// Archive-relative root directory of the skill. Empty string means the
    /// archive root itself is the skill root.
    pub root_directory: String,
    /// Relative path to `SKILL.md` from the skill root (always `SKILL.md`
    /// once the wrapper is stripped).
    pub skill_md_path: String,
    /// Sanitized, lowercase, dash-separated skill id derived from the
    /// frontmatter `name` field.
    pub skill_id: String,
    /// Skill display name from frontmatter (original case preserved).
    pub skill_name: String,
    /// Optional description from frontmatter.
    pub description: Option<String>,
    /// Full file list relative to the skill root (post-strip).
    pub files: Vec<CandidateFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateFile {
    pub path: String,
    pub byte_len: u64,
}

/// Resolve the single skill candidate from an inventory. The caller is
/// responsible for passing the raw archive bytes so the candidate module can
/// read `SKILL.md` frontmatter from the same snapshot used for fingerprinting.
pub(crate) fn resolve_candidate(
    inventory: &ZipInventory,
    archive_bytes: &[u8],
    budget: ResourceBudget,
) -> Result<ResolvedSkillCandidate, LocalArchiveImportError> {
    let skill_md_entries: Vec<&ZipInventoryEntry> =
        inventory.entries.iter().filter(|e| e.is_skill_md).collect();

    if skill_md_entries.is_empty() {
        return Err(LocalArchiveImportError::NoSkillManifest(
            "no SKILL.md found in archive".to_string(),
        ));
    }

    // Determine the wrapper prefix to strip and the post-strip SKILL.md path.
    let (root_directory, skill_md_relative): (String, String) = {
        // Root SKILL.md (exactly one entry whose post-normalize path is "SKILL.md").
        if let Some(entry) = skill_md_entries.iter().find(|e| e.path == "SKILL.md") {
            let _ = entry;
            (String::new(), "SKILL.md".to_string())
        } else {
            // Collect distinct top-level wrapper segments from all SKILL.md paths.
            let mut wrappers: Vec<String> = Vec::new();
            for entry in &skill_md_entries {
                let segments: Vec<&str> = entry.path.split('/').collect();
                if segments.len() < 2
                    || !segments
                        .last()
                        .map(|s| s.eq_ignore_ascii_case("SKILL.md"))
                        .unwrap_or(false)
                {
                    return Err(LocalArchiveImportError::AmbiguousArchiveLayout(format!(
                        "nested SKILL.md path not supported for MVP: {}",
                        entry.path
                    )));
                }
                wrappers.push(segments[0].to_string());
            }
            let distinct_wrappers: Vec<&str> = wrappers
                .iter()
                .map(|w| w.as_str())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            if distinct_wrappers.len() != 1 {
                return Err(LocalArchiveImportError::AmbiguousArchiveLayout(format!(
                    "multiple distinct top-level wrapper directories with SKILL.md: {}",
                    distinct_wrappers.join(", ")
                )));
            }
            let wrapper = distinct_wrappers[0].to_string();
            (wrapper.clone(), "SKILL.md".to_string())
        }
    };

    // Re-check post-strip paths for collisions and budget, and collect the
    // file tree relative to the skill root.
    let strip_prefix = if root_directory.is_empty() {
        String::new()
    } else {
        format!("{}/", root_directory)
    };

    let mut files: Vec<CandidateFile> = Vec::with_capacity(inventory.entries.len());
    let mut seen: HashSet<String> = HashSet::new();
    let mut seen_lower: HashSet<String> = HashSet::new();
    let mut total_bytes: u64 = 0;
    let mut total_files: usize = 0;

    for entry in &inventory.entries {
        let stripped = strip_entry_path(&entry.path, &strip_prefix, &root_directory)?;
        if stripped.is_none() {
            continue;
        }
        let stripped = stripped.unwrap();

        if stripped.is_empty() {
            continue;
        }

        if !seen.insert(stripped.clone()) {
            return Err(LocalArchiveImportError::PathConflict(format!(
                "duplicate post-strip path: {stripped}"
            )));
        }
        let lower = stripped.to_ascii_lowercase();
        if !seen_lower.insert(lower) {
            return Err(LocalArchiveImportError::PathConflict(format!(
                "case-colliding post-strip path: {stripped}"
            )));
        }

        budget
            .reject_file_read_size(&stripped, entry.byte_len)
            .map_err(LocalArchiveImportError::BudgetExceeded)?;
        total_bytes = total_bytes.saturating_add(entry.byte_len);
        total_files += 1;
        if total_files > budget.archive_files {
            return Err(LocalArchiveImportError::BudgetExceeded(
                crate::services::resource_budget::BudgetExceeded::new(
                    "ZIP skill file count",
                    total_files as u64,
                    budget.archive_files as u64,
                ),
            ));
        }
        if total_bytes > budget.archive_expanded_bytes {
            return Err(LocalArchiveImportError::BudgetExceeded(
                crate::services::resource_budget::BudgetExceeded::new(
                    "ZIP skill expanded size",
                    total_bytes,
                    budget.archive_expanded_bytes,
                ),
            ));
        }

        files.push(CandidateFile {
            path: stripped,
            byte_len: entry.byte_len,
        });
    }

    // Parse frontmatter from the resolved SKILL.md entry.
    let skill_md_entry = skill_md_entries
        .iter()
        .find(|e| {
            (root_directory.is_empty() && e.path.eq_ignore_ascii_case("SKILL.md"))
                || (!root_directory.is_empty()
                    && e.path
                        .eq_ignore_ascii_case(&format!("{}/SKILL.md", root_directory)))
        })
        .ok_or_else(|| {
            LocalArchiveImportError::Internal(format!(
                "resolved SKILL.md entry not found in inventory (root='{root_directory}')"
            ))
        })?;

    let content = read_entry_content(archive_bytes, skill_md_entry.path.as_str())?;
    let frontmatter = parse_skill_frontmatter(&content).ok_or_else(|| {
        LocalArchiveImportError::SkillFrontmatterMissing(skill_md_relative.clone())
    })?;

    let skill_id = sanitize_skill_id(&frontmatter.name)?;

    Ok(ResolvedSkillCandidate {
        root_directory,
        skill_md_path: skill_md_relative,
        skill_id,
        skill_name: frontmatter.name,
        description: frontmatter.description,
        files,
    })
}

/// Strip the wrapper prefix from an archive entry path. Returns:
/// - `Ok(Some(path))` when the entry belongs to the skill root.
/// - `Ok(None)` when the entry is the wrapper dir itself (skip).
/// - `Err(AmbiguousArchiveLayout)` when the entry is outside the skill root.
fn strip_entry_path(
    entry_path: &str,
    strip_prefix: &str,
    root_directory: &str,
) -> Result<Option<String>, LocalArchiveImportError> {
    if let Some(suffix) = entry_path.strip_prefix(strip_prefix) {
        return Ok(Some(suffix.to_string()));
    }
    if entry_path == root_directory && !root_directory.is_empty() {
        return Ok(None);
    }
    if root_directory.is_empty() {
        return Ok(Some(entry_path.to_string()));
    }
    Err(LocalArchiveImportError::AmbiguousArchiveLayout(format!(
        "file outside skill root '{}': {}",
        root_directory, entry_path
    )))
}

#[derive(Debug, Clone, serde::Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: Option<String>,
}

fn parse_skill_frontmatter(content: &str) -> Option<SkillFrontmatter> {
    let block = crate::services::scanner::extract_frontmatter_block(content)?;
    serde_norway::from_str::<SkillFrontmatter>(block).ok()
}

/// Extract a single regular-file entry's bytes from the archive snapshot.
fn read_entry_content(
    archive_bytes: &[u8],
    entry_path: &str,
) -> Result<String, LocalArchiveImportError> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(archive_bytes.to_vec());
    let mut zip = zip::ZipArchive::new(cursor)
        .map_err(|e| LocalArchiveImportError::ArchiveReadFailed(format!("reopen zip: {e}")))?;
    let mut found_index: Option<usize> = None;
    for index in 0..zip.len() {
        let entry = zip.by_index_raw(index).map_err(|e| {
            LocalArchiveImportError::ArchiveReadFailed(format!("entry {index}: {e}"))
        })?;
        if entry.name() == entry_path && !entry.is_dir() {
            found_index = Some(index);
            break;
        }
    }
    let index = found_index.ok_or_else(|| {
        LocalArchiveImportError::Internal(format!("entry '{entry_path}' not found in archive"))
    })?;
    let mut entry = zip
        .by_index(index)
        .map_err(|e| LocalArchiveImportError::ArchiveReadFailed(format!("open entry: {e}")))?;
    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf).map_err(|e| {
        LocalArchiveImportError::ArchiveReadFailed(format!("read entry '{entry_path}': {e}"))
    })?;
    String::from_utf8(buf).map_err(|e| LocalArchiveImportError::UnsupportedArchiveEntry {
        path: entry_path.to_string(),
        reason: format!("SKILL.md is not valid UTF-8: {e}"),
    })
}

/// Sanitize a raw skill name into the canonical lowercase dash-separated id
/// used across Central. Reuses the same rule as the GitHub import pipeline so
/// archive skills and GitHub skills can coexist under a single id namespace.
fn sanitize_skill_id(raw: &str) -> Result<String, LocalArchiveImportError> {
    let lowered = raw.trim().to_lowercase();
    let mut sanitized = String::new();
    let mut last_was_dash = false;
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            sanitized.push('-');
            last_was_dash = true;
        }
    }
    let sanitized = sanitized.trim_matches('-').to_string();
    if sanitized.is_empty() {
        return Err(LocalArchiveImportError::InvalidSkillIdentifier(
            raw.to_string(),
        ));
    }
    Ok(sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    fn make_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut buf);
        for (name, content) in files {
            let opts =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            writer.start_file(*name, opts).unwrap();
            writer.write_all(content).unwrap();
        }
        writer.finish().unwrap();
        buf.into_inner()
    }

    fn build_inventory(bytes: &[u8]) -> ZipInventory {
        crate::services::local_archive_import::inventory::build_inventory(
            bytes,
            ResourceBudget::default_skill(),
        )
        .expect("inventory")
    }

    #[test]
    fn resolves_root_skill() {
        let bytes = make_zip(&[("SKILL.md", b"---\nname: Hello World\n---\nbody")]);
        let inv = build_inventory(&bytes);
        let cand = resolve_candidate(&inv, &bytes, ResourceBudget::default_skill()).unwrap();
        assert_eq!(cand.root_directory, "");
        assert_eq!(cand.skill_md_path, "SKILL.md");
        assert_eq!(cand.skill_id, "hello-world");
        assert_eq!(cand.skill_name, "Hello World");
        assert_eq!(cand.files.len(), 1);
        assert_eq!(cand.files[0].path, "SKILL.md");
    }

    #[test]
    fn resolves_wrapper_dir_skill() {
        let bytes = make_zip(&[
            ("my-skill/SKILL.md", b"---\nname: My\n---\nbody"),
            ("my-skill/assets/a.txt", b"asset"),
        ]);
        let inv = build_inventory(&bytes);
        let cand = resolve_candidate(&inv, &bytes, ResourceBudget::default_skill()).unwrap();
        assert_eq!(cand.root_directory, "my-skill");
        assert_eq!(cand.skill_md_path, "SKILL.md");
        assert_eq!(cand.skill_id, "my");
        assert_eq!(cand.files.len(), 2);
        assert_eq!(cand.files[0].path, "SKILL.md");
        assert_eq!(cand.files[1].path, "assets/a.txt");
    }

    #[test]
    fn rejects_multiple_wrappers_as_ambiguous() {
        let bytes = make_zip(&[
            ("a/SKILL.md", b"---\nname: a\n---\n"),
            ("b/SKILL.md", b"---\nname: b\n---\n"),
        ]);
        let inv = build_inventory(&bytes);
        let err = resolve_candidate(&inv, &bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "ambiguous_archive_layout");
    }

    #[test]
    fn rejects_no_skill_manifest() {
        let bytes = make_zip(&[("README.md", b"no skill")]);
        let inv = build_inventory(&bytes);
        let err = resolve_candidate(&inv, &bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "no_skill_manifest");
    }

    #[test]
    fn rejects_missing_frontmatter() {
        let bytes = make_zip(&[("SKILL.md", b"# no frontmatter here\nbody")]);
        let inv = build_inventory(&bytes);
        let err = resolve_candidate(&inv, &bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "skill_frontmatter_missing");
    }

    #[test]
    fn sanitizes_skill_id() {
        assert_eq!(sanitize_skill_id("Hello World").unwrap(), "hello-world");
        assert_eq!(sanitize_skill_id("a__b!!c").unwrap(), "a-b-c");
        assert!(sanitize_skill_id("   ").is_err());
        assert!(sanitize_skill_id("!!!").is_err());
    }
}
