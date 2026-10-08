# Research: Optimization boundaries for small-file skill operations

- Query: Identify minimal D1–D4 edits, digest compatibility, progress bridging, same-shape callers, and isolated performance fixture boundaries.
- Scope: internal source and local specifications; no network or dependency changes.
- Date: 2026-10-08
- Evidence: `STATIC_ANALYSIS`. No production edit, benchmark, or runtime test was performed by this researcher.
- Ownership: this research file only. Other agents own the benchmark and production changes.

## Findings

### 1. Production seams and safety boundaries

| File / symbol | Verified responsibility |
| --- | --- |
| `src-tauri/src/services/github_import/snapshot_import.rs:38` `import_github_repo_skills_from_preview_with_branch` | Wizard entry; acquires the preview lease and consumes/releases it after the outcome. |
| `src-tauri/src/services/github_import/snapshot_import.rs:102` | Recomputes snapshot integrity before mutation. Retain this preflight. |
| `src-tauri/src/services/github_import/import.rs:506` `import_single_staged_skill` | Candidate validation, persisted uid choice, metadata, and journaled upsert call. |
| `src-tauri/src/services/central_updates/core/content_upsert.rs:40` `journaled_central_content_upsert_with_fs` | Content-upsert plan construction and the one production update batch. |
| `src-tauri/src/services/central_updates/core/batch.rs:46` `update_skills_batch` | Target mutation guard, selected-row recovery, durable prepared row, stage, swap, DB commit, copy refresh, finalize. |
| `src-tauri/src/services/central_updates/fs/operation/helpers.rs:3` `stage_local` | Operation-owned marker/staging creation, full contents write, fresh staging fingerprint. |
| `src-tauri/src/services/central_skills/delete.rs:586` `delete_central_skill_local_under_guard` | Explicit copy selection, symlink removals, canonical path validation, delete journal and DB/finalize ordering. |
| `src-tauri/src/services/central_operation/fs.rs:537` `fingerprint_path_blocking` | Byte-exact local delete/restore/finalize fingerprint. |
| `src-tauri/src/services/central_updates/inventory/leftover_cleanup.rs:707` | Local leftover ownership and guarded uninstall route. |

Keep `update_skills_batch` as the only apply orchestrator. Do not introduce a fast importer which bypasses its guard, selected recovery, journal, or transaction. `content_upsert_plan` keeps an existing Central uid (`content_upsert.rs:84–88`). First upsert rejects a target which appeared before preparation (`batch.rs:277–280`).

Fresh disk reads are required at mutable filesystem boundaries. Local stage verifies staging (`helpers.rs:29`); swap verifies staging and old target (`helpers.rs:43–63`); rollback verifies target/backup/staging (`helpers.rs:94–178`); finalize verifies new target and old backup (`helpers.rs:185–219`). A precomputed immutable snapshot hash must not replace these reads.

### 2. D1: write progress must originate after a successful write

Verified current behavior: `import.rs:526–548` validates each source path, increments `completed_files`/`completed_bytes`, and emits one `Writing` event before the journaled upsert at `import.rs:600–614`. Therefore the current counts describe prepared source entries, not completed file writes. `emit_github_import_progress` directly emits through `AppHandle` (`progress.rs:45–51`).

The exact Local success point is immediately after `std::fs::write` in `central_updates/fs.rs:191–196`. Failed writes must not increment counters. Parent creation and validation belong to preparation. Finishing all staging writes does not imply successful swap, DB commit, copy refresh, or cleanup.

Recommended local interface slice, subject to the measured event budget:

1. Add an internal lightweight stage-write progress value/reporter in `central_updates/fs` or its operation module. The value contains cumulative successful files/bytes and the latest skill-relative path. The worker receives only this data bridge, never an `AppHandle` or an event-emitting callback which captures an `AppHandle` indirectly.
2. Thread an optional reporter through a new internal `journaled_central_content_upsert_with_fs_and_progress` and `update_skills_batch_with_progress`, retaining existing no-progress wrappers for the other production callers. Both wrappers delegate to the same orchestration body. Alternatively add the optional argument directly and update all callers listed below; do not duplicate the Saga.
3. Pass the reporter through `stage_operation_updates` / the Local single-stage function / `stage_local` to `write_remote_skill_files`. All of these signatures are internal; public Tauri DTOs and commands need no change.
4. In the blocking writer, count every successful write, but publish only aggregate batches. A `tokio::sync::watch::Sender` is sufficient for one retained aggregate value. Publish after the agreed file/time threshold and always flush on writer success or error. Do not send one channel message per file.
5. The async importer owns a pinned upsert future and polls progress via `tokio::select!` plus an interval or equivalent sampling loop. The async importer calls `emit_github_import_progress`. When the upsert settles, read/emit the latest aggregate even on error before propagating the error. Do not spawn a detached observer which can outlive the import or its borrowed progress state.
6. Advance global `GitHubImportProgressState` by actual stage counts. Preserve ordered per-skill results, monotonic totals, and the final counts. Partial failure does not authorize filling the remaining counts to the total.

`tokio` already enables `macros`, `sync`, and `time` in `src-tauri/Cargo.toml`; this bridge needs no dependency. No existing production blocking-FS progress bridge was found. The existing `SnapshotProgressReporter = Arc<dyn Fn(SnapshotProgressEvent) + Send + Sync + 'static>` (`central_updates/snapshots/mod.rs:115`) and settled counters (`:621–654`) are a useful reporter/test pattern, but those callbacks run on the async snapshot side. Passing a callback which captures `AppHandle` into a blocking writer would violate the Windows constraint. `commands/projects.rs:52–67` has a dialog callback channel, but its blocking receiver does not provide a suitable streaming import bridge.

Remote compatibility: `github_import/remote.rs:378–384` currently uses a skill-count total and zero byte total, and passes `emit_per_file_progress=false` (`:405–417`). Move its one-skill completion increment to after successful service completion. A Local file reporter must not reinterpret those Remote totals or report archive construction as a remote write. Changing remote file-granular progress would require a real transport confirmation source.

Independent D1 tests should exercise the writer and a data-only async observer, with no real `AppHandle`: path validation failure gives zero writes; file N failure reports N−1 successful writes; a many-file fixture yields bounded aggregate publications; counts are monotonic; the final success/error flush is exact; preparing counts stay zero; observer completion leaves no pending source. Use `github_import/progress.rs` unit tests or a separate `github_import/progress/tests.rs` and existing `central_updates/fs/tests.rs` / `fs/operation/tests.rs` seams. Preserve existing preview-token, overwrite, and DB-rollback tests in `github_import/tests.rs`.

#### D1 executable baseline: count the actual source emitter

The existing service benchmark calls `journaled_central_content_upsert_with_fs` directly. That route does not call the per-file importer loop, so its event count cannot establish a source-event baseline.

The smallest executable baseline requires one test-only observation seam at `github_import/progress.rs:45–51`:

- Add a `#[cfg(test)]` thread-local optional handle to a data-only observation struct. The handle can be `Rc<RefCell<ImportProgressObservation>>`; a scoped guard replaces/restores the previous handle and uses a non-Send marker like the lock-path guard. Store counts by phase, final payload counters, monotonicity violations, and event sequence count. No Tauri object or event-emitting callback belongs in the handle.
- Immediately on entry to `emit_github_import_progress`, before `if let Some(app)`, record the payload under `#[cfg(test)]`. Keep the normal `AppHandle::emit` branch unchanged. A payload observed with `AppNone` is a source emission attempt, not an IPC delivery.
- Expose the test-only observer/guard through `#[cfg(test)] pub(crate) use progress::{...}` in `github_import/mod.rs`. The benchmark can then use the same scope and current-thread runtime as its isolated lock. These are proposed internal test symbols, not APIs which already exist.
- Execute `github_import::import_github_repo_skills_from_pinned_snapshot(pool, repo, forty_hex_commit, fixture_snapshot, one_selection, None)`. This function is already re-exported as `pub(crate)` (`github_import/mod.rs:75–79`) and calls the actual snapshot importer (`import.rs:170–210`). It performs no network request. Build a fresh target/pool so first-upsert paths and final disk/DB assertions remain deterministic.
- Keep observation setup and reading outside the timed service segment. A many-event source probe may use a separate sample from the lifecycle benchmark, so its observation overhead is not presented as production wall time. Record fixture files/bytes, actual observed phase counts, final counters, disk/DB result, and the exact executed test command/build identity.
- If the benchmark needs the wizard lease/integrity boundary too, place its narrow test wrapper inside `github_import` and use the registration pattern from `github_import/tests.rs:5100–5140`. This additional wrapper constructs `PreviewSnapshot` using existing repository inventory/candidate digest helpers, then calls the exported preview-with-branch importer. No new public DTO or generic runtime parameter is needed.

For the unchanged one-candidate T13016 pinned snapshot path, static source analysis predicts 13,017 `Writing` calls (initial zero + 13,016 entries), one `Finalizing` call, and zero `Preparing` calls: 13,018 source calls. This number is `STATIC_ANALYSIS_EXPECTATION` until the probe runs. A URL-based importer adds an earlier `Preparing`, so identify the entry point in every sample.

Report the observed result as `SOURCE_EVENT_COUNT_RUNTIME` with `app_handle = none`. Keep `ipc_emit_count`, serialization/dispatch duration, renderer update count, renderer queue depth, and native WebView latency as `NOT_RUN` / `UNVERIFIED`. Do not multiply the observed count by a guessed IPC cost. After D1 implementation, rerun the exact same source probe and compare the observed counts against the locked source-event budget; native event delivery remains a separate test.

Tauri mock runtime is not the minimal seam here. The current lockfile pins Tauri 2.11.5 (`src-tauri/Cargo.lock:5485–5486`). Installed source for that exact version exposes `tauri::test` behind `cfg(any(test, feature = "test"))` (`tauri-2.11.5/src/lib.rs:1098–1100`) and `mock_builder() -> Builder<MockRuntime>` (`src/test/mod.rs:165`). The service import signatures use the default Wry `AppHandle` (`github_import/import.rs:90`, `:515`); `AppHandle<MockRuntime>` does not match. Enabling the Tauri test feature and genericizing the production import chain solely for event counting would enlarge the change. No such dependency or runtime-signature change is recommended.

### 3. D2: ownership transfer before a broader shared-byte representation

Verified deep-byte clone chain:

| Source | Exact shape | Necessary action |
| --- | --- | --- |
| `central_updates/fs.rs:120–145` | `collect_remote_skill_files` clones selected `snapshot.files` byte vectors (`:133`). | One ownership conversion is currently needed because the snapshot remains retained. Avoid redesigning `GitHubRepoSnapshot` merely to remove this first copy. |
| `central_updates/core.rs:303–313` | `pending_updates.iter()` creates plans with `remote.clone()`. | Consume pending tuples, move `remote` into plans, retain only prepared/result metadata needed after the batch. |
| `central_updates/inventory/force.rs:88–99` | Force update repeats `remote.clone()`. | Same move pattern; preserve `before` state and request order. |
| `central_updates/inventory/force.rs:217–228` | Force mirror repeats `remote.clone()`. | Same move pattern; do not drop missing-source delete decisions or audit metadata. |
| `central_updates/core/batch.rs:261–265` | `prepare_update` clones `plan.remote.files` into `CentralSkillWrite`. | Move/take the files from a mutable plan after remote digests are available. |
| `central_updates/core/batch.rs:154–165` | Stage request construction clones each `update.write`. | Move/take the owned file list into `OperationUpdateStage` using `prepared.iter_mut()`. |
| `central_updates/fs/operation.rs:136–143` | Local single-stage method clones `write.files` into the blocking closure. | Accept an owned write or owned stage at this internal boundary; move its files into the closure. |
| `central_updates/fs/operation.rs:147–155` | Remote single-stage method clones `write` for archive construction. | The same owned argument can move into the archive stage. |
| `central_updates/fs/operation.rs:501–508` | Remote batch clones each archive chunk. | Move the chunk into the one blocking closure and return `(chunk, archive_result)` when async metadata is needed afterwards. Retain operation IDs separately for a join failure. |

Why taking `plan.remote.files` is safe in the current source: after preparation, result state uses digests/source/target metadata (`core/state.rs:246–277`); DB persistence uses candidate/source/provenance (`batch.rs:622–658`); journal insertion serializes the manifest and skill ID (`batch.rs:679–708`). None reads file bytes after staging. Search these consumers again after the implementation because other agents may change the code.

Minimal signature change: make the single-stage FS boundary consume `CentralSkillWrite` or `OperationUpdateStage`. `stage_operation_updates` already consumes `Vec<OperationUpdateStage>` (`operation.rs:203`), so its Local branch can pass owned data directly. Keep the manifest accessible on the async side for remote scripts, errors, and verification. Existing single-stage tests need owned fixture arguments; avoid adding a second non-journaled stage implementation.

Alternative if ownership transfers prove larger than the scoped edit: use an `Arc<Vec<RemoteSkillFile>>` at the `RemoteSkillContent.files` / `CentralSkillWrite.files` list boundary. Derived plan/stage clones then share one immutable list. `RemoteSkillFile.bytes` can remain `Vec<u8>`. This avoids changing repository snapshots or creating a new framework. It requires updating list literals/iteration in tests and is a tradeoff to choose after the baseline, not an additional required feature.

### 4. D2: digest reuse without mixing digest domains

Three incompatible encodings exist:

| Digest | Verified encoding | Compatibility rule |
| --- | --- | --- |
| Repository and candidate `sha256-v1` | `github_import/digest.rs:49–68`: domain length + domain bytes + record count; each record has path length/path/byte length/raw 32-byte SHA. Sort by UTF-8 path bytes. Repository and candidate domains differ (`:21–24`). | Reuse raw per-file records, then run each original aggregate domain. |
| Update `sha256-manifest` | `central_updates/fs.rs:545–555`: sorted skill-relative path, `0xff`, lowercase hex per-file SHA, `0xfe`. | Preserve delimiters and lowercase hex; cannot substitute candidate `sha256-v1`. |
| Local delete fingerprint | `central_operation/fs.rs:537–597`: root type prefix; sorted full `PathBuf` entries; OS-rendered relative path plus NUL, type prefix, then raw file bytes or symlink target. | Includes directory/symlink structure and OS path spelling. It cannot be derived from regular-file update/preview records. |

Current immutable repeated work: `candidate_content_digest_from_snapshot` (`snapshot.rs:145–153`) first calls `snapshot_files_from_local` for the entire repository (`:90–101`) and only then filters one candidate (`:155–178`). `core/state.rs:130–139` also hashes the selected files for the manifest. Multiple candidates therefore rehash unrelated repository bytes repeatedly.

Smallest useful change with the existing API: keep `candidate_content_digest_from_snapshot(snapshot, source_path) -> Result<String, GithubImportError>`, filter repository paths using `repo_file_relative_to_source` before computing `file_sha256`, then aggregate selected raw records. Preserve the existing exact `SKILL.md` requirement, empty/missing-manifest error, root `.` behavior, UTF-8 ordering, and domain framing. This removes whole-repository rehashing for a selected candidate across all current callers without changing a struct or adding a cache.

When measuring proves that the selected files themselves are repeatedly hashed, use the already exported `github_import::skill_content_digest_from_hashed_files(&[(String, u64, [u8; 32])]) -> String` (`github_import/mod.rs:70–72`, `digest.rs:88–98`). A local `central_updates/fs` helper can compute per-file raw SHA once and derive both the manifest hash and candidate digest from those records. Preserve the old missing-manifest error by validating the same exact relative path rule before candidate aggregation. Do not compare repository digest to a skill digest.

Exact operation-local metadata reuse option:

- `snapshot_files_from_local(&GitHubRepoSnapshot) -> Vec<PreviewSnapshotFile>` already creates the required path/size/raw-SHA inventory.
- `candidate_content_digest_from_repository_files(&[PreviewSnapshotFile], source_path) -> Result<String, GithubImportError>` already aggregates candidate digests without reading bytes again.
- Both functions and `PreviewSnapshotFile` are currently `pub(super)` to `github_import`. To reuse them in `central_updates`, visibility must become `pub(crate)` and the functions/type need deliberate `github_import/mod.rs` re-exports. Do not import private child modules through an alternate path.
- Pinned multi-candidate import (`import.rs:183–191`) can build that inventory once before its candidate loop and call the existing metadata aggregator for each candidate. The preview path already stores `PreviewSnapshot.files` and candidate digests (`preview.rs:389–405`); do not rebuild that cached metadata after `verify_snapshot_integrity` merely for provenance.
- The update cache already retains `Arc<CentralUpdateRepositorySnapshot>` (`snapshots/mod.rs:30–65`), but the wrapper currently stores only commit/digest/raw snapshot. Adding read-only inventory there can make candidate digests reuse acquisition work. Such a change should compute repository inventory and repository digest together at acquisition (`:443–456`) rather than adding a second full hash inside the constructor. The existing constructor accepts a supplied digest; preserve its identity/error contract and update test fixtures deliberately.
- A precomputed `RemoteSkillContent.remote_hash` can supply the immutable new manifest fingerprint only after the caller invariants are audited. `build_operation_update_manifest` currently recomputes `fingerprint_files(write.files)` (`operation.rs:108`, `helpers.rs:268–279`). Do not accept unchecked caller-provided values under a general API; current tests sometimes use synthetic remote hashes. Keeping this one recomputation is safer than weakening the manifest binding. All fresh disk validations remain mandatory.

Candidate-digest caller inventory (verified with repository search):

| Call site | Expected scope |
| --- | --- |
| `github_import/import.rs:188` | Pinned multi-candidate import. |
| `central_updates/core/state.rs:138` | Ordinary check/apply and force paths through `load_remote_skill_content`. |
| `marketplace/mod.rs:738` | GitHub-backed marketplace import. |
| `skills_cli/updates/apply.rs:219` | Local Skills CLI update plan verification. |
| `skills_cli/updates/apply/remote.rs:123` | Remote Skills CLI update verification. |
| `skills_cli/updates/detect.rs:283` | Skills CLI update detection. |

Changing the shared helper naturally benefits these callers. Do not rewrite their orchestration or network behavior. Existing tests include `digest_is_stable_and_independent_of_input_order`, `digest_domains_separate_repository_and_skill_content`, `digest_framing_prevents_path_boundary_collisions`, `digest_detects_content_and_length_tampering`, `repository_digest_ignores_hashmap_insertion_order`, `remote_inventory_digest_matches_the_local_snapshot_digest` in `github_import/tests.rs`. Add a selected-subtree-vs-old-aggregate assertion with unrelated large/binary files, root `.` and nested source paths, empty files, and missing/exact-case manifest behavior.

### 5. D3: create each exact parent once within owned staging

`write_remote_skill_files` creates staging once (`fs.rs:164–172`) and calls `create_dir_all(parent)` for every file (`:181–190`). A local `HashSet<PathBuf>` of successfully created exact parents is sufficient. Seed the staging root after its successful creation. Check each relative path before consulting the set. Add a parent only after its create succeeds. Do not cache across operations or unknown roots, and do not mark descendant parents created merely because an ancestor exists.

This slice benefits initial import, overwrite, and update because they all call the same stage writer. It has no direct benefit to the OS recursive delete or directory copy. `installation/fs_util.rs:283–334` already creates the destination once per recursive directory, not once per file; do not add the same optimization there without evidence.

Required assertions: many sibling files yield one parent mkdir; nested parents remain correct; unsafe paths still fail before writes; a parent/file collision still produces the existing typed IO error; final content remains byte exact. A small test-only count/probe is sufficient. Keep writer order because reordering changes the first error and current-path semantics.

### 6. D4: preserve fresh fingerprint, optimize its implementation

Minimal verified inefficiency: `central_operation/fs.rs:564` uses `entries.sort_by_key(|entry| entry.path().to_path_buf())`. The key closure allocates a `PathBuf` during comparisons. Use `entries.sort_by(|left, right| left.path().cmp(right.path()))`; the comparison order stays identical and no key path is allocated. This changes no read, marker, fingerprint framing, manifest version, or recovery behavior.

`hash_file` already streams through a 64 KiB buffer (`fs.rs:599–612`); deletion does not allocate an entire file for hashing. A benchmark-supported second option is to allocate/zero that buffer once per `fingerprint_path_blocking` and pass it to each `hash_file`, preserving reads/errors exactly. Whether buffer initialization materially costs time on this build is untested.

Update local hashing currently allocates a fresh whole-file `Vec` (`central_updates/fs.rs:532–539`). A streamed implementation can preserve each file's SHA and manifest framing, but may add an EOF read and affect small-file time. Measure both small-file and large-file fixtures before selecting it. `hash_local_directory` also sorts entries (`:315`) before `hash_entries` sorts again (`:546`); removing the redundant first sort keeps byte-exact output.

Do not immediately replace the delete whole-tree collection with `WalkDir::sort_by_file_name` streaming. Sorted depth-first traversal must first be proven equal to the old global `PathBuf` ordering on Windows and Unix, including a directory `a` alongside `a.ext`, mixed case, Unicode/non-UTF-8 names, empty directories, and symlinks. The old collection visits the whole tree before file hashing; a streaming walk also changes which error occurs first when a later walk error and earlier read error coexist. Keep the old collect/error ordering for the initial slice.

Local/Remote delete manifests already deduplicate physical paths (`central_operation/fs.rs:126–153`, Remote equivalent). Remote leftovers already group unique paths and batch 256 (`leftover_cleanup.rs:30`, `:383` onward). Do not add another planning layer. Local leftover repeated requests still repeat ownership/DB work, but changing shared-root cleanup/report semantics requires its own measured justification.

Regression seams: `central_operation/fs_hash_tests.rs` contains a byte-exact file fingerprint/path token vector; `central_operation/fs.rs` tests cover idempotent stage/restore/finalize, directory-symlink target preservation, and collision evidence; `central_updates/fs/operation/tests.rs` covers Local update restore/finalize, tampered staging, and real subprocess-kill phase recovery. Add an old-algorithm-versus-new-algorithm directory fingerprint test for D4; a file-only fixed vector does not prove directory ordering compatibility.

### 7. Fixture isolation and formal limits

Verified default guard behavior: `CentralFs::Local` returns target ID `local` (`operation.rs:40–44`). `target_mutation_lock_path(Local)` uses `paths::central_mutation_lock_path` even under `cfg(test)` (`central_mutation/mod.rs:116–119`). Only Remote target lock directories are isolated under `cfg(test)` (`:138–145`). The test mutex (`:90–113`) serializes access but does not isolate that default Local file. Redirecting agent DB roots alone therefore does not isolate all file writes.

Main authorized the baseline agent's minimal `cfg(test)` scoped thread-local lock-path override. Resolve and validate the temporary lock path before `spawn_blocking`; keep the benchmark on the current-thread runtime; restore the previous override in Drop. This preserves the OS lock behavior and avoids process-wide home environment changes. The production paths must remain unchanged.

For the baseline's named Local routes, all remaining mutation paths are explicit pool/manifest paths: content upsert gets its target directory explicitly; check/apply gets canonical/agent paths from the fixture pool and a prefilled cache; delete gets pool canonical/installation paths (`delete.rs:593–645`); leftover gets redirected agent roots and calls native uninstall (`leftover_cleanup.rs:707–755`, `installation/native.rs:143–161`); recovery uses journal manifests. No additional direct `app_data_dir` / legacy-home / Skills CLI lock access was found along those routes. Seed repository owner/repo/branch/source explicitly so update source resolution does not take a network fallback.

All agents, canonical paths, install paths, symlink links/targets, DB, operation siblings, and cleanup targets must stay under the validated fixture root. Use existing `test_support::mem_pool` / `file_pool`, `set_agent_dir`, skill fixture constructors, and symlink helpers. A pool seed may contain normal home-derived paths until every agent is redirected; do not run inventory scanning before that redirect.

Baseline caveat: Central deletion cascades `skill_installations` according to the seven-relation deletion contract. Native uninstall defaults a missing installation record to `symlink` (`native.rs:154–156`) and refuses a real untracked copy directory. The existing leftover tests deliberately seed a non-Central skill and copy-install rows (`leftover_cleanup/tests.rs:86–127`). Preserve separate fixtures/evidence for retained-copy delete and the valid tracked-leftover service path. Do not treat a baseline failure after delete as authorization for an adjacent cleanup fix.

## Related specifications

- `.trellis/spec/backend/spawn-blocking-io.md`: one blocking unit, no byte-batch pre-clone, no `AppHandle` capture, preserve error order and real Windows test-binary loading.
- `.trellis/spec/backend/fs-db-operation-journal.md:83–105`: journal ordering, fresh marker/fingerprint checks, uid compatibility, selected recovery, synchronous settle/pending state.
- `.trellis/spec/backend/central-update-batching.md`: single batch orchestrator, bounded Remote archive/copy/leftover batching, selected recovery and duplicate ownership.
- `.trellis/spec/backend/skill-deletion-integrity.md:13–25`: seven owned DB relations and independent observation/history state.
- `.trellis/spec/backend/github-import-preview-contract.md`: immutable preview binding, resource budgets, no branch re-fetch fallback.
- `.trellis/spec/backend/test-support.md`: pool and directory fixtures, crash/Remote harness, real default-lock test mutex.
- `.trellis/spec/backend/layer-boundaries.md`: services call owning DB repos; no command/second-repository framework.
- `docs/agents/security-and-shared-state.md`: shared service boundary, SecretStore, blocking IO, safe durable state.

## External references

None fetched. Local `src-tauri/Cargo.toml` supplies the existing Tokio `sync`/`time`/`macros` capabilities and `walkdir = "2"`. Tauri test/mock signatures were checked against the locally installed source of lockfile-pinned Tauri 2.11.5. No network-current claims are made.

## Caveats / Not Found

- Performance gains, event queue costs, buffer cost, and phase dominance remain runtime hypotheses until the baseline and paired after samples exist.
- No native WebView event/render validation, packaging, real SSH/WSL, or full CI was run by this researcher.
- A production blocking-FS streaming progress bridge was not found; the snapshot reporter is a compatible data-callback pattern, not an existing worker bridge.
- The repository has `CONTEXT.md` for domain vocabulary; `GLOSSARY.md` and `GLOSSARY-MAP.md` were not found. No missing glossary was created.
- Line anchors identify the source inspected during the research. The baseline agent concurrently added a test module to `central_updates/core.rs`; after that file changes, use the named symbols/search strings to resolve shifted anchors.
