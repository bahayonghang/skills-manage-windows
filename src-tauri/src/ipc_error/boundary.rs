use std::time::Instant;

use super::{IpcError, IpcResult};

/// Complete a command boundary through the registry-owned logging policy.
/// The command name is a compile-time literal at macro call sites; arguments
/// and source errors never enter the Runtime event.
pub fn complete_named_boundary<T, E>(
    command: &'static str,
    started_at: Instant,
    result: Result<T, E>,
) -> IpcResult<T>
where
    E: Into<IpcError>,
{
    complete_named_boundary_with_target(command, started_at, None, result)
}

/// Complete a named boundary while carrying a reviewed target kind. Target
/// identifiers, hosts and arguments remain outside the diagnostic event.
pub fn complete_named_boundary_with_target<T, E>(
    command: &'static str,
    started_at: Instant,
    target_kind: Option<crate::observability::OperationTargetKind>,
    result: Result<T, E>,
) -> IpcResult<T>
where
    E: Into<IpcError>,
{
    result.map_err(|error| {
        let entry = crate::ipc_registry::command_policy(command)
            .copied()
            .unwrap_or_else(|| {
                debug_assert!(false, "named IPC boundary is not registered: {command}");
                crate::observability::CommandPolicyEntry::runtime_only(
                    command,
                    crate::observability::RuntimeOnlyReason::InternalRefresh,
                )
            });
        let duration_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        let mut context =
            crate::observability::RuntimeFailureContext::new(entry).duration_ms(duration_ms);
        if let Some(target_kind) = target_kind {
            context = context.target_kind(target_kind);
        }
        crate::observability::record_runtime_failure(context, error.into())
    })
}

/// Preserve existing command internals and convert only the final Tauri
/// rejection boundary into [`IpcError`].
#[macro_export]
macro_rules! ipc_boundary {
    ($command:literal, target_kind = $target_kind:expr, $expression:expr $(,)?) => {{
        let started_at = std::time::Instant::now();
        let result = $expression;
        $crate::ipc_error::complete_named_boundary_with_target(
            $command,
            started_at,
            Some($target_kind),
            result,
        )
    }};
    ($command:literal, $expression:expr $(,)?) => {{
        let started_at = std::time::Instant::now();
        let result = $expression;
        $crate::ipc_error::complete_named_boundary($command, started_at, result)
    }};
    ($expression:expr) => {{
        let result: Result<_, String> = $expression;
        result.map_err($crate::ipc_error::IpcError::from)
    }};
}

#[macro_export]
macro_rules! ipc_boundary_async {
    ($command:literal, target_kind = $target_kind:expr, $body:block $(,)?) => {{
        let started_at = std::time::Instant::now();
        let result = (async move $body).await;
        $crate::ipc_error::complete_named_boundary_with_target(
            $command,
            started_at,
            Some($target_kind),
            result,
        )
    }};
    ($command:literal, $body:block $(,)?) => {{
        let started_at = std::time::Instant::now();
        let result = (async move $body).await;
        $crate::ipc_error::complete_named_boundary($command, started_at, result)
    }};
    ($body:block) => {{
        let result: Result<_, String> = (async move $body).await;
        result.map_err($crate::ipc_error::IpcError::from)
    }};
}
