//! Module-level integration tests for local archive import.
//!
//! These tests exercise the full preview → import pipeline end-to-end using
//! in-memory databases and temp directories, verifying:
//! - Preview is read-only (no filesystem/DB mutation).
//! - Fingerprint verification catches byte-level changes.
//! - Overwrite backs up and replaces the existing skill.
//! - Rename writes to a new skill id without touching the original.
//! - Skip produces no write.
//! - Archive skills have no GitHub repository assignment.

#![cfg(test)]

use std::io::Write;

use tempfile::TempDir;
use zip::write::{SimpleFileOptions, ZipWriter};

use super::import::import_local_skill_archive_impl;
use super::inventory::fingerprint_of;
use super::preview::preview_local_skill_archive_impl;
use super::types::LocalArchiveImportResolution;
use crate::{db, test_support};

fn make_skill_zip(name: &str, description: &str, extra_file: Option<(&str, &[u8])>) -> Vec<u8> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(&mut buffer);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer
        .start_file("SKILL.md", options)
        .expect("start SKILL.md");
    write!(
        writer,
        "---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n"
    )
    .expect("write SKILL.md");
    if let Some((path, bytes)) = extra_file {
        writer.start_file(path, options).expect("start extra file");
        writer.write_all(bytes).expect("write extra file");
    }
    writer.finish().expect("finish ZIP");
    buffer.into_inner()
}

fn write_archive(dir: &TempDir, name: &str, bytes: &[u8]) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).expect("write archive fixture");
    path.to_string_lossy().into_owned()
}

fn work_artifacts(central_root: &std::path::Path) -> Vec<String> {
    let mut names = std::fs::read_dir(central_root)
        .expect("read central root")
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            name.starts_with(".skillport-archive-import-") || name.starts_with(".skillport-backup-")
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[tokio::test]
async fn preview_and_fingerprint_mismatch_do_not_mutate_central_or_logs() {
    let pool = test_support::mem_pool().await;
    let fs = TempDir::new().expect("temp Central root");
    let central_root = fs.path().join("central");
    std::fs::create_dir_all(&central_root).expect("create Central root");
    test_support::set_agent_dir(&pool, "central", &central_root).await;

    let archive_dir = TempDir::new().expect("temp archive root");
    let bytes = make_skill_zip("new-skill", "new description", Some(("new.txt", b"new")));
    let archive_path = write_archive(&archive_dir, "new.zip", &bytes);

    let preview = preview_local_skill_archive_impl(&pool, &archive_path)
        .await
        .expect("preview succeeds");
    assert_eq!(preview.skills[0].skill_id, "new-skill");
    assert!(std::fs::read_dir(&central_root)
        .expect("read Central root")
        .next()
        .is_none());
    assert!(db::get_skill_by_id(&pool, "new-skill")
        .await
        .expect("query preview skill")
        .is_none());

    let mut changed = preview.fingerprint;
    changed.sha256 = "0".repeat(64);
    let error = import_local_skill_archive_impl(
        &pool,
        &archive_path,
        changed,
        LocalArchiveImportResolution::Overwrite,
        None,
    )
    .await
    .expect_err("fingerprint mismatch must fail");
    assert_eq!(error.code(), "archive_changed_since_preview");
    assert_eq!(work_artifacts(&central_root), Vec::<String>::new());
    assert!(db::get_skill_by_id(&pool, "new-skill")
        .await
        .expect("query mismatched skill")
        .is_none());

    let logs = db::list_operation_logs(&pool, Default::default())
        .await
        .expect("list operation logs");
    assert_eq!(logs.total, 0, "service-level preview/import does not log");
}

#[tokio::test]
async fn overwrite_db_failure_restores_old_directory_and_cleans_work_artifacts() {
    let pool = test_support::mem_pool().await;
    let fs = TempDir::new().expect("temp Central root");
    let central_root = fs.path().join("central");
    std::fs::create_dir_all(&central_root).expect("create Central root");
    test_support::set_agent_dir(&pool, "central", &central_root).await;

    let old_dir = central_root.join("demo-skill");
    test_support::seed_central_skill(&pool, &old_dir, "demo-skill", "old description").await;
    std::fs::write(old_dir.join("old.txt"), b"old").expect("write old marker");

    sqlx::query(
        "CREATE TRIGGER fail_local_archive_upsert
         BEFORE INSERT ON skills
         WHEN NEW.source = 'local-archive'
         BEGIN
           SELECT RAISE(FAIL, 'forced local archive DB failure');
         END",
    )
    .execute(&pool)
    .await
    .expect("install DB failure trigger");

    let archive_dir = TempDir::new().expect("temp archive root");
    let bytes = make_skill_zip("demo-skill", "new description", Some(("new.txt", b"new")));
    let archive_path = write_archive(&archive_dir, "demo.zip", &bytes);

    let error = import_local_skill_archive_impl(
        &pool,
        &archive_path,
        fingerprint_of(&bytes),
        LocalArchiveImportResolution::Overwrite,
        None,
    )
    .await
    .expect_err("forced DB failure must abort import");

    assert_eq!(error.code(), "db");
    assert!(
        old_dir.join("old.txt").is_file(),
        "old marker must be restored"
    );
    assert!(
        !old_dir.join("new.txt").exists(),
        "new target must be removed"
    );
    let restored = std::fs::read_to_string(old_dir.join("SKILL.md")).expect("restored SKILL.md");
    assert!(restored.contains("old description"));
    assert_eq!(work_artifacts(&central_root), Vec::<String>::new());

    let stored = db::get_skill_by_id(&pool, "demo-skill")
        .await
        .expect("load old DB row")
        .expect("old DB row remains");
    assert_eq!(stored.description.as_deref(), Some("old description"));
}

#[tokio::test]
async fn rename_skip_and_successful_overwrite_keep_repository_source_unknown() {
    let pool = test_support::mem_pool().await;
    let fs = TempDir::new().expect("temp Central root");
    let central_root = fs.path().join("central");
    std::fs::create_dir_all(&central_root).expect("create Central root");
    test_support::set_agent_dir(&pool, "central", &central_root).await;

    let old_dir = central_root.join("demo-skill");
    test_support::seed_central_skill(&pool, &old_dir, "demo-skill", "old description").await;
    let archive_dir = TempDir::new().expect("temp archive root");
    let bytes = make_skill_zip("demo-skill", "new description", Some(("new.txt", b"new")));
    let archive_path = write_archive(&archive_dir, "demo.zip", &bytes);
    let fingerprint = fingerprint_of(&bytes);

    let skipped = import_local_skill_archive_impl(
        &pool,
        &archive_path,
        fingerprint.clone(),
        LocalArchiveImportResolution::Skip,
        None,
    )
    .await
    .expect("skip succeeds");
    assert_eq!(skipped.imported_skill_id, "demo-skill");
    assert!(!old_dir.join("new.txt").exists());

    let renamed = import_local_skill_archive_impl(
        &pool,
        &archive_path,
        fingerprint.clone(),
        LocalArchiveImportResolution::Rename,
        Some("renamed-skill".to_string()),
    )
    .await
    .expect("rename succeeds");
    assert_eq!(renamed.imported_skill_id, "renamed-skill");
    assert!(central_root.join("renamed-skill/new.txt").is_file());

    let overwritten = import_local_skill_archive_impl(
        &pool,
        &archive_path,
        fingerprint,
        LocalArchiveImportResolution::Overwrite,
        None,
    )
    .await
    .expect("overwrite succeeds");
    assert!(overwritten.replaced_existing);
    assert!(old_dir.join("new.txt").is_file());
    assert_eq!(work_artifacts(&central_root), Vec::<String>::new());

    for skill_id in ["demo-skill", "renamed-skill"] {
        let assignment = db::get_skill_repository_assignment(&pool, skill_id)
            .await
            .expect("load repository assignment");
        assert!(assignment.is_source_unknown);
        assert!(assignment.repository.is_unknown);
        assert!(assignment.source_path.is_none());
    }
}
