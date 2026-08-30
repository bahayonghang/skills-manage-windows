//! Typed directory-link primitive for Skills CLI managed placements.
//!
//! Windows creates junctions through the reparse-point API (no `mklink`, no
//! shell, no symlink privilege, no copy fallback). Unix creates directory
//! symlinks. Ordinary directories never enter the remove helper.

use std::io;
use std::path::{Path, PathBuf};

use super::error::InstallationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedDirectoryLinkKind {
    WindowsJunction,
    Symlink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectorySlotObservation {
    Absent,
    Managed { kind: ManagedDirectoryLinkKind },
    OrdinaryDirectory,
    Conflict { reason_code: &'static str },
}

pub(crate) const REASON_WRONG_LINK_TARGET: &str = "wrong_link_target";
pub(crate) const REASON_BROKEN_LINK: &str = "broken_link";
pub(crate) const REASON_NOT_A_DIRECTORY: &str = "not_a_directory";
pub(crate) const REASON_UNREADABLE_ENTRY: &str = "unreadable_entry";

#[cfg(test)]
thread_local! {
    static CREATE_FAULT_AFTER_DIR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn set_create_fault_after_dir(fault: bool) {
    CREATE_FAULT_AFTER_DIR.with(|cell| cell.set(fault));
}

#[cfg(test)]
pub(crate) fn create_fault_after_dir() -> bool {
    CREATE_FAULT_AFTER_DIR.with(|cell| cell.get())
}

pub(crate) fn inspect_managed_directory_link(
    link: &Path,
    expected: &Path,
) -> Result<Option<ManagedDirectoryLinkKind>, InstallationError> {
    match observe_directory_slot(link, expected) {
        DirectorySlotObservation::Managed { kind } => Ok(Some(kind)),
        DirectorySlotObservation::Absent
        | DirectorySlotObservation::OrdinaryDirectory
        | DirectorySlotObservation::Conflict { .. } => Ok(None),
    }
}

pub(crate) fn observe_directory_slot(link: &Path, expected: &Path) -> DirectorySlotObservation {
    let metadata = match std::fs::symlink_metadata(link) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return DirectorySlotObservation::Absent;
        }
        Err(_) => {
            return DirectorySlotObservation::Conflict {
                reason_code: REASON_UNREADABLE_ENTRY,
            };
        }
    };

    if is_reparse_or_symlink(&metadata) {
        return observe_link_entry(link, expected);
    }
    if metadata.is_dir() {
        return DirectorySlotObservation::OrdinaryDirectory;
    }
    DirectorySlotObservation::Conflict {
        reason_code: REASON_NOT_A_DIRECTORY,
    }
}

pub(crate) fn is_reparse_or_symlink(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn observe_link_entry(link: &Path, expected: &Path) -> DirectorySlotObservation {
    match inspect_link_kind_and_target(link) {
        Ok((kind, resolved)) => {
            if crate::paths::paths_equivalent(&resolved, expected) {
                DirectorySlotObservation::Managed { kind }
            } else if resolved.exists() {
                DirectorySlotObservation::Conflict {
                    reason_code: REASON_WRONG_LINK_TARGET,
                }
            } else {
                DirectorySlotObservation::Conflict {
                    reason_code: REASON_BROKEN_LINK,
                }
            }
        }
        Err(_) => DirectorySlotObservation::Conflict {
            reason_code: REASON_UNREADABLE_ENTRY,
        },
    }
}

fn inspect_link_kind_and_target(
    link: &Path,
) -> Result<(ManagedDirectoryLinkKind, PathBuf), InstallationError> {
    #[cfg(windows)]
    {
        windows_reparse::read_directory_link(link)
    }
    #[cfg(unix)]
    {
        let raw =
            std::fs::read_link(link).map_err(InstallationError::ManagedDirectoryLinkInspect)?;
        let resolved = resolve_link_target(link, &raw);
        Ok((ManagedDirectoryLinkKind::Symlink, resolved))
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = link;
        Err(InstallationError::ManagedDirectoryLinkUnsupported)
    }
}

fn resolve_link_target(link: &Path, raw_target: &Path) -> PathBuf {
    if raw_target.is_absolute() {
        raw_target.to_path_buf()
    } else {
        link.parent()
            .unwrap_or_else(|| Path::new(""))
            .join(raw_target)
    }
}

pub(crate) fn create_skills_cli_directory_link(
    target: &Path,
    link: &Path,
) -> Result<ManagedDirectoryLinkKind, InstallationError> {
    match observe_directory_slot(link, target) {
        DirectorySlotObservation::Absent => {}
        DirectorySlotObservation::Managed { kind } => return Ok(kind),
        DirectorySlotObservation::OrdinaryDirectory | DirectorySlotObservation::Conflict { .. } => {
            return Err(InstallationError::ManagedDirectoryLinkCreate(
                io::Error::new(io::ErrorKind::AlreadyExists, "slot occupied"),
            ));
        }
    }

    #[cfg(windows)]
    {
        windows_reparse::create_junction(target, link)
    }
    #[cfg(unix)]
    {
        create_unix_symlink(target, link)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link);
        Err(InstallationError::ManagedDirectoryLinkUnsupported)
    }
}

#[cfg(unix)]
fn create_unix_symlink(
    target: &Path,
    link: &Path,
) -> Result<ManagedDirectoryLinkKind, InstallationError> {
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent).map_err(InstallationError::ManagedDirectoryLinkCreate)?;
    }
    #[cfg(test)]
    if CREATE_FAULT_AFTER_DIR.with(|cell| cell.get()) {
        return Err(InstallationError::ManagedDirectoryLinkCreate(
            io::Error::other("injected create fault"),
        ));
    }
    std::os::unix::fs::symlink(target, link)
        .map_err(InstallationError::ManagedDirectoryLinkCreate)?;
    match inspect_managed_directory_link(link, target)? {
        Some(ManagedDirectoryLinkKind::Symlink) => Ok(ManagedDirectoryLinkKind::Symlink),
        Some(_) | None => {
            let _ = std::fs::remove_file(link);
            Err(InstallationError::ManagedDirectoryLinkTargetMismatch)
        }
    }
}

pub(crate) fn remove_verified_directory_link(
    link: &Path,
    expected: &Path,
) -> Result<(), InstallationError> {
    match observe_directory_slot(link, expected) {
        DirectorySlotObservation::Absent => Ok(()),
        DirectorySlotObservation::Managed { kind } => remove_link_entry(link, kind),
        DirectorySlotObservation::OrdinaryDirectory | DirectorySlotObservation::Conflict { .. } => {
            Err(InstallationError::ManagedDirectoryLinkTargetMismatch)
        }
    }
}

pub(crate) fn slot_is_directory_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|metadata| is_reparse_or_symlink(&metadata))
        .unwrap_or(false)
}

/// Remove a junction/symlink slot without following or comparing the target.
/// Ordinary directories and files are left untouched (`Ok(false)`).
pub(crate) fn remove_directory_link_slot(link: &Path) -> Result<bool, InstallationError> {
    let metadata = match std::fs::symlink_metadata(link) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(InstallationError::ManagedDirectoryLinkRemove(error)),
    };
    if !is_reparse_or_symlink(&metadata) {
        return Ok(false);
    }
    let kind = inspect_link_kind_and_target(link)
        .map(|(kind, _)| kind)
        .unwrap_or(if cfg!(windows) {
            ManagedDirectoryLinkKind::WindowsJunction
        } else {
            ManagedDirectoryLinkKind::Symlink
        });
    remove_link_entry(link, kind)?;
    Ok(true)
}

fn remove_link_entry(link: &Path, kind: ManagedDirectoryLinkKind) -> Result<(), InstallationError> {
    let result = match kind {
        ManagedDirectoryLinkKind::WindowsJunction => std::fs::remove_dir(link),
        ManagedDirectoryLinkKind::Symlink => {
            #[cfg(windows)]
            {
                std::fs::remove_dir(link)
            }
            #[cfg(not(windows))]
            {
                std::fs::remove_file(link)
            }
        }
    };
    result.map_err(InstallationError::ManagedDirectoryLinkRemove)
}

#[cfg(windows)]
mod windows_reparse {
    use super::{resolve_link_target, ManagedDirectoryLinkKind};
    use crate::services::installation::error::InstallationError;
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};
    use std::{io, ptr};

    use windows_sys::Win32::Foundation::{
        CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, MAXIMUM_REPARSE_DATA_BUFFER_SIZE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Ioctl::{FSCTL_GET_REPARSE_POINT, FSCTL_SET_REPARSE_POINT};
    use windows_sys::Win32::System::SystemServices::{
        IO_REPARSE_TAG_MOUNT_POINT, IO_REPARSE_TAG_SYMLINK,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;

    const MOUNT_POINT_HEADER_BYTES: usize = 16;
    const NAME_HEADER_BYTES: usize = 8;

    struct ReparseHandle(HANDLE);

    impl Drop for ReparseHandle {
        fn drop(&mut self) {
            if self.0 != INVALID_HANDLE_VALUE {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    pub(super) fn create_junction(
        target: &Path,
        link: &Path,
    ) -> Result<ManagedDirectoryLinkKind, InstallationError> {
        let canonical_target = target
            .canonicalize()
            .map_err(InstallationError::ManagedDirectoryLinkCreate)?;
        if !canonical_target.is_dir() {
            return Err(InstallationError::ManagedDirectoryLinkCreate(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "junction target is not a directory",
                ),
            ));
        }
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent)
                .map_err(InstallationError::ManagedDirectoryLinkCreate)?;
        }
        std::fs::create_dir(link).map_err(InstallationError::ManagedDirectoryLinkCreate)?;
        let created_dir = true;

        #[cfg(test)]
        if super::CREATE_FAULT_AFTER_DIR.with(|cell| cell.get()) {
            cleanup_created_dir(link, created_dir);
            return Err(InstallationError::ManagedDirectoryLinkCreate(
                io::Error::other("injected create fault"),
            ));
        }

        let buffer = match mount_point_buffer(&canonical_target) {
            Ok(buffer) => buffer,
            Err(error) => {
                cleanup_created_dir(link, created_dir);
                return Err(error);
            }
        };
        if let Err(error) = set_reparse_point(link, &buffer) {
            cleanup_created_dir(link, created_dir);
            return Err(error);
        }
        match read_directory_link(link) {
            Ok((ManagedDirectoryLinkKind::WindowsJunction, resolved))
                if crate::paths::paths_equivalent(&resolved, &canonical_target) =>
            {
                Ok(ManagedDirectoryLinkKind::WindowsJunction)
            }
            Ok(_) | Err(_) => {
                cleanup_created_dir(link, created_dir);
                Err(InstallationError::ManagedDirectoryLinkTargetMismatch)
            }
        }
    }

    fn cleanup_created_dir(link: &Path, created: bool) {
        if created {
            let _ = std::fs::remove_dir(link);
        }
    }

    pub(super) fn read_directory_link(
        link: &Path,
    ) -> Result<(ManagedDirectoryLinkKind, PathBuf), InstallationError> {
        let handle = open_reparse(link, GENERIC_READ)?;
        let mut buffer = vec![0u8; MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize];
        let mut returned = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                handle.0,
                FSCTL_GET_REPARSE_POINT,
                ptr::null(),
                0,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut returned,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(InstallationError::ManagedDirectoryLinkInspect(
                io::Error::last_os_error(),
            ));
        }
        parse_reparse_buffer(link, &buffer[..returned as usize])
    }

    fn parse_reparse_buffer(
        link: &Path,
        buffer: &[u8],
    ) -> Result<(ManagedDirectoryLinkKind, PathBuf), InstallationError> {
        if buffer.len() < MOUNT_POINT_HEADER_BYTES {
            return Err(InstallationError::ManagedDirectoryLinkInspect(
                io::Error::new(io::ErrorKind::InvalidData, "reparse buffer too small"),
            ));
        }
        let tag = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
        let data_len = u16::from_le_bytes(buffer[4..6].try_into().unwrap()) as usize;
        let (kind, name_offset) = match tag {
            IO_REPARSE_TAG_MOUNT_POINT => (ManagedDirectoryLinkKind::WindowsJunction, 8usize),
            IO_REPARSE_TAG_SYMLINK => (ManagedDirectoryLinkKind::Symlink, 12usize),
            _ => {
                return Err(InstallationError::ManagedDirectoryLinkInspect(
                    io::Error::new(io::ErrorKind::InvalidData, "unsupported reparse tag"),
                ));
            }
        };
        let names_start = 8;
        if buffer.len() < names_start + NAME_HEADER_BYTES.max(data_len)
            && buffer.len() < names_start + name_offset + NAME_HEADER_BYTES
        {
            return Err(InstallationError::ManagedDirectoryLinkInspect(
                io::Error::new(io::ErrorKind::InvalidData, "reparse name header truncated"),
            ));
        }
        let sub_offset =
            u16::from_le_bytes(buffer[names_start..names_start + 2].try_into().unwrap()) as usize;
        let sub_len =
            u16::from_le_bytes(buffer[names_start + 2..names_start + 4].try_into().unwrap())
                as usize;
        let path_buffer_start = names_start + name_offset;
        let sub_bytes_start = path_buffer_start + sub_offset;
        let sub_bytes_end = sub_bytes_start.saturating_add(sub_len);
        if sub_bytes_end > buffer.len() {
            return Err(InstallationError::ManagedDirectoryLinkInspect(
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "reparse substitute name out of range",
                ),
            ));
        }
        let (wide_chunks, remainder) = buffer[sub_bytes_start..sub_bytes_end].as_chunks::<2>();
        if !remainder.is_empty() {
            return Err(InstallationError::ManagedDirectoryLinkInspect(
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "reparse substitute name out of range",
                ),
            ));
        }
        let wide: Vec<u16> = wide_chunks
            .iter()
            .map(|chunk| u16::from_le_bytes(*chunk))
            .collect();
        let nt_path = OsString::from_wide(&wide);
        let dos = nt_to_dos_path(&nt_path);
        let resolved = resolve_link_target(link, Path::new(&dos));
        Ok((kind, resolved))
    }

    fn nt_to_dos_path(nt: &OsString) -> PathBuf {
        let value = nt.to_string_lossy();
        if let Some(rest) = value.strip_prefix("\\??\\UNC\\") {
            return PathBuf::from(format!("\\\\{rest}"));
        }
        if let Some(rest) = value.strip_prefix("\\??\\") {
            return PathBuf::from(rest);
        }
        PathBuf::from(value.as_ref())
    }

    fn mount_point_buffer(target: &Path) -> Result<Vec<u8>, InstallationError> {
        let substitute = to_nt_path(target)?;
        let print_name = to_print_name(target);
        let sub_bytes = wide_bytes_without_nul(&substitute);
        let print_bytes = wide_bytes_without_nul(&print_name);
        let path_buffer_len = sub_bytes.len() + 2 + print_bytes.len() + 2;
        let total = MOUNT_POINT_HEADER_BYTES + path_buffer_len;
        if total > MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize {
            return Err(InstallationError::ManagedDirectoryLinkCreate(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "reparse buffer exceeds the Windows limit",
                ),
            ));
        }
        let data_len = (NAME_HEADER_BYTES + path_buffer_len) as u16;
        let mut buffer = vec![0u8; total];
        buffer[0..4].copy_from_slice(&IO_REPARSE_TAG_MOUNT_POINT.to_le_bytes());
        buffer[4..6].copy_from_slice(&data_len.to_le_bytes());
        buffer[6..8].copy_from_slice(&0u16.to_le_bytes());
        buffer[8..10].copy_from_slice(&0u16.to_le_bytes());
        buffer[10..12].copy_from_slice(&(sub_bytes.len() as u16).to_le_bytes());
        buffer[12..14].copy_from_slice(&((sub_bytes.len() + 2) as u16).to_le_bytes());
        buffer[14..16].copy_from_slice(&(print_bytes.len() as u16).to_le_bytes());
        let mut offset = MOUNT_POINT_HEADER_BYTES;
        buffer[offset..offset + sub_bytes.len()].copy_from_slice(&sub_bytes);
        offset += sub_bytes.len();
        buffer[offset..offset + 2].copy_from_slice(&0u16.to_le_bytes());
        offset += 2;
        buffer[offset..offset + print_bytes.len()].copy_from_slice(&print_bytes);
        Ok(buffer)
    }

    fn wide_bytes_without_nul(wide: &[u16]) -> Vec<u8> {
        let end = wide.len().saturating_sub(1);
        wide[..end]
            .iter()
            .flat_map(|unit| unit.to_le_bytes())
            .collect()
    }

    fn to_nt_path(path: &Path) -> Result<Vec<u16>, InstallationError> {
        let canonical = path
            .canonicalize()
            .map_err(InstallationError::ManagedDirectoryLinkCreate)?;
        let display = canonical.to_string_lossy();
        let stripped = display
            .strip_prefix("\\\\?\\")
            .or_else(|| display.strip_prefix("//?/"))
            .unwrap_or(display.as_ref());
        let nt = if let Some(rest) = stripped
            .strip_prefix("UNC\\")
            .or_else(|| stripped.strip_prefix("UNC/"))
        {
            format!("\\??\\UNC\\{}", rest.replace('/', "\\"))
        } else if stripped.starts_with("\\\\") {
            format!("\\??\\UNC\\{}", stripped.trim_start_matches('\\'))
        } else {
            format!("\\??\\{}", stripped.replace('/', "\\"))
        };
        Ok(encode_wide(&nt))
    }

    fn to_print_name(path: &Path) -> Vec<u16> {
        let display = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let value = display.to_string_lossy();
        let stripped = value
            .strip_prefix("\\\\?\\")
            .unwrap_or(value.as_ref())
            .replace('/', "\\");
        encode_wide(&stripped)
    }

    fn encode_wide(value: &str) -> Vec<u16> {
        std::ffi::OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn set_reparse_point(link: &Path, buffer: &[u8]) -> Result<(), InstallationError> {
        let handle = open_reparse(link, GENERIC_READ | GENERIC_WRITE)?;
        let mut returned = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                handle.0,
                FSCTL_SET_REPARSE_POINT,
                buffer.as_ptr().cast(),
                buffer.len() as u32,
                ptr::null_mut(),
                0,
                &mut returned,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(InstallationError::ManagedDirectoryLinkCreate(
                io::Error::last_os_error(),
            ));
        }
        Ok(())
    }

    fn open_reparse(path: &Path, access: u32) -> Result<ReparseHandle, InstallationError> {
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(InstallationError::ManagedDirectoryLinkInspect(
                io::Error::last_os_error(),
            ));
        }
        Ok(ReparseHandle(handle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_marker(dir: &Path, contents: &[u8]) {
        std::fs::write(dir.join("marker.txt"), contents).unwrap();
    }

    #[test]
    fn observes_ordinary_directory_as_direct_copy_slot() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("canonical");
        let copy = temp.path().join("copy");
        std::fs::create_dir_all(&canonical).unwrap();
        std::fs::create_dir_all(&copy).unwrap();
        write_marker(&canonical, b"owned");
        write_marker(&copy, b"copy");
        assert_eq!(
            observe_directory_slot(&copy, &canonical),
            DirectorySlotObservation::OrdinaryDirectory
        );
        assert!(matches!(
            remove_verified_directory_link(&copy, &canonical),
            Err(InstallationError::ManagedDirectoryLinkTargetMismatch)
        ));
        assert_eq!(std::fs::read(copy.join("marker.txt")).unwrap(), b"copy");
    }

    #[test]
    fn observes_file_as_conflict() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("canonical");
        let file = temp.path().join("slot");
        std::fs::create_dir_all(&canonical).unwrap();
        std::fs::write(&file, b"not-a-dir").unwrap();
        assert_eq!(
            observe_directory_slot(&file, &canonical),
            DirectorySlotObservation::Conflict {
                reason_code: REASON_NOT_A_DIRECTORY,
            }
        );
        assert_eq!(std::fs::read(&file).unwrap(), b"not-a-dir");
    }

    #[test]
    fn observes_absent_slot() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("canonical");
        let missing = temp.path().join("missing");
        std::fs::create_dir_all(&canonical).unwrap();
        assert_eq!(
            observe_directory_slot(&missing, &canonical),
            DirectorySlotObservation::Absent
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_symlink_create_inspect_remove() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("canonical");
        let link = temp.path().join("link");
        std::fs::create_dir_all(&canonical).unwrap();
        write_marker(&canonical, b"owned");
        let kind = create_skills_cli_directory_link(&canonical, &link).unwrap();
        assert_eq!(kind, ManagedDirectoryLinkKind::Symlink);
        assert_eq!(
            inspect_managed_directory_link(&link, &canonical).unwrap(),
            Some(ManagedDirectoryLinkKind::Symlink)
        );
        assert_eq!(std::fs::read(link.join("marker.txt")).unwrap(), b"owned");
        remove_verified_directory_link(&link, &canonical).unwrap();
        assert!(!link.exists());
        assert_eq!(
            std::fs::read(canonical.join("marker.txt")).unwrap(),
            b"owned"
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_wrong_symlink_is_conflict_and_is_not_removed() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("canonical");
        let other = temp.path().join("other");
        let link = temp.path().join("link");
        std::fs::create_dir_all(&canonical).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        write_marker(&other, b"other");
        std::os::unix::fs::symlink(&other, &link).unwrap();
        assert_eq!(
            observe_directory_slot(&link, &canonical),
            DirectorySlotObservation::Conflict {
                reason_code: REASON_WRONG_LINK_TARGET,
            }
        );
        assert!(remove_verified_directory_link(&link, &canonical).is_err());
        assert_eq!(std::fs::read(link.join("marker.txt")).unwrap(), b"other");
    }

    #[cfg(windows)]
    #[test]
    fn windows_junction_create_inspect_remove() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("canonical");
        let link = temp.path().join("link");
        std::fs::create_dir_all(&canonical).unwrap();
        write_marker(&canonical, b"owned");
        let kind = create_skills_cli_directory_link(&canonical, &link)
            .expect("Windows junction create must succeed on NTFS without symlink privilege");
        assert_eq!(kind, ManagedDirectoryLinkKind::WindowsJunction);
        assert_eq!(
            inspect_managed_directory_link(&link, &canonical).unwrap(),
            Some(ManagedDirectoryLinkKind::WindowsJunction)
        );
        assert_eq!(std::fs::read(link.join("marker.txt")).unwrap(), b"owned");
        remove_verified_directory_link(&link, &canonical).unwrap();
        assert!(!link.exists());
        assert_eq!(
            std::fs::read(canonical.join("marker.txt")).unwrap(),
            b"owned"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_junction_create_failure_cleans_partial_directory() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("canonical");
        let link = temp.path().join("link");
        std::fs::create_dir_all(&canonical).unwrap();
        set_create_fault_after_dir(true);
        let result = create_skills_cli_directory_link(&canonical, &link);
        set_create_fault_after_dir(false);
        assert!(result.is_err());
        assert!(!link.exists());
    }

    #[cfg(windows)]
    #[test]
    fn windows_wrong_junction_is_conflict_and_is_not_removed() {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("canonical");
        let other = temp.path().join("other");
        let link = temp.path().join("link");
        std::fs::create_dir_all(&canonical).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        write_marker(&other, b"other");
        create_skills_cli_directory_link(&other, &link).unwrap();
        assert_eq!(
            observe_directory_slot(&link, &canonical),
            DirectorySlotObservation::Conflict {
                reason_code: REASON_WRONG_LINK_TARGET,
            }
        );
        assert!(remove_verified_directory_link(&link, &canonical).is_err());
        assert_eq!(std::fs::read(link.join("marker.txt")).unwrap(), b"other");
    }
}
