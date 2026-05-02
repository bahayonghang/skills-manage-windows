//! AI explanation error classification: maps `reqwest::Error` chains into a
//! structured `ExplanationErrorInfo` with actionable hints, and renders a flat
//! summary string for the non-streaming explanation path.

use serde::{Deserialize, Serialize};

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
    pub message: String,
    pub details: String,
    pub kind: ExplanationErrorKind,
    pub retryable: bool,
    pub fallback_tried: bool,
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

    let (kind, message, retryable) = if low.contains("tunnel")
        || (low.contains("proxy") && low.contains("connect"))
        || (low.contains("proxy") && low.contains("unsuccessful"))
    {
        (
            ExplanationErrorKind::Proxy,
            "代理或网络隧道连接失败，请尝试切换区域端点或在终端执行 `unset HTTPS_PROXY HTTP_PROXY ALL_PROXY` 后重启应用".to_string(),
            true,
        )
    } else if low.contains("proxy") {
        (
            ExplanationErrorKind::Proxy,
            "系统代理可能拦截了请求。请尝试为该域名配置直连规则或切换区域端点".to_string(),
            true,
        )
    } else if e.is_connect() || low.contains("connect") {
        (
            ExplanationErrorKind::Connect,
            "无法建立连接。请确认 URL 可从本机访问，或尝试切换区域端点".to_string(),
            true,
        )
    } else if e.is_timeout() || low.contains("timed out") || low.contains("deadline has elapsed") {
        (
            ExplanationErrorKind::Timeout,
            "请求超时，可能网络不通或被防火墙拦截。可在终端 `curl -v <url>` 验证连通性".to_string(),
            true,
        )
    } else if low.contains("dns") || low.contains("lookup") {
        (
            ExplanationErrorKind::Dns,
            "DNS 解析失败。请确认域名拼写正确，或尝试切换 DNS".to_string(),
            true,
        )
    } else if low.contains("certificate") || low.contains("tls") || low.contains("handshake") {
        (
            ExplanationErrorKind::Tls,
            "TLS/证书握手失败。请检查系统时间是否正确，或排查中间人代理".to_string(),
            false,
        )
    } else {
        (
            ExplanationErrorKind::Unknown,
            "网络请求失败".to_string(),
            false,
        )
    };

    ExplanationErrorInfo {
        message,
        details: chain,
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
