//! Allocation-bounded readers for untrusted HTTP bodies and local files.
//!
//! This module owns only the incremental read mechanisms. Callers choose the
//! policy and map these errors into their domain-specific error enums.

use futures_util::StreamExt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadLimit {
    pub label: &'static str,
    pub max_bytes: u64,
}

impl ReadLimit {
    pub const fn new(label: &'static str, max_bytes: u64) -> Self {
        Self { label, max_bytes }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BoundedReadError {
    #[error("{label} exceeds the resource budget ({actual} bytes > {limit} bytes).")]
    LimitExceeded {
        label: &'static str,
        actual: u64,
        limit: u64,
    },

    #[error("Failed to read {label}.")]
    Http { label: &'static str },

    #[error("Failed to read {label}: {source}")]
    Io {
        label: &'static str,
        #[source]
        source: io::Error,
    },

    #[error("{label} is not valid UTF-8.")]
    InvalidUtf8 { label: &'static str },
}

impl BoundedReadError {
    pub fn actual_and_limit(&self) -> Option<(u64, u64)> {
        match self {
            Self::LimitExceeded { actual, limit, .. } => Some((*actual, *limit)),
            _ => None,
        }
    }
}

pub async fn read_response_bytes_bounded(
    response: reqwest::Response,
    limit: ReadLimit,
) -> Result<Vec<u8>, BoundedReadError> {
    if let Some(content_length) = response.content_length() {
        reject_size(limit, content_length)?;
    }

    let capacity = response
        .content_length()
        .unwrap_or(0)
        .min(limit.max_bytes)
        .try_into()
        .unwrap_or(usize::MAX);
    let mut output = Vec::with_capacity(capacity);
    let mut total = 0_u64;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| BoundedReadError::Http { label: limit.label })?;
        let chunk_len = u64::try_from(chunk.len()).map_err(|_| limit_overflow(limit))?;
        total = total
            .checked_add(chunk_len)
            .ok_or_else(|| limit_overflow(limit))?;
        reject_size(limit, total)?;
        output.extend_from_slice(&chunk);
    }

    Ok(output)
}

pub async fn read_response_text_bounded(
    response: reqwest::Response,
    limit: ReadLimit,
) -> Result<String, BoundedReadError> {
    let bytes = read_response_bytes_bounded(response, limit).await?;
    String::from_utf8(bytes).map_err(|_| BoundedReadError::InvalidUtf8 { label: limit.label })
}

pub fn read_file_bytes_bounded(path: &Path, limit: ReadLimit) -> Result<Vec<u8>, BoundedReadError> {
    let file = File::open(path).map_err(|source| BoundedReadError::Io {
        label: limit.label,
        source,
    })?;
    let metadata_len = file
        .metadata()
        .map_err(|source| BoundedReadError::Io {
            label: limit.label,
            source,
        })?
        .len();
    read_open_file_bounded(file, metadata_len, limit)
}

pub fn read_file_text_bounded(path: &Path, limit: ReadLimit) -> Result<String, BoundedReadError> {
    let bytes = read_file_bytes_bounded(path, limit)?;
    String::from_utf8(bytes).map_err(|_| BoundedReadError::InvalidUtf8 { label: limit.label })
}

fn read_open_file_bounded(
    file: File,
    metadata_len: u64,
    limit: ReadLimit,
) -> Result<Vec<u8>, BoundedReadError> {
    reject_size(limit, metadata_len)?;
    let capacity = metadata_len
        .min(limit.max_bytes)
        .try_into()
        .unwrap_or(usize::MAX);
    let mut output = Vec::with_capacity(capacity);
    let read_limit = limit.max_bytes.saturating_add(1);
    file.take(read_limit)
        .read_to_end(&mut output)
        .map_err(|source| BoundedReadError::Io {
            label: limit.label,
            source,
        })?;
    let actual = u64::try_from(output.len()).map_err(|_| limit_overflow(limit))?;
    reject_size(limit, actual)?;
    Ok(output)
}

pub fn safe_utf8_prefix_bytes(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

pub fn truncate_chars(value: &str, max_chars: usize) -> (&str, bool) {
    let Some((byte_index, _)) = value.char_indices().nth(max_chars) else {
        return (value, false);
    };
    (&value[..byte_index], true)
}

fn reject_size(limit: ReadLimit, actual: u64) -> Result<(), BoundedReadError> {
    if actual > limit.max_bytes {
        return Err(BoundedReadError::LimitExceeded {
            label: limit.label,
            actual,
            limit: limit.max_bytes,
        });
    }
    Ok(())
}

fn limit_overflow(limit: ReadLimit) -> BoundedReadError {
    BoundedReadError::LimitExceeded {
        label: limit.label,
        actual: u64::MAX,
        limit: limit.max_bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};

    #[test]
    fn local_reader_rejects_growth_after_metadata() {
        let mut fixture = tempfile::NamedTempFile::new().unwrap();
        fixture.write_all(b"1234").unwrap();
        fixture.flush().unwrap();

        let mut reader = File::open(fixture.path()).unwrap();
        let metadata_len = reader.metadata().unwrap().len();
        fixture.as_file_mut().seek(SeekFrom::End(0)).unwrap();
        fixture.write_all(b"56789").unwrap();
        fixture.flush().unwrap();
        reader.seek(SeekFrom::Start(0)).unwrap();

        let error = read_open_file_bounded(reader, metadata_len, ReadLimit::new("test file", 8))
            .unwrap_err();
        assert_eq!(error.actual_and_limit(), Some((9, 8)));
    }

    #[test]
    fn local_text_reader_rejects_invalid_utf8_without_content_in_error() {
        let mut fixture = tempfile::NamedTempFile::new().unwrap();
        fixture
            .write_all(&[0xff, b's', b'e', b'c', b'r', b'e', b't'])
            .unwrap();
        let error =
            read_file_text_bounded(fixture.path(), ReadLimit::new("skill file", 64)).unwrap_err();
        assert!(matches!(error, BoundedReadError::InvalidUtf8 { .. }));
        assert!(!error.to_string().contains("secret"));
    }

    #[test]
    fn utf8_helpers_preserve_scalar_and_byte_boundaries() {
        let value = format!("{}{}e\u{301}", "中".repeat(8_000), "🙂");
        let (prefix, truncated) = truncate_chars(&value, 8_000);
        assert!(truncated);
        assert_eq!(prefix.chars().count(), 8_000);
        assert!(prefix.chars().all(|ch| ch == '中'));

        assert_eq!(safe_utf8_prefix_bytes("中🙂a", 0), "");
        assert_eq!(safe_utf8_prefix_bytes("中🙂a", 3), "中");
        assert_eq!(safe_utf8_prefix_bytes("中🙂a", 6), "中");
        assert_eq!(safe_utf8_prefix_bytes("中🙂a", 7), "中🙂");
        assert_eq!(safe_utf8_prefix_bytes("ascii", 99), "ascii");
    }
}
