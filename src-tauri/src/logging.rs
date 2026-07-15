use crate::paths;
use chrono::{DateTime, Duration, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

const RUNTIME_LOG_RETENTION_DAYS: i64 = 14;
const RUNTIME_LOG_PREFIX: &str = "skillport-";
const RUNTIME_LOG_SUFFIX: &str = ".log";
const DEFAULT_RUNTIME_LOG_LIMIT: usize = 200;
const MAX_RUNTIME_LOG_LIMIT: usize = 1_000;
const MAX_FRONTEND_SOURCE_CHARS: usize = 80;
const MAX_FRONTEND_MESSAGE_CHARS: usize = 2_000;
const MAX_FRONTEND_DETAILS_CHARS: usize = 4_000;

#[derive(Debug)]
struct DailyLogWriter {
    log_dir: PathBuf,
}

impl DailyLogWriter {
    fn new(log_dir: PathBuf) -> Self {
        Self { log_dir }
    }

    fn current_log_path(&self) -> PathBuf {
        daily_log_path(&self.log_dir)
    }
}

impl Write for DailyLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let path = self.current_log_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLogFile {
    pub file_name: String,
    pub date: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLogReadRequest {
    pub file_name: String,
    pub query: Option<String>,
    pub level: Option<String>,
    pub source: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    #[serde(default)]
    pub tail: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLogLine {
    pub line_number: usize,
    pub timestamp: Option<String>,
    pub level: Option<String>,
    pub source: String,
    pub message: String,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLogReadResult {
    pub file_name: String,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
    pub lines: Vec<RuntimeLogLine>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLogClearRequest {
    pub file_name: Option<String>,
    pub older_than_days: Option<i64>,
    pub all: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendRuntimeLogPayload {
    pub level: Option<String>,
    pub source: Option<String>,
    pub message: Option<String>,
    pub details: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SanitizedFrontendRuntimeLog {
    level: String,
    source: String,
    message: String,
    details: String,
}

/// Failure categories for runtime log initialization and file management.
/// Display texts preserve the historical string-error wording verbatim; the
/// IPC boundary (`commands/logs.rs`) stringifies these errors for the UI.
#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    /// IO failure with an operation-context prefix (e.g. "Failed to read
    /// runtime log file 'x': <io error>").
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// Directory entry / metadata read failure surfaced without extra
    /// context (historical `error.to_string()` sites).
    #[error(transparent)]
    DirEntry(#[from] std::io::Error),

    #[error("Failed to install tracing subscriber: {0}")]
    InstallSubscriber(tracing::subscriber::SetGlobalDefaultError),

    #[error("Failed to retain tracing worker guard.")]
    RetainWorkerGuard,

    #[error("Failed to install stderr tracing subscriber: {0}")]
    InstallStderrSubscriber(Box<dyn std::error::Error + Send + Sync>),

    #[error("Runtime log clear request must include fileName, all, or olderThanDays.")]
    ClearRequestMissingSelector,

    #[error("Invalid runtime log file name '{0}'. Expected skillport-YYYY-MM-DD.log.")]
    InvalidLogFileName(String),
}

impl LoggingError {
    /// Build a [`LoggingError::Io`] with an operation-context prefix.
    fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}

pub fn logs_dir() -> PathBuf {
    paths::app_data_dir().join("logs")
}

pub fn init_file_logging() -> Result<(), LoggingError> {
    match init_file_logging_with_dir(&logs_dir()) {
        Ok(()) => {
            tracing::info!(
                target: "skillport::startup",
                log_dir = %paths::path_to_string(&logs_dir()),
                "SkillPort file logging initialized"
            );
            Ok(())
        }
        Err(error) => {
            init_stderr_logging()?;
            tracing::warn!(
                target: "skillport::startup",
                error = %error,
                "SkillPort file logging unavailable; using stderr logging"
            );
            Ok(())
        }
    }
}

pub fn init_file_logging_with_dir(log_dir: &Path) -> Result<(), LoggingError> {
    if LOG_GUARD.get().is_some() {
        return Ok(());
    }

    fs::create_dir_all(log_dir).map_err(|error| {
        LoggingError::io(
            format!("Failed to create log directory '{}'", log_dir.display()),
            error,
        )
    })?;

    if let Err(error) = cleanup_expired_runtime_logs_in_dir(log_dir, RUNTIME_LOG_RETENTION_DAYS) {
        eprintln!("Failed to clean expired SkillPort runtime logs: {error}");
    }

    let appender = DailyLogWriter::new(log_dir.to_path_buf());
    init_file_logging_with_appender(appender)
}

fn daily_log_path(log_dir: &Path) -> PathBuf {
    log_dir.join(format!("skillport-{}.log", Local::now().format("%Y-%m-%d")))
}

fn init_file_logging_with_appender(appender: DailyLogWriter) -> Result<(), LoggingError> {
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(default_env_filter())
        .with_writer(writer)
        .with_ansi(false)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber).map_err(LoggingError::InstallSubscriber)?;
    LOG_GUARD
        .set(guard)
        .map_err(|_| LoggingError::RetainWorkerGuard)
}

fn init_stderr_logging() -> Result<(), LoggingError> {
    tracing_subscriber::fmt()
        .with_env_filter(default_env_filter())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .compact()
        .try_init()
        .map_err(LoggingError::InstallStderrSubscriber)
}

fn default_env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

pub fn list_runtime_log_files() -> Result<Vec<RuntimeLogFile>, LoggingError> {
    list_runtime_log_files_in_dir(&logs_dir())
}

fn list_runtime_log_files_in_dir(log_dir: &Path) -> Result<Vec<RuntimeLogFile>, LoggingError> {
    if !log_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(log_dir).map_err(|error| {
        LoggingError::io(
            format!(
                "Failed to read runtime log directory '{}'",
                log_dir.display()
            ),
            error,
        )
    })? {
        let entry = entry?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let Some(date) = runtime_log_file_date(&file_name) else {
            continue;
        };
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }
        files.push(RuntimeLogFile {
            file_name,
            date: date.to_string(),
            size_bytes: metadata.len(),
            modified_at: metadata
                .modified()
                .ok()
                .map(|modified| DateTime::<Utc>::from(modified).to_rfc3339()),
        });
    }

    files.sort_by(|left, right| right.date.cmp(&left.date));
    Ok(files)
}

pub fn read_runtime_log_file(
    request: RuntimeLogReadRequest,
) -> Result<RuntimeLogReadResult, LoggingError> {
    read_runtime_log_file_from_dir(&logs_dir(), request)
}

fn read_runtime_log_file_from_dir(
    log_dir: &Path,
    request: RuntimeLogReadRequest,
) -> Result<RuntimeLogReadResult, LoggingError> {
    let path = runtime_log_path(log_dir, &request.file_name)?;
    let content = fs::read_to_string(&path).map_err(|error| {
        LoggingError::io(
            format!("Failed to read runtime log file '{}'", request.file_name),
            error,
        )
    })?;

    let query = normalize_filter_value(request.query.as_deref()).map(|value| value.to_lowercase());
    let level = normalize_filter_value(request.level.as_deref()).map(|value| value.to_lowercase());
    let source =
        normalize_filter_value(request.source.as_deref()).map(|value| value.to_lowercase());

    let filtered = content
        .lines()
        .enumerate()
        .map(|(index, raw)| parse_runtime_log_line(index + 1, raw))
        .filter(|line| {
            runtime_line_matches(line, query.as_deref(), level.as_deref(), source.as_deref())
        })
        .collect::<Vec<_>>();

    let total = filtered.len();
    let limit = runtime_log_limit(request.limit);
    let requested_offset = request.offset.unwrap_or(0).min(total);
    let offset = if request.tail {
        total.saturating_sub(limit)
    } else {
        requested_offset
    };
    let lines = filtered.into_iter().skip(offset).take(limit).collect();

    Ok(RuntimeLogReadResult {
        file_name: request.file_name,
        total,
        limit,
        offset,
        lines,
    })
}

pub fn export_runtime_log_file(file_name: String) -> Result<String, LoggingError> {
    export_runtime_log_file_from_dir(&logs_dir(), &file_name)
}

fn export_runtime_log_file_from_dir(
    log_dir: &Path,
    file_name: &str,
) -> Result<String, LoggingError> {
    let path = runtime_log_path(log_dir, file_name)?;
    let content = fs::read_to_string(&path).map_err(|error| {
        LoggingError::io(
            format!("Failed to export runtime log file '{file_name}'"),
            error,
        )
    })?;
    Ok(content
        .lines()
        .map(crate::redaction::redact_runtime_line)
        .collect::<Vec<_>>()
        .join("\n"))
}

pub fn clear_runtime_logs(request: RuntimeLogClearRequest) -> Result<u64, LoggingError> {
    clear_runtime_logs_in_dir(&logs_dir(), request)
}

fn clear_runtime_logs_in_dir(
    log_dir: &Path,
    request: RuntimeLogClearRequest,
) -> Result<u64, LoggingError> {
    if !log_dir.exists() {
        return Ok(0);
    }

    if let Some(file_name) = request.file_name {
        let path = runtime_log_path(log_dir, &file_name)?;
        if path.exists() {
            fs::remove_file(&path).map_err(|error| {
                LoggingError::io(
                    format!("Failed to delete runtime log file '{file_name}'"),
                    error,
                )
            })?;
            return Ok(1);
        }
        return Ok(0);
    }

    let cutoff = if request.all.unwrap_or(false) {
        None
    } else if let Some(days) = request.older_than_days {
        let days = days.max(0);
        Some((Local::now() - Duration::days(days)).date_naive())
    } else {
        return Err(LoggingError::ClearRequestMissingSelector);
    };

    let mut deleted = 0;
    for file in list_runtime_log_files_in_dir(log_dir)? {
        let Some(date) = runtime_log_file_date(&file.file_name) else {
            continue;
        };
        if cutoff.is_some_and(|cutoff| date >= cutoff) {
            continue;
        }
        let path = runtime_log_path(log_dir, &file.file_name)?;
        fs::remove_file(&path).map_err(|error| {
            LoggingError::io(
                format!("Failed to delete runtime log file '{}'", file.file_name),
                error,
            )
        })?;
        deleted += 1;
    }

    Ok(deleted)
}

pub fn cleanup_expired_runtime_logs() -> Result<u64, LoggingError> {
    cleanup_expired_runtime_logs_in_dir(&logs_dir(), RUNTIME_LOG_RETENTION_DAYS)
}

fn cleanup_expired_runtime_logs_in_dir(
    log_dir: &Path,
    retention_days: i64,
) -> Result<u64, LoggingError> {
    clear_runtime_logs_in_dir(
        log_dir,
        RuntimeLogClearRequest {
            file_name: None,
            older_than_days: Some(retention_days),
            all: None,
        },
    )
}

pub fn record_frontend_runtime_log(payload: FrontendRuntimeLogPayload) {
    let log = sanitize_frontend_runtime_log_payload(payload);
    match log.level.as_str() {
        "error" => tracing::error!(
            target: "skillport::frontend",
            source = %log.source,
            details = %log.details,
            "{}",
            log.message
        ),
        "warn" => tracing::warn!(
            target: "skillport::frontend",
            source = %log.source,
            details = %log.details,
            "{}",
            log.message
        ),
        "debug" => tracing::debug!(
            target: "skillport::frontend",
            source = %log.source,
            details = %log.details,
            "{}",
            log.message
        ),
        _ => tracing::info!(
            target: "skillport::frontend",
            source = %log.source,
            details = %log.details,
            "{}",
            log.message
        ),
    }
}

fn runtime_log_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_RUNTIME_LOG_LIMIT)
        .clamp(1, MAX_RUNTIME_LOG_LIMIT)
}

fn normalize_filter_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn runtime_line_matches(
    line: &RuntimeLogLine,
    query: Option<&str>,
    level: Option<&str>,
    source: Option<&str>,
) -> bool {
    if let Some(level) = level {
        if line.level.as_deref().map(str::to_lowercase).as_deref() != Some(level) {
            return false;
        }
    }

    if let Some(source) = source {
        let line_source = line.source.to_lowercase();
        let raw = line.raw.to_lowercase();
        if !line_source.contains(source) && !raw.contains(source) {
            return false;
        }
    }

    if let Some(query) = query {
        let haystack = format!("{} {} {}", line.source, line.message, line.raw).to_lowercase();
        if !haystack.contains(query) {
            return false;
        }
    }

    true
}

fn parse_runtime_log_line(line_number: usize, raw: &str) -> RuntimeLogLine {
    let raw = crate::redaction::redact_runtime_line(raw);
    let level = detect_runtime_level(&raw);
    let timestamp = detect_runtime_timestamp(&raw);
    let target_source = detect_runtime_target_source(&raw);
    let source = extract_log_field(&raw, "source")
        .or(target_source)
        .unwrap_or_else(|| "runtime".to_string());
    let message = detect_runtime_message(&raw);

    RuntimeLogLine {
        line_number,
        timestamp,
        level,
        source,
        message,
        raw,
    }
}

fn detect_runtime_level(raw: &str) -> Option<String> {
    raw.split_whitespace().find_map(|token| {
        let cleaned = token
            .trim_matches(|value: char| !value.is_ascii_alphabetic())
            .to_uppercase();
        match cleaned.as_str() {
            "ERROR" | "WARN" | "INFO" | "DEBUG" | "TRACE" => Some(cleaned.to_lowercase()),
            _ => None,
        }
    })
}

fn detect_runtime_timestamp(raw: &str) -> Option<String> {
    let first = raw.split_whitespace().next()?;
    if first.len() >= 10
        && first.as_bytes().get(4) == Some(&b'-')
        && first.as_bytes().get(7) == Some(&b'-')
    {
        Some(first.trim_end_matches(',').to_string())
    } else {
        None
    }
}

fn detect_runtime_target_source(raw: &str) -> Option<String> {
    let markers = ["skillport::", "src_tauri::", "src-tauri::"];
    for marker in markers {
        if let Some(index) = raw.find(marker) {
            let rest = &raw[index..];
            let end = rest
                .find([':', ' ', '\t'])
                .filter(|end| *end > 0)
                .unwrap_or(rest.len());
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn detect_runtime_message(raw: &str) -> String {
    if let Some(index) = raw.rfind(": ") {
        return raw[index + 2..].trim().to_string();
    }
    raw.trim().to_string()
}

fn extract_log_field(raw: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=");
    let start = raw.find(&needle)? + needle.len();
    let rest = &raw[start..];
    let trimmed = rest.trim_start_matches(['\"', '\'']);
    let end = trimmed
        .find([' ', ',', ';', '\"', '\''])
        .unwrap_or(trimmed.len());
    let value = trimmed[..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn runtime_log_path(log_dir: &Path, file_name: &str) -> Result<PathBuf, LoggingError> {
    validate_runtime_log_file_name(file_name)?;
    Ok(log_dir.join(file_name))
}

fn validate_runtime_log_file_name(file_name: &str) -> Result<(), LoggingError> {
    if runtime_log_file_date(file_name).is_some() {
        Ok(())
    } else {
        Err(LoggingError::InvalidLogFileName(file_name.to_string()))
    }
}

fn runtime_log_file_date(file_name: &str) -> Option<NaiveDate> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return None;
    }
    let date = file_name
        .strip_prefix(RUNTIME_LOG_PREFIX)?
        .strip_suffix(RUNTIME_LOG_SUFFIX)?;
    if date.len() != 10 {
        return None;
    }
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
}

fn sanitize_frontend_runtime_log_payload(
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

    let source = truncate_chars(
        payload
            .source
            .as_deref()
            .map(str::trim)
            .filter(|source| !source.is_empty())
            .unwrap_or("frontend.runtime"),
        MAX_FRONTEND_SOURCE_CHARS,
    );
    let message = truncate_chars(
        &crate::redaction::redact_runtime_line(
            payload
                .message
                .as_deref()
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .unwrap_or("Frontend runtime event"),
        ),
        MAX_FRONTEND_MESSAGE_CHARS,
    );
    let details = payload
        .details
        .map(crate::redaction::redact_runtime_json)
        .and_then(|value| serde_json::to_string(&value).ok())
        .map(|value| truncate_chars(&value, MAX_FRONTEND_DETAILS_CHARS))
        .unwrap_or_else(|| "{}".to_string());

    SanitizedFrontendRuntimeLog {
        level,
        source,
        message,
        details: crate::redaction::redact_runtime_line(&details),
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

#[cfg(test)]
mod tests;
