# Research: Repeated GitHub snapshot transport failures

- Query: Why do bypass-cache refreshes persist `github_import.transport_failed` for `github:anthropics-skills-main`, `github:bahayonghang-my-ai-cli-toolkit-main`, and `github:tw93-kami-main`; is the cause redirect/HTTP policy, concurrency/resource exhaustion, URL handling, or true network failure; and what fast offline harness can distinguish them?
- Scope: internal
- Date: 2026-08-09

## Findings

### Read-only live evidence

- The latest persisted skills/regular inventory was generated at `2026-08-09T05:16:20.286508+00:00` with `cache_policy=bypass`. All three named repositories have `bucket=failed_repository`, `cache_hit=0`, no snapshot timestamp, `errorCode=github_import.transport_failed`, and the fixed public message. No raw transport or HTTP detail is persisted.
- Successive bypass refresh Operation Logs show changing partial outcomes and long tails: `05:12:27Z` took 58.869 s with 4 failed repositories, `05:14:46Z` took 41.371 s with 2, and `05:16:20Z` took 69.272 s with 3. The Operation Log stores only the count, not repository/error subfamilies.
- The same stored repository identities are structurally normal GitHub `main` refs. `github:bahayonghang-my-ai-cli-toolkit-main` was successfully synchronized at `05:13:30Z`; `github:tw93-kami-main` produced an `updatable` row at `01:18:15Z`; `github:bahayonghang-my-ai-cli-toolkit-main` produced a repository-backed removal decision in the same earlier run. These prior successes make a deterministic malformed URL unlikely.
- Evidence was read through SQLite URI `mode=ro`. Source URLs, credentials, response bodies, raw request errors, and repository contents were not read or emitted. No network request was made.

### Snapshot and download path

1. A bypass inventory maps directly to `SnapshotCachePolicy::Bypass` (`src-tauri/src/services/central_updates/inventory/mod.rs:677-681`). In snapshot preparation, bypass skips `cache.get_fresh` and places every deduplicated repository into the download list (`src-tauri/src/services/central_updates/snapshots/mod.rs:411-430`).
2. Bulk acquisition uses one shared reqwest client, a semaphore of four, and one `download_repo_snapshot` future per missing repository (`src-tauri/src/services/central_updates/snapshots/mod.rs:24-26`, `:432-484`). `join_all` preserves all successes and per-repository failures, which inventory then persists independently (`src-tauri/src/services/central_updates/snapshots/mod.rs:486-501`; `src-tauri/src/services/central_updates/inventory/mod.rs:222-249`).
3. `download_repo_snapshot` downloads the API tarball and then parses it (`src-tauri/src/services/github_import/archive.rs:78-99`). The initial request tries the direct GitHub API and three built-in mirrors (`src-tauri/src/services/github_import/types.rs:464-484`) using endpoint-policy validation before transport (`src-tauri/src/services/github_import/raw_http.rs:248-296`, `:441-483`).
4. The shared production client has a 5-second connect timeout, 30-second total request timeout, system proxy behavior, and global redirect following disabled (`src-tauri/src/services/github_import/pat.rs:8-9`, `:41-57`). Redirects are handled explicitly only by the archive wrapper.
5. Archive 301/302 policy failures return `GithubImportError::ArchiveRedirectRejected`, including an untrusted numeric 301, malformed Location, or a second codeload redirect (`src-tauri/src/services/github_import/archive.rs:201-239`, `:274-306`). That variant maps to `github_import.archive_redirect_rejected`, not `transport_failed` (`src-tauri/src/services/github_import/error.rs:362-370`). Therefore the observed code rules out the explicit deterministic redirect-rejection path.
6. URL/ref validation happens before requests. Invalid structured components return `InvalidRepoComponent`; endpoint-policy violations return `InvalidUrl` (`src-tauri/src/services/github_import/raw_http.rs:146-183`, `:248-296`). `InvalidUrl` has its own `github_import.invalid_url` code (`src-tauri/src/services/github_import/error.rs:369-370`). The tarball URL is built from validated owner/repo/branch and fixed endpoint bases, not from `normalized_url` (`src-tauri/src/services/github_import/archive.rs:101-123`). This rules out normal renderer URL handling as the direct source of the three `transport_failed` rows.

### Where evidence is lost

`github_import.transport_failed` does not mean only a socket/network failure:

- `GithubImportError::Http(String)` maps wholesale to `github_import.transport_failed` (`src-tauri/src/services/github_import/error.rs:362-365`).
- A direct/mirror send timeout, connect, request, or body error is summarized and stored as `Http`; after all endpoints fail, their attempt summaries are also collapsed to `Http` (`src-tauri/src/services/github_import/raw_http.rs:566-597`, `:614-635`).
- An ordinary non-2xx response that is not 401/403/429/404 and is not accepted for fallback becomes the same `Http` variant (`src-tauri/src/services/github_import/raw_http.rs:483-564`). Repeated 5xx responses across every endpoint also end as `Http`.
- A terminal codeload non-success response becomes `Http`, and a response-body read failure is replaced with a fixed `Http` value (`src-tauri/src/services/github_import/archive.rs:243-271`).
- `failed_repository_reason` deliberately discards the `Display` detail and persists only the public message plus stable code (`src-tauri/src/services/central_updates/inventory/mod.rs:684-698`). The inventory schema has no attempt class/status/endpoint-family field.

The live database and Runtime Log therefore cannot distinguish: DNS/connect/TLS/proxy failure, request timeout, response-body failure, deterministic non-special HTTP status, or all-mirror 5xx exhaustion. A definitive root-cause claim would exceed the available evidence.

### Ranked falsifiable hypotheses

1. **Transient network/intermediary or upstream HTTP failure (most likely).** The long 41-69 s tails align with the 30 s per-request timeout plus fallback work; failure counts change between otherwise identical bypass runs; and two affected refs succeeded earlier. Falsifier: a sequential single-repository offline/live-authorized probe consistently returns the same non-special HTTP status or typed policy failure with no connect/timeout errors.
2. **Concurrency or shared-client resource pressure (plausible, not demonstrated).** Bypass forces every repo to re-download and four run concurrently. `join_all` also retains every completed snapshot until all futures settle, so memory retention spans the full round (`src-tauri/src/services/central_updates/snapshots/mod.rs:432-501`). Against this hypothesis, active network concurrency is bounded at four, the cache is bounded to eight entries/256 MiB (`src-tauri/src/services/central_updates/snapshots/mod.rs:24-26`, `:140-223`), and a resource-budget breach has its own `github_import.budget_exceeded` code. Falsifier: the same endpoint fixture or authorized repository set fails at concurrency four but passes repeatedly at concurrency one, with transport-class rather than budget errors.
3. **Deterministic ordinary HTTP status or mirror policy (possible because of mapping loss).** Non-special 4xx and all-endpoint 5xx are mislabeled `transport_failed`; the persisted code cannot exclude them. Falsifier: preserve a static HTTP status family at the error boundary and observe only connect/timeout/body classes for the failing repos.
4. **Deterministic archive redirect policy (strongly disfavored).** Explicit 301/302 rejection maps to `github_import.archive_redirect_rejected`, a code not present in these rows. Falsifier: an offline fixture producing the exact redirect chain unexpectedly maps to `transport_failed`; existing code/tests predict it will not.
5. **Repository URL/ref handling (strongly disfavored).** Structured refs pass validation, tarball authority is built from fixed endpoints, invalid endpoint URLs map separately, and affected repositories have prior successful results. Falsifier: calling the pure validators/URL builder with the persisted owner/repo/branch returns an error before transport.

### Fast red-capable offline harness

Use the existing local HTTP seam rather than the live repositories:

- Archive test seam: `download_repository_archive_with_test_endpoints` accepts test endpoint definitions and a local redirect authority (`src-tauri/src/services/github_import/archive.rs:136-173`).
- Existing TCP fixtures already prove no-auto-redirect and sanitized transport summaries (`src-tauri/src/services/github_import/tests.rs:1178-1205`, `:1279-1303`). Redirect chain fixtures begin at `src-tauri/src/services/github_import/tests.rs:1519`.

The smallest diagnostic regression should add a table-driven `github_import/tests.rs` test with a `.no_proxy()` client and sub-second timeout:

1. Local endpoint returns terminal `503` for every built-in test endpoint.
2. Local endpoint accepts then closes/reset the connection for every endpoint.
3. Local endpoint returns a valid 302 followed by a delayed or truncated codeload body.
4. Local endpoint returns a policy-invalid Location.

The desired assertions are distinct typed subfamilies before public redaction: `HttpStatus`, `Transport` (connect/timeout/request), `Body`, and existing `ArchiveRedirectRejected`. The current implementation is red because cases 1-3 all become `GithubImportError::Http` and therefore the same `github_import.transport_failed`; only case 4 remains distinct. The public UI may still group retryable network families, but Operation/Runtime diagnostics need a static non-sensitive subcategory to make the cause falsifiable.

To test the concurrency hypothesis, add a `#[cfg(test)]` variant of `prepare_snapshots_for_repo_refs_collecting_failures` that accepts a downloader closure and concurrency limit. A local server tracks active requests and deliberately fails requests above one concurrent connection. Run the same four repositories at limit 4 and limit 1, asserting both the peak active count and outcomes. This is fast, deterministic, and offline; the current hard-coded production downloader/concurrency at `snapshots/mod.rs:432-461` prevents this experiment without that narrow injection seam.

At the inventory boundary, extend `refresh_snapshot_failure_settles_the_repository_and_keeps_the_run` (`src-tauri/src/services/central_updates/inventory/tests.rs:810-876`) to assert the new static failure subcategory survives persistence/reload while raw URL/status/body data does not. That is the red regression for the current evidence-loss defect; it should not log raw `reqwest::Error` strings.

## Files Found

- `src-tauri/src/services/central_updates/inventory/mod.rs` - bypass mapping, snapshot acquisition, and lossy failed-repository persistence mapping.
- `src-tauri/src/services/central_updates/snapshots/mod.rs` - cache policy, concurrency four, shared client clones, `join_all`, and per-repository outcomes.
- `src-tauri/src/services/github_import/archive.rs` - archive acquisition, explicit redirects, codeload requests, HTTP/body mapping, and test endpoint seam.
- `src-tauri/src/services/github_import/raw_http.rs` - endpoint policy, mirror fallback, transport classification, and sanitized attempt summaries.
- `src-tauri/src/services/github_import/pat.rs` - production client timeouts and redirect policy.
- `src-tauri/src/services/github_import/types.rs` - built-in endpoint list and shared client state.
- `src-tauri/src/services/github_import/error.rs` - stable code mapping that collapses every `Http` cause to `transport_failed`.
- `src-tauri/src/services/central_updates/error.rs` - delegation of GitHub failure codes to the repository-snapshot phase.
- `src-tauri/src/db/schema/metadata.rs` - inventory persistence fields; no transport subfamily is stored.
- `src-tauri/src/services/github_import/tests.rs` - local TCP/redirect/transport harnesses.
- `src-tauri/src/services/central_updates/snapshots/tests.rs` - partial snapshot acquisition tests, but no injected downloader/concurrency harness.
- `src-tauri/src/services/central_updates/inventory/tests.rs` - persisted failed-repository test ownership.

## Related Specs

- `.trellis/spec/backend/github-import-preview-contract.md` - fixed HTTPS authorities, no global redirects, finite archive redirects, budgets, and required local HTTP fixtures.
- `.trellis/spec/backend/redaction-policy.md` - static GitHub codes and prohibition on persisting URLs, response bodies, or raw transport errors.
- `.trellis/spec/backend/domain-error-enums.md` - typed error families must remain inspectable without parsing Display text.
- `.trellis/spec/backend/test-support.md` - deterministic local fixtures and shared test support.
- `.trellis/spec/quality/test-suite-layout.md` - archive redirect regression ownership and focused test command.

## External References

- `src-tauri/Cargo.toml:41` selects reqwest `0.12` with `json`, `rustls-tls`, and `stream`.
- `src-tauri/Cargo.lock:3996-3999` locks the product dependency to reqwest `0.12.28`.
- No external network documentation was required; behavior claims above cite the repository's configured client and error mappings.

## Caveats / Not Found

- The current persisted/logging contract intentionally removes raw request detail. That is correct for secrets and URLs, but it also removes the static cause class needed to decide among timeout/connect/body/HTTP-status hypotheses.
- Earlier refresh rows with the same inventory ID are replaced transactionally, so Operation Logs retain only failure counts, not the exact failed repository set for each attempt. The user's observation identifies the repeated repositories; the database confirms the latest three, not every overwritten intermediate set.
- No live GitHub request, proxy change, cache mutation, refresh, retry, or filesystem write was performed. True network versus deterministic ordinary HTTP status remains unverified until the offline taxonomy harness exists or a separately authorized live probe records only safe static attempt classes.
