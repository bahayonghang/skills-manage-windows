//! Preview pipeline: builds the read-only preview DTO.
//!
//! Reads the archive bytes once, builds the inventory, resolves the skill
//! candidate, looks up Central conflicts, and produces a [`LocalArchivePreview`].
//! Preview never touches the filesystem beyond reading the archive, never
//! writes to the database, and never records an Operation Log. It only
//! attaches the archive fingerprint so the import step can verify the
//! archive on disk is byte-identical to the one the user confirmed.

use std::path::Path;

use crate::db::{self, DbPool};
use crate::services::local_archive_import::candidate::resolve_candidate;
use crate::services::local_archive_import::error::{task_join, LocalArchiveImportError};
use crate::services::local_archive_import::inventory::{build_inventory, read_archive_bytes};
use crate::services::local_archive_import::types::{
    LocalArchivePreview, LocalArchivePreviewFile, LocalArchivePreviewSkill, LocalSkillConflict,
};
use crate::services::resource_budget::ResourceBudget;

/// Build the preview DTO for a local `.zip` archive.
///
/// `archive_path` is an absolute user-directory path on the local machine.
/// The function never returns the absolute path; the DTO exposes only the
/// archive basename.
pub(crate) async fn preview_local_skill_archive_impl(
    pool: &DbPool,
    archive_path: &str,
) -> Result<LocalArchivePreview, LocalArchiveImportError> {
    let budget = ResourceBudget::default_skill();
    let archive_path_owned = archive_path.to_string();
    let archive_bytes = crate::fs_util::run_blocking_fs_with(
        "read local skill archive for preview",
        move || read_archive_bytes(&archive_path_owned, budget),
        task_join,
    )
    .await?;
    let display_name = archive_basename(archive_path);
    build_preview_from_bytes(pool, &archive_bytes, display_name).await
}

pub(crate) async fn build_preview_from_bytes(
    pool: &DbPool,
    archive_bytes: &[u8],
    archive_display_name: String,
) -> Result<LocalArchivePreview, LocalArchiveImportError> {
    let budget = ResourceBudget::default_skill();
    let inventory = build_inventory(archive_bytes, budget)?;
    let candidate = resolve_candidate(&inventory, archive_bytes, budget)?;

    // Build the file tree DTO relative to the skill root.
    let mut files: Vec<LocalArchivePreviewFile> = candidate
        .files
        .iter()
        .map(|f| LocalArchivePreviewFile {
            path: f.path.clone(),
            byte_len: f.byte_len,
        })
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let file_count = candidate.files.len();
    let total_expanded_bytes: u64 = candidate.files.iter().map(|f| f.byte_len).sum();
    let total_compressed_bytes: u64 = inventory.entries.iter().map(|e| e.compressed_len).sum();

    // Central conflict lookup.
    let conflict = lookup_central_conflict(pool, &candidate.skill_id).await?;

    let skill = LocalArchivePreviewSkill {
        root_directory: candidate.root_directory.clone(),
        skill_id: candidate.skill_id.clone(),
        skill_name: candidate.skill_name.clone(),
        description: candidate.description.clone(),
        skill_md_path: candidate.skill_md_path.clone(),
        files,
        file_count,
        total_expanded_bytes,
        conflict,
    };

    Ok(LocalArchivePreview {
        archive_display_name,
        fingerprint: inventory.fingerprint.clone(),
        skills: vec![skill],
        total_files: file_count,
        total_expanded_bytes,
        total_compressed_bytes,
        archive_byte_len: inventory.archive_bytes,
    })
}

/// Extract the archive basename (file name only) from a user-provided path.
/// Never returns the absolute path; used as the preview display name so the
/// user can recognise the archive they selected without leaking their home
/// directory into IPC.
pub(crate) fn archive_basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "archive.zip".to_string())
}

/// Look up a Central skill with the same id and build a conflict DTO. Used
/// only by the preview path; the import path performs its own conflict
/// resolution using the user-selected strategy.
async fn lookup_central_conflict(
    pool: &DbPool,
    skill_id: &str,
) -> Result<Option<LocalSkillConflict>, LocalArchiveImportError> {
    let existing = db::get_skill_by_id(pool, skill_id).await?;
    Ok(existing.and_then(|skill| {
        if skill.is_central {
            Some(LocalSkillConflict {
                existing_skill_id: skill.id.clone(),
                existing_name: skill.name.clone(),
                existing_canonical_path: skill.canonical_path.clone(),
                proposed_skill_id: skill_id.to_string(),
                proposed_name: skill_id.to_string(),
            })
        } else {
            None
        }
    }))
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

    #[tokio::test]
    async fn preview_root_skill_has_no_central_conflict_in_empty_db() {
        let pool = crate::test_support::mem_pool().await;
        let bytes = make_zip(&[("SKILL.md", b"---\nname: My Skill\n---\nbody")]);
        let preview = build_preview_from_bytes(&pool, &bytes, "my-skill.zip".to_string())
            .await
            .unwrap();
        assert_eq!(preview.skills.len(), 1);
        let skill = &preview.skills[0];
        assert_eq!(skill.skill_id, "my-skill");
        assert_eq!(skill.skill_name, "My Skill");
        assert_eq!(skill.root_directory, "");
        assert!(skill.conflict.is_none());
        assert_eq!(preview.total_files, 1);
        assert!(preview.fingerprint.sha256.len() == 64);
        assert_eq!(preview.fingerprint.byte_len, bytes.len() as u64);
        assert_eq!(preview.archive_display_name, "my-skill.zip");
    }

    #[tokio::test]
    async fn preview_wrapper_skill_lists_files() {
        let pool = crate::test_support::mem_pool().await;
        let bytes = make_zip(&[
            ("demo/SKILL.md", b"---\nname: Demo\n---\nbody"),
            ("demo/assets/x.txt", b"hi"),
        ]);
        let preview = build_preview_from_bytes(&pool, &bytes, "demo.zip".to_string())
            .await
            .unwrap();
        let skill = &preview.skills[0];
        assert_eq!(skill.root_directory, "demo");
        assert_eq!(skill.skill_id, "demo");
        assert_eq!(skill.files.len(), 2);
        assert_eq!(skill.files[0].path, "SKILL.md");
        assert_eq!(skill.files[1].path, "assets/x.txt");
    }

    #[tokio::test]
    async fn preview_reports_conflict_for_existing_central_skill() {
        let pool = crate::test_support::mem_pool().await;
        // Seed the database with an existing central skill.
        let existing = crate::db::Skill {
            id: "existing".to_string(),
            uid: uuid::Uuid::new_v4().to_string(),
            name: "existing".to_string(),
            description: None,
            file_path: "/tmp/existing/SKILL.md".to_string(),
            canonical_path: Some("/tmp/existing".to_string()),
            is_central: true,
            source: None,
            content: None,
            scanned_at: chrono::Utc::now().to_rfc3339(),
            fs_created_at: None,
            fs_updated_at: None,
        };
        crate::db::upsert_skill(&pool, &existing).await.unwrap();
        let bytes = make_zip(&[("SKILL.md", b"---\nname: Existing\n---\nbody")]);
        let preview = build_preview_from_bytes(&pool, &bytes, "existing.zip".to_string())
            .await
            .unwrap();
        let skill = &preview.skills[0];
        assert_eq!(skill.skill_id, "existing");
        let conflict = skill
            .conflict
            .as_ref()
            .expect("conflict should be reported");
        assert_eq!(conflict.existing_skill_id, "existing");
    }
}
