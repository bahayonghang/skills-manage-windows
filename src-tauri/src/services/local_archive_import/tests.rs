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
use super::inventory::{build_inventory, fingerprint_of, ArchiveFingerprint};
use super::preview::preview_local_skill_archive_impl;
use super::types::LocalArchiveImportResolution;
use crate::services::resource_budget::ResourceBudget;
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

fn duplicate_zip_with_shadow_eocd_comment() -> Vec<u8> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(&mut buffer);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, contents) in [("a.txt", b"a".as_slice()), ("b.txt", b"b".as_slice())] {
        writer.start_file(name, options).expect("start file");
        writer.write_all(contents).expect("write file");
    }
    writer.finish().expect("finish ZIP");
    let mut bytes = buffer.into_inner();
    let mut search_start = 0;
    while let Some(relative) = bytes[search_start..]
        .windows(5)
        .position(|window| window == b"b.txt")
    {
        let index = search_start + relative;
        bytes[index..index + 5].copy_from_slice(b"a.txt");
        search_start = index + 5;
    }

    let real_eocd = bytes.len() - 22;
    let central_size =
        u32::from_le_bytes(bytes[real_eocd + 12..real_eocd + 16].try_into().unwrap());
    bytes[real_eocd + 20..real_eocd + 22].copy_from_slice(&22_u16.to_le_bytes());
    let mut shadow = [0_u8; 22];
    shadow[..4].copy_from_slice(b"PK\x05\x06");
    shadow[8..10].copy_from_slice(&1_u16.to_le_bytes());
    shadow[10..12].copy_from_slice(&1_u16.to_le_bytes());
    shadow[12..16].copy_from_slice(&(central_size + 22).to_le_bytes());
    bytes.extend_from_slice(&shadow);
    bytes
}

fn make_zip_with_options(name: &str, contents: &[u8], options: SimpleFileOptions) -> Vec<u8> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(&mut buffer);
    writer.start_file(name, options).expect("start file");
    writer.write_all(contents).expect("write file");
    writer.finish().expect("finish ZIP");
    buffer.into_inner()
}

fn replace_all(bytes: &mut [u8], old: &[u8], new: &[u8]) {
    assert_eq!(old.len(), new.len());
    let mut search_start = 0;
    while let Some(relative) = bytes[search_start..]
        .windows(old.len())
        .position(|window| window == old)
    {
        let index = search_start + relative;
        bytes[index..index + new.len()].copy_from_slice(new);
        search_start = index + new.len();
    }
}

fn mutate_header_u16(bytes: &mut [u8], signature: &[u8; 4], offset: usize, value: u16) {
    let mut index = 0;
    while index + offset + 2 <= bytes.len() {
        if bytes[index..].starts_with(signature) {
            bytes[index + offset..index + offset + 2].copy_from_slice(&value.to_le_bytes());
        }
        index += 1;
    }
}

fn mutate_header_u32(bytes: &mut [u8], signature: &[u8; 4], offset: usize, value: u32) {
    let mut index = 0;
    while index + offset + 4 <= bytes.len() {
        if bytes[index..].starts_with(signature) {
            bytes[index + offset..index + offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        index += 1;
    }
}

fn promote_footer_to_zip64(mut bytes: Vec<u8>) -> Vec<u8> {
    let eocd_offset = bytes.len() - 22;
    let eocd = bytes[eocd_offset..].to_vec();
    let central_size = u32::from_le_bytes(eocd[12..16].try_into().unwrap());
    let central_offset = u32::from_le_bytes(eocd[16..20].try_into().unwrap());
    bytes.truncate(eocd_offset);

    bytes.extend_from_slice(b"PK\x06\x06");
    bytes.extend_from_slice(&44_u64.to_le_bytes());
    bytes.extend_from_slice(&45_u16.to_le_bytes());
    bytes.extend_from_slice(&45_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&u64::from(central_size).to_le_bytes());
    bytes.extend_from_slice(&u64::from(central_offset).to_le_bytes());
    bytes.extend_from_slice(b"PK\x06\x07");
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&(eocd_offset as u64).to_le_bytes());
    bytes.extend_from_slice(&1_u32.to_le_bytes());

    let mut classic = eocd;
    classic[8..12].fill(0xff);
    classic[12..20].fill(0xff);
    bytes.extend_from_slice(&classic);
    bytes
}

#[test]
fn shadow_eocd_in_comment_cannot_hide_duplicate_raw_names() {
    let bytes = duplicate_zip_with_shadow_eocd_comment();
    let error = build_inventory(
        &bytes,
        crate::services::resource_budget::ResourceBudget::default_skill(),
    )
    .expect_err("real EOCD count must expose the collapsed duplicate");
    assert_eq!(error.code(), "archive_read_failed", "{error:?}");
}

#[test]
fn accepts_stored_and_deflated_regular_files() {
    for method in [
        zip::CompressionMethod::Stored,
        zip::CompressionMethod::Deflated,
    ] {
        let options = SimpleFileOptions::default().compression_method(method);
        let bytes = make_zip_with_options(
            "SKILL.md",
            b"---\nname: compression-fixture\n---\nsmall body",
            options,
        );
        let inventory = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap();
        assert_eq!(inventory.entries[0].path, "SKILL.md");
    }
}

#[test]
fn accepts_utf8_and_cp437_filename_bytes_without_path_drift() {
    let utf8 = make_zip_with_options(
        "\u{6280}\u{80fd}/SKILL.md",
        b"---\nname: unicode-skill\n---\n",
        SimpleFileOptions::default(),
    );
    let inventory = build_inventory(&utf8, ResourceBudget::default_skill()).unwrap();
    assert_eq!(inventory.entries[0].path, "\u{6280}\u{80fd}/SKILL.md");

    let mut cp437 = make_zip_with_options(
        "cafX/SKILL.md",
        b"---\nname: cp437-skill\n---\n",
        SimpleFileOptions::default(),
    );
    replace_all(&mut cp437, b"cafX/SKILL.md", b"caf\x82/SKILL.md");
    let inventory = build_inventory(&cp437, ResourceBudget::default_skill()).unwrap();
    assert_eq!(inventory.entries[0].path, "caf\u{e9}/SKILL.md");
}

#[test]
fn rejects_non_regular_unix_entry() {
    let mut bytes = make_zip_with_options(
        "named-pipe",
        b"payload",
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
    );
    mutate_header_u16(&mut bytes, b"PK\x01\x02", 4, 0x0314);
    mutate_header_u32(&mut bytes, b"PK\x01\x02", 38, 0o010777_u32 << 16);
    let error = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap_err();
    assert_eq!(error.code(), "unsupported_archive_entry");
}

#[test]
fn rejects_truncated_archive_and_nul_path() {
    let mut truncated = make_skill_zip("x", "x", None);
    truncated.truncate(truncated.len() - 10);
    assert_eq!(
        build_inventory(&truncated, ResourceBudget::default_skill())
            .unwrap_err()
            .code(),
        "archive_read_failed"
    );

    let mut nul = make_zip_with_options(
        "evilX/SKILL.md",
        b"---\nname: nul-fixture\n---\n",
        SimpleFileOptions::default(),
    );
    replace_all(&mut nul, b"evilX/SKILL.md", b"evil\0/SKILL.md");
    assert_eq!(
        build_inventory(&nul, ResourceBudget::default_skill())
            .unwrap_err()
            .code(),
        "invalid_archive_entry"
    );
}

#[test]
fn rejects_multidisk_count_mismatch_and_zip64_footer() {
    let mut multidisk = make_skill_zip("x", "x", None);
    mutate_header_u16(&mut multidisk, b"PK\x05\x06", 4, 1);
    mutate_header_u16(&mut multidisk, b"PK\x05\x06", 6, 1);
    assert_eq!(
        build_inventory(&multidisk, ResourceBudget::default_skill())
            .unwrap_err()
            .code(),
        "unsupported_archive_entry"
    );

    let mut count_mismatch = make_skill_zip("x", "x", None);
    mutate_header_u16(&mut count_mismatch, b"PK\x05\x06", 8, 0);
    assert!(build_inventory(&count_mismatch, ResourceBudget::default_skill()).is_err());

    let zip64 = promote_footer_to_zip64(make_skill_zip("x", "x", None));
    assert_eq!(
        build_inventory(&zip64, ResourceBudget::default_skill())
            .unwrap_err()
            .code(),
        "unsupported_archive_entry"
    );
}

#[test]
fn rejects_malformed_central_record_boundaries() {
    let mut oversized_name = make_skill_zip("x", "x", None);
    mutate_header_u16(&mut oversized_name, b"PK\x01\x02", 28, u16::MAX);
    assert!(build_inventory(&oversized_name, ResourceBudget::default_skill()).is_err());

    let mut non_central_signature = make_skill_zip("x", "x", None);
    replace_all(&mut non_central_signature, b"PK\x01\x02", b"PX\x01\x02");
    assert!(build_inventory(&non_central_signature, ResourceBudget::default_skill()).is_err());

    let mut trailing_residual = make_skill_zip("x", "x", None);
    let eocd = trailing_residual.len() - 22;
    let central_size =
        u32::from_le_bytes(trailing_residual[eocd + 12..eocd + 16].try_into().unwrap());
    mutate_header_u32(&mut trailing_residual, b"PK\x05\x06", 12, central_size + 1);
    assert!(build_inventory(&trailing_residual, ResourceBudget::default_skill()).is_err());
}

#[test]
fn fingerprint_bytes_match_the_published_sha256_format() {
    assert_eq!(
        fingerprint_of(b"SkillPort archive fingerprint v1\n"),
        ArchiveFingerprint {
            sha256: "873716dbf306c51e2638bc0b8cd05a08a63a758a478484fa138c32835f46a6ee".to_string(),
            byte_len: 33,
        }
    );
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
