//! AI provider domain errors and explanation error classification.
//!
//! `AiProviderError` is the typed error for the explanation / key-management
//! flows; Display texts intentionally preserve the historical string-error
//! wording (including the `ai.*:` coded prefixes the frontend parses) because
//! the IPC boundary stringifies these errors verbatim. The rest of this module
//! maps `reqwest::Error` chains into a structured `ExplanationErrorInfo` with
//! actionable hints, and renders a flat summary string for the non-streaming
//! explanation path.

use serde::{Deserialize, Serialize};

use crate::secrets::SecretStorageState;

/// Failure categories for AI provider explanation and API-key management.
#[derive(Debug, thiserror::Error)]
pub enum AiProviderError {
    /// skill_explanations cache reads/deletes and settings-repo calls flow
    /// through transparently.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// Upsert into the `skill_explanations` cache failed.
    #[error("Failed to cache AI explanation: {0}")]
    CacheWrite(#[source] sqlx::Error),

    /// Secret-store interaction failure (read / save / verify / clear /
    /// migrate). Message preformatted at the call site ("Failed to {action}
    /// AI API key: ..." wrapping the typed `SecretError` Display).
    #[error("{0}")]
    Secret(String),

    /// No API key configured. Coded "ai.missing_api_key:..." message
    /// preformatted at the call site.
    #[error("{0}")]
    MissingApiKey(String),

    /// HTTP transport/protocol failure: client build, request send, non-2xx
    /// response, body/stream read. Coded message preformatted at the call site.
    #[error("{0}")]
    Http(String),

    /// 429 from the provider. Coded "ai.rate_limit:..." message preformatted
    /// at the call site.
    #[error("{0}")]
    RateLimited(String),

    /// 401/403 from the provider. Coded "ai.invalid_api_key:..." message
    /// preformatted at the call site.
    #[error("{0}")]
    AccessDenied(String),

    /// Response parse failure. Coded "ai.response_parse_failed:..." message
    /// preformatted at the call site.
    #[error("{0}")]
    Parse(String),

    #[error("AI API key cannot be empty; clear the key instead.")]
    EmptyApiKey,

    #[error("Failed to save AI API key: unavailable storage state {0:?}")]
    UnavailableKeyStorage(SecretStorageState),

    #[error("Failed to verify saved AI API key.")]
    SavedKeyVerificationFailed,

    #[error("AI explanation returned no content.")]
    EmptyExplanation,

    #[error("ai.response_too_large:The AI provider {phase} exceeded the {limit}-byte limit.")]
    ResponseTooLarge { phase: &'static str, limit: u64 },

    #[error("ai.timeout:The AI provider {phase} timed out after {timeout_ms} ms.")]
    ResponseTimeout {
        phase: &'static str,
        timeout_ms: u128,
    },
}

impl AiProviderError {
    /// Classify a non-2xx provider response by status code, preserving the
    /// preformatted coded message (mirrors `error_code_for_status`).
    pub(crate) fn from_status(status_code: u16, message: String) -> Self {
        if status_code == 401 || status_code == 403 {
            Self::AccessDenied(message)
        } else if status_code == 429 {
            Self::RateLimited(message)
        } else {
            Self::Http(message)
        }
    }
}

/// Error kind for AI explanation network failures, used by the frontend
/// to render targeted UI (friendly summary + expandable details).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExplanationErrorKind {
    Proxy,
    Connect,
    Timeout,
    Dns,
    Tls,
    Auth,
    Response,
    Unknown,
}

/// Structured AI explanation error payload sent via Tauri events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplanationErrorInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
    pub details: String,
    pub kind: ExplanationErrorKind,
    pub retryable: bool,
    pub fallback_tried: bool,
}

pub(crate) const AI_MISSING_API_KEY: &str = "ai.missing_api_key";
pub(crate) const AI_RATE_LIMIT: &str = "ai.rate_limit";
pub(crate) const AI_INVALID_API_KEY: &str = "ai.invalid_api_key";
pub(crate) const AI_REQUEST_FAILED: &str = "ai.request_failed";
pub(crate) const AI_CLIENT_BUILD_FAILED: &str = "ai.client_build_failed";
pub(crate) const AI_RESPONSE_ERROR: &str = "ai.response_error";
pub(crate) const AI_RESPONSE_READ_FAILED: &str = "ai.response_read_failed";
pub(crate) const AI_RESPONSE_PARSE_FAILED: &str = "ai.response_parse_failed";
pub(crate) const AI_PROXY: &str = "ai.proxy";
pub(crate) const AI_CONNECT: &str = "ai.connect";
pub(crate) const AI_TIMEOUT: &str = "ai.timeout";
pub(crate) const AI_DNS: &str = "ai.dns";
pub(crate) const AI_TLS: &str = "ai.tls";
pub(crate) const AI_NETWORK: &str = "ai.network";
pub(crate) const AI_EMPTY_RESPONSE: &str = "ai.empty_response";

pub(crate) fn coded_error(code: &str, message: impl AsRef<str>) -> String {
    format!("{code}:{}", message.as_ref())
}

pub(crate) fn coded_error_with_details(
    code: &str,
    message: impl AsRef<str>,
    details: impl AsRef<str>,
) -> String {
    let details = details.as_ref().trim();
    if details.is_empty() {
        coded_error(code, message)
    } else {
        format!("{}\n{}", coded_error(code, message), details)
    }
}

/// Classify a reqwest error into a structured `ExplanationErrorInfo`.
pub(crate) fn classify_reqwest_error(
    e: &reqwest::Error,
    fallback_tried: bool,
) -> ExplanationErrorInfo {
    use std::error::Error as _;

    let mut parts: Vec<String> = vec![e.to_string()];
    let mut cur: Option<&(dyn std::error::Error + 'static)> = e.source();
    while let Some(src) = cur {
        parts.push(src.to_string());
        cur = src.source();
    }
    let chain = parts.join(" → ");
    let low = chain.to_ascii_lowercase();

    let (kind, code, message, details, retryable) = if low.contains("tunnel")
        || (low.contains("proxy") && low.contains("connect"))
        || (low.contains("proxy") && low.contains("unsuccessful"))
    {
        (
            ExplanationErrorKind::Proxy,
            AI_PROXY,
            "Proxy or network tunnel connection failed. Try another region endpoint, or clear HTTPS_PROXY, HTTP_PROXY, and ALL_PROXY before restarting the app."
                .to_string(),
            "The request failed while establishing a proxy or network tunnel.",
            true,
        )
    } else if low.contains("proxy") {
        (
            ExplanationErrorKind::Proxy,
            AI_PROXY,
            "A system proxy may be intercepting the request. Add a direct-connect rule for this domain or switch region endpoint.".to_string(),
            "The request failed while using the configured proxy.",
            true,
        )
    } else if e.is_connect() || low.contains("connect") {
        (
            ExplanationErrorKind::Connect,
            AI_CONNECT,
            "Unable to connect. Confirm the URL is reachable from this machine, or try another region endpoint.".to_string(),
            "The provider connection could not be established.",
            true,
        )
    } else if e.is_timeout() || low.contains("timed out") || low.contains("deadline has elapsed") {
        (
            ExplanationErrorKind::Timeout,
            AI_TIMEOUT,
            "The request timed out. The network may be blocked or intercepted by a firewall; verify connectivity with curl if needed.".to_string(),
            "The provider request exceeded its deadline.",
            true,
        )
    } else if low.contains("dns") || low.contains("lookup") {
        (
            ExplanationErrorKind::Dns,
            AI_DNS,
            "DNS lookup failed. Confirm the domain is correct, or try another DNS resolver."
                .to_string(),
            "The provider hostname could not be resolved.",
            true,
        )
    } else if low.contains("certificate") || low.contains("tls") || low.contains("handshake") {
        (
            ExplanationErrorKind::Tls,
            AI_TLS,
            "TLS or certificate handshake failed. Check the system clock and any intercepting proxy.".to_string(),
            "The secure provider connection could not be established.",
            false,
        )
    } else {
        (
            ExplanationErrorKind::Unknown,
            AI_NETWORK,
            "The network request failed.".to_string(),
            "The provider request failed before a response was available.",
            false,
        )
    };

    ExplanationErrorInfo {
        code: Some(code.to_string()),
        message,
        details: details.to_string(),
        kind,
        retryable,
        fallback_tried,
    }
}

/// Expand a `reqwest::Error` into a single readable string (for non-streaming path).
pub(crate) fn format_reqwest_error(e: &reqwest::Error) -> String {
    let info = classify_reqwest_error(e, false);
    if info.message.is_empty() {
        info.details
    } else {
        format!("{}\n{}", info.details, info.message)
    }
}
