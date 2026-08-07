use super::*;

pub(super) fn apply_operation_spec(
    target_context: OperationLogTargetContext,
    request_details: Value,
) -> OperationSpec<'static, SkillUpdateApplyResult, UpdateCommandError> {
    let failure_details = request_details.clone();
    OperationSpec::new(
        target_context,
        move |result, duration_ms| {
            let success_count = apply_success_count(result);
            let failure_count = result.failures.len();
            let status = apply_operation_status(result);
            if failure_count > 0 {
                tracing::warn!(
                    target: "skillport::update_center",
                    action = "update_center.apply",
                    status,
                    success_count,
                    failure_count,
                    duration_ms,
                    "Update Center apply completed with item failures"
                );
            }
            let summary = match status {
                "failed" => "Skill update decisions failed",
                "partial" => "Skill update decisions partially applied",
                _ => "Applied skill update decisions",
            };
            update_operation_event(
                "update_center.apply",
                status,
                summary,
                merge_details(request_details, apply_result_details(result)),
                duration_ms,
            )
        },
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

fn apply_result_details(result: &SkillUpdateApplyResult) -> Value {
    let mut failure_codes = result
        .failures
        .iter()
        .filter_map(|failure| failure.error_code.as_deref())
        .collect::<Vec<_>>();
    failure_codes.sort_unstable();
    failure_codes.dedup();
    let mut failure_categories = result
        .failures
        .iter()
        .filter_map(|failure| failure.error_category.as_deref())
        .collect::<Vec<_>>();
    failure_categories.sort_unstable();
    failure_categories.dedup();
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
        "failureCodes": failure_codes,
        "failureCategories": failure_categories,
    })
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
                "update",
                "demo",
                "token=secret https://example.invalid C:/Users/private".to_string(),
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
    }
}
