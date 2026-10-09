//! Typed IPC shells for the Local rules library.
use crate::ipc_error::{IpcError, IpcResult};
use crate::observability::{
    CommandLogPolicy, OperationContext, OperationTarget, OperationTargetKind, ReviewedDiagnostic,
    ReviewedFailure, SafeDetailKey, SafeOperationResult,
};
use crate::services::rules::*;
use crate::AppState;
use tauri::State;

trait RulesOutcome {
    fn safe_outcome(&self) -> SafeOperationResult {
        SafeOperationResult::succeeded("Rules operation completed.")
            .count(SafeDetailKey::AffectedCount, 1)
    }
}
impl RulesOutcome for RuleDetail {}
impl RulesOutcome for RuleMutationResult {}
impl RulesOutcome for RulesImportResult {
    fn safe_outcome(&self) -> SafeOperationResult {
        let succeeded = self
            .entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.status,
                    RuleImportStatus::Imported | RuleImportStatus::Same
                )
            })
            .count() as u64;
        let failed = self.entries.len() as u64 - succeeded;
        let result = if failed == 0 {
            SafeOperationResult::succeeded("Rules import completed.")
        } else if succeeded == 0 {
            SafeOperationResult::partial("Rules could not be imported.")
        } else {
            SafeOperationResult::partial("Some rules could not be imported.")
        };
        result
            .count(SafeDetailKey::AffectedCount, self.entries.len() as u64)
            .count(SafeDetailKey::SucceededCount, succeeded)
            .count(SafeDetailKey::FailedCount, failed)
    }
}

async fn resolve_local_target(
    state: &AppState,
    target_id: &str,
) -> IpcResult<crate::targets::ActiveTarget> {
    // Reject non-Local requests before resolving a home or opening rule files.
    RulesService::ensure_local(target_id).map_err(|e| e.ipc())?;
    let target = state
        .targets
        .target_by_id(&state.db, target_id)
        .await
        .map_err(IpcError::from_display)?;
    if !matches!(target, crate::targets::ActiveTarget::Local) {
        return Err(RulesError::LocalOnly.ipc());
    }
    Ok(target)
}

async fn execute_read<T, F>(state: &AppState, target_id: &str, task: F) -> IpcResult<T>
where
    T: Send + 'static,
    F: FnOnce(RulesService) -> Result<T, RulesError> + Send + 'static,
{
    resolve_local_target(state, target_id).await?;
    crate::fs_util::run_blocking_fs_with(
        "rules read",
        move || task(RulesService::local()),
        |_, _| RulesError::Io,
    )
    .await
    .map_err(|e| e.ipc())
}

async fn run_operation<T, F>(
    state: &AppState,
    command: &'static str,
    target_id: &str,
    task: F,
) -> IpcResult<T>
where
    T: Send + RulesOutcome + 'static,
    F: FnOnce(RulesService) -> Result<T, RulesError> + Send + 'static,
{
    let target = resolve_local_target(state, target_id).await?;
    let definition = match crate::ipc_registry::command_policy(command)
        .expect("registered rules command")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => unreachable!("rules mutation policy"),
    };
    crate::observability::run_operation(
        state,
        definition,
        OperationContext::new(OperationTarget::local()),
        |result: &T| result.safe_outcome(),
        || async {
            let result = async {
                let _guard = crate::services::central_mutation::acquire_target_mutation_guard(
                    &target,
                    command,
                    crate::services::central_mutation::DEFAULT_CENTRAL_MUTATION_TIMEOUT,
                )
                .await
                .map_err(|error| match error {
                    crate::services::central_mutation::CentralMutationError::Busy { .. }
                    | crate::services::central_mutation::CentralMutationError::Timeout { .. } => {
                        RulesError::LockBusy
                    }
                    crate::services::central_mutation::CentralMutationError::Io {
                        source, ..
                    } => RulesError::from(source),
                    crate::services::central_mutation::CentralMutationError::TaskJoin {
                        ..
                    } => RulesError::Io,
                })?;
                crate::fs_util::run_blocking_fs_with(
                    "rules mutation",
                    move || task(RulesService::local()),
                    |_, _| RulesError::Io,
                )
                .await
            }
            .await;
            result.map_err(|e| {
                ReviewedFailure::new(ReviewedDiagnostic::new(
                    e.code(),
                    definition.category().as_str(),
                    definition.default_phase(),
                    crate::ipc_error::public_message_for_code(e.code())
                        .expect("reviewed rules error"),
                    matches!(e, RulesError::LockBusy),
                ))
            })
        },
    )
    .await
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn list_rules(state: State<'_, AppState>, target_id: String) -> IpcResult<RulesSnapshot> {
    crate::ipc_boundary!(
        "list_rules",
        target_kind = OperationTargetKind::Local,
        execute_read(&state, &target_id, move |service| service.list()).await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn read_rule(
    state: State<'_, AppState>,
    target_id: String,
    name: String,
) -> IpcResult<RuleDetail> {
    crate::ipc_boundary!(
        "read_rule",
        target_kind = OperationTargetKind::Local,
        execute_read(&state, &target_id, move |service| service.read(&name)).await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn preview_rules_import(
    state: State<'_, AppState>,
    target_id: String,
) -> IpcResult<RulesImportPreview> {
    crate::ipc_boundary!(
        "preview_rules_import",
        target_kind = OperationTargetKind::Local,
        execute_read(&state, &target_id, move |service| service.preview_import()).await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn import_existing_rules(
    state: State<'_, AppState>,
    target_id: String,
    entries: Vec<RuleImportEntry>,
) -> IpcResult<RulesImportResult> {
    crate::ipc_boundary!(
        "import_existing_rules",
        target_kind = OperationTargetKind::Local,
        run_operation(
            &state,
            "import_existing_rules",
            &target_id,
            move |service| service.import(&entries)
        )
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn create_rule(
    state: State<'_, AppState>,
    target_id: String,
    name: String,
    description: String,
    body: String,
) -> IpcResult<RuleDetail> {
    crate::ipc_boundary!(
        "create_rule",
        target_kind = OperationTargetKind::Local,
        run_operation(&state, "create_rule", &target_id, move |service| {
            service.create(&name, &description, &body)
        })
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn save_rule(
    state: State<'_, AppState>,
    target_id: String,
    name: String,
    body: String,
    expected_revision: String,
) -> IpcResult<RuleDetail> {
    crate::ipc_boundary!(
        "save_rule",
        target_kind = OperationTargetKind::Local,
        run_operation(&state, "save_rule", &target_id, move |service| {
            service.save(&name, &body, &expected_revision)
        })
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn set_rule_target_enabled(
    state: State<'_, AppState>,
    target_id: String,
    name: String,
    tool: RuleTool,
    enabled: bool,
    expected_destination_fingerprint: Option<String>,
) -> IpcResult<RuleDetail> {
    crate::ipc_boundary!(
        "set_rule_target_enabled",
        target_kind = OperationTargetKind::Local,
        run_operation(
            &state,
            "set_rule_target_enabled",
            &target_id,
            move |service| service.set_enabled(
                &name,
                tool,
                enabled,
                expected_destination_fingerprint.as_deref()
            )
        )
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn delete_rule(
    state: State<'_, AppState>,
    target_id: String,
    name: String,
    expected_revision: String,
) -> IpcResult<RuleMutationResult> {
    crate::ipc_boundary!(
        "delete_rule",
        target_kind = OperationTargetKind::Local,
        run_operation(&state, "delete_rule", &target_id, move |service| {
            service.delete(&name, &expected_revision)
        })
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn recover_rule_operation(
    state: State<'_, AppState>,
    target_id: String,
    operation_id: String,
) -> IpcResult<RuleMutationResult> {
    crate::ipc_boundary!(
        "recover_rule_operation",
        target_kind = OperationTargetKind::Local,
        run_operation(
            &state,
            "recover_rule_operation",
            &target_id,
            move |service| service.recover(&operation_id)
        )
        .await
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_audit_counts_match_complete_partial_and_failed_batches() {
        let cases = [
            (
                vec![RuleImportStatus::Imported, RuleImportStatus::Same],
                SafeOperationResult::succeeded("Rules import completed."),
                2,
                0,
            ),
            (
                vec![RuleImportStatus::Imported, RuleImportStatus::Failed],
                SafeOperationResult::partial("Some rules could not be imported."),
                1,
                1,
            ),
            (
                vec![
                    RuleImportStatus::Conflict,
                    RuleImportStatus::Unsupported,
                    RuleImportStatus::Failed,
                ],
                SafeOperationResult::partial("Rules could not be imported."),
                0,
                3,
            ),
        ];
        for (statuses, expected, succeeded, failed) in cases {
            let result = RulesImportResult {
                operation_id: uuid::Uuid::new_v4().to_string(),
                entries: statuses
                    .into_iter()
                    .map(|status| RuleImportResultEntry {
                        name: "private-rule-name.md".into(),
                        status,
                        error_code: None,
                    })
                    .collect(),
            };
            assert_eq!(
                result.safe_outcome(),
                expected
                    .count(SafeDetailKey::AffectedCount, succeeded + failed)
                    .count(SafeDetailKey::SucceededCount, succeeded)
                    .count(SafeDetailKey::FailedCount, failed)
            );
        }
    }
}
