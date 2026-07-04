//! Operation Log Module
//!
//! Deep Module that owns the policy for SkillPort Operation Log entries.
//!
//! Responsibilities:
//!
//! - Build target context from `ActiveTarget`, target summaries, or fallback
//!   to the Local target.
//! - Build [`OperationLogEvent`] records carrying category, action, status,
//!   subject, error summary, sanitized details, duration and batch id.
//! - Redact details JSON through the crate-wide redaction policy seam
//!   (`crate::redaction`) before persistence.
//! - Summarize free-form error text by collapsing whitespace and truncating
//!   to a stable maximum width.
//! - Map status to log level for the persisted entry.
//! - Persist entries through a best-effort recorder that never surfaces
//!   business errors back to the caller.
//!
//! Callers (Tauri commands) only provide the business outcome and necessary
//! context; everything else stays here. `commands/logs.rs` keeps just the
//! IPC surface for list / get / clear / export.

use serde_json::Value;
use std::future::Future;
use std::time::Instant;

use crate::db::{self, DbPool, NewOperationLogEntry};
use crate::targets::{ActiveTarget, LOCAL_TARGET_ID};
use crate::AppState;

/// Maximum number of characters allowed in an error summary stored on the log.
/// Longer strings are truncated and ended with the ellipsis character.
const MAX_ERROR_SUMMARY_CHARS: usize = 500;

/// Target context attached to every Operation Log entry.
///
/// Identifies whether the entry was produced against the Local target or
/// against a specific SSH remote target. Constructed via the `target_context_*`
/// helpers in this module, never assembled ad-hoc by callers.
#[derive(Debug, Clone)]
pub struct OperationLogTargetContext {
    pub kind: String,
    pub id: String,
    pub label: Option<String>,
}

/// Event payload describing a single operation outcome.
///
/// Built with [`OperationLogEvent::new`] plus optional builder methods. The
/// recorder takes ownership and persists it together with the supplied
/// `OperationLogTargetContext`.
#[derive(Debug, Clone)]
pub struct OperationLogEvent {
    pub category: String,
    pub action: String,
    pub status: String,
    pub subject_type: Option<String>,
    pub subject_id: Option<String>,
    pub subject_label: Option<String>,
    pub summary: String,
    pub error_summary: Option<String>,
    pub details_json: Option<String>,
    pub duration_ms: Option<i64>,
    pub batch_id: Option<String>,
}

/// Specification for [`with_operation_log`].
///
/// Keeps command wrappers responsible for domain-specific summaries and
/// subjects while centralizing the repeated run → time → persist template.
type OperationLogSuccessBuilder<'a, R> = Box<dyn FnOnce(&R, i64) -> OperationLogEvent + Send + 'a>;
type OperationLogFailureBuilder<'a, E> = Box<dyn FnOnce(&E, i64) -> OperationLogEvent + Send + 'a>;

pub struct OperationSpec<'a, R, E> {
    target_context: OperationLogTargetContext,
    on_success: OperationLogSuccessBuilder<'a, R>,
    on_failure: OperationLogFailureBuilder<'a, E>,
}

impl<'a, R, E> OperationSpec<'a, R, E> {
    pub fn new<OnSuccess, OnFailure>(
        target_context: OperationLogTargetContext,
        on_success: OnSuccess,
        on_failure: OnFailure,
    ) -> Self
    where
        OnSuccess: FnOnce(&R, i64) -> OperationLogEvent + Send + 'a,
        OnFailure: FnOnce(&E, i64) -> OperationLogEvent + Send + 'a,
    {
        Self {
            target_context,
            on_success: Box::new(on_success),
            on_failure: Box::new(on_failure),
        }
    }
}

impl OperationLogEvent {
    pub fn new(category: &str, action: &str, status: &str, summary: impl Into<String>) -> Self {
        Self {
            category: category.to_string(),
            action: action.to_string(),
            status: status.to_string(),
            subject_type: None,
            subject_id: None,
            subject_label: None,
            summary: summary.into(),
            error_summary: None,
            details_json: None,
            duration_ms: None,
            batch_id: None,
        }
    }

    pub fn subject(mut self, subject_type: &str, subject_id: &str, subject_label: &str) -> Self {
        self.subject_type = Some(subject_type.to_string());
        self.subject_id = Some(subject_id.to_string());
        self.subject_label = Some(subject_label.to_string());
        self
    }

    pub fn error(mut self, error: impl AsRef<str>) -> Self {
        self.error_summary = Some(summarize_error(error.as_ref()));
        self
    }

    pub fn details(mut self, details: Value) -> Self {
        self.details_json = Some(crate::redaction::redact_operation_details(details).to_string());
        self
    }

    pub fn duration_ms(mut self, duration_ms: i64) -> Self {
        self.duration_ms = Some(duration_ms.max(0));
        self
    }

    pub fn batch_id(mut self, batch_id: impl Into<String>) -> Self {
        self.batch_id = Some(batch_id.into());
        self
    }
}

/// Run an async business operation and record one best-effort Operation Log.
///
/// The business `Result` is returned unchanged. Logging failures remain
/// best-effort and are swallowed by [`record_operation_log_best_effort`].
pub async fn with_operation_log<'a, F, Fut, R, E>(
    state: &AppState,
    spec: OperationSpec<'a, R, E>,
    f: F,
) -> Result<R, E>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<R, E>>,
    E: std::fmt::Display,
{
    let started_at = Instant::now();
    let result = f().await;
    let duration_ms = started_at.elapsed().as_millis() as i64;
    let OperationSpec {
        target_context,
        on_success,
        on_failure,
    } = spec;
    match result {
        Ok(value) => {
            record_operation_log_best_effort(
                &state.db,
                target_context,
                on_success(&value, duration_ms),
            )
            .await;
            Ok(value)
        }
        Err(error) => {
            let error_text = error.to_string();
            let event = on_failure(&error, duration_ms).error(error_text);
            record_operation_log_best_effort(&state.db, target_context, event).await;
            Err(error)
        }
    }
}

/// Build a target context from an [`ActiveTarget`].
pub fn target_context_from_active_target(
    active_target: &ActiveTarget,
) -> OperationLogTargetContext {
    match active_target {
        ActiveTarget::Local => local_target_context(),
        ActiveTarget::Ssh(target) => OperationLogTargetContext {
            kind: "ssh".to_string(),
            id: target.id.clone(),
            label: Some(target.label.clone()),
        },
        ActiveTarget::Wsl(target) => OperationLogTargetContext {
            kind: "wsl".to_string(),
            id: target.id.clone(),
            label: Some(target.label.clone()),
        },
    }
}

/// Build a target context from a target summary tuple.
///
/// Used by `commands/targets.rs` where the original `ActiveTarget` has not
/// been materialized yet (for example, on creation or before activation).
pub fn target_context_from_target_summary(
    target_id: &str,
    target_kind: &str,
    target_label: &str,
) -> OperationLogTargetContext {
    OperationLogTargetContext {
        kind: target_kind.to_string(),
        id: target_id.to_string(),
        label: Some(target_label.to_string()),
    }
}

/// Default Local target context. Used as a fallback when the active target
/// cannot be resolved (for example, settings recorded during portable state
/// import where DB access may be transient).
pub fn local_target_context() -> OperationLogTargetContext {
    OperationLogTargetContext {
        kind: "local".to_string(),
        id: LOCAL_TARGET_ID.to_string(),
        label: Some("Local".to_string()),
    }
}

/// Persist an [`OperationLogEvent`] without surfacing failures.
///
/// Operation Log writes are deliberately best-effort: a logging failure must
/// never mask or replace the underlying business outcome. Errors are written
/// to stderr and otherwise swallowed.
pub async fn record_operation_log_best_effort(
    pool: &DbPool,
    target: OperationLogTargetContext,
    event: OperationLogEvent,
) {
    let level = level_for_status(&event.status);
    let entry = NewOperationLogEntry {
        level,
        target_kind: target.kind,
        target_id: target.id,
        target_label: target.label,
        category: event.category,
        action: event.action,
        status: event.status,
        subject_type: event.subject_type,
        subject_id: event.subject_id,
        subject_label: event.subject_label,
        summary: event.summary,
        error_summary: event.error_summary,
        details_json: event.details_json,
        duration_ms: event.duration_ms,
        batch_id: event.batch_id,
    };

    if let Err(error) = db::insert_operation_log(pool, entry).await {
        tracing::warn!(error = %error, "Failed to record operation log");
    }
}

/// Collapse internal whitespace and truncate `error` to a stable summary.
///
/// Trailing ellipsis ('…') is appended only when truncation actually happens.
pub fn summarize_error(error: &str) -> String {
    let normalized = error.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= MAX_ERROR_SUMMARY_CHARS {
        return normalized;
    }

    let mut truncated = normalized
        .chars()
        .take(MAX_ERROR_SUMMARY_CHARS.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

fn level_for_status(status: &str) -> String {
    match status {
        "failed" => "error".to_string(),
        "partial" | "cancelled" => "warn".to_string(),
        _ => "info".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[test]
    fn summarize_error_collapses_whitespace_and_keeps_short_text() {
        let summary = summarize_error("failed\nstep\t1   complete");
        assert_eq!(summary, "failed step 1 complete");
    }

    #[test]
    fn summarize_error_truncates_long_text_with_ellipsis() {
        let long_error = format!("failed\n{}", "x".repeat(700));
        let summary = summarize_error(&long_error);

        let chars: Vec<char> = summary.chars().collect();
        assert_eq!(chars.len(), MAX_ERROR_SUMMARY_CHARS);
        assert!(!summary.contains('\n'));
        assert_eq!(*chars.last().unwrap(), '…');
    }

    #[test]
    fn summarize_error_returns_empty_string_for_blank_input() {
        assert_eq!(summarize_error(""), "");
        assert_eq!(summarize_error("   \n\t  "), "");
    }

    #[test]
    fn level_for_status_maps_known_statuses() {
        assert_eq!(level_for_status("failed"), "error");
        assert_eq!(level_for_status("partial"), "warn");
        assert_eq!(level_for_status("cancelled"), "warn");
        assert_eq!(level_for_status("succeeded"), "info");
        assert_eq!(level_for_status("started"), "info");
        assert_eq!(level_for_status("anything-else"), "info");
    }

    #[test]
    fn local_target_context_uses_local_constants() {
        let ctx = local_target_context();
        assert_eq!(ctx.kind, "local");
        assert_eq!(ctx.id, LOCAL_TARGET_ID);
        assert_eq!(ctx.label.as_deref(), Some("Local"));
    }

    #[test]
    fn target_context_from_active_target_local_matches_local_helper() {
        let from_active = target_context_from_active_target(&ActiveTarget::Local);
        let direct = local_target_context();

        assert_eq!(from_active.kind, direct.kind);
        assert_eq!(from_active.id, direct.id);
        assert_eq!(from_active.label, direct.label);
    }

    #[test]
    fn target_context_from_target_summary_keeps_caller_values() {
        let ctx = target_context_from_target_summary("ssh-1", "ssh", "My Server");
        assert_eq!(ctx.kind, "ssh");
        assert_eq!(ctx.id, "ssh-1");
        assert_eq!(ctx.label.as_deref(), Some("My Server"));
    }

    #[test]
    fn event_builder_chains_optional_fields() {
        let event = OperationLogEvent::new("scan", "scan.all", "succeeded", "Scanned 1 directory")
            .subject("scan_root", "id-1", "/path")
            .error("ignored\nfor success")
            .details(serde_json::json!({ "items": 1, "token": "secret" }))
            .duration_ms(-5)
            .batch_id("batch-1");

        assert_eq!(event.category, "scan");
        assert_eq!(event.action, "scan.all");
        assert_eq!(event.status, "succeeded");
        assert_eq!(event.summary, "Scanned 1 directory");
        assert_eq!(event.subject_type.as_deref(), Some("scan_root"));
        assert_eq!(event.subject_id.as_deref(), Some("id-1"));
        assert_eq!(event.subject_label.as_deref(), Some("/path"));
        assert_eq!(event.error_summary.as_deref(), Some("ignored for success"));
        let details: Value = serde_json::from_str(event.details_json.as_deref().unwrap()).unwrap();
        assert_eq!(details["items"], 1);
        assert_eq!(details["token"], "[redacted]");
        // Negative durations are clamped to zero so timing math stays sane.
        assert_eq!(event.duration_ms, Some(0));
        assert_eq!(event.batch_id.as_deref(), Some("batch-1"));
    }

    #[tokio::test]
    async fn best_effort_logger_does_not_return_business_errors() {
        // Connecting to an empty in-memory pool means the operation_log table
        // does not exist. The recorder must still complete without panicking.
        // （豁免 test_support::mem_pool：本测试需要未建 schema 的裸池。）
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        record_operation_log_best_effort(
            &pool,
            local_target_context(),
            OperationLogEvent::new("test", "test.action", "succeeded", "No schema"),
        )
        .await;
    }

    #[tokio::test]
    async fn with_operation_log_records_success_and_preserves_result() {
        let pool = crate::test_support::mem_pool().await;
        let app_state = test_app_state(pool);

        let result = with_operation_log(
            &app_state,
            OperationSpec::new(
                local_target_context(),
                |value, duration_ms| {
                    OperationLogEvent::new(
                        "test",
                        "test.wrapper",
                        "succeeded",
                        format!("Returned {value}"),
                    )
                    .duration_ms(duration_ms)
                },
                |error: &String, duration_ms| {
                    OperationLogEvent::new("test", "test.wrapper", "failed", "Failed")
                        .error(error)
                        .duration_ms(duration_ms)
                },
            ),
            || async { Ok::<_, String>(7) },
        )
        .await
        .unwrap();

        assert_eq!(result, 7);
        let page =
            crate::db::list_operation_logs(&app_state.db, crate::db::OperationLogFilter::default())
                .await
                .unwrap();
        assert_eq!(page.entries.len(), 1);
        let entry = &page.entries[0];
        assert_eq!(entry.action, "test.wrapper");
        assert_eq!(entry.status, "succeeded");
        assert_eq!(entry.summary, "Returned 7");
        assert!(entry.duration_ms.is_some());
        assert!(entry.error_summary.is_none());
    }

    #[tokio::test]
    async fn with_operation_log_records_failure_and_preserves_error() {
        let pool = crate::test_support::mem_pool().await;
        let app_state = test_app_state(pool);

        let result = with_operation_log(
            &app_state,
            OperationSpec::new(
                local_target_context(),
                |value: &(), duration_ms| {
                    OperationLogEvent::new(
                        "test",
                        "test.wrapper",
                        "succeeded",
                        format!("Returned {value:?}"),
                    )
                    .duration_ms(duration_ms)
                },
                |error: &String, duration_ms| {
                    OperationLogEvent::new("test", "test.wrapper", "failed", "Failed")
                        .error(error)
                        .duration_ms(duration_ms)
                },
            ),
            || async { Err::<(), _>("boom\nboom".to_string()) },
        )
        .await;

        assert_eq!(result, Err("boom\nboom".to_string()));
        let page =
            crate::db::list_operation_logs(&app_state.db, crate::db::OperationLogFilter::default())
                .await
                .unwrap();
        assert_eq!(page.entries.len(), 1);
        let entry = &page.entries[0];
        assert_eq!(entry.action, "test.wrapper");
        assert_eq!(entry.status, "failed");
        assert_eq!(entry.summary, "Failed");
        assert_eq!(entry.error_summary.as_deref(), Some("boom boom"));
        assert!(entry.duration_ms.is_some());
    }

    fn test_app_state(pool: SqlitePool) -> crate::AppState {
        crate::AppState {
            db: pool,
            ai_tag_jobs: crate::AiTagJobRegistry::default(),
            central_update_cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            central_update_snapshots: crate::CentralUpdateSnapshotCache::default(),
            portable_state_cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            secrets: std::sync::Arc::new(crate::secrets::MockSecretStore::default()),
            targets: crate::targets::TargetRegistry::default(),
        }
    }
}
