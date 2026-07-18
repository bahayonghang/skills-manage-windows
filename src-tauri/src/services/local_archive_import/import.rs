//! Import pipeline: re-verify fingerprint, stage, atomically swap, persist.
//!
//! Steps (all failures after staging start must restore the backup and clean
//! up staging):
//! 1. Read the archive bytes once under the budget.
//! 2. Verify the archive fingerprint (SHA-256 + byte length) matches the
//!    user-confirmed `expected_fingerprint`. Mismatch returns
//!    `archive_changed_since_preview` before any staging/Central/DB write.
//! 3. Rebuild inventory and resolve the candidate from the same bytes.
//!    Re-running the safety matrix here is deliberate: the import path never
//!    trusts the preview's inventory.
//! 4. Apply the user's resolution strategy:
//!    - `Skip` -> return early with `replaced_existing: false` and no write.
//!    - `Rename` -> the caller-provided `renamed_skill_id` must pass the same
//!      sanitization rule; if it collides with Central, fail closed.
//!    - `Overwrite` -> the existing Central skill directory is backed up
//!      atomically (rename to a `.skillport-backup-*` sibling) before any
//!      staging write, and restored on any later failure.
//! 5. Stage the archive into a unique `.skillport-import-*` work directory:
//!    extract each entry under the stripped skill root, re-check budgets,
//!    and verify `SKILL.md` parses.
//! 6. Acquire the Central mutation guard.
//! 7. Atomically swap the staging directory into the final target directory
//!    (`<central_root>/<skill_id>`).
//! 8. Upsert the skill row (`is_central: true`, no GitHub repository
//!    assignment, `source: "local-archive"`).
//!
//! Invariants:
//! - No Central, staging, database, or Operation Log mutation occurs before
//!   fingerprint verification succeeds.
//! - The final skill row never carries a GitHub repository assignment so the
//!   update center treats it as unsupported/unknown rather than remote-missing.

use std::path::{Path, PathBuf};

use chrono::Utc;
use uuid::Uuid;

use crate::db::{self, DbPool, Skill};
use crate::services::central_mutation::{
    acquire_central_mutation_guard, DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};
use crate::services::local_archive_import::candidate::{resolve_candidate, CandidateFile};
use crate::services::local_archive_import::error::{task_join, LocalArchiveImportError};
use crate::services::local_archive_import::inventory::{
    build_inventory, fingerprint_of, read_archive_bytes, ArchiveFingerprint,
};
use crate::services::local_archive_import::types::{
    LocalArchiveImportResolution, LocalArchiveImportResult,
};
use crate::services::resource_budget::ResourceBudget;

/// Outcome returned by [`import_local_skill_archive_impl`].
pub type ImportOutcome = Result<LocalArchiveImportResult, LocalArchiveImportError>;

/// Re-verify the archive fingerprint matches the one the user confirmed.
///
/// This is the single gate that prevents a TOCTOU between preview and import:
/// no staging, Central, database, or Operation Log write happens before it.
pub(crate) fn verify_fingerprint(
    archive_bytes: &[u8],
    expected: &ArchiveFingerprint,
) -> Result<(), LocalArchiveImportError> {
    let actual = fingerprint_of(archive_bytes);
    if actual.byte_len != expected.byte_len {
        return Err(LocalArchiveImportError::ArchiveChangedSincePreview {
            detail: format!(
                "byte length changed: expected {} got {}",
                expected.byte_len, actual.byte_len
            ),
        });
    }
    if !actual.sha256.eq_ignore_ascii_case(&expected.sha256) {
        return Err(LocalArchiveImportError::ArchiveChangedSincePreview {
            detail: format!(
                "sha256 changed: expected {} got {}",
                expected.sha256, actual.sha256
            ),
        });
    }
    Ok(())
}

/// Run the full import pipeline for a local `.zip` archive.
pub(crate) async fn import_local_skill_archive_impl(
    pool: &DbPool,
    archive_path: &str,
    expected_fingerprint: ArchiveFingerprint,
    resolution: LocalArchiveImportResolution,
    renamed_skill_id: Option<String>,
) -> ImportOutcome {
    let budget = ResourceBudget::default_skill();
    let archive_path_owned = archive_path.to_string();
    let archive_bytes = crate::fs_util::run_blocking_fs_with(
        "read local skill archive for import",
        move || read_archive_bytes(&archive_path_owned, budget),
        task_join,
    )
    .await?;

    verify_fingerprint(&archive_bytes, &expected_fingerprint)?;

    let inventory = build_inventory(&archive_bytes, budget)?;
    let candidate = resolve_candidate(&inventory, &archive_bytes, budget)?;

    let final_skill_id = resolve_final_skill_id(
        pool,
        &candidate.skill_id,
        resolution.clone(),
        renamed_skill_id.as_deref(),
    )
    .await?;

    if matches!(resolution, LocalArchiveImportResolution::Skip) {
        return Ok(LocalArchiveImportResult {
            imported_skill_id: final_skill_id,
            skill_name: candidate.skill_name.clone(),
            root_directory: candidate.root_directory.clone(),
            resolution,
            file_count: candidate.files.len(),
            total_expanded_bytes: candidate.files.iter().map(|f| f.byte_len).sum(),
            replaced_existing: false,
        });
    }

    let central_root = central_skills_root(pool).await?;
    let staging_path = create_unique_work_dir(&central_root, ".skillport-archive-import-")?;
    let stage_result = stage_archive(
        &archive_bytes,
        &candidate.files,
        &staging_path,
        &candidate.root_directory,
        &candidate.skill_md_path,
        budget,
    )
    .await;
    if let Err(error) = stage_result {
        discard_staging_dir(&staging_path).await;
        return Err(error);
    }

    let _guard = match acquire_central_mutation_guard(
        "Local archive skill import",
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await
    {
        Ok(guard) => guard,
        Err(error) => {
            discard_staging_dir(&staging_path).await;
            return Err(error.into());
        }
    };

    let target_dir = central_root.join(&final_skill_id);
    let mut existing_backup: Option<PathBuf> = None;
    if target_dir.exists() {
        if !matches!(resolution, LocalArchiveImportResolution::Overwrite) {
            discard_staging_dir(&staging_path).await;
            return Err(LocalArchiveImportError::PathConflict(format!(
                "target directory already exists for skill id '{final_skill_id}'"
            )));
        }
        existing_backup = Some(
            match backup_existing_skill_dir(&central_root, &target_dir) {
                Ok(backup) => backup,
                Err(error) => {
                    discard_staging_dir(&staging_path).await;
                    return Err(error);
                }
            },
        );
    }

    if let Err(error) = std::fs::rename(&staging_path, &target_dir) {
        restore_or_cleanup_target_dir(&target_dir, existing_backup.take()).await?;
        discard_staging_dir(&staging_path).await;
        return Err(LocalArchiveImportError::Io(error));
    }

    let replaced_existing = existing_backup.is_some();
    let db_skill = Skill {
        id: final_skill_id.clone(),
        uid: Uuid::new_v4().to_string(),
        name: candidate.skill_name.clone(),
        description: candidate.description.clone(),
        file_path: target_dir.join("SKILL.md").to_string_lossy().into_owned(),
        canonical_path: Some(target_dir.to_string_lossy().into_owned()),
        is_central: true,
        source: Some("local-archive".to_string()),
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    if let Err(error) = db::upsert_skill(pool, &db_skill).await {
        restore_or_cleanup_target_dir(&target_dir, existing_backup.take()).await?;
        return Err(LocalArchiveImportError::Db(error));
    }

    drop_existing_backup(existing_backup.take()).await;

    Ok(LocalArchiveImportResult {
        imported_skill_id: final_skill_id,
        skill_name: candidate.skill_name.clone(),
        root_directory: candidate.root_directory.clone(),
        resolution,
        file_count: candidate.files.len(),
        total_expanded_bytes: candidate.files.iter().map(|f| f.byte_len).sum(),
        replaced_existing,
    })
}

async fn resolve_final_skill_id(
    pool: &DbPool,
    candidate_skill_id: &str,
    resolution: LocalArchiveImportResolution,
    renamed_skill_id: Option<&str>,
) -> Result<String, LocalArchiveImportError> {
    match resolution {
        LocalArchiveImportResolution::Overwrite | LocalArchiveImportResolution::Skip => {
            Ok(candidate_skill_id.to_string())
        }
        LocalArchiveImportResolution::Rename => {
            let raw = renamed_skill_id.ok_or_else(|| {
                LocalArchiveImportError::Internal(
                    "rename resolution requires a renamed_skill_id".to_string(),
                )
            })?;
            let sanitized = sanitize_skill_id(raw)?;
            if sanitized == candidate_skill_id {
                return Err(LocalArchiveImportError::InvalidSkillIdentifier(format!(
                    "renamed id '{sanitized}' must differ from the archive's id"
                )));
            }
            if let Some(existing) = db::get_skill_by_id(pool, &sanitized).await? {
                if existing.is_central {
                    return Err(LocalArchiveImportError::PathConflict(format!(
                        "renamed skill id '{sanitized}' already exists in Central"
                    )));
                }
            }
            Ok(sanitized)
        }
    }
}

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

async fn stage_archive(
    archive_bytes: &[u8],
    files: &[CandidateFile],
    staging_path: &Path,
    root_directory: &str,
    skill_md_relative: &str,
    budget: ResourceBudget,
) -> Result<(), LocalArchiveImportError> {
    let staging_path_owned = staging_path.to_path_buf();
    let archive_bytes_owned = archive_bytes.to_vec();
    let files_owned = files.to_vec();
    let root_owned = root_directory.to_string();
    let skill_md_owned = skill_md_relative.to_string();
    crate::fs_util::run_blocking_fs_with(
        "stage local skill archive",
        move || {
            stage_archive_blocking(
                &archive_bytes_owned,
                &files_owned,
                &staging_path_owned,
                &root_owned,
                &skill_md_owned,
                budget,
            )
        },
        task_join,
    )
    .await
}

fn stage_archive_blocking(
    archive_bytes: &[u8],
    files: &[CandidateFile],
    staging_path: &Path,
    root_directory: &str,
    skill_md_relative: &str,
    budget: ResourceBudget,
) -> Result<(), LocalArchiveImportError> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(archive_bytes.to_vec());
    let mut zip = zip::ZipArchive::new(cursor)
        .map_err(|e| LocalArchiveImportError::ArchiveReadFailed(format!("reopen zip: {e}")))?;
    let mut entry_lookup: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for index in 0..zip.len() {
        let entry = zip.by_index_raw(index).map_err(|e| {
            LocalArchiveImportError::ArchiveReadFailed(format!("entry {index}: {e}"))
        })?;
        entry_lookup.insert(entry.name().to_string(), index);
    }
    let prefix = if root_directory.is_empty() {
        String::new()
    } else {
        format!("{}/", root_directory)
    };
    for file in files {
        let raw_name = format!("{}{}", prefix, file.path);
        let entry_index = *entry_lookup.get(&raw_name).ok_or_else(|| {
            LocalArchiveImportError::Internal(format!(
                "staging: entry for '{}' not found",
                file.path
            ))
        })?;
        let mut entry = zip
            .by_index(entry_index)
            .map_err(|e| LocalArchiveImportError::ArchiveReadFailed(format!("open entry: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        let dest = staging_path.join(&file.path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(LocalArchiveImportError::Io)?;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf).map_err(|e| {
            LocalArchiveImportError::ArchiveReadFailed(format!("read '{}': {e}", file.path))
        })?;
        budget
            .reject_file_read_size(&file.path, buf.len() as u64)
            .map_err(LocalArchiveImportError::BudgetExceeded)?;
        std::fs::write(&dest, &buf).map_err(|e| {
            LocalArchiveImportError::Internal(format!("write '{}': {e}", file.path))
        })?;
    }
    let skill_md_path = staging_path.join(skill_md_relative);
    let content = std::fs::read_to_string(&skill_md_path).map_err(|e| {
        LocalArchiveImportError::SkillFrontmatterMissing(format!("staged SKILL.md: {e}"))
    })?;
    if crate::services::scanner::extract_frontmatter_block(&content).is_none() {
        return Err(LocalArchiveImportError::SkillFrontmatterMissing(
            "no frontmatter".to_string(),
        ));
    }
    Ok(())
}

async fn central_skills_root(pool: &DbPool) -> Result<PathBuf, LocalArchiveImportError> {
    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| LocalArchiveImportError::Internal("central agent missing".to_string()))?;
    Ok(PathBuf::from(central.global_skills_dir))
}

fn create_unique_work_dir(parent: &Path, prefix: &str) -> Result<PathBuf, LocalArchiveImportError> {
    let path = parent.join(format!("{prefix}{}", Uuid::new_v4()));
    std::fs::create_dir_all(&path)
        .map_err(|e| LocalArchiveImportError::Internal(format!("staging dir: {e}")))?;
    Ok(path)
}

fn backup_existing_skill_dir(
    central_root: &Path,
    target_dir: &Path,
) -> Result<PathBuf, LocalArchiveImportError> {
    let backup = central_root.join(format!(".skillport-backup-{}", Uuid::new_v4()));
    std::fs::rename(target_dir, &backup)
        .map_err(|e| LocalArchiveImportError::Internal(format!("backup: {e}")))?;
    Ok(backup)
}

async fn discard_staging_dir(staging_path: &Path) {
    let p = staging_path.to_path_buf();
    let _ = crate::fs_util::run_blocking_fs_with(
        "archive staging cleanup",
        move || {
            let _ = std::fs::remove_dir_all(&p);
            Ok::<(), LocalArchiveImportError>(())
        },
        task_join,
    )
    .await;
}

async fn restore_or_cleanup_target_dir(
    target_dir: &Path,
    backup: Option<PathBuf>,
) -> Result<(), LocalArchiveImportError> {
    let t = target_dir.to_path_buf();
    crate::fs_util::run_blocking_fs_with(
        "archive target restore",
        move || {
            if t.exists() {
                std::fs::remove_dir_all(&t).map_err(|source| {
                    LocalArchiveImportError::RollbackFailed {
                        stage: "remove replacement target",
                        source,
                    }
                })?;
            }
            if let Some(b) = backup {
                std::fs::rename(&b, &t).map_err(|source| {
                    LocalArchiveImportError::RollbackFailed {
                        stage: "restore previous target",
                        source,
                    }
                })?;
            }
            Ok(())
        },
        task_join,
    )
    .await
}

async fn drop_existing_backup(backup: Option<PathBuf>) {
    let _ = crate::fs_util::run_blocking_fs_with(
        "archive backup cleanup",
        move || {
            if let Some(b) = backup {
                let _ = std::fs::remove_dir_all(&b);
            }
            Ok::<(), LocalArchiveImportError>(())
        },
        task_join,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_fingerprint_passes_for_identical() {
        let bytes = b"hello";
        let fp = fingerprint_of(bytes);
        verify_fingerprint(bytes, &fp).unwrap();
    }

    #[test]
    fn verify_fingerprint_rejects_length_mismatch() {
        let bytes = b"hello";
        let mut fp = fingerprint_of(bytes);
        fp.byte_len += 1;
        let err = verify_fingerprint(bytes, &fp).unwrap_err();
        assert_eq!(err.code(), "archive_changed_since_preview");
    }

    #[test]
    fn verify_fingerprint_rejects_sha_mismatch() {
        let bytes = b"hello";
        let mut fp = fingerprint_of(bytes);
        fp.sha256 = format!("0{}", &fp.sha256[1..]);
        let err = verify_fingerprint(bytes, &fp).unwrap_err();
        assert_eq!(err.code(), "archive_changed_since_preview");
    }

    #[test]
    fn sanitize_skill_id_rules() {
        assert_eq!(sanitize_skill_id("Hello World").unwrap(), "hello-world");
        assert_eq!(sanitize_skill_id("a!!b").unwrap(), "a-b");
        assert!(sanitize_skill_id("").is_err());
        assert!(sanitize_skill_id("!!!").is_err());
    }
}
