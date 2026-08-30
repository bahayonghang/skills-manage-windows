# Research: Frontend risk-driven test coverage

- Query: Analyze the React/TypeScript frontend, Zustand stores, IPC/runtime adapters, repository scripts, and quality contracts for meaningful untested business logic, with emphasis on failures, boundaries, credentials, target authority, and parameter validation.
- Scope: internal
- Date: 2026-08-30

## Findings

### Executive result

This is a semantic risk review, not a line-coverage exercise. The repository has broad renderer coverage, but the highest-risk remaining gaps cluster around multi-step mutations whose first backend write can succeed before a later refresh/save fails. Those branches can leave the UI in a misleading state or make a retry repeat a mutation. Five focused additions are justified; the already-dense IPC normalization/redaction tests should not be padded merely to increase coverage.

### 1. Critical: portable-state import has no regression for a successful mutation followed by refresh failure

- Production flow: `importSkillportState` marks the portability job running and catches failure from `import_skillport_state` itself (`src/stores/centralSkillsStore.updateSlice.ts:584-616`), but the four post-import refreshes run outside that catch (`src/stores/centralSkillsStore.updateSlice.ts:617-622`). If one refresh rejects after the import committed, the action rejects while the job can remain `running`; the terminal state write at `src/stores/centralSkillsStore.updateSlice.ts:623-636` is skipped.
- Business risk: this is a restore/mutation boundary. A user may see an indefinitely active job and retry an import that already changed Central data. It also violates the store-state/error responsibility described in `.trellis/spec/frontend/async-error-feedback.md:11-18`.
- Existing-test mapping: the store test proves only the all-success route and refreshed collections (`src/test/stores/centralSkillsStore.test.ts:1765-1811`). Portability cancel and busy-state tests exist (`src/test/stores/centralSkillsStore.test.ts:1622-1681`), but there is no rejection after `import_skillport_state` succeeds.
- Focused tests to add:
  - Reject `get_central_skills` (and preferably one table-driven case for each refresh command) after a successful import; assert a terminal failed state, cleared active/busy state, preserved job ID, stored error, and rethrow.
  - Resolve the mutation, call `resetForTargetChange`, then settle the old refresh; assert every post-await write remains generation/job correlated. The job contract requires stale completion/error writes to be ignored (`.trellis/spec/frontend/job-correlation-cancellation.md:30-45`).
- Focused command: `pnpm exec vitest run src/test/stores/centralSkillsStore.test.ts -t "portable state"`

### 2. Critical: AI credential/settings persistence lacks partial-failure and overlapping-save tests

- Production flow: `flushAiSettings` can persist the API key first and then persist ordinary settings (`src/stores/settingsStore.aiSlice.ts:401-430`). A `set_settings` rejection after `set_ai_api_key` succeeds enters the generic catch (`src/stores/settingsStore.aiSlice.ts:444-452`), leaving the renderer's `aiSettings.apiKey` populated. The sequence number guards only Zustand writes (`src/stores/settingsStore.aiSlice.ts:431-451`); it does not prevent an older overlapping flush from reaching the backend after a newer flush.
- Business risk: a partial save can make UI state disagree with secure storage, retain a plaintext secret longer than expected, and make retries ambiguous. Concurrent flushes can persist an older provider/model configuration after the UI reports the newer one. Saved PATs/API keys are explicitly write-only (`.trellis/spec/frontend/renderer-authority-boundary.md:51-54`) and configured state must never reveal plaintext (`.trellis/spec/frontend/renderer-authority-boundary.md:62-70`).
- Existing-test mapping: debounce, flush-before-test, provider switch, URL normalization, and key clear are success-only tests (`src/test/stores/settingsStore.test.ts:423-475`, `src/test/stores/settingsStore.test.ts:477-570`, `src/test/stores/settingsStore.test.ts:685-707`). There is no `flushAiSettings` rejection, partial-success, or deferred overlap test.
- Focused tests to add:
  - Make `set_ai_api_key` resolve and `set_settings` reject; assert a deterministic error state and the intended secret-clearing/retry contract without ever serializing the secret into non-secret settings or store diagnostics.
  - Start two deferred flushes with different provider/model values and settle them in reverse order; assert the final persisted call/order and renderer state correspond to the newest edit.
  - Make the pre-switch flush reject; assert `switchAiProvider` does not issue the new provider read/write and does not strand `isLoadingAiSettings`/save status.
- Focused command: `pnpm exec vitest run src/test/stores/settingsStore.test.ts`

### 3. High: target mutations and credential updates are tested almost entirely on success paths

- Production flow: create/update/delete/switch actions combine a backend mutation with `loadTargets` (`src/stores/targetStore.ts:153-190`, `src/stores/targetStore.ts:207-245`, `src/stores/targetStore.ts:261-309`). A refresh failure after the mutation is handled as if the whole action failed. Create actions already optimistically append the target before the refresh (`src/stores/targetStore.ts:159-170`, `src/stores/targetStore.ts:213-224`).
- Business risk: target selection controls which filesystem/database/remote host later actions mutate. An ambiguous post-mutation failure can invite a duplicate create/delete/switch retry. Password-bearing requests also deserve a direct assertion that no secret enters Zustand state or diagnostic errors.
- Existing-test mapping: create/update/switch/password tests cover successful IPC argument forwarding (`src/test/stores/targetStore.test.ts:87-180`, `src/test/stores/targetStore.test.ts:218-332`). The only direct failure test is WSL distribution discovery (`src/test/stores/targetStore.test.ts:206-216`); mutation rejection, follow-up refresh failure, local-target deletion/invalid target IDs, test-command rejection, and password-state absence are not covered.
- Focused tests to add:
  - Table-drive first-command rejection for create/update/test/password/delete/switch; assert each loading ID clears, existing active target/list remains coherent, error is set, and the rejection propagates.
  - For create/delete/switch, let the mutation succeed and `list_targets` fail; assert the explicitly chosen post-mutation contract (authoritative local state versus reload-required state) and prevent duplicate retry semantics.
  - After password create/update/test failure and success, recursively inspect store state to prove the password/token seed is absent.
  - Validate boundary no-ops/fail-closed behavior for `local`, empty, and unknown target IDs at the owning frontend entry point; do not add tests that merely duplicate backend validation.
- Focused command: `pnpm exec vitest run src/test/stores/targetStore.test.ts`

### 4. High: Central repository/tag/AI-review mutations have large untested production branches

- Production flow: repository creation, tag creation, tag removal, pending-review load, accept, and skip are real store actions with loading/error/refetch behavior (`src/stores/centralSkillsStore.metadataSlice.ts:35-59`, `src/stores/centralSkillsStore.metadataSlice.ts:93-109`, `src/stores/centralSkillsStore.metadataSlice.ts:123-173`). `unassignSkillTags` also has a meaningful empty-list no-op boundary (`src/stores/centralSkillsStore.metadataSlice.ts:123-125`).
- Business risk: these actions change Central classification and AI-review state. Failure after the mutation but during refetch can present stale tags/reviews and make a repeated accept/skip ambiguous.
- Existing-test mapping: production-store tests cover assign repository, pin, assign tags, and bulk suggestion success only (`src/test/stores/centralSkillsStore.test.ts:970-1066`). Page tests replace the missing actions with mocks (`src/test/pages/centralSkillsViewTestSupport.tsx:629-644`), so they do not execute the store/IPC/error logic. No direct production test calls `createRepository`, `createTag`, `unassignSkillTags`, `loadAiTagReviews`, `acceptAiTagReview`, or `skipAiTagReview`.
- Focused tests to add:
  - Cover success plus first-command rejection for create repository/tag and accept/skip review, asserting loading cleanup, error, rethrow, and the exact refresh commands.
  - Cover `unassignSkillTags([], ...)`-equivalent empty tag IDs as a strict no-IPC/no-state-change boundary, then cover a non-empty success and failure.
  - For accept/skip, reject the post-mutation review refresh and assert an explicit reload-required/error contract rather than silently appearing successful.
- Focused command: `pnpm exec vitest run src/test/stores/centralSkillsStore.test.ts -t "repository|tag|review"`

### 5. Medium: release metadata producers are not functionally tested, only their downstream consumer is

- Production flow: `generate-latest-json.mjs` parses release arguments, selects an NSIS asset, reads its signature, builds two updater platform records, and writes `latest.json` (`scripts/release/generate-latest-json.mjs:6-90`). `prepare-release-body.mjs` chooses exact-version, series, or fallback notes and writes the public release body (`scripts/release/prepare-release-body.mjs:6-69`).
- Business risk: these scripts create public updater/release inputs. `findAsset` currently chooses the lexicographically last match instead of rejecting duplicates (`scripts/release/generate-latest-json.mjs:53-60`), and `readSignature` accepts an existing but empty signature file (`scripts/release/generate-latest-json.mjs:45-50`). The later preflight is a useful defense, but producer regressions should fail closer to creation.
- Existing-test mapping: `releaseWorkflowContract.test.ts` only reads the generator source as a static contract (`src/test/contracts/releaseWorkflowContract.test.ts:27-31`). `releasePreflight.test.ts` validates the downstream metadata consumer, including malformed JSON and duplicate candidates (`src/test/scripts/releasePreflight.test.ts:81-113`). There is no test importing either producer module.
- Focused tests to add:
  - Add a script test using an owned temp directory for argument parsing, exact/series/fallback release notes, deterministic JSON, dual Windows keys, URL/tag/version normalization, and write location.
  - Assert missing/empty signature and zero/duplicate NSIS assets fail closed at generation time.
- Focused command: `pnpm exec vitest run src/test/scripts/releaseMetadataGeneration.test.ts src/test/scripts/releasePreflight.test.ts src/test/contracts/releaseWorkflowContract.test.ts`

### Runtime adapter assessment: no additional test recommended solely for coverage

- `src/test/runtime/ipc.test.ts:97-353` already covers recorder routing, recursive-log exclusion, structured/legacy/plain/unknown rejection normalization, correlation validation, sensitive error/argument redaction, and adversarial seeds.
- `src/test/runtime/ipc.test.ts:355-452` covers Tauri/fixture dispatch and browser/desktop `listen` behavior.
- `src/test/runtime/runtimeLogger.test.ts:47-307` covers global errors, unhandled rejections, reviewed-code filtering, backend/frontend correlation, concurrent pending writes, write failure recovery, invalid fields, and console non-proxying.
- Adding another happy-path adapter test would be low-value. Revisit only if one of the store fixes changes adapter semantics.

## Files Found

- `src/stores/centralSkillsStore.updateSlice.ts` — Central update, portable-state import/export, cancellation, and correlated job lifecycle.
- `src/stores/settingsStore.aiSlice.ts` — debounced AI settings and secure API-key persistence.
- `src/stores/targetStore.ts` — target discovery, SSH/WSL mutation, password update, delete, and active-target switching.
- `src/stores/centralSkillsStore.metadataSlice.ts` — repository, tag, and AI-review mutations.
- `src/lib/ipc/invoke.ts` and `src/lib/runtimeLogger.ts` — typed IPC rejection normalization and privacy-preserving runtime diagnostics.
- `scripts/release/generate-latest-json.mjs` and `scripts/release/prepare-release-body.mjs` — public release metadata producers.
- `src/test/stores/*.test.ts`, `src/test/runtime/*.test.ts`, and `src/test/scripts/*.test.ts` — existing focused coverage mapped above.

## Recommended execution order

1. Portable-state import terminal/error correlation.
2. AI settings partial failure and overlapping saves.
3. Target mutation/error/secret-retention boundaries.
4. Central metadata/review actions and empty-input boundary.
5. Release metadata producer validation.

After each item, run its focused command above. Then run `pnpm test` for the full frontend suite, followed by the repository completion gate `just ci`, as required by `.trellis/spec/quality/ci-quality-gate.md:328-336` and `.trellis/spec/quality/test-suite-layout.md:116-133`.

## External References

- None. The review is repository-grounded; no external behavior claim was needed.

## Related Specs

- `.trellis/spec/frontend/job-correlation-cancellation.md` — stale result, busy-state, cancellation, and post-await state-write contract.
- `.trellis/spec/frontend/renderer-authority-boundary.md` — secure credential and renderer authority boundaries.
- `.trellis/spec/frontend/async-error-feedback.md` — store loading/error/rethrow and visible error requirements.
- `.trellis/spec/frontend/ipc-adapter.md` — command-routed IPC test seam and structured rejection contract.
- `.trellis/spec/quality/test-suite-layout.md` — test ownership/discovery and final `just ci` gate.
- `.trellis/spec/quality/ci-quality-gate.md` — release script and repository completion gates.

## Caveats / Not Found

- No coverage provider or threshold is configured: `package.json:27-28` runs Vitest normally/serially, and `vite.config.ts:105-110` configures discovery but not instrumentation. Therefore no trustworthy statement about line/branch percentages is made.
- Tests were not executed during this read-only planning research. Existing-test mapping comes from source inspection, not a fresh green run.
- Backend/database transaction coverage is owned by the separate backend research agent and intentionally excluded here. Frontend recommendations do not claim that backend validation or transaction tests are absent.
- Some proposed tests intentionally expose ambiguous current behavior (post-mutation refresh failure and overlapping writes). Their expected product semantics should be fixed in the task design before implementation; do not lock the current misleading state merely to make a test pass.
