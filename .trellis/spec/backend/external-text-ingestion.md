# Bounded External Text Ingestion Contract

## 1. Scope / Trigger

Apply this contract when runtime code reads an HTTP response, SSE stream,
`SKILL.md`, arbitrary skill file, or remote file whose bytes are not already
retained under a stronger immutable snapshot/archive budget.

## 2. Shared Mechanisms

- `services::bounded_ingestion` owns HTTP/local incremental readers and UTF-8
  boundary helpers. Domain modules choose limits and map `BoundedReadError` to
  typed domain errors.
- HTTP readers reject an oversized `Content-Length`, then checked-add every
  chunk before appending. They stop as soon as the next chunk exceeds the cap.
- Local readers open first, inspect the opened file, then use
  `Read::take(limit + 1)` so growth after metadata cannot bypass the limit.
- `ConnectedRemoteTarget::read_file_bounded(path, max_bytes)` performs remote
  `wc -c` preflight and reads at most `max_bytes + 1`. SSH and WSL use the same
  shell contract and a supervisor stdout cap of exactly `max_bytes + 1`.
- Path containment remains the caller's responsibility and must complete
  before an arbitrary remote file is read.

## 3. Production Policies

| Input | Limit / deadline |
| --- | --- |
| Main or arbitrary `SKILL.md`, scanner frontmatter, AI tagging fallback | 1 MiB |
| Git tree response | 16 MiB |
| Git archive response | 128 MiB |
| Git repository file | signed manifest size, never above the 32 MiB entry cap |
| AI finite success body | 1 MiB |
| AI diagnostic body | 64 KiB |
| AI finite response headers / body | 30 seconds / 30 seconds |
| AI SSE wire / event / decoded output | 4 MiB / 256 KiB / 1 MiB |
| AI SSE idle / total | 30 seconds / 5 minutes |

Remote scanner reads at most four 1 MiB files per process. This keeps the
aggregate payload below the standard 8 MiB process cap while retaining the
existing batched transport shape.

## 4. Contracts

- SSE parsing keeps bytes until a full line is available; UTF-8 split across
  chunks must decode normally. No-newline buffers, wire bytes, and decoded
  output are independently bounded.
- Every SSE `next()` has an idle deadline and the whole stream has an absolute
  total deadline. Failure does not write a partial cache entry or emit a
  complete event.
- Character-count truncation uses Unicode scalar values. Byte-count summaries
  move backward to a valid UTF-8 boundary.
- Public errors never include response bodies, URLs, credentials, absolute
  paths, remote stderr/stdout, or file content. Auth, rate-limit, timeout,
  budget, UTF-8, and transport categories remain distinguishable.
- Provider response bodies may inform a typed classification, but their text
  never enters a public error or connection-test result.
- The generic unbounded remote `read_file` primitive remains only for reviewed
  domain-specific exemptions such as usage history. New skill/repository reads
  must use `read_file_bounded`.

## 5. Tests Required

- Chunked HTTP body crosses the cap and proves the producer was dropped before
  EOF.
- Local file grows after metadata; remote reports a small size but returns
  `limit + 1`; invalid UTF-8 returns a typed error without content leakage.
- Local, SSH, and WSL main/arbitrary skill reads have size and UTF-8 parity.
- SSE covers fragmented UTF-8, no-newline event overflow, wire/output limits,
  paused-time idle timeout, paused-time total timeout, and success sequencing.
- Scanner covers oversized and invalid local files plus bounded remote batch
  script construction.
- Run focused domain tests, locked all-target Clippy/tests, and `just ci`.
