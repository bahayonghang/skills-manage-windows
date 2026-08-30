//! Bounded SKILL.md reads and path-safe folder reveal for Skills CLI.

use std::path::{Path, PathBuf};

use crate::fs_util::run_blocking_fs_with;
use crate::services::bounded_ingestion::{read_file_text_bounded, BoundedReadError, ReadLimit};
use crate::services::installation::fs_util::is_reparse_or_symlink;

use super::error::SkillsCliError;
use super::is_valid_skill_token;
use super::lock::{load_cli_lock_ownership, parse_lock_content};
use super::placement::canonical_is_owned_directory;
use super::probe;
use super::SkillsCliSkillDoc;

const SKILL_DOC_LIMIT: u64 = 1_048_576;
const SKILL_DOC_LABEL: &str = "Skills CLI SKILL.md";

#[cfg(test)]
thread_local! {
    static REVEAL_SPAWN_FAULT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn set_reveal_spawn_fault(fault: bool) {
    REVEAL_SPAWN_FAULT.with(|cell| cell.set(fault));
}

pub(crate) struct OwnedCanonical {
    pub name: String,
    pub path: PathBuf,
}

pub(crate) fn resolve_owned_canonical(
    skill_name: &str,
    canonical_root: &Path,
    lock_path: &Path,
) -> Result<OwnedCanonical, SkillsCliError> {
    if !is_valid_skill_token(skill_name) || skill_name_escapes(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    let ownership = load_cli_lock_ownership(lock_path)?;
    if !ownership.contains_name(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    let root = canonical_root
        .canonicalize()
        .map_err(|_| SkillsCliError::CanonicalMissing)?;
    if !root.is_dir() {
        return Err(SkillsCliError::CanonicalMissing);
    }
    let candidate = root.join(skill_name);
    let metadata =
        std::fs::symlink_metadata(&candidate).map_err(|_| SkillsCliError::CanonicalMissing)?;
    if !metadata.is_dir() || is_reparse_or_symlink(&metadata) {
        return Err(SkillsCliError::CanonicalMissing);
    }
    let resolved = candidate
        .canonicalize()
        .map_err(|_| SkillsCliError::CanonicalMissing)?;
    if !is_component_contained(&root, &resolved) {
        return Err(SkillsCliError::CanonicalMissing);
    }
    Ok(OwnedCanonical {
        name: skill_name.to_string(),
        path: resolved,
    })
}

fn skill_name_escapes(name: &str) -> bool {
    name.contains('/')
        || name.contains('\\')
        || name == "."
        || name == ".."
        || Path::new(name).components().count() != 1
}

pub(crate) fn is_component_contained(root: &Path, candidate: &Path) -> bool {
    let root_components: Vec<_> = root.components().collect();
    let candidate_components: Vec<_> = candidate.components().collect();
    candidate_components.starts_with(&root_components)
}

pub(crate) async fn read_skill_md(
    tx: &super::SkillsCliTransport,
    skill_name: &str,
) -> Result<SkillsCliSkillDoc, SkillsCliError> {
    if !tx.is_remote() {
        let paths = tx.paths();
        return read_skill_md_at(
            skill_name,
            &paths.canonical_root_path(),
            &paths.lock_path_buf(),
        )
        .await;
    }
    if !is_valid_skill_token(skill_name) || skill_name_escapes(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    let paths = tx.paths();
    let ownership = match tx
        .fs()
        .read_file_bounded(paths.lock_path(), SKILL_DOC_LIMIT)
        .await
    {
        Ok(bytes) => parse_lock_content(&String::from_utf8_lossy(&bytes)),
        Err(SkillsCliError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            super::lock::CliLockOwnership::default()
        }
        Err(error) => return Err(error),
    };
    if !ownership.contains_name(skill_name) {
        return Err(SkillsCliError::SkillNotOwned);
    }
    let canonical = paths.join_child(paths.canonical_root(), skill_name);
    let probes = tx
        .fs()
        .probe_paths(std::slice::from_ref(&canonical))
        .await?;
    if !probe::canonical_owned_from_probe(probes.first()) {
        return Err(SkillsCliError::CanonicalMissing);
    }
    let md_path = paths.join_child(&canonical, "SKILL.md");
    let bytes = match tx.fs().read_file_bounded(&md_path, SKILL_DOC_LIMIT).await {
        Ok(bytes) => bytes,
        Err(SkillsCliError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            return Err(SkillsCliError::SkillDocMissing);
        }
        Err(error) => return Err(error),
    };
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > SKILL_DOC_LIMIT {
        return Err(SkillsCliError::SkillDocTooLarge);
    }
    let content = String::from_utf8(bytes).map_err(|_| SkillsCliError::SkillDocInvalidUtf8)?;
    let byte_size = u32::try_from(content.len()).unwrap_or(u32::MAX);
    Ok(SkillsCliSkillDoc {
        skill_name: skill_name.to_string(),
        content,
        byte_size,
    })
}

pub(crate) async fn read_skill_md_at(
    skill_name: &str,
    canonical_root: &Path,
    lock_path: &Path,
) -> Result<SkillsCliSkillDoc, SkillsCliError> {
    let skill_name = skill_name.to_string();
    let canonical_root = canonical_root.to_path_buf();
    let lock_path = lock_path.to_path_buf();
    run_blocking_fs_with(
        "Skills CLI SKILL.md read",
        move || {
            let owned = resolve_owned_canonical(&skill_name, &canonical_root, &lock_path)?;
            if !canonical_is_owned_directory(&owned.path) {
                return Err(SkillsCliError::CanonicalMissing);
            }
            read_skill_md_file(&owned.name, &owned.path.join("SKILL.md"))
        },
        SkillsCliError::task_join,
    )
    .await
}

fn map_bounded_doc_error(error: BoundedReadError) -> SkillsCliError {
    match error {
        BoundedReadError::LimitExceeded { .. } => SkillsCliError::SkillDocTooLarge,
        BoundedReadError::InvalidUtf8 { .. } => SkillsCliError::SkillDocInvalidUtf8,
        BoundedReadError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
            SkillsCliError::SkillDocMissing
        }
        BoundedReadError::Io { source, .. } => SkillsCliError::Io {
            context: "read SKILL.md",
            source,
        },
        BoundedReadError::Http { .. } => SkillsCliError::Io {
            context: "read SKILL.md",
            source: std::io::Error::other("unexpected HTTP reader"),
        },
    }
}

fn read_skill_md_file(skill_name: &str, path: &Path) -> Result<SkillsCliSkillDoc, SkillsCliError> {
    if !path.exists() {
        return Err(SkillsCliError::SkillDocMissing);
    }
    let content = read_file_text_bounded(path, ReadLimit::new(SKILL_DOC_LABEL, SKILL_DOC_LIMIT))
        .map_err(map_bounded_doc_error)?;
    let byte_size = u32::try_from(content.len()).unwrap_or(u32::MAX);
    Ok(SkillsCliSkillDoc {
        skill_name: skill_name.to_string(),
        content,
        byte_size,
    })
}

pub(crate) fn reveal_skill_folder(
    tx: &super::SkillsCliTransport,
    skill_name: &str,
) -> Result<(), SkillsCliError> {
    tx.ensure_capability(super::SkillsCliCapability::RevealFolder)?;
    let paths = tx.paths();
    reveal_skill_folder_at(
        skill_name,
        &paths.canonical_root_path(),
        &paths.lock_path_buf(),
        open_path_in_file_manager,
    )
}

pub(crate) fn reveal_skill_folder_at(
    skill_name: &str,
    canonical_root: &Path,
    lock_path: &Path,
    opener: fn(&Path) -> Result<(), SkillsCliError>,
) -> Result<(), SkillsCliError> {
    let owned = resolve_owned_canonical(skill_name, canonical_root, lock_path)?;
    opener(&owned.path)
}

pub(crate) fn open_path_in_file_manager(path: &Path) -> Result<(), SkillsCliError> {
    #[cfg(test)]
    if REVEAL_SPAWN_FAULT.with(|cell| cell.get()) {
        return Err(SkillsCliError::RevealFailed);
    }
    let mut command = file_manager_command();
    command.arg(path);
    command.spawn().map_err(|_| SkillsCliError::RevealFailed)?;
    Ok(())
}

fn file_manager_command() -> std::process::Command {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_lock(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(".skill-lock.json");
        std::fs::write(
            &path,
            format!(r#"{{"version":3,"skills":{{"{name}":{{}}}}}}"#),
        )
        .unwrap();
        path
    }

    #[tokio::test]
    async fn reads_exact_limit_utf8_document() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("skills");
        let skill = root.join("demo");
        std::fs::create_dir_all(&skill).unwrap();
        let content = "a".repeat(SKILL_DOC_LIMIT as usize);
        std::fs::write(skill.join("SKILL.md"), &content).unwrap();
        let lock = write_lock(temp.path(), "demo");
        let doc = read_skill_md_at("demo", &root, &lock).await.unwrap();
        assert_eq!(doc.byte_size, SKILL_DOC_LIMIT as u32);
        assert_eq!(doc.content.len(), SKILL_DOC_LIMIT as usize);
    }

    #[tokio::test]
    async fn rejects_metadata_oversize() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("skills");
        let skill = root.join("demo");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(
            skill.join("SKILL.md"),
            vec![b'a'; SKILL_DOC_LIMIT as usize + 1],
        )
        .unwrap();
        let lock = write_lock(temp.path(), "demo");
        let err = read_skill_md_at("demo", &root, &lock).await.unwrap_err();
        assert!(matches!(err, SkillsCliError::SkillDocTooLarge));
        assert!(!format!("{err:?}").contains("demo"));
    }

    #[test]
    fn opened_handle_growth_maps_to_too_large_without_path_or_size() {
        let error = map_bounded_doc_error(BoundedReadError::LimitExceeded {
            label: SKILL_DOC_LABEL,
            actual: SKILL_DOC_LIMIT + 1,
            limit: SKILL_DOC_LIMIT,
        });
        assert!(matches!(error, SkillsCliError::SkillDocTooLarge));
        let rendered = format!("{error:?}{error}");
        assert!(!rendered.contains("1048577"));
        assert!(!rendered.contains('\\'));
        assert!(!rendered.contains('/'));
        let public = crate::ipc_error::public_message_for_code(error.ipc_code()).unwrap();
        assert!(!public.contains("1048577"));
        assert!(!public.contains(SKILL_DOC_LABEL));
    }

    #[tokio::test]
    async fn rejects_invalid_utf8_without_leaking_bytes() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("skills");
        let skill = root.join("demo");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), [0xff, 0xfe]).unwrap();
        let lock = write_lock(temp.path(), "demo");
        let err = read_skill_md_at("demo", &root, &lock).await.unwrap_err();
        assert!(matches!(err, SkillsCliError::SkillDocInvalidUtf8));
        let serialized = format!("{err}");
        assert!(!serialized.contains('\u{fffd}') && !serialized.contains("ff"));
    }

    #[tokio::test]
    async fn rejects_missing_and_unowned_and_escape() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("skills");
        let skill = root.join("demo");
        std::fs::create_dir_all(&skill).unwrap();
        let lock = write_lock(temp.path(), "demo");
        let missing = read_skill_md_at("demo", &root, &lock).await.unwrap_err();
        assert!(matches!(missing, SkillsCliError::SkillDocMissing));
        let unowned = read_skill_md_at("other", &root, &lock).await.unwrap_err();
        assert!(matches!(unowned, SkillsCliError::SkillNotOwned));
        let escape = read_skill_md_at("../demo", &root, &lock).await.unwrap_err();
        assert!(matches!(escape, SkillsCliError::SkillNotOwned));
    }

    #[test]
    fn reveal_rejects_non_directory_and_symlink_escape() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("skills");
        std::fs::create_dir_all(&root).unwrap();
        let outside = temp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        let lock = write_lock(temp.path(), "demo");
        std::fs::write(root.join("demo"), b"file").unwrap();
        let err = reveal_skill_folder_at("demo", &root, &lock, |_| Ok(())).unwrap_err();
        assert!(matches!(err, SkillsCliError::CanonicalMissing));

        std::fs::remove_file(root.join("demo")).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, root.join("demo")).unwrap();
            let err = reveal_skill_folder_at("demo", &root, &lock, |_| Ok(())).unwrap_err();
            assert!(matches!(err, SkillsCliError::CanonicalMissing));
        }
        #[cfg(windows)]
        {
            crate::services::installation::fs_util::create_skills_cli_directory_link(
                &outside,
                &root.join("demo"),
            )
            .unwrap();
            let err = reveal_skill_folder_at("demo", &root, &lock, |_| Ok(())).unwrap_err();
            assert!(matches!(err, SkillsCliError::CanonicalMissing));
        }
    }

    #[test]
    fn reveal_spawn_failure_is_typed() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("skills");
        std::fs::create_dir_all(root.join("demo")).unwrap();
        let lock = write_lock(temp.path(), "demo");
        set_reveal_spawn_fault(true);
        let err =
            reveal_skill_folder_at("demo", &root, &lock, open_path_in_file_manager).unwrap_err();
        set_reveal_spawn_fault(false);
        assert!(matches!(err, SkillsCliError::RevealFailed));
    }

    #[test]
    fn prefix_trap_is_not_contained() {
        let root = Path::new("/tmp/skills/demo");
        let candidate = Path::new("/tmp/skills/demo-extra");
        assert!(!is_component_contained(root, candidate));
        assert!(is_component_contained(root, Path::new("/tmp/skills/demo")));
        assert!(is_component_contained(
            root,
            Path::new("/tmp/skills/demo/SKILL.md")
        ));
    }
}
