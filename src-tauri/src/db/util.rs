//! Shared db utilities — Phase 2c.

use chrono::Utc;

/// RFC 3339 timestamp string used as `created_at` / `updated_at` etc. across
/// all repos (collections / repositories / tags / operation_logs / scan_dirs).
pub(crate) fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}
