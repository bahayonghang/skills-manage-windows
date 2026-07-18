//! Intent-only deep-link parsing and bounded delivery queue.

mod error;

use std::{collections::VecDeque, sync::Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::services::github_import::normalize_github_source_url;

pub use error::DeepLinkError;

pub const IMPORT_INTENT_EVENT: &str = "skillport://import-intent";
pub const MAX_DEEP_LINK_BYTES: usize = 4096;
pub const MAX_PENDING_IMPORT_INTENTS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportIntent {
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EnqueueOutcome {
    pub emit_now: Option<ImportIntent>,
    pub duplicate: bool,
    pub dropped_oldest: bool,
}

#[derive(Debug, Default)]
pub struct PendingImportIntentQueue {
    ready: bool,
    pending: VecDeque<ImportIntent>,
}

#[derive(Debug, Default)]
pub struct ImportIntentState {
    queue: Mutex<PendingImportIntentQueue>,
}

impl PendingImportIntentQueue {
    pub fn enqueue(&mut self, intent: ImportIntent) -> EnqueueOutcome {
        if self.ready {
            return EnqueueOutcome {
                emit_now: Some(intent),
                ..EnqueueOutcome::default()
            };
        }

        if self
            .pending
            .iter()
            .any(|queued| queued.source == intent.source)
        {
            return EnqueueOutcome {
                duplicate: true,
                ..EnqueueOutcome::default()
            };
        }

        let dropped_oldest = if self.pending.len() == MAX_PENDING_IMPORT_INTENTS {
            self.pending.pop_front();
            true
        } else {
            false
        };
        self.pending.push_back(intent);

        EnqueueOutcome {
            dropped_oldest,
            ..EnqueueOutcome::default()
        }
    }

    pub fn mark_ready(&mut self) -> Vec<ImportIntent> {
        if self.ready {
            return Vec::new();
        }
        self.ready = true;
        self.pending.drain(..).collect()
    }
}

pub fn parse_import_deep_link(raw: &str) -> Result<ImportIntent, DeepLinkError> {
    if raw.len() > MAX_DEEP_LINK_BYTES {
        return Err(DeepLinkError::UriTooLong);
    }
    if raw.is_empty() || raw.chars().any(char::is_control) {
        return Err(DeepLinkError::InvalidUri);
    }

    let uri = reqwest::Url::parse(raw).map_err(|_| DeepLinkError::InvalidUri)?;
    if uri.scheme() != "skillport" {
        return Err(DeepLinkError::UnsupportedScheme);
    }
    if !uri.username().is_empty() || uri.password().is_some() || uri.port().is_some() {
        return Err(DeepLinkError::InvalidUriAuthority);
    }
    if uri.host_str() != Some("import") {
        return Err(DeepLinkError::UnknownAction);
    }
    if !uri.path().is_empty() {
        return Err(DeepLinkError::UnexpectedPath);
    }
    if uri.fragment().is_some() {
        return Err(DeepLinkError::FragmentNotAllowed);
    }

    let raw_query = uri.query().ok_or(DeepLinkError::MissingSource)?;
    let raw_source = raw_query
        .split('&')
        .find_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key.eq_ignore_ascii_case("source")).then_some(value)
        })
        .ok_or(DeepLinkError::MissingSource)?;
    if raw_source.contains([':', '/', '\\']) {
        return Err(DeepLinkError::SourceNotPercentEncoded);
    }

    let mut source = None;
    for (key, value) in uri.query_pairs() {
        if key != "source" {
            return Err(if is_sensitive_parameter(&key) {
                DeepLinkError::SensitiveParameter
            } else {
                DeepLinkError::UnknownParameter
            });
        }
        if source.is_some() {
            return Err(DeepLinkError::DuplicateSource);
        }
        if value.is_empty() {
            return Err(DeepLinkError::MissingSource);
        }
        source = Some(value.into_owned());
    }
    let source = source.ok_or(DeepLinkError::MissingSource)?;

    validate_decoded_source(&source)?;
    let source_authority = source
        .split_once("://")
        .map(|(_, remainder)| remainder.split('/').next().unwrap_or(remainder))
        .ok_or(DeepLinkError::InvalidSource)?;
    if source_authority
        .rsplit('@')
        .next()
        .is_some_and(|host| host.contains(':'))
    {
        return Err(DeepLinkError::SourcePort);
    }
    let source_url = reqwest::Url::parse(&source).map_err(|_| DeepLinkError::InvalidSource)?;
    if source_url.scheme() != "https" {
        return Err(DeepLinkError::SourceNotHttps);
    }
    if source_url.host_str() != Some("github.com")
        && source_url.host_str() != Some("www.github.com")
    {
        return Err(DeepLinkError::SourceNotGithub);
    }
    if !source_url.username().is_empty() || source_url.password().is_some() {
        return Err(DeepLinkError::SourceCredentials);
    }
    if source_url.port().is_some() {
        return Err(DeepLinkError::SourcePort);
    }
    if source_url.query().is_some() || source_url.fragment().is_some() {
        return Err(DeepLinkError::SourceParameters);
    }

    let normalized =
        normalize_github_source_url(&source).map_err(|_| DeepLinkError::InvalidGithubSource)?;
    Ok(ImportIntent { source: normalized })
}

pub fn parse_import_intent_from_argv(argv: &[String]) -> Result<ImportIntent, DeepLinkError> {
    match argv {
        [_, uri] => parse_os_import_deep_link(uri),
        [_] | [] => Err(DeepLinkError::MissingImportArgument),
        _ => Err(DeepLinkError::UnexpectedImportArguments),
    }
}

pub fn parse_os_import_deep_link(raw: &str) -> Result<ImportIntent, DeepLinkError> {
    let uri = reqwest::Url::parse(raw).map_err(|_| DeepLinkError::InvalidUri)?;
    if uri.scheme() == "skillport"
        && uri.host_str() == Some("import")
        && uri.path() == "/"
        && uri.fragment().is_none()
        && uri.username().is_empty()
        && uri.password().is_none()
        && uri.port().is_none()
    {
        let query = uri.query().ok_or(DeepLinkError::MissingSource)?;
        return parse_import_deep_link(&format!("skillport://import?{query}"));
    }

    parse_import_deep_link(raw)
}

pub fn submit_import_deep_link(
    app: &AppHandle,
    state: &ImportIntentState,
    raw: &str,
) -> Result<(), DeepLinkError> {
    let intent = parse_os_import_deep_link(raw)?;
    submit_import_intent(app, state, intent)
}

pub fn submit_import_intent(
    app: &AppHandle,
    state: &ImportIntentState,
    intent: ImportIntent,
) -> Result<(), DeepLinkError> {
    let outcome = state
        .queue
        .lock()
        .map_err(|_| DeepLinkError::QueueUnavailable)?
        .enqueue(intent);

    if outcome.dropped_oldest {
        tracing::warn!(
            code = "import_intent_queue_overflow",
            capacity = MAX_PENDING_IMPORT_INTENTS,
            "Dropped the oldest queued import intent"
        );
    }
    if let Some(intent) = outcome.emit_now {
        app.emit(IMPORT_INTENT_EVENT, intent)
            .map_err(|_| DeepLinkError::EventDelivery)?;
    }
    Ok(())
}

pub fn mark_import_intent_ready(
    app: &AppHandle,
    state: &ImportIntentState,
) -> Result<(), DeepLinkError> {
    let pending = state
        .queue
        .lock()
        .map_err(|_| DeepLinkError::QueueUnavailable)?
        .mark_ready();

    for intent in pending {
        app.emit(IMPORT_INTENT_EVENT, intent)
            .map_err(|_| DeepLinkError::EventDelivery)?;
    }
    Ok(())
}

fn validate_decoded_source(source: &str) -> Result<(), DeepLinkError> {
    let mut decoded = source.to_string();
    for _ in 0..3 {
        if decoded.chars().any(char::is_control) || decoded.contains('\\') {
            return Err(DeepLinkError::UnsafeSource);
        }
        let path = decoded.split(['?', '#']).next().unwrap_or(decoded.as_str());
        if path
            .replace('\\', "/")
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        {
            return Err(DeepLinkError::UnsafeSource);
        }

        let next = urlencoding::decode(&decoded)
            .map_err(|_| DeepLinkError::InvalidSource)?
            .into_owned();
        if next == decoded {
            return Ok(());
        }
        decoded = next;
    }

    if decoded.contains('%') {
        return Err(DeepLinkError::UnsafeSource);
    }
    Ok(())
}

fn is_sensitive_parameter(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "token"
            | "pat"
            | "auth"
            | "credential"
            | "password"
            | "secret"
            | "target"
            | "agent"
            | "overwrite"
            | "rename"
            | "skip"
            | "confirm"
            | "auto"
            | "command"
    )
}

#[cfg(test)]
mod tests;
