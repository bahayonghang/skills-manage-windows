# Final Cross-Child Acceptance

Date: 2026-08-03
Baseline: `dev@b242ed92`
Planning baseline: `2e369281`

## 1. Delivery Inventory

| Child | Work commit | Archive commit | Journal commit |
| --- | --- | --- | --- |
| Marketplace Central contract | `a52591c9` | `a7569148` | `1835aa49` |
| GitHub snapshot lifecycle | `c2aaea06` | `9052fce6` | `00cba2a4` |
| Bounded external ingestion | `c126b3cf` | `bb9d8697` | `5e5fce2d` |
| Transactional metadata mutations | `1dfb7068` | `ab668434` | `798da592` |
| SQL Central pagination | `dc7aa090` | `a8e87444` | `4251c334` |

All five children are archived. Each journal entry references only its work
commit, not its archive or bookkeeping commit.

## 2. Cross-Child Acceptance Matrix

| Requirement | Final evidence | Result |
| --- | --- | --- |
| Marketplace path and install authority | `install_marketplace_pinned_snapshot` derives the target from the sanitized candidate ID and calls `journaled_central_content_upsert`. The structural test rejects the old direct URL/display-name writer. Malicious-name and complete-directory tests cover Local, Fake SSH, and Fake WSL. | PASS |
| Durable first-import Saga | The generalized `central_update` path uses `UpdateManifest(had_target=false)` and the existing target mutation guard, pending-operation recovery, FS stage/swap, DB/provenance transaction, rollback, and finalization. No schema or operation-kind fork was added. | PASS |
| Snapshot/cache lifecycle | Central update snapshots use `Arc`, 8 entries, 256 MiB aggregate, TTL, and deterministic LRU. Preview snapshots use per-target/global entry and byte caps, reservation, exclusive import lease, `CleanupPending`, owning-target cleanup tickets, and acknowledgement before removal. | PASS |
| Bounded external ingestion | The final inventory maps scoped HTTP, SSE, Local, SSH, and WSL reads to incremental byte limits. SSE has separate wire/event/output caps plus idle and total deadlines. UTF-8 boundary tests pass; remaining broad reads are reviewed domain/tooling exemptions. | PASS |
| Transactional mutations | Nine scoped metadata/cache APIs validate before mutation and use one outer transaction or verified FK cascade. Later-row, later-chunk, status, and deferred-commit failures restore the prior state. Marketplace sync atomically replaces a registry snapshot and removes stale rows. | PASS |
| SQL pagination | SQLite owns filter/count/order/limit/offset in one read snapshot. Literal `%`, `_`, and backslash, source/tag/install aliases, shared-root state, stable ID ties, and persisted timestamp authority match the reference oracle. The 5,000-row fixture enriches 25 rows; relation reads chunk at 500 IDs. | PASS |

## 3. Removed Legacy Paths

- Marketplace production install code contains no `central_skill_dir_for_name`,
  direct registry `download_url` request, `std::fs::write`, or remote
  `write_file`. The cache URL remains DTO data only.
- Scoped response and skill-text readers no longer use response `.text()`,
  response `.bytes()`, generic remote `read_file`, or unbounded
  `read_to_string`. Remaining search hits are the shared bounded readers or the
  documented ingestion exemptions.
- Central page production code no longer calls `get_central_skills_impl`; it
  enriches repository page rows with `TimestampAuthority::Persisted` and does
  not stat the filesystem.
- The scoped metadata/cache mutations no longer expose multi-statement pool
  loops without an outer transaction. Marketplace success deletes and inserts
  cache rows inside the same transaction before updating success metadata.

## 4. Integration Review

### Lock, transaction, and cleanup order

- Marketplace acquisition and candidate validation happen before Central
  mutation. `update_skills_batch` acquires one target mutation guard, performs
  pending recovery, then stages and commits each journaled update.
- DB failure paths release/rollback the SQL transaction before filesystem
  rollback; the dedicated batch regression pins this ordering. No scoped
  repository helper opens a nested transaction or returns to the pool while an
  outer transaction is active.
- Preview registry locks only transition in-memory ownership. Remote cleanup is
  executed from owning-target tickets; acknowledgement removes the entry only
  after successful cleanup. Active leases are never eviction victims.

### Shared constants and module ownership

- `ResourceBudget` remains the shared per-skill file/tree authority. Bounded
  HTTP and SSE policies live in `bounded_ingestion` / `ai_provider::sse`.
- Snapshot-cache limits remain owned by their distinct Central-update and
  preview-registry lifecycle modules; retained-byte accounting is checked.
- SQLite writes use the shared 900-bind budget. Read-side relation enrichment
  uses the single 500-ID `SQLITE_IN_QUERY_BATCH_SIZE` constant.
- No duplicate Marketplace writer, response reader, snapshot cache, pagination
  evaluator, or transaction framework was introduced.

### Errors, redaction, and shared entry points

- Marketplace, GitHub import, bounded ingestion, and Central page validation
  remain typed service errors. Command mapping emits stable IPC codes and
  redacted messages; URL, path, response body, and synthetic secret tests pass.
- Tauri commands remain adapters over services. The generalized content-upsert
  boundary is target/DB based and independent of `AppState`; Local, SSH, and WSL
  callers share it. `cli_api` continues to call services/repositories rather
  than command modules.
- IPC command names and DTO shapes did not change. Schema migrations were not
  added. `docs:gen:check` confirms both generated architecture artifacts are
  current; active backend specs and task architecture docs describe the final
  behavior.

## 5. Validation Evidence

- Every child passed its focused acceptance tests, Rust format, all-targets
  locked Clippy, full locked Rust tests, and Node 22 `just ci` before archive.
- Final SQL child totals: pagination `9 passed / 1 ignored` benchmark; full Rust
  `1125 passed / 7 ignored`; frontend `1609 passed / 1 skipped`.
- Release fixture: pre-change `p50=163,868 us`, `p95=167,783 us`, 5,000 enriched;
  final recorded SQL run `p50=25,342 us`, `p95=31,442 us`, 25 enriched. Repeated
  checker runs also passed. Timing is evidence, not a CI threshold.
- Node `22.23.2`, pnpm `10.12.3`, and Rust `1.97.0` were confirmed by
  `just doctor`. Final Node 22 `just ci` passed after all child code/spec edits.
- `just audit` passed after the dev-only Tokio `test-util` feature change: two
  blocking advisories are covered by two existing exact approved exceptions.

No live external SSH host, WSL distro, or manual GUI walkthrough was required
by the child contracts. Windows execution plus exact Fake SSH/WSL transport
tests provide the required parity evidence; IPC/DTO and frontend behavior are
unchanged. There is no remaining acceptance gate.
