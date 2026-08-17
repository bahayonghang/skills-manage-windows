use super::*;
use std::collections::BTreeMap;

const MAX_FAILURE_ITEMS: usize = 50;

pub(super) fn apply_operation_spec(
    target_context: OperationLogTargetContext,
    request_details: Value,
) -> OperationSpec<'static, SkillUpdateApplyResult, UpdateCommandError> {
    let failure_details = request_details.clone();
    OperationSpec::new(
        target_context,
        move |result, duration_ms| apply_success_event(result, request_details, duration_ms),
        move |error: &UpdateCommandError, duration_ms| {
            tracing::error!(
                target: "skillport::update_center",
                action = "update_center.apply",
                error_code = error.error_code.unwrap_or("none"),
                error_category = error.category,
                phase = error.phase.unwrap_or("none"),
                duration_ms,
                "Update Center apply failed"
            );
            update_operation_event(
                "update_center.apply",
                "failed",
                "Failed to apply skill update decisions",
                merge_details(failure_details, error.operation_details()),
                duration_ms,
            )
        },
    )
}

const GENERIC_APPLY_ITEM_FAILURE: &str = "This update item could not be applied.";

fn apply_success_event(
    result: &SkillUpdateApplyResult,
    request_details: Value,
    duration_ms: i64,
) -> OperationLogEvent {
    let success_count = apply_success_count(result);
    let failure_count = result.failures.len();
    let status = apply_operation_status(result);
    if failure_count > 0 {
        let diagnostics = apply_failure_diagnostics(result);
        tracing::warn!(
            target: "skillport::update_center",
            action = "update_center.apply",
            status,
            success_count,
            failure_count,
            failure_codes = ?diagnostics.failure_codes,
            failure_categories = ?diagnostics.failure_categories,
            phase_counts = ?diagnostics.phase_counts,
            duration_ms,
            "Update Center apply completed with item failures"
        );
    }
    let summary = match status {
        "failed" => "Skill update decisions failed",
        "partial" => "Skill update decisions partially applied",
        _ => "Applied skill update decisions",
    };
    let event = update_operation_event(
        "update_center.apply",
        status,
        summary,
        merge_details(request_details, apply_result_details(result)),
        duration_ms,
    );
    if failure_count == 0 {
        return event;
    }
    let message = result
        .failures
        .first()
        .and_then(|failure| failure.error_code.as_deref())
        .and_then(crate::ipc_error::public_message_for_code)
        .unwrap_or(GENERIC_APPLY_ITEM_FAILURE);
    event.error(message)
}

fn apply_result_details(result: &SkillUpdateApplyResult) -> Value {
    let diagnostics = apply_failure_diagnostics(result);
    json!({
        "updated": result.updated_skill_ids.len(),
        "keptMissing": result.kept_missing_skill_ids.len(),
        "deleted": result.deleted_skill_ids.len(),
        "imported": result.imported_skill_ids.len(),
        "skippedAdditions": result.skipped_additions.len(),
        "unskippedAdditions": result.unskipped_additions.len(),
        "removedPlatformDuplicates": result.removed_platform_duplicate_paths.len(),
        "removedDeletedCopies": result.removed_deleted_platform_copy_paths.len(),
        "succeeded": apply_success_count(result),
        "failures": result.failures.len(),
        "failureCodes": diagnostics.failure_codes,
        "failureCategories": diagnostics.failure_categories,
        "failureItems": diagnostics.failure_items,
        "failureItemsTruncated": diagnostics.failure_items_truncated,
    })
}

struct ApplyFailureDiagnostics {
    failure_codes: Vec<String>,
    failure_categories: Vec<String>,
    phase_counts: BTreeMap<String, usize>,
    failure_items: Vec<Value>,
    failure_items_truncated: usize,
}

fn apply_failure_diagnostics(result: &SkillUpdateApplyResult) -> ApplyFailureDiagnostics {
    let mut failure_codes = result
        .failures
        .iter()
        .map(|failure| {
            failure
                .error_code
                .clone()
                .unwrap_or_else(|| "central_updates.item_failure".to_string())
        })
        .collect::<Vec<_>>();
    failure_codes.sort_unstable();
    failure_codes.dedup();
    let mut failure_categories = result
        .failures
        .iter()
        .map(|failure| {
            failure
                .error_category
                .clone()
                .unwrap_or_else(|| "central_updates.item_failure".to_string())
        })
        .collect::<Vec<_>>();
    failure_categories.sort_unstable();
    failure_categories.dedup();
    let mut phase_counts = BTreeMap::new();
    let failure_items = result
        .failures
        .iter()
        .take(MAX_FAILURE_ITEMS)
        .map(|failure| {
            let phase = failure.phase.as_deref().unwrap_or("decision_apply");
            *phase_counts.entry(phase.to_string()).or_insert(0) += 1;
            json!({
                "step": failure.step,
                "identifier": failure.identifier,
                "phase": phase,
                "errorCode": failure.error_code.as_deref().unwrap_or("central_updates.item_failure"),
                "errorCategory": failure.error_category.as_deref().unwrap_or("central_updates.item_failure"),
            })
        })
        .collect::<Vec<_>>();
    for failure in result.failures.iter().skip(MAX_FAILURE_ITEMS) {
        let phase = failure.phase.as_deref().unwrap_or("decision_apply");
        *phase_counts.entry(phase.to_string()).or_insert(0) += 1;
    }
    ApplyFailureDiagnostics {
        failure_codes,
        failure_categories,
        phase_counts,
        failure_items,
        failure_items_truncated: result.failures.len().saturating_sub(MAX_FAILURE_ITEMS),
    }
}

fn apply_success_count(result: &SkillUpdateApplyResult) -> usize {
    result.updated_skill_ids.len()
        + result.kept_missing_skill_ids.len()
        + result.deleted_skill_ids.len()
        + result.imported_skill_ids.len()
        + result.skipped_additions.len()
        + result.unskipped_additions.len()
        + result.removed_platform_duplicate_paths.len()
        + result.removed_deleted_platform_copy_paths.len()
}

fn apply_operation_status(result: &SkillUpdateApplyResult) -> &'static str {
    crate::services::installation::batch_operation_status(
        apply_success_count(result),
        0,
        result.failures.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_operation_status_reflects_item_outcomes() {
        let failure = || {
            crate::services::central_updates::inventory::SkillUpdateApplyFailure::new(
                "update", "demo",
            )
        };
        let failed = SkillUpdateApplyResult {
            failures: (0..4).map(|_| failure()).collect(),
            ..SkillUpdateApplyResult::default()
        };
        assert_eq!(apply_operation_status(&failed), "failed");
        assert_eq!(
            apply_result_details(&failed)["failureCodes"],
            json!(["central_updates.update_failed"])
        );
        assert_eq!(
            apply_result_details(&failed)["failureCategories"],
            json!(["central_updates.item_failure"])
        );
        assert_eq!(
            apply_result_details(&failed)["failureItems"][0],
            json!({
                "step": "update",
                "identifier": "demo",
                "phase": "decision_apply",
                "errorCode": "central_updates.update_failed",
                "errorCategory": "central_updates.item_failure",
            })
        );
        assert_eq!(
            apply_result_details(&failed)["failureItemsTruncated"],
            json!(0)
        );
        let mut partial = SkillUpdateApplyResult::default();
        partial.updated_skill_ids.push("demo".to_string());
        partial.failures.push(failure());
        assert_eq!(apply_operation_status(&partial), "partial");
        let mut succeeded = SkillUpdateApplyResult::default();
        succeeded.updated_skill_ids.push("demo".to_string());
        assert_eq!(apply_operation_status(&succeeded), "succeeded");
        let serialized = serde_json::to_string(&failed).unwrap();
        assert!(serialized.contains("This update item could not be applied."));
        assert!(serialized.contains("central_updates.update_failed"));
        assert!(serialized.contains("central_updates.item_failure"));
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("example.invalid"));
        assert!(!serialized.contains("Users/private"));
        let event = apply_success_event(&failed, json!({}), 12);
        assert_eq!(
            event.error_summary.as_deref(),
            Some("This update item could not be applied.")
        );
    }

    #[test]
    fn apply_import_addition_failure_records_github_code_and_public_error_summary() {
        let seeds = "token=secret https://example.invalid C:/Users/private";
        let failed = SkillUpdateApplyResult {
            failures: vec![
                crate::services::central_updates::inventory::SkillUpdateApplyFailure::from_github_import(
                    "github:emilkowalski-skill-main",
                    crate::services::github_import::GithubImportError::AccessDenied(
                        seeds.to_string(),
                    ),
                ),
            ],
            ..SkillUpdateApplyResult::default()
        };

        let details = apply_result_details(&failed);
        assert_eq!(
            details["failureItems"][0],
            json!({
                "step": "import_addition",
                "identifier": "github:emilkowalski-skill-main",
                "phase": "decision_apply",
                "errorCode": "github_import.access_denied",
                "errorCategory": "github_import.access_denied",
            })
        );
        let serialized = serde_json::to_string(&details).unwrap();
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("example.invalid"));
        assert!(!serialized.contains("Users/private"));

        let event = apply_success_event(&failed, json!({}), 12);
        assert_eq!(
            event.error_summary.as_deref(),
            crate::ipc_error::public_message_for_code("github_import.access_denied")
        );
        let summary = event.error_summary.as_deref().unwrap();
        assert!(!summary.contains("secret"));
        assert!(!summary.contains("example.invalid"));
        assert!(!summary.contains("Users/private"));
        assert!(!summary.contains("AccessDenied"));
    }

    #[test]
    fn apply_failure_items_are_bounded_and_keep_result_order() {
        let failed = SkillUpdateApplyResult {
            failures: (0..51)
                .map(|index| {
                    crate::services::central_updates::inventory::SkillUpdateApplyFailure::new(
                        "update",
                        format!("skill-{index:02}"),
                    )
                })
                .collect(),
            ..SkillUpdateApplyResult::default()
        };

        let details = apply_result_details(&failed);
        let items = details["failureItems"].as_array().unwrap();
        assert_eq!(items.len(), 50);
        assert_eq!(items[0]["identifier"], "skill-00");
        assert_eq!(items[49]["identifier"], "skill-49");
        assert_eq!(details["failureItemsTruncated"], 1);
        let serialized = serde_json::to_string(&details).unwrap();
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("example.invalid"));
    }

    #[test]
    fn runtime_failure_diagnostics_are_sorted_and_historical_fields_fall_back() {
        let mut recovery =
            crate::services::central_updates::inventory::SkillUpdateApplyFailure::new(
                "update", "skill-b",
            );
        recovery.phase = Some("recovery".to_string());
        recovery.error_code = Some("central_operation.delete_restore_collision".to_string());
        recovery.error_category = Some("central_updates.central_operation".to_string());
        let historical: crate::services::central_updates::inventory::SkillUpdateApplyFailure =
            serde_json::from_value(json!({
                "step": "delete_missing",
                "identifier": "skill-a",
                "error": "token=secret https://example.invalid C:/Users/private"
            }))
            .unwrap();
        let result = SkillUpdateApplyResult {
            failures: vec![recovery, historical],
            ..SkillUpdateApplyResult::default()
        };

        let diagnostics = apply_failure_diagnostics(&result);
        assert_eq!(
            diagnostics.failure_codes,
            vec![
                "central_operation.delete_restore_collision".to_string(),
                "central_updates.item_failure".to_string(),
            ]
        );
        assert_eq!(
            diagnostics.failure_categories,
            vec![
                "central_updates.central_operation".to_string(),
                "central_updates.item_failure".to_string(),
            ]
        );
        assert_eq!(diagnostics.phase_counts["decision_apply"], 1);
        assert_eq!(diagnostics.phase_counts["recovery"], 1);
        let details = serde_json::to_string(&apply_result_details(&result)).unwrap();
        assert!(!details.contains("secret"));
        assert!(!details.contains("example.invalid"));
        assert!(!details.contains("Users/private"));
    }

    #[test]
    fn failure_items_never_retain_dynamic_identifier_text() {
        let result = SkillUpdateApplyResult {
            failures: vec![
                crate::services::central_updates::inventory::SkillUpdateApplyFailure::new(
                    "update",
                    "token=secret https://example.invalid/C:/Users/private",
                ),
            ],
            ..SkillUpdateApplyResult::default()
        };

        let details = apply_result_details(&result);
        assert_eq!(details["failureItems"][0]["identifier"], "batch");
        let serialized = serde_json::to_string(&details).unwrap();
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("example.invalid"));
        assert!(!serialized.contains("Users/private"));
    }
}
