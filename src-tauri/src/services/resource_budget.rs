//! Shared filesystem/network resource budgets.
//!
//! These caps are intentionally conservative and centralized so archive,
//! tree, and copy operations fail with bounded, explainable errors instead of
//! consuming unbounded memory or disk.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceBudget {
    pub archive_bytes: u64,
    pub archive_files: usize,
    pub archive_entry_bytes: u64,
    pub file_bytes: u64,
    pub tree_depth: usize,
    pub tree_entries: usize,
    pub copy_bytes: u64,
    pub copy_entries: usize,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            // Repository tarballs are compressed; keep the inbound cap separate
            // from expanded file/content caps.
            archive_bytes: 128 * 1024 * 1024,
            archive_files: 20_000,
            archive_entry_bytes: 8 * 1024 * 1024,
            file_bytes: 1024 * 1024,
            tree_depth: 8,
            tree_entries: 2_048,
            copy_bytes: 256 * 1024 * 1024,
            copy_entries: 20_000,
        }
    }
}

impl ResourceBudget {
    pub fn default_skill() -> Self {
        Self::default()
    }

    pub fn reject_archive_size(self, size: u64) -> Result<(), String> {
        reject_over_limit("GitHub repository archive", size, self.archive_bytes)
    }

    pub fn reject_archive_entry_size(self, path: &str, size: u64) -> Result<(), String> {
        reject_over_limit(
            &format!("GitHub repository archive entry '{path}'"),
            size,
            self.archive_entry_bytes,
        )
    }

    pub fn reject_file_read_size(self, path: &str, size: u64) -> Result<(), String> {
        reject_over_limit(&format!("File '{path}'"), size, self.file_bytes)
    }

    pub fn reject_copy_file_size(self, path: &str, size: u64) -> Result<(), String> {
        reject_over_limit(&format!("Copied file '{path}'"), size, self.copy_bytes)
    }
}

fn reject_over_limit(label: &str, actual: u64, limit: u64) -> Result<(), String> {
    if actual > limit {
        return Err(format!(
            "{label} exceeds the resource budget ({actual} bytes > {limit} bytes)."
        ));
    }
    Ok(())
}
