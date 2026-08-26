//! Auditable command policy, operation identity and lifecycle.
//!
//! This is the small public seam between command policy and the legacy
//! Operation Log persistence layer. Callers supply registered definitions and
//! reviewed outcomes; UUID generation, stable diagnostic fields, redaction,
//! timing and best-effort persistence stay private in the module.

mod operation;
mod policy;

pub use operation::{
    mark_interrupted_operations_best_effort, record_runtime_failure, record_terminal,
    run_operation, OperationBatchId, OperationContext, OperationId, OperationStatus,
    OperationSubjectKind, OperationTarget, OperationTargetKind, ReviewedDiagnostic,
    ReviewedFailure, RuntimeFailureContext, SafeDetailKey, SafeIdentifier, SafeOperationResult,
};
pub use policy::{
    CommandLogPolicy, CommandPolicyEntry, ExclusionReason, OperationAction, OperationCategory,
    OperationDefinition, OperationLifecycle, OperationPhase, RuntimeOnlyReason,
};
