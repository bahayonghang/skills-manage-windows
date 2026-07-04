//! Redaction Policy Module
//!
//! Deep Module that owns SkillPort's single policy for sensitive-field
//! redaction. The needle table, matching semantics, markers and regex
//! patterns are all implementation details; callers only pick the entry
//! point matching the log layer and payload shape.
//!
//! The two callers of this seam:
//!
//! - Operation Log (`operation_log.rs`) redacts details JSON through
//!   [`redact_operation_details`] before persistence.
//! - Runtime Log (`logging.rs`) redacts text lines on read/export through
//!   [`redact_runtime_line`] and frontend event payloads through
//!   [`redact_runtime_json`] (plus [`redact_runtime_line`] for messages).
//!
//! Matching semantics (single definition point): keys are normalized
//! (lowercase, `-` folded to `_`); long needles match as substrings so
//! camelCase compounds like `accessToken` stay covered; the short needle
//! `pat` requires token boundaries so `path`/`pattern` style keys are not
//! falsely redacted.

use regex::{Captures, Regex};
use serde_json::Value;
use std::sync::OnceLock;

/// Keys containing one of these needles (after normalization) are redacted.
const SENSITIVE_KEY_NEEDLES: &[&str] = &[
    "password",
    "passphrase",
    "token",
    "api_key",
    "apikey",
    "secret",
    "private_key",
    "privatekey",
    "credential",
];

/// Short needles that only match as a whole token (string edges or
/// non-alphanumeric neighbours), keeping `path`/`pattern` style keys safe
/// while still catching `pat` / `github_pat`.
const TOKEN_BOUNDED_NEEDLES: &[&str] = &["pat"];

/// Marker persisted into Operation Log details.
const OPERATION_MARKER: &str = "[redacted]";
/// Marker used across Runtime Log lines and payloads.
const RUNTIME_MARKER: &str = "[REDACTED]";

static REDACTION_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

/// Recursively redact Operation Log details JSON. Marker: `[redacted]`.
pub fn redact_operation_details(value: Value) -> Value {
    redact_value(value, OPERATION_MARKER)
}

/// Recursively redact a Runtime Log JSON payload. Marker: `[REDACTED]`.
pub fn redact_runtime_json(value: Value) -> Value {
    redact_value(value, RUNTIME_MARKER)
}

/// Redact a Runtime Log text line (read / export / frontend messages).
/// Covers JSON-style (`"token":"x"`) and key-value (`token=x`) shapes.
/// Marker: `[REDACTED]`.
pub fn redact_runtime_line(raw: &str) -> String {
    let patterns = REDACTION_PATTERNS.get_or_init(|| {
        vec![
            Regex::new(
                r#"(?ix)(?P<prefix>["']?(?:password|passphrase|token|pat|api[_-]?key|apikey|secret|private[_-]?key|credential)["']?\s*:\s*["'])[^"']+(?P<suffix>["'])"#,
            )
            .expect("valid JSON redaction regex"),
            Regex::new(
                r#"(?ix)(?P<prefix>\b(?:password|passphrase|token|pat|api[_-]?key|apikey|secret|private[_-]?key|credential)\b\s*=\s*["']?)[^"'\s,;})\]]+"#,
            )
            .expect("valid key-value redaction regex"),
        ]
    });

    let mut redacted = patterns[0]
        .replace_all(raw, |captures: &Captures<'_>| {
            format!(
                "{}{RUNTIME_MARKER}{}",
                &captures["prefix"], &captures["suffix"]
            )
        })
        .to_string();
    redacted = patterns[1]
        .replace_all(&redacted, |captures: &Captures<'_>| {
            format!("{}{RUNTIME_MARKER}", &captures["prefix"])
        })
        .to_string();
    redacted
}

fn redact_value(value: Value, marker: &str) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| redact_value(item, marker))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if is_sensitive_key(&key) {
                        (key, Value::String(marker.to_string()))
                    } else {
                        (key, redact_value(value, marker))
                    }
                })
                .collect(),
        ),
        other => other,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_lowercase().replace('-', "_");
    if SENSITIVE_KEY_NEEDLES
        .iter()
        .any(|needle| normalized.contains(needle))
    {
        return true;
    }
    TOKEN_BOUNDED_NEEDLES
        .iter()
        .any(|needle| contains_bounded_token(&normalized, needle))
}

fn contains_bounded_token(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut start = 0;
    while let Some(index) = haystack[start..].find(needle) {
        let begin = start + index;
        let end = begin + needle.len();
        let left_bounded = begin == 0 || !bytes[begin - 1].is_ascii_alphanumeric();
        let right_bounded = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if left_bounded && right_bounded {
            return true;
        }
        start = begin + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sensitive_key_categories_are_detected() {
        for key in [
            "password",
            "user_password",
            "passphrase",
            "sshPassphrase",
            "token",
            "accessToken",
            "api_key",
            "apiKey",
            "api-key",
            "apikey",
            "secret",
            "clientSecret",
            "private_key",
            "private-key",
            "privateKey",
            "credential",
            "credentials",
            "pat",
            "github_pat",
            "GitHub-PAT",
        ] {
            assert!(is_sensitive_key(key), "expected `{key}` to be sensitive");
        }
    }

    #[test]
    fn path_like_keys_are_not_falsely_redacted() {
        for key in [
            "path",
            "paths",
            "pattern",
            "central_path",
            "skill_path",
            "dispatch",
            "compatible",
        ] {
            assert!(!is_sensitive_key(key), "expected `{key}` to stay visible");
        }
    }

    #[test]
    fn operation_details_use_lowercase_marker_and_keep_paths() {
        let redacted = redact_operation_details(json!({
            "token": "secret",
            "path": "/scan/root",
            "nested": { "apiKey": "secret", "safe": "kept" }
        }));

        assert_eq!(redacted["token"], "[redacted]");
        assert_eq!(redacted["path"], "/scan/root");
        assert_eq!(redacted["nested"]["apiKey"], "[redacted]");
        assert_eq!(redacted["nested"]["safe"], "kept");
    }

    #[test]
    fn runtime_json_uses_uppercase_marker() {
        let redacted = redact_runtime_json(json!({
            "passphrase": "secret",
            "nested": { "api-key": "secret", "visible": "ok" }
        }));

        assert_eq!(redacted["passphrase"], "[REDACTED]");
        assert_eq!(redacted["nested"]["api-key"], "[REDACTED]");
        assert_eq!(redacted["nested"]["visible"], "ok");
    }

    #[test]
    fn redaction_recurses_into_arrays_and_keeps_scalars() {
        let redacted = redact_operation_details(json!({
            "items": [
                { "Token": "t", "value": 1 },
                { "private_KEY": "p", "value": 2 }
            ],
            "count": 2
        }));

        assert_eq!(redacted["items"][0]["Token"], "[redacted]");
        assert_eq!(redacted["items"][0]["value"], 1);
        assert_eq!(redacted["items"][1]["private_KEY"], "[redacted]");
        assert_eq!(redacted["items"][1]["value"], 2);
        assert_eq!(redacted["count"], 2);

        assert_eq!(redact_operation_details(json!("hello")), json!("hello"));
        assert_eq!(redact_operation_details(json!(42)), json!(42));
        assert_eq!(redact_operation_details(json!(null)), json!(null));
    }

    /// Parity guard: both JSON entry points must redact exactly the same
    /// keys — only the marker may differ. Fails if the policy ever forks
    /// between the two log layers again.
    #[test]
    fn operation_and_runtime_redact_the_same_keys() {
        let payload = json!({
            "password": "a",
            "passphrase": "b",
            "accessToken": "c",
            "api-key": "d",
            "github_pat": "e",
            "nested": { "privateKey": "f", "path": "/keep", "count": 2 },
            "items": [ { "secret": "g", "value": 1 } ]
        });

        let operation = redact_operation_details(payload.clone());
        let runtime = redact_runtime_json(payload);
        assert_redaction_parity(&operation, &runtime);
    }

    fn assert_redaction_parity(operation: &Value, runtime: &Value) {
        match (operation, runtime) {
            (Value::Object(op), Value::Object(rt)) => {
                for (key, op_value) in op {
                    let rt_value = rt.get(key).expect("payloads share one shape");
                    let op_redacted = op_value == &Value::String(OPERATION_MARKER.to_string());
                    let rt_redacted = rt_value == &Value::String(RUNTIME_MARKER.to_string());
                    assert_eq!(
                        op_redacted, rt_redacted,
                        "divergent redaction for key `{key}`"
                    );
                    if !op_redacted {
                        assert_redaction_parity(op_value, rt_value);
                    }
                }
            }
            (Value::Array(op), Value::Array(rt)) => {
                for (op_item, rt_item) in op.iter().zip(rt.iter()) {
                    assert_redaction_parity(op_item, rt_item);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn runtime_line_redacts_json_style_values() {
        assert_eq!(
            redact_runtime_line(r#"{"passphrase":"top-secret"}"#),
            r#"{"passphrase":"[REDACTED]"}"#
        );
        assert_eq!(
            redact_runtime_line(r#"INFO token=abc {"apiKey":"sk-test"}"#),
            r#"INFO token=[REDACTED] {"apiKey":"[REDACTED]"}"#
        );
    }

    #[test]
    fn runtime_line_redacts_key_value_style_and_keeps_paths() {
        let line = redact_runtime_line("INFO connect passphrase=abc token=xyz path=/keep");

        assert!(line.contains("passphrase=[REDACTED]"));
        assert!(line.contains("token=[REDACTED]"));
        assert!(line.contains("path=/keep"));
        assert!(!line.contains("abc"));
        assert!(!line.contains("xyz"));
    }
}
