use futures_util::{Stream, StreamExt};
use std::time::Duration;
use tokio::time::{timeout_at, Instant};

pub(super) const DEFAULT_SSE_WIRE_BYTES: u64 = 4 * 1024 * 1024;
pub(super) const DEFAULT_SSE_EVENT_BYTES: usize = 256 * 1024;
pub(super) const DEFAULT_SSE_OUTPUT_BYTES: usize = 1024 * 1024;
pub(super) const DEFAULT_SSE_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
pub(super) const DEFAULT_SSE_TOTAL_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Copy)]
pub(super) struct StreamPolicy {
    pub wire_bytes: u64,
    pub event_bytes: usize,
    pub output_bytes: usize,
    pub idle_timeout: Duration,
    pub total_timeout: Duration,
}

impl Default for StreamPolicy {
    fn default() -> Self {
        Self {
            wire_bytes: DEFAULT_SSE_WIRE_BYTES,
            event_bytes: DEFAULT_SSE_EVENT_BYTES,
            output_bytes: DEFAULT_SSE_OUTPUT_BYTES,
            idle_timeout: DEFAULT_SSE_IDLE_TIMEOUT,
            total_timeout: DEFAULT_SSE_TOTAL_TIMEOUT,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(super) enum SseIngestionError {
    #[error("AI response stream transport failed.")]
    Transport,
    #[error("AI response stream exceeded the {limit}-byte wire limit.")]
    WireLimit { limit: u64 },
    #[error("AI response event exceeded the {limit}-byte limit.")]
    EventLimit { limit: usize },
    #[error("AI explanation exceeded the {limit}-byte output limit.")]
    OutputLimit { limit: usize },
    #[error("AI response stream was idle for {timeout_ms} ms.")]
    IdleTimeout { timeout_ms: u128 },
    #[error("AI response stream exceeded the {timeout_ms} ms total deadline.")]
    TotalTimeout { timeout_ms: u128 },
    #[error("AI response stream contained invalid UTF-8.")]
    InvalidUtf8,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct SseOutcome {
    pub full_text: String,
    pub saw_thinking_delta: bool,
}

pub(super) async fn consume_sse_stream<S, B, E, F>(
    stream: S,
    is_anthropic: bool,
    policy: StreamPolicy,
    mut on_text: F,
) -> Result<SseOutcome, SseIngestionError>
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
    F: FnMut(&str),
{
    futures_util::pin_mut!(stream);
    let total_deadline = Instant::now() + policy.total_timeout;
    let mut wire_bytes = 0_u64;
    let mut output_bytes = 0_usize;
    let mut line_buffer = Vec::with_capacity(policy.event_bytes.min(8 * 1024));
    let mut completed_event_bytes = 0_usize;
    let mut full_text = String::new();
    let mut saw_thinking_delta = false;

    loop {
        let idle_deadline = Instant::now() + policy.idle_timeout;
        let next_deadline = idle_deadline.min(total_deadline);
        let next = timeout_at(next_deadline, stream.next()).await;
        let Some(chunk) = (match next {
            Ok(next) => next,
            Err(_) if next_deadline == total_deadline => {
                return Err(SseIngestionError::TotalTimeout {
                    timeout_ms: policy.total_timeout.as_millis(),
                });
            }
            Err(_) => {
                return Err(SseIngestionError::IdleTimeout {
                    timeout_ms: policy.idle_timeout.as_millis(),
                });
            }
        }) else {
            break;
        };
        let chunk = chunk.map_err(|_| SseIngestionError::Transport)?;
        let bytes = chunk.as_ref();
        let chunk_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        wire_bytes = wire_bytes
            .checked_add(chunk_len)
            .ok_or(SseIngestionError::WireLimit {
                limit: policy.wire_bytes,
            })?;
        if wire_bytes > policy.wire_bytes {
            return Err(SseIngestionError::WireLimit {
                limit: policy.wire_bytes,
            });
        }

        process_chunk(
            bytes,
            &mut line_buffer,
            &mut completed_event_bytes,
            is_anthropic,
            policy,
            &mut output_bytes,
            &mut full_text,
            &mut saw_thinking_delta,
            &mut on_text,
        )?;
    }

    if !line_buffer.is_empty() {
        process_line(
            &line_buffer,
            is_anthropic,
            policy.output_bytes,
            &mut output_bytes,
            &mut full_text,
            &mut saw_thinking_delta,
            &mut on_text,
        )?;
    }

    Ok(SseOutcome {
        full_text,
        saw_thinking_delta,
    })
}

#[allow(clippy::too_many_arguments)]
fn process_chunk<F>(
    bytes: &[u8],
    line_buffer: &mut Vec<u8>,
    completed_event_bytes: &mut usize,
    is_anthropic: bool,
    policy: StreamPolicy,
    output_bytes: &mut usize,
    full_text: &mut String,
    saw_thinking_delta: &mut bool,
    on_text: &mut F,
) -> Result<(), SseIngestionError>
where
    F: FnMut(&str),
{
    let mut start = 0;
    while let Some(relative_end) = bytes[start..].iter().position(|byte| *byte == b'\n') {
        let end = start + relative_end;
        let line_tail = &bytes[start..end];
        let is_blank = line_buffer
            .iter()
            .chain(line_tail)
            .all(u8::is_ascii_whitespace);
        if is_blank {
            line_buffer.clear();
            *completed_event_bytes = 0;
            start = end + 1;
            continue;
        }
        append_event_bytes(
            line_buffer,
            line_tail,
            *completed_event_bytes,
            policy.event_bytes,
        )?;
        *completed_event_bytes = completed_event_bytes
            .checked_add(line_buffer.len())
            .and_then(|total| total.checked_add(1))
            .ok_or(SseIngestionError::EventLimit {
                limit: policy.event_bytes,
            })?;
        if *completed_event_bytes > policy.event_bytes {
            return Err(SseIngestionError::EventLimit {
                limit: policy.event_bytes,
            });
        }
        process_line(
            line_buffer,
            is_anthropic,
            policy.output_bytes,
            output_bytes,
            full_text,
            saw_thinking_delta,
            on_text,
        )?;
        line_buffer.clear();
        start = end + 1;
    }
    append_event_bytes(
        line_buffer,
        &bytes[start..],
        *completed_event_bytes,
        policy.event_bytes,
    )
}

fn append_event_bytes(
    buffer: &mut Vec<u8>,
    bytes: &[u8],
    completed_event_bytes: usize,
    limit: usize,
) -> Result<(), SseIngestionError> {
    let next_len = completed_event_bytes
        .checked_add(buffer.len())
        .and_then(|total| total.checked_add(bytes.len()))
        .ok_or(SseIngestionError::EventLimit { limit })?;
    if next_len > limit {
        return Err(SseIngestionError::EventLimit { limit });
    }
    buffer.extend_from_slice(bytes);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_line<F>(
    line: &[u8],
    is_anthropic: bool,
    output_limit: usize,
    output_bytes: &mut usize,
    full_text: &mut String,
    saw_thinking_delta: &mut bool,
    on_text: &mut F,
) -> Result<(), SseIngestionError>
where
    F: FnMut(&str),
{
    let line = std::str::from_utf8(line).map_err(|_| SseIngestionError::InvalidUtf8)?;
    let line = line.trim();
    if line.is_empty() || line.starts_with(':') {
        return Ok(());
    }
    let data = if let Some(data) = line.strip_prefix("data: ") {
        data
    } else if let Some(data) = line.strip_prefix("data:") {
        data.trim()
    } else {
        return Ok(());
    };
    if data == "[DONE]" {
        return Ok(());
    }

    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) else {
        return Ok(());
    };
    let text = if is_anthropic {
        let event_type = parsed.get("type").and_then(|value| value.as_str());
        let delta_type = parsed
            .get("delta")
            .and_then(|delta| delta.get("type"))
            .and_then(|value| value.as_str());
        if event_type == Some("content_block_delta") && delta_type == Some("thinking_delta") {
            *saw_thinking_delta = true;
        }
        if event_type == Some("content_block_delta") {
            parsed
                .get("delta")
                .and_then(|delta| delta.get("text"))
                .and_then(|value| value.as_str())
                .unwrap_or_default()
        } else {
            ""
        }
    } else {
        parsed
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("delta"))
            .and_then(|delta| delta.get("content"))
            .and_then(|value| value.as_str())
            .unwrap_or_default()
    };
    if text.is_empty() {
        return Ok(());
    }

    let next_output =
        output_bytes
            .checked_add(text.len())
            .ok_or(SseIngestionError::OutputLimit {
                limit: output_limit,
            })?;
    if next_output > output_limit {
        return Err(SseIngestionError::OutputLimit {
            limit: output_limit,
        });
    }
    *output_bytes = next_output;
    full_text.push_str(text);
    on_text(text);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;

    fn policy() -> StreamPolicy {
        StreamPolicy {
            wire_bytes: 512,
            event_bytes: 128,
            output_bytes: 64,
            idle_timeout: Duration::from_secs(5),
            total_timeout: Duration::from_secs(20),
        }
    }

    #[tokio::test]
    async fn fragmented_utf8_is_decoded_only_after_the_complete_line() {
        let event = "data: {\"choices\":[{\"delta\":{\"content\":\"中🙂\"}}]}\n\n";
        let emoji = event
            .as_bytes()
            .windows(4)
            .position(|part| part == "🙂".as_bytes())
            .unwrap();
        let chunks = vec![
            Ok::<_, ()>(event.as_bytes()[..emoji + 1].to_vec()),
            Ok(event.as_bytes()[emoji + 1..emoji + 3].to_vec()),
            Ok(event.as_bytes()[emoji + 3..].to_vec()),
        ];
        let mut emitted = Vec::new();
        let outcome = consume_sse_stream(stream::iter(chunks), false, policy(), |text| {
            emitted.push(text.to_string());
        })
        .await
        .unwrap();

        assert_eq!(outcome.full_text, "中🙂");
        assert_eq!(emitted, ["中🙂"]);
    }

    #[tokio::test]
    async fn no_newline_event_is_bounded() {
        let mut limits = policy();
        limits.event_bytes = 8;
        let error = consume_sse_stream(
            stream::iter([Ok::<_, ()>(b"data: 123".to_vec())]),
            false,
            limits,
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(error, SseIngestionError::EventLimit { limit: 8 });
    }

    #[tokio::test]
    async fn event_limit_counts_all_lines_until_the_blank_delimiter() {
        let mut limits = policy();
        limits.event_bytes = 15;
        let error = consume_sse_stream(
            stream::iter([Ok::<_, ()>(b": 12345\n: 67890\n\n".to_vec())]),
            false,
            limits,
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(error, SseIngestionError::EventLimit { limit: 15 });
    }

    #[tokio::test]
    async fn wire_and_decoded_output_have_independent_limits() {
        let mut wire_policy = policy();
        wire_policy.wire_bytes = 3;
        let wire = consume_sse_stream(
            stream::iter([Ok::<_, ()>(b": ping\n".to_vec())]),
            false,
            wire_policy,
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(wire, SseIngestionError::WireLimit { limit: 3 });

        let mut output_policy = policy();
        output_policy.output_bytes = 3;
        let output = consume_sse_stream(
            stream::iter([Ok::<_, ()>(
                b"data: {\"choices\":[{\"delta\":{\"content\":\"four\"}}]}\n".to_vec(),
            )]),
            false,
            output_policy,
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(output, SseIngestionError::OutputLimit { limit: 3 });
    }

    #[tokio::test(start_paused = true)]
    async fn idle_timeout_is_deterministic_under_paused_time() {
        let delayed = stream::once(async {
            tokio::time::sleep(Duration::from_secs(6)).await;
            Ok::<_, ()>(b": late\n".to_vec())
        });
        let error = consume_sse_stream(delayed, false, policy(), |_| {})
            .await
            .unwrap_err();
        assert_eq!(error, SseIngestionError::IdleTimeout { timeout_ms: 5_000 });
    }

    #[tokio::test(start_paused = true)]
    async fn total_timeout_wins_while_chunks_keep_beating_idle_timeout() {
        let chunks = stream::unfold(0, |index| async move {
            tokio::time::sleep(Duration::from_secs(4)).await;
            Some((Ok::<_, ()>(b": tick\n".to_vec()), index + 1))
        });
        let mut limits = policy();
        limits.total_timeout = Duration::from_secs(10);
        let error = consume_sse_stream(chunks, false, limits, |_| {})
            .await
            .unwrap_err();
        assert_eq!(
            error,
            SseIngestionError::TotalTimeout { timeout_ms: 10_000 }
        );
    }

    #[tokio::test]
    async fn invalid_utf8_line_is_typed_and_not_emitted() {
        let mut emitted = Vec::new();
        let error = consume_sse_stream(
            stream::iter([Ok::<_, ()>(vec![b'd', b'a', b't', b'a', b':', 0xff, b'\n'])]),
            false,
            policy(),
            |text| emitted.push(text.to_string()),
        )
        .await
        .unwrap_err();
        assert_eq!(error, SseIngestionError::InvalidUtf8);
        assert!(emitted.is_empty());
    }
}
