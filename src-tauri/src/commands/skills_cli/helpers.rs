//! Shared IPC shells for Skills CLI commands.

use crate::ipc_error::{public_message_for_code, IpcError, REVIEWED_IPC_ERROR_CODES};
use crate::observability::{
    CommandLogPolicy, OperationDefinition, ReviewedDiagnostic, ReviewedFailure,
};
use crate::services::exclusive_job::ExclusiveJobError;
use crate::services::skills_cli::{SkillsCliCapability, SkillsCliError, SkillsCliTransport};
use crate::targets::ActiveTarget;

pub(super) fn to_ipc_error(error: &SkillsCliError) -> IpcError {
    let code = error.ipc_code();
    let message = public_message_for_code(code).unwrap_or(
        // Only the internal family falls through; keep its fixed sentence.
        "The operation failed. See runtime logs for details.",
    );
    IpcError::new(code, message, error.retryable())
}

pub(super) fn require_capability(
    target: &ActiveTarget,
    cap: SkillsCliCapability,
) -> Result<(), IpcError> {
    SkillsCliTransport::ensure_capability_for_target(target, cap)
        .map_err(|error| to_ipc_error(&error))
}

pub(super) async fn open_transport(target: &ActiveTarget) -> Result<SkillsCliTransport, IpcError> {
    SkillsCliTransport::for_target(target)
        .await
        .map_err(|error| to_ipc_error(&error))
}

pub(super) fn job_lease_error(error: ExclusiveJobError) -> IpcError {
    match error {
        ExclusiveJobError::InvalidId => {
            IpcError::new("job.invalid_id", "The job identifier is invalid.", false)
        }
        ExclusiveJobError::Busy { .. } => IpcError::new(
            "skills_cli.busy",
            "Another skill operation is using this target.",
            true,
        ),
        ExclusiveJobError::IdMismatch => IpcError::new(
            "job.id_mismatch",
            "The cancellation request does not match the active job.",
            false,
        ),
        ExclusiveJobError::RegistryUnavailable => IpcError::new(
            "job.registry_unavailable",
            "The job registry is unavailable.",
            false,
        ),
    }
}

pub(super) fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("Skills CLI command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => panic!("Skills CLI mutation must use Operation policy"),
    }
}

pub(super) fn reviewed_failure(
    definition: OperationDefinition,
    error: IpcError,
) -> ReviewedFailure {
    let code = REVIEWED_IPC_ERROR_CODES
        .iter()
        .copied()
        .find(|code| *code == error.safe_code())
        .unwrap_or("internal.unexpected");
    let message = public_message_for_code(code)
        .unwrap_or("The operation failed. See runtime logs for details.");
    ReviewedFailure::new(ReviewedDiagnostic::new(
        code,
        definition.category().as_str(),
        definition.default_phase(),
        message,
        error.retryable,
    ))
}

pub(super) fn skills_cli_failure(
    definition: OperationDefinition,
    error: &SkillsCliError,
) -> ReviewedFailure {
    reviewed_failure(definition, to_ipc_error(error))
}
