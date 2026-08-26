//! Redaction Policy Module
//!
//! Deep Module that owns SkillPort's single policy for private diagnostic
//! redaction. Key classification, value inspection, markers and regex
//! patterns are all implementation details; callers only pick the entry
//! point matching the log layer and payload shape.
//!
//! The two callers of this seam:
//!
//! - Operation Log (`operation_log.rs`) redacts details JSON through
//!   [`redact_operation_details`] before persistence.
//! - Runtime Log (`logging.rs`) redacts text lines before persistence and
//!   again on read/export through [`redact_runtime_line`], and frontend event
//!   payloads through [`redact_runtime_json`] (plus [`redact_runtime_line`]
//!   for messages).
//!
//! Matching semantics (single definition point): keys are normalized
//! (lowercase, `-` folded to `_`); long needles match as substrings so
//! camelCase compounds like `accessToken` stay covered; the short needle
//! `pat` requires token boundaries so `pattern` style keys are not falsely
//! redacted. Private diagnostic locations and process/error captures are
//! deliberately fail-closed even when they do not contain a credential.

use regex::{Captures, Regex};
use serde_json::Value;
use std::net::IpAddr;
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

/// Diagnostic locations and raw process/provider output must not cross a
/// persistence or IPC boundary. Long needles intentionally match compounds
/// such as `repositoryUrl`, `logsDir`, and `sourceChain`.
const PRIVATE_DIAGNOSTIC_KEY_NEEDLES: &[&str] = &[
    "path",
    "directory",
    "log_dir",
    "logs_dir",
    "url",
    "uri",
    "host",
    "hostname",
    "args",
    "argv",
    "stdout",
    "stderr",
    "stack",
    "backtrace",
    "source_chain",
    "sourcechain",
    "cause",
    "raw_error",
    "rawerror",
    "error_message",
    "errormessage",
    "error_detail",
    "errordetail",
    "command_line",
    "commandline",
    "environment",
    "env_vars",
];

const PRIVATE_DIAGNOSTIC_EXACT_KEYS: &[&str] = &["dir", "error", "errors", "reason", "raw"];

/// Reviewed stable diagnostic fields remain queryable. They must not be
/// swallowed merely because their names contain the word `error`.
const REVIEWED_DIAGNOSTIC_KEYS: &[&str] = &[
    "error_code",
    "errorcode",
    "error_category",
    "errorcategory",
    "error_count",
    "errorcount",
];

/// Marker persisted into Operation Log details.
const OPERATION_MARKER: &str = "[redacted]";
/// Marker used across Runtime Log lines and payloads.
const RUNTIME_MARKER: &str = "[REDACTED]";

struct RuntimeRedactionPatterns {
    json_field: Regex,
    single_json_field: Regex,
    quoted_field: Regex,
    unquoted_field: Regex,
    url: Regex,
    windows_path: Regex,
    posix_path: Regex,
    network_candidate: Regex,
    credential_value: Regex,
    raw_diagnostic: Regex,
}

static REDACTION_PATTERNS: OnceLock<RuntimeRedactionPatterns> = OnceLock::new();

/// Recursively redact Operation Log details JSON. Marker: `[redacted]`.
pub fn redact_operation_details(value: Value) -> Value {
    redact_value(value, OPERATION_MARKER, false)
}

/// Recursively redact a Runtime Log JSON payload. Marker: `[REDACTED]`.
pub fn redact_runtime_json(value: Value) -> Value {
    redact_value(value, RUNTIME_MARKER, true)
}

/// Fail-closed representation of an unterminated Runtime writer fragment.
/// The newline is part of the contract: later bytes must start on a distinct
/// physical line and cannot complete a sensitive key split by `flush()`.
pub(crate) fn redacted_runtime_fragment_line() -> &'static str {
    concat!("[REDACTED]", "\n")
}

/// Redact a Runtime Log text line (persistence / read / export / frontend
/// messages). Covers JSON-style (`"token":"x"`) and key-value (`token=x`)
/// shapes, then inspects unstructured values for private URLs, absolute paths,
/// credential shapes and raw diagnostic signatures.
/// Marker: `[REDACTED]`.
pub fn redact_runtime_line(raw: &str) -> String {
    let patterns = runtime_redaction_patterns();
    let mut redacted = redact_captured_field(raw, &patterns.json_field, true);
    redacted = redact_captured_field(&redacted, &patterns.single_json_field, true);
    redacted = redact_captured_field(&redacted, &patterns.quoted_field, true);
    redacted = patterns
        .url
        .replace_all(&redacted, RUNTIME_MARKER)
        .to_string();
    redacted = patterns
        .windows_path
        .replace_all(&redacted, |captures: &Captures<'_>| {
            format!("{RUNTIME_MARKER}{}", &captures["suffix"])
        })
        .to_string();
    redacted = patterns
        .posix_path
        .replace_all(&redacted, |captures: &Captures<'_>| {
            format!(
                "{}{RUNTIME_MARKER}{}",
                &captures["prefix"], &captures["suffix"]
            )
        })
        .to_string();
    redacted = redact_network_locations(&redacted, patterns);
    redacted = redact_captured_field(&redacted, &patterns.unquoted_field, false);
    redacted = patterns
        .credential_value
        .replace_all(&redacted, RUNTIME_MARKER)
        .to_string();
    redacted = patterns
        .raw_diagnostic
        .replace_all(&redacted, RUNTIME_MARKER)
        .to_string();
    redacted
}

fn runtime_redaction_patterns() -> &'static RuntimeRedactionPatterns {
    REDACTION_PATTERNS.get_or_init(|| RuntimeRedactionPatterns {
        json_field: Regex::new(
            r#"(?x)(?P<prefix>["']?(?P<key>[A-Za-z_][A-Za-z0-9_-]*)["']?\s*:\s*")(?P<value>(?:\\.|[^"\\])*)(?P<suffix>")"#,
        )
        .expect("valid JSON field regex"),
        single_json_field: Regex::new(
            r#"(?x)(?P<prefix>["']?(?P<key>[A-Za-z_][A-Za-z0-9_-]*)["']?\s*:\s*')(?P<value>(?:\\.|[^'\\])*)(?P<suffix>')"#,
        )
        .expect("valid single-quoted JSON-like field regex"),
        quoted_field: Regex::new(
            r#"(?x)(?P<prefix>\b(?P<key>[A-Za-z_][A-Za-z0-9_-]*)\b\s*=\s*["'])(?P<value>[^"']*)(?P<suffix>["'])"#,
        )
        .expect("valid quoted key-value field regex"),
        unquoted_field: Regex::new(
            r#"(?x)(?P<prefix>\b(?P<key>[A-Za-z_][A-Za-z0-9_-]*)\b\s*=\s*)(?P<value>[^\s,;})\]]+)"#,
        )
        .expect("valid key-value field regex"),
        url: Regex::new(r#"(?i)\b(?:https?|file|ssh)://[^\s"'<>]+"#)
            .expect("valid URL redaction regex"),
        windows_path: Regex::new(
            r#"(?i)(?:\\\\|[a-z]:[\\/])[^\r\n]*?(?P<suffix>["']|\s+[A-Za-z_][A-Za-z0-9_-]*\s*=|$)"#,
        )
            .expect("valid Windows path redaction regex"),
        posix_path: Regex::new(
            r#"(?P<prefix>(?:^|[\s"'=(]))(?:/|~/|\.\.?/)[^\r\n]*?(?P<suffix>["']|\s+[A-Za-z_][A-Za-z0-9_-]*\s*=|$)"#,
        )
            .expect("valid POSIX path redaction regex"),
        network_candidate: Regex::new(r#"(?i)\[?[A-Za-z0-9:][A-Za-z0-9.:%-]*\]?"#)
            .expect("valid network candidate regex"),
        credential_value: Regex::new(
            r#"(?i)\b(?:gh[pousr]_[A-Za-z0-9_=-]{4,}|github_pat_[A-Za-z0-9_=-]{4,}|sk-[A-Za-z0-9_-]{8,})\b"#,
        )
        .expect("valid credential value regex"),
        raw_diagnostic: Regex::new(
            r#"(?i)\b(?:raw[-_ ]?error|stack[-_ ]?trace|caused[-_ ]?by)\b[^\s"'<>]*"#,
        )
        .expect("valid raw diagnostic regex"),
    })
}

fn redact_network_locations(raw: &str, patterns: &RuntimeRedactionPatterns) -> String {
    patterns
        .network_candidate
        .replace_all(raw, |captures: &Captures<'_>| {
            let candidate = captures
                .get(0)
                .expect("network regex always captures the full match")
                .as_str();
            if is_private_network_location(candidate) {
                RUNTIME_MARKER.to_string()
            } else {
                candidate.to_string()
            }
        })
        .to_string()
}

fn is_private_network_location(candidate: &str) -> bool {
    let mut normalized = candidate
        .trim_matches(|character: char| matches!(character, '[' | ']' | '(' | ')' | ',' | ';'));
    if let Some((host, port)) = normalized.rsplit_once(':') {
        if normalized.matches(':').count() == 1
            && port.chars().all(|character| character.is_ascii_digit())
        {
            normalized = host;
        }
    }
    let address = normalized.split('%').next().unwrap_or(normalized);
    if address.parse::<IpAddr>().is_ok() {
        return true;
    }

    let hostname = normalized.trim_end_matches('.').to_ascii_lowercase();
    hostname == "localhost"
        || [
            ".internal",
            ".local",
            ".lan",
            ".corp",
            ".home",
            ".test",
            ".invalid",
            ".example",
        ]
        .iter()
        .any(|suffix| hostname.ends_with(suffix))
}

fn redact_captured_field(raw: &str, pattern: &Regex, has_suffix: bool) -> String {
    pattern
        .replace_all(raw, |captures: &Captures<'_>| {
            let key = &captures["key"];
            let value = &captures["value"];
            if is_sensitive_key(key) || is_private_string(value) {
                let suffix = if has_suffix { &captures["suffix"] } else { "" };
                format!("{}{RUNTIME_MARKER}{suffix}", &captures["prefix"])
            } else {
                captures
                    .get(0)
                    .expect("field regex always captures the full match")
                    .as_str()
                    .to_string()
            }
        })
        .to_string()
}

fn redact_value(value: Value, marker: &str, allow_reviewed_error_envelope: bool) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| redact_value(item, marker, allow_reviewed_error_envelope))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if is_sensitive_key(&key) {
                        let reviewed = allow_reviewed_error_envelope
                            .then(|| reviewed_error_envelope(&key, &value))
                            .flatten();
                        (
                            key,
                            reviewed.unwrap_or_else(|| Value::String(marker.to_string())),
                        )
                    } else {
                        (
                            key,
                            redact_value(value, marker, allow_reviewed_error_envelope),
                        )
                    }
                })
                .collect(),
        ),
        Value::String(value) if is_private_string(&value) => Value::String(marker.to_string()),
        other => other,
    }
}

/// The renderer Runtime boundary has already converted raw IPC failures into
/// this exact reviewed envelope. Preserve only a registered public code and a
/// boolean retry hint; any extra field or unreviewed code makes the entire
/// `error` value private. Operation details never take this exception.
fn reviewed_error_envelope(key: &str, value: &Value) -> Option<Value> {
    if !key.eq_ignore_ascii_case("error") {
        return None;
    }
    let object = value.as_object()?;
    if object
        .keys()
        .any(|key| !matches!(key.as_str(), "code" | "retryable"))
    {
        return None;
    }
    let code = object.get("code")?.as_str()?;
    if code != "internal.unexpected" && crate::ipc_error::public_message_for_code(code).is_none() {
        return None;
    }
    let retryable = object.get("retryable")?.as_bool()?;
    Some(serde_json::json!({ "code": code, "retryable": retryable }))
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = normalize_key(key);
    if REVIEWED_DIAGNOSTIC_KEYS.contains(&normalized.as_str()) {
        return false;
    }
    if SENSITIVE_KEY_NEEDLES
        .iter()
        .any(|needle| normalized.contains(needle))
    {
        return true;
    }
    if TOKEN_BOUNDED_NEEDLES
        .iter()
        .any(|needle| contains_bounded_token(&normalized, needle))
    {
        return true;
    }
    PRIVATE_DIAGNOSTIC_EXACT_KEYS.contains(&normalized.as_str())
        || PRIVATE_DIAGNOSTIC_KEY_NEEDLES
            .iter()
            .any(|needle| normalized.contains(needle))
}

fn is_private_string(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("://")
        || lower.starts_with('/')
        || lower.starts_with("~/")
        || lower.starts_with("./")
        || lower.starts_with("../")
        || lower.starts_with("\\\\")
        || lower.contains("raw-error")
        || lower.contains("raw_error")
        || lower.contains("raw error")
        || lower.contains("error:")
        || lower.contains("stack trace")
        || lower.contains("stack_trace")
        || lower.contains("caused by:")
    {
        return true;
    }

    let patterns = runtime_redaction_patterns();
    patterns.windows_path.is_match(trimmed)
        || patterns.posix_path.is_match(trimmed)
        || patterns
            .network_candidate
            .find_iter(trimmed)
            .any(|candidate| is_private_network_location(candidate.as_str()))
        || patterns.credential_value.is_match(trimmed)
}

fn normalize_key(key: &str) -> String {
    let mut normalized = String::with_capacity(key.len());
    let mut previous_was_lower_or_digit = false;
    for character in key.chars() {
        if matches!(character, '-' | ' ' | '.') {
            if !normalized.ends_with('_') {
                normalized.push('_');
            }
            previous_was_lower_or_digit = false;
            continue;
        }
        if character.is_ascii_uppercase() && previous_was_lower_or_digit {
            normalized.push('_');
        }
        normalized.extend(character.to_lowercase());
        previous_was_lower_or_digit = character.is_ascii_lowercase() || character.is_ascii_digit();
    }
    normalized
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
    fn private_diagnostic_key_categories_are_detected() {
        for key in [
            "path",
            "logsDir",
            "remoteHost",
            "repositoryUrl",
            "args",
            "argv",
            "stdout",
            "stderr",
            "error",
            "rawError",
            "stackTrace",
            "sourceChain",
        ] {
            assert!(is_sensitive_key(key), "expected `{key}` to be private");
        }
    }

    #[test]
    fn non_private_keys_are_not_falsely_redacted() {
        for key in [
            "pattern",
            "dispatch",
            "compatible",
            "error_code",
            "error_category",
            "source",
            "target_kind",
        ] {
            assert!(!is_sensitive_key(key), "expected `{key}` to stay visible");
        }
    }

    #[test]
    fn operation_details_fail_closed_for_private_diagnostics() {
        let path_seed = r"C:\Users\alice\private\skill.md";
        let url_seed = "https://private.example.invalid/repo?token=ghp_private";
        let raw_error_seed = "raw-error-seed-from-provider";
        let redacted = redact_operation_details(json!({
            "token": "secret",
            "path": path_seed,
            "logsDir": "/home/alice/.skillport/logs",
            "remoteHost": "private.internal",
            "repositoryUrl": url_seed,
            "args": ["--token", "secret"],
            "stdout": "provider output",
            "stderr": "provider error",
            "error": raw_error_seed,
            "sourceChain": { "message": raw_error_seed },
            "nested": {
                "apiKey": "secret",
                "safe": "kept",
                "safeName": format!("skill stored at {path_seed}"),
                "safeSummary": url_seed,
                "safeDiagnostic": raw_error_seed
            },
            "errorCode": "storage.unavailable",
            "errorCategory": "storage"
        }));

        assert_eq!(redacted["token"], "[redacted]");
        for key in [
            "path",
            "logsDir",
            "remoteHost",
            "repositoryUrl",
            "args",
            "stdout",
            "stderr",
            "error",
            "sourceChain",
        ] {
            assert_eq!(redacted[key], "[redacted]", "private key `{key}` leaked");
        }
        assert_eq!(redacted["nested"]["apiKey"], "[redacted]");
        assert_eq!(redacted["nested"]["safe"], "kept");
        assert_eq!(redacted["nested"]["safeName"], "[redacted]");
        assert_eq!(redacted["nested"]["safeSummary"], "[redacted]");
        assert_eq!(redacted["nested"]["safeDiagnostic"], "[redacted]");
        assert_eq!(redacted["errorCode"], "storage.unavailable");
        assert_eq!(redacted["errorCategory"], "storage");
        let serialized = redacted.to_string();
        for seed in [path_seed, url_seed, raw_error_seed, "private.internal"] {
            assert!(!serialized.contains(seed), "private seed leaked: {seed}");
        }
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
    fn only_runtime_reviewed_error_envelopes_keep_public_codes() {
        let reviewed = json!({
            "error": { "code": "marketplace.install_failed", "retryable": false }
        });
        let runtime = redact_runtime_json(reviewed.clone());
        let operation = redact_operation_details(reviewed);
        let unknown = redact_runtime_json(json!({
            "error": { "code": "secret.value", "retryable": false }
        }));
        let extended = redact_runtime_json(json!({
            "error": {
                "code": "marketplace.install_failed",
                "retryable": false,
                "message": "raw provider failure"
            }
        }));

        assert_eq!(runtime["error"]["code"], "marketplace.install_failed");
        assert_eq!(runtime["error"]["retryable"], false);
        assert_eq!(operation["error"], OPERATION_MARKER);
        assert_eq!(unknown["error"], RUNTIME_MARKER);
        assert_eq!(extended["error"], RUNTIME_MARKER);
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
            "nested": { "privateKey": "f", "path": "/private", "count": 2 },
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
        assert_eq!(
            redact_runtime_line(r#"{"error":"provider said \"private detail\" after"}"#),
            r#"{"error":"[REDACTED]"}"#
        );
    }

    #[test]
    fn runtime_line_redacts_private_fields_and_unstructured_values() {
        let seeds = [
            r"C:\Users\alice\AppData\Roaming\SkillPort\logs",
            "/home/alice/.skillport/logs",
            "https://private.example.invalid/repo?token=ghp_private",
            "private.internal",
            "ghp_super_secret_value",
            "raw-error-seed-from-provider",
        ];
        let line = redact_runtime_line(&format!(
            "INFO log_dir={} path={} url={} host={} secret={} error={} safe_name={} error_code=storage.unavailable",
            seeds[0], seeds[1], seeds[2], seeds[3], seeds[4], seeds[5], seeds[0]
        ));

        for seed in seeds {
            assert!(!line.contains(seed), "private seed leaked: {seed}");
        }
        assert!(line.contains("error_code=storage.unavailable"));
        assert!(line.matches(RUNTIME_MARKER).count() >= 7);
    }
}
