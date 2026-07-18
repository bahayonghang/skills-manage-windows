//! Shared filesystem/network resource budgets.
//!
//! These caps are intentionally conservative and centralized so archive,
//! tree, and copy operations fail with bounded, explainable errors instead of
//! consuming unbounded memory or disk.

pub const DEFAULT_ARCHIVE_BYTES: u64 = 128 * 1024 * 1024;
pub const DEFAULT_ARCHIVE_FILES: usize = 20_000;
pub const DEFAULT_ARCHIVE_EXPANDED_BYTES: u64 = 256 * 1024 * 1024;
pub const DEFAULT_ARCHIVE_ENTRY_BYTES: u64 = 32 * 1024 * 1024;
pub const DEFAULT_FILE_BYTES: u64 = 1024 * 1024;
pub const DEFAULT_TREE_DEPTH: usize = 8;
pub const DEFAULT_TREE_ENTRIES: usize = 2_048;
pub const DEFAULT_COPY_BYTES: u64 = 256 * 1024 * 1024;
pub const DEFAULT_COPY_ENTRIES: usize = 20_000;

/// A measured size exceeded one of the configured budget limits.
///
/// Display text intentionally preserves the historical string-error wording;
/// domain error enums wrap this in their `Budget` variant via `map_err`.
#[derive(Debug, thiserror::Error)]
#[error("{label} exceeds the resource budget ({actual} bytes > {limit} bytes).")]
pub struct BudgetExceeded {
    pub(crate) label: String,
    pub(crate) actual: u64,
    pub(crate) limit: u64,
}

impl BudgetExceeded {
    /// Construct a `BudgetExceeded` for callers that need to build the error
    /// directly (e.g. aggregate checks in local archive import).
    pub fn new(label: impl Into<String>, actual: u64, limit: u64) -> Self {
        Self {
            label: label.into(),
            actual,
            limit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceBudget {
    pub archive_bytes: u64,
    pub archive_files: usize,
    pub archive_expanded_bytes: u64,
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
            archive_bytes: DEFAULT_ARCHIVE_BYTES,
            archive_files: DEFAULT_ARCHIVE_FILES,
            archive_expanded_bytes: DEFAULT_ARCHIVE_EXPANDED_BYTES,
            archive_entry_bytes: DEFAULT_ARCHIVE_ENTRY_BYTES,
            file_bytes: DEFAULT_FILE_BYTES,
            tree_depth: DEFAULT_TREE_DEPTH,
            tree_entries: DEFAULT_TREE_ENTRIES,
            copy_bytes: DEFAULT_COPY_BYTES,
            copy_entries: DEFAULT_COPY_ENTRIES,
        }
    }
}

impl ResourceBudget {
    pub fn default_skill() -> Self {
        Self::default()
    }

    pub fn reject_archive_size(self, size: u64) -> Result<(), BudgetExceeded> {
        reject_over_limit("GitHub repository archive", size, self.archive_bytes)
    }

    pub fn reject_archive_expanded_size(self, size: u64) -> Result<(), BudgetExceeded> {
        reject_over_limit(
            "GitHub repository expanded archive contents",
            size,
            self.archive_expanded_bytes,
        )
    }

    pub fn reject_archive_entry_size(self, path: &str, size: u64) -> Result<(), BudgetExceeded> {
        reject_over_limit(
            &format!("GitHub repository archive entry '{path}'"),
            size,
            self.archive_entry_bytes,
        )
    }

    pub fn reject_file_read_size(self, path: &str, size: u64) -> Result<(), BudgetExceeded> {
        reject_over_limit(&format!("File '{path}'"), size, self.file_bytes)
    }

    pub fn reject_copy_file_size(self, path: &str, size: u64) -> Result<(), BudgetExceeded> {
        reject_over_limit(&format!("Copied file '{path}'"), size, self.copy_bytes)
    }
}

fn reject_over_limit(label: &str, actual: u64, limit: u64) -> Result<(), BudgetExceeded> {
    if actual > limit {
        return Err(BudgetExceeded {
            label: label.to_string(),
            actual,
            limit,
        });
    }
    Ok(())
}
