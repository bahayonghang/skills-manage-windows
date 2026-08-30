//! Operation identity, safe outcomes and best-effort audit lifecycle.

use std::future::Future;
use std::time::Instant;

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::db::{self, DbPool, NewOperationLogEntry};
use crate::ipc_error::{IpcError, IpcResult};
use crate::AppState;

use super::policy::{
    CommandLogPolicy, CommandPolicyEntry, OperationDefinition, OperationLifecycle, OperationPhase,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OperationId(String);

impl OperationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn parse(value: &str) -> Option<Self> {
        Uuid::parse_str(value)
            .ok()
            .map(|value| Self(value.to_string()))
    }
}

impl Default for OperationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for OperationId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// UUID grouping multiple audit attempts without conflating the group with an
/// individual Operation Log row id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OperationBatchId(String);

impl OperationBatchId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn parse(value: &str) -> Option<Self> {
        Uuid::parse_str(value)
            .ok()
            .map(|value| Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for OperationBatchId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationTargetKind {
    Local,
    Ssh,
    Wsl,
}

impl OperationTargetKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Ssh => "ssh",
            Self::Wsl => "wsl",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeIdentifier(String);

impl SafeIdentifier {
    pub fn new(value: impl AsRef<str>) -> Self {
        let value = value.as_ref();
        let valid = !value.is_empty()
            && value.len() <= 160
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':')
            });
        if valid {
            Self(value.to_string())
        } else {
            Self("unknown".to_string())
        }
    }

    fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationTarget {
    kind: OperationTargetKind,
    id: SafeIdentifier,
}

impl OperationTarget {
    pub fn new(kind: OperationTargetKind, id: impl AsRef<str>) -> Self {
        let id = id.as_ref();
        let has_expected_identity = match kind {
            OperationTargetKind::Local => id == crate::targets::LOCAL_TARGET_ID,
            OperationTargetKind::Ssh => id.starts_with("ssh-"),
            OperationTargetKind::Wsl => id.starts_with("wsl-"),
        };
        Self {
            kind,
            id: if has_expected_identity {
                SafeIdentifier::new(id)
            } else {
                SafeIdentifier::new("")
            },
        }
    }

    pub fn local() -> Self {
        Self::new(OperationTargetKind::Local, crate::targets::LOCAL_TARGET_ID)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationSubjectKind {
    Target,
    Skill,
    Agent,
    Repository,
    Tag,
    Collection,
    SavedView,
    TagGroup,
    Project,
    Registry,
    Vault,
    Operation,
}

impl OperationSubjectKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Target => "target",
            Self::Skill => "skill",
            Self::Agent => "agent",
            Self::Repository => "repository",
            Self::Tag => "tag",
            Self::Collection => "collection",
            Self::SavedView => "saved_view",
            Self::TagGroup => "tag_group",
            Self::Project => "project",
            Self::Registry => "registry",
            Self::Vault => "vault",
            Self::Operation => "operation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OperationSubject {
    kind: OperationSubjectKind,
    id: SafeIdentifier,
}

/// Safe identity and grouping facts known before an operation starts. Keeping
/// these outside the terminal result ensures interrupted rows remain useful
/// without admitting labels, paths, hosts or arbitrary JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationContext {
    target: OperationTarget,
    subject: Option<OperationSubject>,
    batch_id: Option<OperationBatchId>,
}

impl OperationContext {
    pub fn new(target: OperationTarget) -> Self {
        Self {
            target,
            subject: None,
            batch_id: None,
        }
    }

    pub fn subject(mut self, kind: OperationSubjectKind, identifier: SafeIdentifier) -> Self {
        self.subject = Some(OperationSubject {
            kind,
            id: identifier,
        });
        self
    }

    pub fn batch(mut self, batch_id: OperationBatchId) -> Self {
        self.batch_id = Some(batch_id);
        self
    }
}

impl From<OperationTarget> for OperationContext {
    fn from(target: OperationTarget) -> Self {
        Self::new(target)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationStatus {
    Succeeded,
    Partial,
    Cancelled,
}

impl OperationStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Partial => "partial",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafeDetailKey {
    AffectedCount,
    RequestedCount,
    SucceededCount,
    FailedCount,
    SkippedCount,
    Changed,
    Mode,
    Scope,
    Identifier,
}

impl SafeDetailKey {
    const fn as_str(self) -> &'static str {
        match self {
            Self::AffectedCount => "affectedCount",
            Self::RequestedCount => "requestedCount",
            Self::SucceededCount => "succeededCount",
            Self::FailedCount => "failedCount",
            Self::SkippedCount => "skippedCount",
            Self::Changed => "changed",
            Self::Mode => "mode",
            Self::Scope => "scope",
            Self::Identifier => "identifier",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SafeDetailValue {
    Count(u64),
    Bool(bool),
    Static(&'static str),
    Identifier(SafeIdentifier),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeOperationResult {
    status: OperationStatus,
    summary: &'static str,
    details: Vec<(SafeDetailKey, SafeDetailValue)>,
}

impl SafeOperationResult {
    pub fn succeeded(summary: &'static str) -> Self {
        Self::new(OperationStatus::Succeeded, summary)
    }

    pub fn partial(summary: &'static str) -> Self {
        Self::new(OperationStatus::Partial, summary)
    }

    pub fn cancelled(summary: &'static str) -> Self {
        Self::new(OperationStatus::Cancelled, summary)
    }

    fn new(status: OperationStatus, summary: &'static str) -> Self {
        Self {
            status,
            summary,
            details: Vec::new(),
        }
    }

    pub fn count(mut self, key: SafeDetailKey, value: u64) -> Self {
        self.details.push((key, SafeDetailValue::Count(value)));
        self
    }

    pub fn flag(mut self, key: SafeDetailKey, value: bool) -> Self {
        self.details.push((key, SafeDetailValue::Bool(value)));
        self
    }

    pub fn stable(mut self, key: SafeDetailKey, value: &'static str) -> Self {
        self.details.push((key, SafeDetailValue::Static(value)));
        self
    }

    pub fn identifier(mut self, key: SafeDetailKey, value: SafeIdentifier) -> Self {
        self.details.push((key, SafeDetailValue::Identifier(value)));
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewedDiagnostic {
    code: &'static str,
    category: &'static str,
    phase: OperationPhase,
    public_message: &'static str,
    retryable: bool,
}

impl ReviewedDiagnostic {
    pub const fn new(
        code: &'static str,
        category: &'static str,
        phase: OperationPhase,
        public_message: &'static str,
        retryable: bool,
    ) -> Self {
        Self {
            code,
            category,
            phase,
            public_message,
            retryable,
        }
    }

    pub const fn unexpected(definition: OperationDefinition) -> Self {
        Self::new(
            "internal.unexpected",
            definition.category().as_str(),
            definition.default_phase(),
            "The operation failed. See runtime logs for details.",
            false,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewedFailure {
    diagnostic: ReviewedDiagnostic,
}

impl ReviewedFailure {
    pub const fn new(diagnostic: ReviewedDiagnostic) -> Self {
        Self { diagnostic }
    }
}

/// Execute one registered auditable operation and preserve its business value.
/// Failure construction accepts only reviewed static diagnostics, never a raw
/// source error or `Display` string.
pub async fn run_operation<R, F, Fut, BuildSuccess, Context>(
    state: &AppState,
    definition: OperationDefinition,
    context: Context,
    build_success: BuildSuccess,
    operation: F,
) -> IpcResult<R>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<R, ReviewedFailure>>,
    BuildSuccess: FnOnce(&R) -> SafeOperationResult,
    Context: Into<OperationContext>,
{
    let context = context.into();
    let operation_id = OperationId::new();
    if definition.lifecycle() == OperationLifecycle::StartedThenTerminal {
        record_started_best_effort(&state.db, &operation_id, definition, &context).await;
    }

    let started_at = Instant::now();
    match operation().await {
        Ok(value) => {
            let result = build_success(&value);
            let entry = success_entry(
                &operation_id,
                definition,
                &context,
                result,
                elapsed_ms(started_at),
            );
            record_final_best_effort(&state.db, &operation_id, definition.lifecycle(), entry).await;
            Ok(value)
        }
        Err(failure) => {
            let diagnostic = failure.diagnostic;
            let entry = failure_entry(
                &operation_id,
                definition,
                &context,
                diagnostic,
                elapsed_ms(started_at),
            );
            record_final_best_effort(&state.db, &operation_id, definition.lifecycle(), entry).await;
            Err(IpcError::new(
                diagnostic.code,
                diagnostic.public_message,
                diagnostic.retryable,
            )
            .with_correlation_id(operation_id.as_str()))
        }
    }
}

/// Record an already-computed safe terminal result under a fresh operation id.
pub async fn record_terminal(
    pool: &DbPool,
    definition: OperationDefinition,
    context: impl Into<OperationContext>,
    result: SafeOperationResult,
) -> OperationId {
    let context = context.into();
    let operation_id = OperationId::new();
    let entry = success_entry(&operation_id, definition, &context, result, 0);
    record_final_best_effort(pool, &operation_id, OperationLifecycle::TerminalOnly, entry).await;
    operation_id
}

/// Typed backend Runtime view for a command rejection. Operation commands can
/// supply their pre-generated id; runtime-only commands receive a fresh id.
/// Excluded policies remain silent so self-logging cannot recurse.
#[derive(Debug, Clone)]
pub struct RuntimeFailureContext {
    entry: CommandPolicyEntry,
    operation_id: Option<OperationId>,
    target_kind: Option<OperationTargetKind>,
    duration_ms: u64,
}

impl RuntimeFailureContext {
    pub fn new(entry: impl Into<CommandPolicyEntry>) -> Self {
        Self {
            entry: entry.into(),
            operation_id: None,
            target_kind: None,
            duration_ms: 0,
        }
    }

    pub fn operation_id(mut self, operation_id: &OperationId) -> Self {
        self.operation_id = Some(operation_id.clone());
        self
    }

    pub fn target_kind(mut self, target_kind: OperationTargetKind) -> Self {
        self.target_kind = Some(target_kind);
        self
    }

    pub fn duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Record a safe backend failure view. The runtime-boundary child adds the
/// universal command adapter; this core helper deliberately logs no arguments,
/// source errors, paths, host values or output.
pub fn record_runtime_failure(context: RuntimeFailureContext, mut error: IpcError) -> IpcError {
    let (category, phase) = match context.entry.policy {
        CommandLogPolicy::Operation(definition) => (
            definition.category().as_str(),
            definition.default_phase().as_str(),
        ),
        CommandLogPolicy::RuntimeOnly(_) => ("runtime", OperationPhase::Command.as_str()),
        CommandLogPolicy::Excluded(_) => return error,
    };
    let operation_id = context
        .operation_id
        .or_else(|| error.correlation_id.as_deref().and_then(OperationId::parse))
        .unwrap_or_default();
    error = error.with_correlation_id(operation_id.as_str());
    let target_kind = context
        .target_kind
        .map(OperationTargetKind::as_str)
        .unwrap_or("unknown");
    tracing::error!(
        target: "skillport::ipc",
        source = "backend",
        event_source = "backend",
        command = context.entry.command,
        category,
        phase,
        code = error.safe_code(),
        retryable = error.retryable,
        target_kind,
        duration_ms = context.duration_ms,
        operation_id = %operation_id,
        "IPC operation failed"
    );
    error
}

/// Startup audit sweep. Failure remains best-effort and emits only a static
/// warning; recovery-journal rows and business state are untouched.
pub async fn mark_interrupted_operations_best_effort(pool: &DbPool) {
    if db::mark_started_operation_logs_interrupted(pool)
        .await
        .is_err()
    {
        tracing::warn!(
            phase = "startup",
            "Could not mark interrupted operation logs"
        );
    }
}

fn elapsed_ms(started_at: Instant) -> i64 {
    i64::try_from(started_at.elapsed().as_millis()).unwrap_or(i64::MAX)
}

fn level_for_status(status: &str) -> &'static str {
    match status {
        "failed" => "error",
        "partial" | "cancelled" | "interrupted" => "warn",
        _ => "info",
    }
}

fn base_details(operation_id: &OperationId, definition: OperationDefinition) -> Map<String, Value> {
    let mut details = Map::new();
    details.insert(
        "operationId".to_string(),
        Value::String(operation_id.to_string()),
    );
    details.insert(
        "phase".to_string(),
        Value::String(definition.default_phase().as_str().to_string()),
    );
    details
}

fn safe_result_details(
    operation_id: &OperationId,
    definition: OperationDefinition,
    fields: Vec<(SafeDetailKey, SafeDetailValue)>,
) -> String {
    let mut details = base_details(operation_id, definition);
    for (key, value) in fields {
        let value = match value {
            SafeDetailValue::Count(value) => Value::from(value),
            SafeDetailValue::Bool(value) => Value::from(value),
            SafeDetailValue::Static(value) => Value::String(value.to_string()),
            SafeDetailValue::Identifier(value) => Value::String(value.into_string()),
        };
        details.insert(key.as_str().to_string(), value);
    }
    crate::redaction::redact_operation_details(Value::Object(details)).to_string()
}

fn diagnostic_details(
    operation_id: &OperationId,
    definition: OperationDefinition,
    diagnostic: ReviewedDiagnostic,
) -> String {
    let mut details = base_details(operation_id, definition);
    details.insert(
        "errorCode".to_string(),
        Value::String(diagnostic.code.to_string()),
    );
    details.insert(
        "errorCategory".to_string(),
        Value::String(diagnostic.category.to_string()),
    );
    details.insert(
        "phase".to_string(),
        Value::String(diagnostic.phase.as_str().to_string()),
    );
    details.insert("retryable".to_string(), Value::Bool(diagnostic.retryable));
    crate::redaction::redact_operation_details(Value::Object(details)).to_string()
}

fn started_entry(
    operation_id: &OperationId,
    definition: OperationDefinition,
    context: &OperationContext,
) -> NewOperationLogEntry {
    let details = base_details(operation_id, definition);
    let subject_type = context
        .subject
        .as_ref()
        .map(|subject| subject.kind.as_str().to_string());
    let subject_id = context
        .subject
        .as_ref()
        .map(|subject| subject.id.clone().into_string());
    NewOperationLogEntry {
        level: "info".to_string(),
        target_kind: context.target.kind.as_str().to_string(),
        target_id: context.target.id.clone().into_string(),
        target_label: None,
        category: definition.category().as_str().to_string(),
        action: definition.action().as_str().to_string(),
        status: "started".to_string(),
        subject_type,
        subject_id,
        subject_label: None,
        summary: "Operation started.".to_string(),
        error_summary: None,
        details_json: Some(
            crate::redaction::redact_operation_details(Value::Object(details)).to_string(),
        ),
        duration_ms: None,
        batch_id: context
            .batch_id
            .as_ref()
            .map(|batch_id| batch_id.as_str().to_string()),
    }
}

fn success_entry(
    operation_id: &OperationId,
    definition: OperationDefinition,
    context: &OperationContext,
    result: SafeOperationResult,
    duration_ms: i64,
) -> NewOperationLogEntry {
    let status = result.status.as_str();
    let subject_type = context
        .subject
        .as_ref()
        .map(|subject| subject.kind.as_str().to_string());
    let subject_id = context
        .subject
        .as_ref()
        .map(|subject| subject.id.clone().into_string());
    NewOperationLogEntry {
        level: level_for_status(status).to_string(),
        target_kind: context.target.kind.as_str().to_string(),
        target_id: context.target.id.clone().into_string(),
        target_label: None,
        category: definition.category().as_str().to_string(),
        action: definition.action().as_str().to_string(),
        status: status.to_string(),
        subject_type,
        subject_id,
        subject_label: None,
        summary: result.summary.to_string(),
        error_summary: None,
        details_json: Some(safe_result_details(
            operation_id,
            definition,
            result.details,
        )),
        duration_ms: Some(duration_ms.max(0)),
        batch_id: context
            .batch_id
            .as_ref()
            .map(|batch_id| batch_id.as_str().to_string()),
    }
}

fn failure_entry(
    operation_id: &OperationId,
    definition: OperationDefinition,
    context: &OperationContext,
    diagnostic: ReviewedDiagnostic,
    duration_ms: i64,
) -> NewOperationLogEntry {
    let status = if matches!(
        diagnostic.code,
        "operation.cancelled" | "skills_cli.cancelled"
    ) {
        "cancelled"
    } else {
        "failed"
    };
    let subject_type = context
        .subject
        .as_ref()
        .map(|subject| subject.kind.as_str().to_string());
    let subject_id = context
        .subject
        .as_ref()
        .map(|subject| subject.id.clone().into_string());
    NewOperationLogEntry {
        level: level_for_status(status).to_string(),
        target_kind: context.target.kind.as_str().to_string(),
        target_id: context.target.id.clone().into_string(),
        target_label: None,
        category: definition.category().as_str().to_string(),
        action: definition.action().as_str().to_string(),
        status: status.to_string(),
        subject_type,
        subject_id,
        subject_label: None,
        summary: diagnostic.public_message.to_string(),
        error_summary: Some(diagnostic.public_message.to_string()),
        details_json: Some(diagnostic_details(operation_id, definition, diagnostic)),
        duration_ms: Some(duration_ms.max(0)),
        batch_id: context
            .batch_id
            .as_ref()
            .map(|batch_id| batch_id.as_str().to_string()),
    }
}

async fn record_started_best_effort(
    pool: &DbPool,
    operation_id: &OperationId,
    definition: OperationDefinition,
    context: &OperationContext,
) {
    if db::insert_operation_log_with_id(
        pool,
        operation_id.as_str(),
        started_entry(operation_id, definition, context),
    )
    .await
    .is_err()
    {
        tracing::warn!(
            operation_id = %operation_id,
            phase = "started",
            "Could not persist operation lifecycle event"
        );
    }
}

async fn record_final_best_effort(
    pool: &DbPool,
    operation_id: &OperationId,
    lifecycle: OperationLifecycle,
    entry: NewOperationLogEntry,
) {
    let result = match lifecycle {
        OperationLifecycle::TerminalOnly => {
            db::insert_operation_log_with_id(pool, operation_id.as_str(), entry)
                .await
                .map(Some)
        }
        OperationLifecycle::StartedThenTerminal => {
            match db::update_operation_log(pool, operation_id.as_str(), entry.clone()).await {
                Ok(Some(updated)) => Ok(Some(updated)),
                Ok(None) => db::insert_operation_log_with_id(pool, operation_id.as_str(), entry)
                    .await
                    .map(Some),
                Err(error) => Err(error),
            }
        }
    };

    if result.is_err() {
        tracing::warn!(
            operation_id = %operation_id,
            phase = "terminal",
            "Could not persist operation lifecycle event"
        );
    }
}

#[cfg(test)]
mod tests;
