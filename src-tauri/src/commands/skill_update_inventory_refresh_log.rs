//! Refresh/retry progress and bounded repository diagnostics.

use std::sync::Arc;

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::services::central_updates::inventory::{
    safe_logical_identifier, SkillRefreshMode, SkillRefreshScope, SkillRefreshScopeKind,
    SkillUpdateInventory,
};
use crate::services::central_updates::{
    SnapshotProgressEvent, SnapshotProgressReporter, SnapshotProgressStatus,
};

const UPDATE_INVENTORY_PROGRESS_EVENT: &str = "central://skill-update-inventory-progress";
const MAX_REFRESH_FAILURE_ITEMS: usize = 50;
pub(super) const REFRESH_FAILURE_CODE_FALLBACK: &str = "central_updates.repository_check_failed";
pub(super) const REFRESH_FAILURE_CATEGORY_FALLBACK: &str = "central_updates.repository_failure";
pub(super) const REFRESH_RUNTIME_ACTION: &str = "update_center.refresh";
pub(super) const RETRY_RUNTIME_ACTION: &str = "update_center.retry_repositories";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillUpdateInventoryProgressPayload {
    operation_id: String,
    status: &'static str,
    total: usize,
    completed: usize,
    repository_key: Option<String>,
    repository_name: Option<String>,
}

pub(super) struct RefreshFailureDiagnostics {
    pub(super) failure_codes: Vec<String>,
    pub(super) failure_categories: Vec<String>,
    pub(super) failure_items: Vec<Value>,
    pub(super) failure_items_truncated: usize,
    pub(super) retry_attempted: u32,
    pub(super) retry_recovered: u32,
}

pub(super) fn refresh_request_details(scope: &SkillRefreshScope) -> Value {
    let kind = match scope.kind {
        SkillRefreshScopeKind::All => "all",
        SkillRefreshScopeKind::Skills => "skills",
        SkillRefreshScopeKind::Repositories => "repositories",
        SkillRefreshScopeKind::Platform => "platform",
    };
    json!({
        "scopeKind": kind,
        "requestedSkills": scope.skill_ids.as_ref().map_or(0, Vec::len),
        "requestedRepositories": scope.repository_ids.as_ref().map_or(0, Vec::len),
        "requestedAgents": scope.agent_ids.as_ref().map_or(0, Vec::len),
    })
}

pub(super) fn retry_request_details(
    scope: &SkillRefreshScope,
    repository_ids: &[String],
    mode_override: Option<SkillRefreshMode>,
) -> Value {
    let mut details = refresh_request_details(scope);
    details["retriedRepositories"] = json!(repository_ids.len());
    details["modeOverride"] = json!(mode_override.map(|mode| match mode {
        SkillRefreshMode::Regular => "regular",
        SkillRefreshMode::Sync => "sync",
    }));
    details
}

pub(super) fn inventory_progress_reporter(
    app: AppHandle,
    operation_id: String,
) -> SnapshotProgressReporter {
    let progress_app = Arc::new(app);
    Arc::new(move |event: SnapshotProgressEvent| {
        let payload = SkillUpdateInventoryProgressPayload {
            operation_id: operation_id.clone(),
            status: match event.status {
                SnapshotProgressStatus::Started => "started",
                SnapshotProgressStatus::RepositoryStarted => "repository_started",
                SnapshotProgressStatus::RepositoryCompleted => "repository_completed",
                SnapshotProgressStatus::RepositoryFailed => "repository_failed",
                SnapshotProgressStatus::Finalizing => "finalizing",
            },
            total: event.total,
            completed: event.completed,
            repository_key: event.repository_key,
            repository_name: event.repository_name,
        };
        let _ = progress_app.emit(UPDATE_INVENTORY_PROGRESS_EVENT, payload);
    })
}

pub(super) fn refresh_result_details(result: &SkillUpdateInventory) -> Value {
    refresh_result_details_for_action(REFRESH_RUNTIME_ACTION, result)
}

pub(super) fn retry_refresh_result_details(result: &SkillUpdateInventory) -> Value {
    refresh_result_details_for_action(RETRY_RUNTIME_ACTION, result)
}

fn static_diagnostic_label(value: &str) -> Option<&str> {
    let is_safe = !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    is_safe.then_some(value)
}

pub(super) fn refresh_failure_diagnostics(
    result: &SkillUpdateInventory,
) -> RefreshFailureDiagnostics {
    let mut failure_codes = Vec::new();
    let mut failure_categories = Vec::new();
    let mut failure_items = Vec::new();
    for failure in &result.failed_repositories {
        let error_code = failure
            .error_code
            .as_deref()
            .and_then(static_diagnostic_label)
            .unwrap_or(REFRESH_FAILURE_CODE_FALLBACK);
        let error_category = failure
            .diagnostic_category
            .as_deref()
            .and_then(static_diagnostic_label)
            .unwrap_or(REFRESH_FAILURE_CATEGORY_FALLBACK);
        failure_codes.push(error_code.to_string());
        failure_categories.push(error_category.to_string());
        if failure_items.len() < MAX_REFRESH_FAILURE_ITEMS {
            failure_items.push(json!({
                "repositoryId": safe_logical_identifier(failure.repository_id.clone()),
                "errorCode": error_code,
                "errorCategory": error_category,
            }));
        }
    }
    failure_codes.sort_unstable();
    failure_codes.dedup();
    failure_categories.sort_unstable();
    failure_categories.dedup();
    RefreshFailureDiagnostics {
        failure_codes,
        failure_categories,
        failure_items,
        failure_items_truncated: result
            .failed_repositories
            .len()
            .saturating_sub(MAX_REFRESH_FAILURE_ITEMS),
        retry_attempted: result.snapshot_retry_attempted.unwrap_or_default(),
        retry_recovered: result.snapshot_retry_recovered.unwrap_or_default(),
    }
}

fn refresh_result_details_for_action(action: &'static str, result: &SkillUpdateInventory) -> Value {
    let diagnostics = refresh_failure_diagnostics(result);
    if !diagnostics.failure_items.is_empty() || diagnostics.retry_attempted > 0 {
        tracing::warn!(
            target: "skillport::update_center",
            action,
            failure_count = result.failed_repositories.len(),
            failure_codes = ?diagnostics.failure_codes,
            failure_categories = ?diagnostics.failure_categories,
            retry_attempted = diagnostics.retry_attempted,
            retry_recovered = diagnostics.retry_recovered,
            "Update inventory repository acquisition settled with diagnostics"
        );
    }
    json!({
        "updatable": result.updatable.len(),
        "remoteAdded": result.remote_added.len(),
        "remoteMissing": result.remote_missing.len(),
        "failedRepositories": result.failed_repositories.len(),
        "failureCodes": diagnostics.failure_codes,
        "failureCategories": diagnostics.failure_categories,
        "failureItems": diagnostics.failure_items,
        "failureItemsTruncated": diagnostics.failure_items_truncated,
        "snapshotRetryAttempted": diagnostics.retry_attempted,
        "snapshotRetryRecovered": diagnostics.retry_recovered,
    })
}
