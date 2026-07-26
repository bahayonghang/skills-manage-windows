use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use tempfile::Builder;

use crate::fs_util::run_blocking_fs_with;
use crate::services::resource_budget::ResourceBudget;

use super::{parse_manifest, PortableStateError};

const READ_TASK_LABEL: &str = "portable state file read";
const WRITE_TASK_LABEL: &str = "portable state file write";
const TEMP_FILE_PREFIX: &str = ".skillport-state-";

pub(crate) async fn read_skillport_state_file(path: PathBuf) -> Result<String, PortableStateError> {
    run_blocking_fs_with(
        READ_TASK_LABEL,
        move || read_skillport_state_file_sync(&path, ResourceBudget::default_skill()),
        PortableStateError::task_join,
    )
    .await
}

pub(crate) async fn write_skillport_state_file(
    path: PathBuf,
    json: String,
) -> Result<(), PortableStateError> {
    run_blocking_fs_with(
        WRITE_TASK_LABEL,
        move || write_skillport_state_file_sync(&path, &json, ResourceBudget::default_skill()),
        PortableStateError::task_join,
    )
    .await
}

fn read_skillport_state_file_sync(
    path: &Path,
    budget: ResourceBudget,
) -> Result<String, PortableStateError> {
    validate_json_extension(path)?;
    let file = open_read_handle(path).map_err(|source| {
        PortableStateError::io(
            format!("Failed to open SkillPort state file '{}'", path.display()),
            source,
        )
    })?;
    let metadata = file.metadata().map_err(|source| {
        PortableStateError::io(
            format!(
                "Failed to read SkillPort state file metadata '{}'",
                path.display()
            ),
            source,
        )
    })?;
    if !metadata.is_file() {
        return Err(PortableStateError::NotRegularFile(
            path.display().to_string(),
        ));
    }
    read_open_file(path, file, metadata.len(), budget)
}

#[cfg(windows)]
fn open_read_handle(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    // FILE_FLAG_BACKUP_SEMANTICS lets Windows open a directory handle so the
    // regular-file decision is made from metadata on the opened handle.
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x0200_0000)
        .open(path)
}

#[cfg(not(windows))]
fn open_read_handle(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

fn read_open_file(
    path: &Path,
    file: File,
    metadata_len: u64,
    budget: ResourceBudget,
) -> Result<String, PortableStateError> {
    let path_text = path.display().to_string();
    budget
        .reject_file_read_size(&path_text, metadata_len)
        .map_err(PortableStateError::Budget)?;

    let mut bytes = Vec::with_capacity(metadata_len.min(budget.file_bytes) as usize);
    file.take(budget.file_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| {
            PortableStateError::io(
                format!("Failed to read SkillPort state file '{path_text}'"),
                source,
            )
        })?;
    budget
        .reject_file_read_size(&path_text, bytes.len() as u64)
        .map_err(PortableStateError::Budget)?;
    if bytes.len() as u64 > metadata_len {
        return Err(PortableStateError::FileChangedDuringRead(path_text));
    }

    String::from_utf8(bytes).map_err(|source| PortableStateError::InvalidUtf8 {
        path: path_text,
        source,
    })
}

fn write_skillport_state_file_sync(
    path: &Path,
    json: &str,
    budget: ResourceBudget,
) -> Result<(), PortableStateError> {
    validate_json_extension(path)?;
    budget
        .reject_file_read_size(&path.display().to_string(), json.len() as u64)
        .map_err(PortableStateError::Budget)?;
    parse_manifest(json)?;

    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp = Builder::new()
        .prefix(TEMP_FILE_PREFIX)
        .tempfile_in(parent)
        .map_err(|source| {
            PortableStateError::io(
                format!(
                    "Failed to create temporary SkillPort state file in '{}'",
                    parent.display()
                ),
                source,
            )
        })?;
    temp.write_all(json.as_bytes()).map_err(|source| {
        PortableStateError::io(
            format!("Failed to write SkillPort state file '{}'", path.display()),
            source,
        )
    })?;
    temp.flush().map_err(|source| {
        PortableStateError::io(
            format!("Failed to flush SkillPort state file '{}'", path.display()),
            source,
        )
    })?;
    temp.as_file().sync_all().map_err(|source| {
        PortableStateError::io(
            format!("Failed to sync SkillPort state file '{}'", path.display()),
            source,
        )
    })?;
    temp.persist(path).map_err(|error| {
        PortableStateError::io(
            format!(
                "Failed to replace SkillPort state file '{}'",
                path.display()
            ),
            error.error,
        )
    })?;
    Ok(())
}

fn validate_json_extension(path: &Path) -> Result<(), PortableStateError> {
    let is_json = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    if is_json {
        Ok(())
    } else {
        Err(PortableStateError::InvalidFileExtension(
            path.display().to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, OpenOptions};

    use tempfile::TempDir;

    use super::*;

    const VALID_JSON: &str = r#"{"kind":"skillport/state-export","version":1,"exportedAt":"2026-07-26T00:00:00Z","exportedFrom":{"app":"SkillPort"},"githubSources":[],"centralSkills":[],"unrestorableSkills":[]}"#;

    fn budget(file_bytes: u64) -> ResourceBudget {
        ResourceBudget {
            file_bytes,
            ..ResourceBudget::default_skill()
        }
    }

    #[test]
    fn rejects_non_json_extension_before_opening() {
        let path = Path::new("missing.txt");
        assert!(matches!(
            read_skillport_state_file_sync(path, budget(1024)),
            Err(PortableStateError::InvalidFileExtension(_))
        ));
    }

    #[test]
    fn reports_missing_json_file_as_io_error() {
        let path = Path::new("missing.json");
        assert!(matches!(
            read_skillport_state_file_sync(path, budget(1024)),
            Err(PortableStateError::Io { .. })
        ));
    }

    #[test]
    fn accepts_case_insensitive_json_extension() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.JSON");
        fs::write(&path, VALID_JSON).unwrap();
        assert_eq!(
            read_skillport_state_file_sync(&path, budget(1024)).unwrap(),
            VALID_JSON
        );
    }

    #[test]
    fn rejects_opened_directory_handle() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        fs::create_dir(&path).unwrap();
        assert!(matches!(
            read_skillport_state_file_sync(&path, budget(1024)),
            Err(PortableStateError::NotRegularFile(_))
        ));
    }

    #[test]
    fn rejects_metadata_over_budget() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        fs::write(&path, b"12345").unwrap();
        assert!(matches!(
            read_skillport_state_file_sync(&path, budget(4)),
            Err(PortableStateError::Budget(_))
        ));
    }

    #[test]
    fn rejects_file_growth_after_metadata_snapshot() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        fs::write(&path, b"1234").unwrap();
        let file = File::open(&path).unwrap();
        let metadata_len = file.metadata().unwrap().len();
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"56")
            .unwrap();

        assert!(matches!(
            read_open_file(&path, file, metadata_len, budget(5)),
            Err(PortableStateError::Budget(_))
        ));
    }

    #[test]
    fn rejects_file_growth_that_remains_within_budget() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        fs::write(&path, b"1234").unwrap();
        let file = File::open(&path).unwrap();
        let metadata_len = file.metadata().unwrap().len();
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"5")
            .unwrap();

        assert!(matches!(
            read_open_file(&path, file, metadata_len, budget(5)),
            Err(PortableStateError::FileChangedDuringRead(_))
        ));
    }

    #[test]
    fn rejects_invalid_utf8() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        fs::write(&path, [0xff, 0xfe]).unwrap();
        assert!(matches!(
            read_skillport_state_file_sync(&path, budget(1024)),
            Err(PortableStateError::InvalidUtf8 { .. })
        ));
    }

    #[test]
    fn rejects_invalid_json_before_creating_destination() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        assert!(matches!(
            write_skillport_state_file_sync(&path, "{bad json", budget(1024)),
            Err(PortableStateError::InvalidManifestJson(_))
        ));
        assert!(!path.exists());
    }

    #[test]
    fn atomically_overwrites_existing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        fs::write(&path, "old").unwrap();
        write_skillport_state_file_sync(&path, VALID_JSON, budget(1024)).unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), VALID_JSON);
    }

    #[test]
    fn persist_failure_cleans_up_temporary_file() {
        let dir = TempDir::new().unwrap();
        let destination = dir.path().join("state.json");
        fs::create_dir(&destination).unwrap();

        assert!(write_skillport_state_file_sync(&destination, VALID_JSON, budget(1024)).is_err());
        let leftovers = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(TEMP_FILE_PREFIX)
            })
            .count();
        assert_eq!(leftovers, 0);
        assert!(destination.is_dir());
    }
}
