//! Fail-closed normalization for renderer-originated Runtime diagnostics.

use serde_json::Value;

use super::FrontendRuntimeLogPayload;

const MAX_DETAILS_CHARS: usize = 4_000;
const MAX_DETAIL_DEPTH: usize = 4;
const MAX_ARRAY_ITEMS: usize = 20;
const MAX_OBJECT_FIELDS: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SanitizedFrontendRuntimeLog {
    pub(super) level: String,
    pub(super) source: String,
    pub(super) message: String,
    pub(super) details: String,
    pub(super) operation_id: Option<String>,
}

pub(super) fn sanitize_frontend_runtime_log_payload(
    payload: FrontendRuntimeLogPayload,
) -> SanitizedFrontendRuntimeLog {
    let level = match payload
        .level
        .as_deref()
        .map(str::trim)
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("error") => "error",
        Some("warn") | Some("warning") => "warn",
        Some("debug") => "debug",
        _ => "info",
    }
    .to_string();

    let source = normalize_frontend_source(payload.source.as_deref()).to_string();
    let message = frontend_runtime_message(&source).to_string();
    let details = serde_json::to_string(&crate::redaction::redact_runtime_json(
        sanitize_frontend_details(&source, payload.details),
    ))
    .ok()
    .map(|value| truncate_chars(&value, MAX_DETAILS_CHARS))
    .unwrap_or_else(|| "{}".to_string());
    let operation_id = payload
        .operation_id
        .as_deref()
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
        .map(|value| value.to_string());

    SanitizedFrontendRuntimeLog {
        level,
        source,
        message,
        details: crate::redaction::redact_runtime_line(&details),
        operation_id,
    }
}

fn normalize_frontend_source(source: Option<&str>) -> &'static str {
    match source.map(str::trim) {
        Some("ipc.failure") => "ipc.failure",
        Some("window.error") => "window.error",
        Some("window.unhandledrejection") => "window.unhandledrejection",
        _ => "frontend.runtime",
    }
}

fn frontend_runtime_message(source: &str) -> &'static str {
    match source {
        "ipc.failure" => "IPC command failed",
        "window.error" => "A window error occurred",
        "window.unhandledrejection" => "An unhandled promise rejection occurred",
        _ => "Frontend runtime event",
    }
}

fn sanitize_frontend_details(source: &str, details: Option<Value>) -> Value {
    let details = details.unwrap_or(Value::Null);
    match source {
        "ipc.failure" => sanitize_ipc_failure_details(details),
        "window.error" => sanitize_global_failure_details(details, true),
        "window.unhandledrejection" => sanitize_global_failure_details(details, false),
        _ => sanitize_frontend_value(details, 0),
    }
}

fn sanitize_ipc_failure_details(details: Value) -> Value {
    let object = details.as_object();
    let command = object
        .and_then(|value| value.get("command"))
        .and_then(Value::as_str)
        .and_then(crate::ipc_registry::command_policy)
        .map(|entry| entry.command)
        .unwrap_or("unknown");
    let args = object
        .and_then(|value| value.get("args"))
        .cloned()
        .map(|value| sanitize_frontend_value(value, 0))
        .unwrap_or(Value::Null);
    let error = object
        .and_then(|value| value.get("error"))
        .and_then(Value::as_object);
    let code = error
        .and_then(|value| value.get("code"))
        .and_then(Value::as_str)
        .and_then(reviewed_ipc_code)
        .unwrap_or("internal.unexpected");
    let retryable = error
        .and_then(|value| value.get("retryable"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let correlation_origin = object
        .and_then(|value| value.get("correlationOrigin"))
        .and_then(Value::as_str)
        .filter(|value| matches!(*value, "backend" | "frontend"))
        .unwrap_or("frontend");

    serde_json::json!({
        "command": command,
        "args": args,
        "error": { "code": code, "retryable": retryable },
        "correlationOrigin": correlation_origin,
    })
}

fn reviewed_ipc_code(value: &str) -> Option<&str> {
    (value == "internal.unexpected" || crate::ipc_error::public_message_for_code(value).is_some())
        .then_some(value)
}

fn sanitize_global_failure_details(details: Value, include_position: bool) -> Value {
    let object = details.as_object();
    let error_name = object
        .and_then(|value| value.get("errorName"))
        .and_then(Value::as_str)
        .and_then(reviewed_error_name)
        .unwrap_or("Error");
    let mut safe = serde_json::Map::new();
    safe.insert(
        "errorName".to_string(),
        Value::String(error_name.to_string()),
    );
    if include_position {
        for key in ["line", "column"] {
            if let Some(value) = object
                .and_then(|object| object.get(key))
                .and_then(Value::as_u64)
                .filter(|value| *value <= u64::from(u32::MAX))
            {
                safe.insert(key.to_string(), Value::from(value));
            }
        }
    }
    Value::Object(safe)
}

fn reviewed_error_name(value: &str) -> Option<&'static str> {
    match value {
        "Error" => Some("Error"),
        "TypeError" => Some("TypeError"),
        "RangeError" => Some("RangeError"),
        "ReferenceError" => Some("ReferenceError"),
        "SyntaxError" => Some("SyntaxError"),
        "URIError" => Some("URIError"),
        "EvalError" => Some("EvalError"),
        "AggregateError" => Some("AggregateError"),
        "DOMException" => Some("DOMException"),
        _ => None,
    }
}

fn sanitize_frontend_value(value: Value, depth: usize) -> Value {
    if depth >= MAX_DETAIL_DEPTH {
        return Value::String("[MaxDepth]".to_string());
    }
    match value {
        Value::String(_) => Value::String("[REDACTED]".to_string()),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .take(MAX_ARRAY_ITEMS)
                .map(|value| sanitize_frontend_value(value, depth + 1))
                .collect(),
        ),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .take(MAX_OBJECT_FIELDS)
                .enumerate()
                .map(|(index, (_, value))| {
                    (
                        format!("field_{index}"),
                        sanitize_frontend_value(value, depth + 1),
                    )
                })
                .collect(),
        ),
        scalar => scalar,
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut iter = value.chars();
    let truncated = iter.by_ref().take(max_chars).collect::<String>();
    if iter.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}
