//! ZIP inventory: safe, deterministic entry enumeration.
//!
//! Reads archive bytes once, validates every regular-file entry against the
//! safety matrix (absolute, traversal, Windows drive/UNC, symlink, encrypted,
//! unsupported method, case/prefix collision), enforces the archive budget
//! (archive bytes, entry count, expanded bytes, single-entry bytes, and a
//! compression-ratio guard against zip bombs), and returns a deterministic
//! list of entries normalized to a forward-slash, POSIX-style relative path.
//!
//! Inventory is pure: it never touches Central, staging, or the database.
//! Errors here fail closed and surface as typed `LocalArchiveImportError`.

use std::collections::HashSet;

use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::services::local_archive_import::error::LocalArchiveImportError;
use crate::services::resource_budget::{BudgetExceeded, ResourceBudget};

/// A normalized regular-file entry discovered in a local skill ZIP archive.
///
/// `path` is the raw archive path (still relative, forward-slash); the
/// wrapper-strip transformation happens in `candidate.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipInventoryEntry {
    pub path: String,
    pub byte_len: u64,
    pub compressed_len: u64,
    pub is_skill_md: bool,
}

/// Archive fingerprint returned to the frontend so import can prove the
/// archive on disk is byte-identical to the one the user previewed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveFingerprint {
    /// Lowercase hex SHA-256 of the archive bytes captured during preview.
    pub sha256: String,
    /// Total archive file size (ZIP file byte length on disk).
    pub byte_len: u64,
}

/// Full inventory produced by [`build_inventory`].
#[derive(Debug, Clone)]
pub struct ZipInventory {
    pub entries: Vec<ZipInventoryEntry>,
    pub total_expanded_bytes: u64,
    pub archive_bytes: u64,
    pub fingerprint: ArchiveFingerprint,
}

/// Read archive bytes from disk into memory under the archive-byte budget.
pub(crate) fn read_archive_bytes(
    archive_path: &str,
    budget: ResourceBudget,
) -> Result<Vec<u8>, LocalArchiveImportError> {
    let metadata = std::fs::metadata(archive_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            LocalArchiveImportError::ArchiveNotFound(archive_path.to_string())
        } else {
            LocalArchiveImportError::ArchiveReadFailed(format!("stat: {e}"))
        }
    })?;
    let byte_len = metadata.len();
    budget.reject_archive_size(byte_len).map_err(map_budget)?;

    let bytes = std::fs::read(archive_path)
        .map_err(|e| LocalArchiveImportError::ArchiveReadFailed(format!("read: {e}")))?;
    Ok(bytes)
}

/// Compute the archive fingerprint (SHA-256 + byte length) for a snapshot of
/// archive bytes.
pub(crate) fn fingerprint_of(bytes: &[u8]) -> ArchiveFingerprint {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    ArchiveFingerprint {
        sha256: hex_encode(hasher.finalize().as_slice()),
        byte_len: bytes.len() as u64,
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    crate::hashing::encode_lower_hex(bytes)
}

fn map_budget(error: BudgetExceeded) -> LocalArchiveImportError {
    LocalArchiveImportError::BudgetExceeded(error)
}

/// Conservative maximum compressed-to-expanded ratio. A real skill archive is
/// mostly Markdown and small assets; anything above this ratio is treated as
/// a potential zip bomb and rejected before any staging write.
const MAX_COMPRESSION_RATIO: u64 = 200;

const UNIX_FILE_TYPE_MASK: u32 = 0o170000;
const UNIX_FILE_TYPE_REGULAR: u32 = 0o100000;
const UNIX_FILE_TYPE_DIRECTORY: u32 = 0o040000;
const UNIX_FILE_TYPE_SYMLINK: u32 = 0o120000;

/// Validate one unambiguous classic EOCD and return its raw entry count.
///
/// `zip` stores entries in an `IndexMap` keyed by raw filename, so duplicate
/// names are collapsed before callers can inspect them. We therefore count the
/// fixed central-directory record boundaries independently and require them to
/// end exactly at the EOCD. This does not interpret entry data or extra fields.
/// ZIP64 is rejected because the local entry budget is below its threshold.
fn validated_classic_entry_count<R: std::io::Read + std::io::Seek>(
    archive_bytes: &[u8],
    zip: &ZipArchive<R>,
) -> Result<usize, LocalArchiveImportError> {
    const EOCD_SIGNATURE: &[u8; 4] = b"PK\x05\x06";
    const ZIP64_LOCATOR_SIGNATURE: &[u8; 4] = b"PK\x06\x07";
    const CENTRAL_SIGNATURE: &[u8; 4] = b"PK\x01\x02";
    const EOCD_FIXED_LEN: usize = 22;
    const CENTRAL_FIXED_LEN: usize = 46;
    const MAX_COMMENT_LEN: usize = u16::MAX as usize;

    let read_u16 = |bytes: &[u8], offset| u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    let read_u32 = |bytes: &[u8], offset| {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    };
    let search_start = archive_bytes
        .len()
        .saturating_sub(EOCD_FIXED_LEN + MAX_COMMENT_LEN);
    let search_end = archive_bytes.len().saturating_sub(EOCD_FIXED_LEN);
    let mut valid_footer = None;

    for eocd_offset in (search_start..=search_end).rev() {
        let Some(eocd) = archive_bytes.get(eocd_offset..eocd_offset + EOCD_FIXED_LEN) else {
            continue;
        };
        if &eocd[..4] != EOCD_SIGNATURE
            || eocd_offset + EOCD_FIXED_LEN + read_u16(eocd, 20) as usize != archive_bytes.len()
        {
            continue;
        }

        let count_on_disk = read_u16(eocd, 8);
        let total_count = read_u16(eocd, 10);
        let central_size = read_u32(eocd, 12);
        let central_offset = read_u32(eocd, 16);
        let has_zip64_locator = eocd_offset >= 20
            && &archive_bytes[eocd_offset - 20..eocd_offset - 16] == ZIP64_LOCATOR_SIGNATURE;
        if has_zip64_locator
            || count_on_disk == u16::MAX
            || total_count == u16::MAX
            || central_size == u32::MAX
            || central_offset == u32::MAX
        {
            return Err(LocalArchiveImportError::UnsupportedArchiveEntry {
                path: "archive".to_string(),
                reason: "ZIP64 archives are not supported".to_string(),
            });
        }
        if read_u16(eocd, 4) != 0 || read_u16(eocd, 6) != 0 || count_on_disk != total_count {
            return Err(LocalArchiveImportError::UnsupportedArchiveEntry {
                path: "archive".to_string(),
                reason: "multi-disk archives are not supported".to_string(),
            });
        }

        let Some(central_start) = eocd_offset.checked_sub(central_size as usize) else {
            continue;
        };
        let Some(archive_offset) = central_start.checked_sub(central_offset as usize) else {
            continue;
        };
        let mut cursor = central_start;
        let mut raw_count = 0_usize;
        while cursor < eocd_offset {
            let Some(header) = archive_bytes.get(cursor..cursor + CENTRAL_FIXED_LEN) else {
                break;
            };
            if &header[..4] != CENTRAL_SIGNATURE {
                break;
            }
            let variable_len = read_u16(header, 28) as usize
                + read_u16(header, 30) as usize
                + read_u16(header, 32) as usize;
            let Some(next) = cursor
                .checked_add(CENTRAL_FIXED_LEN)
                .and_then(|value| value.checked_add(variable_len))
            else {
                break;
            };
            if next > eocd_offset {
                break;
            }
            cursor = next;
            raw_count += 1;
        }
        if cursor != eocd_offset || raw_count != total_count as usize {
            continue;
        }
        if valid_footer
            .replace((raw_count, archive_offset, central_start))
            .is_some()
        {
            return Err(LocalArchiveImportError::ArchiveReadFailed(
                "ambiguous ZIP footer".to_string(),
            ));
        }
    }

    let (raw_count, archive_offset, central_start) = valid_footer.ok_or_else(|| {
        LocalArchiveImportError::ArchiveReadFailed("invalid ZIP central directory".to_string())
    })?;
    if zip.offset() != archive_offset as u64
        || zip.central_directory_start() != central_start as u64
    {
        return Err(LocalArchiveImportError::ArchiveReadFailed(
            "ZIP parser metadata mismatch".to_string(),
        ));
    }
    Ok(raw_count)
}

/// Build the inventory of validated regular-file entries from raw archive
/// bytes. Performs all safety, budget, and structure checks.
pub(crate) fn build_inventory(
    archive_bytes: &[u8],
    budget: ResourceBudget,
) -> Result<ZipInventory, LocalArchiveImportError> {
    let cursor = std::io::Cursor::new(archive_bytes);
    let mut zip = ZipArchive::new(cursor).map_err(|error| match error {
        zip::result::ZipError::UnsupportedArchive(reason) => {
            LocalArchiveImportError::UnsupportedArchiveEntry {
                path: "archive".to_string(),
                reason: reason.to_string(),
            }
        }
        other => LocalArchiveImportError::ArchiveReadFailed(format!("open zip: {other}")),
    })?;

    let total_entries = zip.len();
    if validated_classic_entry_count(archive_bytes, &zip)? != total_entries {
        return Err(LocalArchiveImportError::PathConflict(
            "duplicate archive entry names".to_string(),
        ));
    }
    if total_entries > budget.archive_files {
        return Err(LocalArchiveImportError::BudgetExceeded(
            BudgetExceeded::new(
                "ZIP archive entries",
                total_entries as u64,
                budget.archive_files as u64,
            ),
        ));
    }

    let archive_byte_len = archive_bytes.len() as u64;
    budget
        .reject_archive_size(archive_byte_len)
        .map_err(map_budget)?;

    let mut entries: Vec<ZipInventoryEntry> = Vec::with_capacity(total_entries);
    let mut total_expanded_bytes: u64 = 0;
    let mut total_compressed: u64 = 0;
    let mut seen_paths: HashSet<String> = HashSet::new();
    let mut seen_paths_lower: HashSet<String> = HashSet::new();
    let mut seen_dirs: HashSet<String> = HashSet::new();
    let mut seen_dirs_lower: HashSet<String> = HashSet::new();

    for index in 0..total_entries {
        let entry = zip.by_index_raw(index).map_err(|error| match error {
            zip::result::ZipError::UnsupportedArchive(reason) => {
                LocalArchiveImportError::UnsupportedArchiveEntry {
                    path: format!("entry #{index}"),
                    reason: reason.to_string(),
                }
            }
            other => LocalArchiveImportError::ArchiveReadFailed(format!("entry {index}: {other}")),
        })?;

        let raw_name = entry.name().to_string();
        let is_dir = entry.is_dir();
        let encrypted = entry.encrypted();
        let compression = entry.compression();
        let is_symlink = entry.is_symlink();
        let unix_mode = entry.unix_mode();

        if encrypted {
            return Err(LocalArchiveImportError::UnsupportedArchiveEntry {
                path: raw_name,
                reason: "encrypted entries are not supported".to_string(),
            });
        }
        if is_symlink {
            return Err(LocalArchiveImportError::UnsupportedArchiveEntry {
                path: raw_name,
                reason: "symlink entries are not supported".to_string(),
            });
        }
        if unix_mode.is_some_and(|mode| {
            !matches!(
                mode & UNIX_FILE_TYPE_MASK,
                0 | UNIX_FILE_TYPE_REGULAR | UNIX_FILE_TYPE_DIRECTORY | UNIX_FILE_TYPE_SYMLINK
            )
        }) {
            return Err(LocalArchiveImportError::UnsupportedArchiveEntry {
                path: raw_name,
                reason: "non-regular entries are not supported".to_string(),
            });
        }
        if !matches!(
            compression,
            zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
        ) {
            return Err(LocalArchiveImportError::UnsupportedArchiveEntry {
                path: raw_name,
                reason: format!("unsupported compression method: {:?}", compression),
            });
        }

        if is_dir {
            let normalized = normalize_dir_path(&raw_name)?;
            if !normalized.is_empty() {
                register_directory(
                    &normalized,
                    &mut seen_dirs,
                    &mut seen_dirs_lower,
                    &mut seen_paths,
                    &mut seen_paths_lower,
                )?;
            }
            continue;
        }

        let normalized = normalize_file_path(&raw_name)?;
        if normalized.is_empty() {
            return Err(LocalArchiveImportError::InvalidArchiveEntry {
                path: raw_name,
                reason: "empty path after normalization".to_string(),
            });
        }

        if !seen_paths.insert(normalized.clone()) {
            return Err(LocalArchiveImportError::PathConflict(format!(
                "duplicate entry path: {normalized}"
            )));
        }
        let lower = normalized.to_ascii_lowercase();
        if !seen_paths_lower.insert(lower) {
            return Err(LocalArchiveImportError::PathConflict(format!(
                "case-colliding entry path: {normalized}"
            )));
        }
        if prefix_collides_with_directory(&normalized, &seen_dirs) {
            return Err(LocalArchiveImportError::PathConflict(format!(
                "file path collides with a directory prefix: {normalized}"
            )));
        }

        let byte_len = entry.size();
        let compressed_len = entry.compressed_size();
        total_compressed = total_compressed.saturating_add(compressed_len);

        budget
            .reject_archive_entry_size(&normalized, byte_len)
            .map_err(map_budget)?;
        budget
            .reject_file_read_size(&normalized, byte_len)
            .map_err(map_budget)?;

        total_expanded_bytes = total_expanded_bytes.saturating_add(byte_len);
        if total_expanded_bytes > budget.archive_expanded_bytes {
            return Err(LocalArchiveImportError::BudgetExceeded(
                BudgetExceeded::new(
                    "ZIP expanded archive contents",
                    total_expanded_bytes,
                    budget.archive_expanded_bytes,
                ),
            ));
        }

        let is_skill_md = normalized.eq_ignore_ascii_case("SKILL.md")
            || normalized.to_ascii_lowercase().ends_with("/skill.md");

        entries.push(ZipInventoryEntry {
            path: normalized,
            byte_len,
            compressed_len,
            is_skill_md,
        });
    }

    // Zip-bomb guard: compare total compressed size to total expanded size.
    // `total_compressed` is the sum of per-entry `compressed_size`; when the
    // archive is stored uncompressed this underestimates the real on-disk
    // archive byte length, which only makes the guard stricter.
    if total_compressed > 0 {
        let ratio = total_expanded_bytes / total_compressed.max(1);
        if ratio > MAX_COMPRESSION_RATIO {
            return Err(LocalArchiveImportError::BudgetExceeded(
                BudgetExceeded::new(
                    "ZIP compression ratio (zip-bomb guard)",
                    ratio,
                    MAX_COMPRESSION_RATIO,
                ),
            ));
        }
    }

    let fingerprint = fingerprint_of(archive_bytes);
    Ok(ZipInventory {
        entries,
        total_expanded_bytes,
        archive_bytes: archive_byte_len,
        fingerprint,
    })
}

/// Normalize and validate a regular-file path from a ZIP entry.
///
/// Rules (fail closed on any violation):
/// - Reject empty paths.
/// - Reject absolute paths (leading `/`, Windows drive `C:\`, UNC `\\?\`).
/// - Reject any `..` segment.
/// - Reject backslashes: ZIP paths are forward-slash by spec. Allowing
///   backslashes would let Windows-style traversal (`..\..`) sneak through
///   case-folding or path normalization on a non-Windows host.
/// - Convert to forward slashes and collapse redundant `.`/empty segments.
fn normalize_file_path(raw: &str) -> Result<String, LocalArchiveImportError> {
    if raw.is_empty() {
        return Ok(String::new());
    }
    if raw.contains('\0') {
        return Err(LocalArchiveImportError::InvalidArchiveEntry {
            path: raw.to_string(),
            reason: "NUL byte in path".to_string(),
        });
    }
    if raw.starts_with('/') {
        return Err(LocalArchiveImportError::InvalidArchiveEntry {
            path: raw.to_string(),
            reason: "absolute path".to_string(),
        });
    }
    if raw.len() >= 2 {
        let bytes = raw.as_bytes();
        if bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
            return Err(LocalArchiveImportError::InvalidArchiveEntry {
                path: raw.to_string(),
                reason: "Windows drive path".to_string(),
            });
        }
    }
    if raw.starts_with("\\\\") {
        return Err(LocalArchiveImportError::InvalidArchiveEntry {
            path: raw.to_string(),
            reason: "UNC path".to_string(),
        });
    }
    if raw.contains('\\') {
        return Err(LocalArchiveImportError::InvalidArchiveEntry {
            path: raw.to_string(),
            reason: "backslash in path".to_string(),
        });
    }
    let mut segments: Vec<&str> = Vec::new();
    for segment in raw.split('/') {
        match segment {
            "" | "." => continue,
            ".." => {
                return Err(LocalArchiveImportError::InvalidArchiveEntry {
                    path: raw.to_string(),
                    reason: "traversal segment '..'".to_string(),
                });
            }
            other => segments.push(other),
        }
    }
    if segments.is_empty() {
        return Ok(String::new());
    }
    Ok(segments.join("/"))
}

/// Normalize a directory path: same rules as [`normalize_file_path`] but the
/// trailing slash is stripped and an empty result means the archive root.
fn normalize_dir_path(raw: &str) -> Result<String, LocalArchiveImportError> {
    let trimmed = raw.trim_end_matches('/');
    normalize_file_path(trimmed)
}

fn register_directory(
    normalized: &str,
    seen_dirs: &mut HashSet<String>,
    seen_dirs_lower: &mut HashSet<String>,
    seen_paths: &mut HashSet<String>,
    seen_paths_lower: &mut HashSet<String>,
) -> Result<(), LocalArchiveImportError> {
    if !seen_dirs.insert(normalized.to_string()) {
        return Err(LocalArchiveImportError::PathConflict(format!(
            "duplicate directory path: {normalized}"
        )));
    }
    let lower = normalized.to_ascii_lowercase();
    if !seen_dirs_lower.insert(lower) {
        return Err(LocalArchiveImportError::PathConflict(format!(
            "case-colliding directory path: {normalized}"
        )));
    }
    // A directory and a file with the same path prefix is a conflict.
    if seen_paths.contains(normalized) {
        return Err(LocalArchiveImportError::PathConflict(format!(
            "directory path collides with a file entry: {normalized}"
        )));
    }
    if seen_paths_lower.contains(&normalized.to_ascii_lowercase()) {
        return Err(LocalArchiveImportError::PathConflict(format!(
            "directory path case-collides with a file entry: {normalized}"
        )));
    }
    Ok(())
}

fn prefix_collides_with_directory(path: &str, seen_dirs: &HashSet<String>) -> bool {
    let mut current = path;
    while let Some(idx) = current.rfind('/') {
        current = &current[..idx];
        if seen_dirs.contains(current) {
            return false; // parent dir is fine
        }
    }
    // The path itself must not be a registered directory.
    seen_dirs.contains(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::ZipWriter;

    fn make_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut buf);
        for (name, content) in files {
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file(name, opts).unwrap();
            writer.write_all(content).unwrap();
        }
        writer.finish().unwrap();
        buf.into_inner()
    }

    fn make_dir_zip(entries: &[&str]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut buf);
        for name in entries {
            writer
                .add_directory::<&str, ()>(*name, Default::default())
                .unwrap();
        }
        writer.finish().unwrap();
        buf.into_inner()
    }

    fn make_zip_with_options(
        name: &str,
        content: &[u8],
        options: zip::write::SimpleFileOptions,
    ) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut buf);
        writer.start_file(name, options).unwrap();
        writer.write_all(content).unwrap();
        writer.finish().unwrap();
        buf.into_inner()
    }

    fn mutate_header_u16(bytes: &mut [u8], signature: [u8; 4], offset: usize, value: u16) {
        let mut index = 0;
        while index + offset + 2 <= bytes.len() {
            if bytes[index..].starts_with(&signature) {
                bytes[index + offset..index + offset + 2].copy_from_slice(&value.to_le_bytes());
            }
            index += 1;
        }
    }

    fn mutate_header_u32(bytes: &mut [u8], signature: [u8; 4], offset: usize, value: u32) {
        let mut index = 0;
        while index + offset + 4 <= bytes.len() {
            if bytes[index..].starts_with(&signature) {
                bytes[index + offset..index + offset + 4].copy_from_slice(&value.to_le_bytes());
            }
            index += 1;
        }
    }

    #[test]
    fn builds_inventory_for_root_skill() {
        let bytes = make_zip(&[("SKILL.md", b"---\nname: my-skill\n---\nbody")]);
        let inv = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap();
        assert_eq!(inv.entries.len(), 1);
        assert!(inv.entries[0].is_skill_md);
        assert_eq!(
            inv.total_expanded_bytes,
            b"---\nname: my-skill\n---\nbody".len() as u64
        );
    }

    #[test]
    fn accepts_explicit_directory_entries() {
        let bytes = make_dir_zip(&["references/"]);
        let inv = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap();
        assert!(inv.entries.is_empty());
    }

    #[test]
    fn rejects_symlink_entry() {
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let mut bytes = make_zip_with_options("linked-skill", b"SKILL.md", options);
        let central_header = [0x50, 0x4b, 0x01, 0x02];
        mutate_header_u16(&mut bytes, central_header, 4, 0x0314);
        mutate_header_u32(&mut bytes, central_header, 38, 0o120777_u32 << 16);
        let err = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "unsupported_archive_entry");
    }

    #[test]
    fn rejects_encrypted_entry() {
        let mut bytes = make_zip(&[("SKILL.md", b"---\nname: x\n---\n")]);
        mutate_header_u16(&mut bytes, [0x50, 0x4b, 0x03, 0x04], 6, 1);
        mutate_header_u16(&mut bytes, [0x50, 0x4b, 0x01, 0x02], 8, 1);
        let err = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "unsupported_archive_entry");
    }

    #[test]
    fn rejects_unsupported_compression_method() {
        let mut bytes = make_zip(&[("SKILL.md", b"---\nname: x\n---\n")]);
        mutate_header_u16(&mut bytes, [0x50, 0x4b, 0x03, 0x04], 8, 99);
        mutate_header_u16(&mut bytes, [0x50, 0x4b, 0x01, 0x02], 10, 99);
        let err = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap_err();
        assert!(
            matches!(
                err.code(),
                "unsupported_archive_entry" | "archive_read_failed"
            ),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_archive_byte_budget() {
        let bytes = make_zip(&[("SKILL.md", b"---\nname: x\n---\n")]);
        let mut budget = ResourceBudget::default_skill();
        budget.archive_bytes = bytes.len() as u64 - 1;
        let err = build_inventory(&bytes, budget).unwrap_err();
        assert_eq!(err.code(), "budget_exceeded");
    }

    #[test]
    fn rejects_archive_file_count_budget() {
        let bytes = make_zip(&[("SKILL.md", b"---\nname: x\n---\n"), ("README.md", b"x")]);
        let mut budget = ResourceBudget::default_skill();
        budget.archive_files = 1;
        let err = build_inventory(&bytes, budget).unwrap_err();
        assert_eq!(err.code(), "budget_exceeded");
    }

    #[test]
    fn rejects_entry_and_file_read_budgets() {
        let bytes = make_zip(&[("SKILL.md", &[b'x'; 64])]);
        let mut entry_budget = ResourceBudget::default_skill();
        entry_budget.archive_entry_bytes = 63;
        assert_eq!(
            build_inventory(&bytes, entry_budget).unwrap_err().code(),
            "budget_exceeded"
        );

        let mut file_budget = ResourceBudget::default_skill();
        file_budget.file_bytes = 63;
        assert_eq!(
            build_inventory(&bytes, file_budget).unwrap_err().code(),
            "budget_exceeded"
        );
    }

    #[test]
    fn rejects_expanded_byte_budget() {
        let bytes = make_zip(&[("SKILL.md", &[b'x'; 64])]);
        let mut budget = ResourceBudget::default_skill();
        budget.archive_expanded_bytes = 63;
        let err = build_inventory(&bytes, budget).unwrap_err();
        assert_eq!(err.code(), "budget_exceeded");
    }

    #[test]
    fn rejects_excessive_compression_ratio() {
        let content = vec![0_u8; 512 * 1024];
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let bytes = make_zip_with_options("SKILL.md", &content, options);
        let err = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "budget_exceeded");
    }

    #[test]
    fn rejects_absolute_path() {
        let bytes = make_zip(&[("/SKILL.md", b"---\nname: x\n---\n")]);
        let err = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "invalid_archive_entry");
    }

    #[test]
    fn rejects_windows_drive_path() {
        let bytes = make_zip(&[("C:/SKILL.md", b"---\nname: x\n---\n")]);
        let err = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "invalid_archive_entry");
    }

    #[test]
    fn rejects_unc_path() {
        let bytes = make_zip(&[("\\\\?\\SKILL.md", b"---\nname: x\n---\n")]);
        let err = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "invalid_archive_entry");
    }

    #[test]
    fn rejects_traversal_segment() {
        let bytes = make_zip(&[("../SKILL.md", b"---\nname: x\n---\n")]);
        let err = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "invalid_archive_entry");
    }

    #[test]
    fn rejects_backslash_path() {
        let bytes = make_zip(&[("dir\\SKILL.md", b"---\nname: x\n---\n")]);
        let err = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "invalid_archive_entry");
    }

    #[test]
    fn rejects_duplicate_case_collision() {
        let bytes = make_zip(&[
            ("SKILL.md", b"---\nname: x\n---\n"),
            ("skill.md", b"---\nname: y\n---\n"),
        ]);
        let err = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "path_conflict");
    }

    #[test]
    fn rejects_file_dir_prefix_collision() {
        // A file entry named "a" collides with directory "a/".
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut buf);
        writer
            .add_directory::<&str, ()>("a/", Default::default())
            .unwrap();
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("a", opts).unwrap();
        writer.write_all(b"---\nname: x\n---\n").unwrap();
        writer.finish().unwrap();
        let bytes = buf.into_inner();
        let err = build_inventory(&bytes, ResourceBudget::default_skill()).unwrap_err();
        assert_eq!(err.code(), "path_conflict");
    }

    #[test]
    fn fingerprint_stable_for_identical_bytes() {
        let bytes = make_zip(&[("SKILL.md", b"---\nname: x\n---\n")]);
        let a = fingerprint_of(&bytes);
        let b = fingerprint_of(&bytes);
        assert_eq!(a, b);
        assert_eq!(a.byte_len, bytes.len() as u64);
    }
}
