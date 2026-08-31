# Research: Frontend risk coverage (round 2)

- **Query**: Fresh static risk-oriented assessment of remaining HIGH/MEDIUM React/TypeScript frontend, Zustand store, IPC adapter, and release-script test gaps after round 1. Persist remaining business-risk items with production paths, existing-test mapping, focused tests, and vitest commands.
- **Scope**: internal
- **Date**: 2026-08-31

## Findings

### Executive result

Round 1 closed the three frontend items it implemented: portable-import terminal/stale writes, AI settings partial-save plus pre-switch flush failure, and target first-command rejection plus post-`list_targets` failure plus secret retention. Those must not be re-recommended.

The remaining HIGH gaps are still store-owned mutation/error/credential branches, not page or adapter padding. Ranked remaining work: (1) Central metadata/review actions that still have zero production-store tests, (2) GitHub PAT save/clear/test failure plus plaintext retention, (3) Central install/delete/toggle post-mutation refresh failure. Two MEDIUM items remain: release metadata producers, and Update Center apply followed by inventory refresh failure.

No coverage percentages are claimed. `package.json` Vitest scripts and `vite.config.ts:105-110` configure discovery only.

### Round 1 status (do not re-recommend)

| Round 1 item | Current evidence | Status |
|---|---|---|
| Portable import post-refresh failure + stale completion/error isolation | `src/test/stores/centralSkillsStore.test.ts:1813-1923` | Tested |
| AI secret save success + `set_settings` failure; pre-switch flush failure | `src/test/stores/settingsStore.test.ts:477-571` | Tested |
| Target first-command rejection table; create/delete/switch `list_targets` failure; password sentinel absence | `src/test/stores/targetStore.test.ts:370-499` | Tested |
| Central metadata/review comprehensive tests | No production-store calls to `createRepository`, `createTag`, `unassignSkillTags`, `loadAiTagReviews`, `acceptAiTagReview`, or `skipAiTagReview` | Still untested |
| Release metadata producers | `generate-latest-json.mjs` / `prepare-release-body.mjs` still only statically read or untested | Still untested |
| AI settings concurrent flush latest-edit-wins | Sequence still only guards Zustand writes (`src/stores/settingsStore.aiSlice.ts:401-460`) | Deferred — product decision required |
| SSH/WSL `Ok({ ok: false })` success-payload redaction | Store returns the invoke result unchanged (`src/stores/targetStore.ts:210-221`, `:268-278`) | Deferred — spec decision required |

### 1. HIGH: Central metadata and AI-review mutations still have no production-store tests

- Production flow:
  - `createRepository` / `createTag` mutate then refresh (`src/stores/centralSkillsStore.metadataSlice.ts:35-59`, `:93-109`).
  - `unassignSkillTags` is a strict empty-list no-op before IPC (`src/stores/centralSkillsStore.metadataSlice.ts:123-125`), then mutate+refresh (`:126-133`).
  - `loadAiTagReviews` has no try/catch; a rejection neither writes `error` nor preserves prior reviews (`src/stores/centralSkillsStore.metadataSlice.ts:136-145`).
  - `acceptAiTagReview` / `skipAiTagReview` mutate then refresh skills/reviews in the same try (`src/stores/centralSkillsStore.metadataSlice.ts:147-173`).
  - `bulkSuggestSkillTags([])` returns `[]` with no IPC (`src/stores/centralSkillsStore.metadataSlice.ts:176-179`); a later `get_central_skills` / `get_pending_ai_tag_reviews` rejection marks the AI-tag job `failed` even if `bulk_suggest_skill_tags` already succeeded (`:186-218`).
  - None of these post-`await` writes consult `getGeneration()` (`src/stores/centralSkillsStore.ts:15-16`, `src/stores/centralSkillsStore.updateSlice.ts:651-654`), unlike `loadCentralSkills` (`src/stores/centralSkillsStore.listSlice.ts:66-82`).
- Business risk: these actions change Central classification and AI-review state. A refresh failure after a successful accept/skip/create can look like the mutation never happened and invite a retry. Empty `unassignSkillTags` must not fire IPC. `loadAiTagReviews` can surface an unhandled rejection with no store error. A target switch during an in-flight metadata refresh can write the previous target's repositories/tags/reviews into the new generation.
- Existing-test mapping:
  - Production-store tests cover assign repository, pin, assign tags, and bulk-suggest **success only** (`src/test/stores/centralSkillsStore.test.ts:970-1066`).
  - Page tests replace the missing actions with mocks (`src/test/pages/centralSkillsViewTestSupport.tsx:629-643`), so they never execute store/IPC/error logic.
  - Bulk-suggest job cancel/progress exists (`src/test/stores/centralSkillsStore.test.ts:1363-1454`, `:1926-1947`); the empty-input and post-suggest refresh-failure branches do not.
- Focused tests to add:
  - Success plus first-command rejection for `createRepository`, `createTag`, `acceptAiTagReview`, and `skipAiTagReview`: `isMetadataUpdating` cleared, `error` set, rethrow, exact refresh commands.
  - `unassignSkillTags(skillId, [])`: no IPC and no state change; then one non-empty success and one failure.
  - Accept/skip (and optionally create repository/tag): mutation resolves, review/list refresh rejects; assert an explicit reload-required/error contract rather than silent success. Do not lock “treat refresh failure as mutation-never-happened” without the product decision below.
  - `loadAiTagReviews` rejection: deterministic error/idle contract (once specified).
  - `bulkSuggestSkillTags([])`: no IPC, no job/loading change; then suggest success + refresh reject with job/error/loading assertions.
  - Optional: after `resetForTargetChange`, a stale metadata refresh must not write repositories/tags/reviews. Only add this if design adopts generation-gating for these short mutations (see Product/spec decisions).
- Focused command (already discovers related tests; keep names so this filter stays nonzero):

```text
pnpm exec vitest run src/test/stores/centralSkillsStore.test.ts -t "repository|tag|review|bulkSuggest"
```

### 2. HIGH: GitHub PAT save/clear/test failure and renderer secret retention are success-only

- Production flow: `saveGitHubPat` / `clearGitHubPat` / `testGitHubPat` send the secret only as the IPC argument and store `GitHubPatState` / `GitHubPatTestResult` (`src/stores/settingsStore.ts:266-342`). Saved PATs are write-only (`.trellis/spec/frontend/renderer-authority-boundary.md:51-54`, `:62-70`). Failure clears the saving/testing flag and sets `error`, then rethrows; it does not put `value` into Zustand, but nothing currently asserts that.
- Business risk: a failed save/test that retained the typed PAT in store state, `error`, or `githubPatTestResult.message` would contradict the renderer authority boundary. Round 1 closed the analogous AI-key partial-save path; PAT is the remaining credential surface in this store.
- Existing-test mapping:
  - Success-only PAT tests (`src/test/stores/settingsStore.test.ts:300-366`) check `configured` / `isSavingGitHubPat` / invoke args, including forwarding `"  github_pat_abc  "`.
  - They do not reject `set_github_pat` / `clear_github_pat` / `test_github_pat`, and they do not recursively inspect store/error for a secret sentinel.
  - `SettingsView` PAT tests mock the store actions (`src/test/pages/SettingsView.test.tsx:1048-1074`) and do not execute this logic.
  - AI-key sentinel assertions must not be counted as PAT coverage (`src/test/stores/settingsStore.test.ts:477-530`).
- Focused tests to add:
  - `set_github_pat` rejects: `isSavingGitHubPat === false`, `error` set, rethrow, `JSON.stringify(store)` and error strings omit the sentinel, `githubPatState` does not contain the typed value.
  - Same pattern for `clearGitHubPat` and `testGitHubPat` loading-flag cleanup (no input secret on test/clear except whatever was already not stored).
  - Success path: recursive sentinel absence after `saveGitHubPat(SECRET)`, matching the target-store helper (`src/test/stores/targetStore.test.ts:50-53`).
- Focused command (already nonzero: four PAT tests):

```text
pnpm exec vitest run src/test/stores/settingsStore.test.ts -t "GitHubPat"
```

### 3. HIGH: Central install/delete/toggle still treat post-mutation refresh failure as “mutation never happened”

- Production flow: `installSkill` / `batchInstallSkills` invoke the install command, then `get_central_skills` + `get_skill_repositories`, then clear `isInstalling` (`src/stores/centralSkillsStore.installSlice.ts:54-115`). `deleteCentralSkill` / `deleteCentralSkills` / `deleteSkillRepository` / `resetUnknownSourceSkills` do the same with a larger refresh set (`:162-288`). `togglePlatformLink` uninstalls or installs, then refreshes (`:296-314`). The refresh sits in the same `try`; a later reject writes `error`, clears the loading flag, and rethrows, so callers cannot tell mutation-success from mutation-failure.
- Business risk: install and delete are filesystem/DB mutations. A refresh failure after a successful delete/install can invite a duplicate retry. Round 1 already required this contract for portable import and target `list_targets`; these Central install/delete branches were not in that round and remain untested.
- Existing-test mapping:
  - Success: install + refresh (`src/test/stores/centralSkillsStore.test.ts:729-876`), delete + refresh (`:430-709`), toggle uninstall/install (`:897-949`).
  - First-command rejection only: install (`:879-893`), delete (`:712-727`), toggle (`:952-968`).
  - No test lets `batch_install_to_agents` / `delete_central_skill` / `uninstall_skill_from_agent` resolve and then rejects `get_central_skills`.
- Focused tests to add:
  - Table-drive install, batch install, delete, and toggle: mutation resolves, `get_central_skills` rejects; assert loading flags cleared, error stored, rethrow, and an explicit “mutation committed / reload required” contract (do not assert the list was unchanged as if the install/delete never ran).
  - One stale-completion case: start delete/install, `resetForTargetChange` during the refresh, assert the old refresh does not write skills into the new generation if design adopts that guard. Today `installSlice` does not take `getGeneration`.
- Focused command (already nonzero):

```text
pnpm exec vitest run src/test/stores/centralSkillsStore.test.ts -t "installSkill|batchInstall|deleteCentralSkill|togglePlatformLink"
```

### 4. MEDIUM: Release metadata producers are still untested; only the downstream preflight consumer is

- Production flow:
  - `generate-latest-json.mjs` parses args, picks the lexicographically last NSIS match, reads `${asset}.sig` with `trim()` only (empty file is accepted), builds two Windows updater keys, writes `latest.json` (`scripts/release/generate-latest-json.mjs:6-90`). `findAsset` does not reject duplicates (`:53-60`). `readSignature` does not reject empty signatures (`:45-50`).
  - `prepare-release-body.mjs` chooses exact-version notes, series `major.minor.md`, or generated fallback (`scripts/release/prepare-release-body.mjs:34-64`) and writes the public body (`:66-69`).
  - Preflight **does** reject empty signatures and duplicate NSIS candidates (`scripts/release/release-preflight.mjs:96-102`; `src/test/scripts/releasePreflight.test.ts:105-113`), so producer bugs are caught late, after `latest.json` already exists.
- Business risk: these scripts create public updater/release inputs. Duplicate NSIS selection and empty `.sig` content can ship a wrong or unsigned updater pointer. Notes fallback vs exact file is user-visible release text.
- Existing-test mapping:
  - `releaseWorkflowContract.test.ts` reads `generate-latest-json.mjs` as source text only (`src/test/contracts/releaseWorkflowContract.test.ts:27-31`).
  - `releasePreflight.test.ts` validates the consumer (`src/test/scripts/releasePreflight.test.ts:81-113`).
  - `releaseArtifacts.test.ts` / `releaseContext.test.ts` / `releaseDraftState.test.ts` / `releaseSigningState.test.ts` cover other scripts; none `import()` the two producers.
- Focused tests to add (new file, same ESM import pattern as `src/test/scripts/releaseArtifacts.test.ts:16-24`):
  - Owned temp `--asset-dir`: argument parsing, `v` prefix strip, default tag `v${version}`, dual `windows-x86_64-nsis` / `windows-x86_64` keys, URL/tag/repo normalization, write path `assetDir/latest.json`.
  - Missing signature file fails closed at generation time.
  - Empty `.sig` and zero/duplicate NSIS assets: assert the **intended** fail-closed contract. Current producer accepts empty signatures and last-match duplicates; do not snapshot that as desired without the product decision below.
  - `prepare-release-body`: exact `release-notes/<version>.md`, series `release-notes/<major.minor>.md`, fallback body, `--output` path.
- Focused command (current two files already nonzero; add the new file once it exists):

```text
pnpm exec vitest run src/test/scripts/releaseMetadataGeneration.test.ts src/test/scripts/releasePreflight.test.ts src/test/contracts/releaseWorkflowContract.test.ts
```

Until the new file lands, `pnpm exec vitest run src/test/scripts/releasePreflight.test.ts src/test/contracts/releaseWorkflowContract.test.ts` already discovers tests.

### 5. MEDIUM: Update Center `apply` swallows a successful mutation when inventory reload fails

- Production flow: `apply` generates a job ID, invokes `apply_skill_update_decisions`, then `loadInventory(scope)` in the same try (`src/stores/updateCenterStore.ts:351-374`). Inventory reload failure sets `error`, clears `isApplying`, and rethrows; the apply result is not returned.
- Business risk: apply can update, delete, import, skip, and remove platform copies. A follow-up `get_skill_update_inventory` failure can look like apply failed and invite a second apply of the same decisions.
- Existing-test mapping:
  - Apply success forwards a renderer-owned `jobId` (`src/test/stores/updateCenterStore.test.ts:218-256`).
  - Stale progress events and retry-repository rejection are covered (`:165-197`, `:344-357`).
  - Leftover cleanup is covered at the dialog (`src/test/components/central/updateCenter/UpdateCenterDialog.leftover-cleanup.test.tsx`), which is not this store branch.
  - No test resolves `apply_skill_update_decisions` and then rejects `get_skill_update_inventory`.
- Focused tests to add:
  - Apply resolves, inventory reload rejects: `isApplying === false`, error stored, rethrow, and an explicit “decisions already applied / reload required” contract rather than implying apply never ran.
- Focused command (already nonzero):

```text
pnpm exec vitest run src/test/stores/updateCenterStore.test.ts
```

## What NOT to test this round

- **Already closed in round 1:** portable import terminal/stale writes; AI `set_ai_api_key` success + `set_settings` failure; AI pre-switch flush failure; target first-command rejection; target post-`list_targets` failure; target password sentinel absence.
- **IPC / runtime adapters:** `src/test/runtime/ipc.test.ts` and `src/test/runtime/runtimeLogger.test.ts` already cover normalization, redaction, correlation, listen/dispatch. Do not add happy-path adapter tests.
- **Dense stores already covering jobs/errors:** `skillsCliStore.test.ts` (busy, stale job, trailing refresh), `marketplaceStore.test.ts` (preview snapshot token retry, generation), `platformStore.test.ts` / `usageStore.test.ts` / `operationLogStore.test.ts` / `appUpdateStore.test.ts`.
- **Page/component tests that mock the store away:** Central page support (`src/test/pages/centralSkillsViewTestSupport.tsx:629-643`), Settings PAT UI mocks, collection page mocks of `batchInstallCollection`.
- **Getters, initial-state boilerplate, browser fixture returns, `isTauriRuntime()` desktop-only throw strings.**
- **Thin pass-throughs:** `previewCentralStoreLocationChange` / `applyCentralStoreLocationChange` (`src/stores/centralSkillsStore.listSlice.ts:88-97`) have no store state. Backend owns that risk.
- **Collection / project success-path padding:** `collectionStore.test.ts` and `projectsStore.test.ts` already cover happy-path mutate+reload. Their post-refresh gaps are the same pattern as items 3 and 5 but lower severity (collections do not mutate agent files; project FS/DB consistency was a round-1 **backend** item). Do not expand this round unless the three HIGH items are done.
- **GitHub import / local-archive wizard happy paths and payload redaction:** marketplace token retry and local-archive preview/import failure localization already exist (`src/test/stores/marketplaceStore.test.ts:514-538`, `src/test/components/central/LocalArchiveImportWizard.test.tsx:149-194`).
- **Import-intent URL credential rejection:** `https://user@github.com/...` and `?token=secret` are already ignored (`src/test/components/import/ImportIntentController.test.tsx:192-222`).
- **Release preflight consumer cases already in `releasePreflight.test.ts`.** Do not duplicate them inside the producer test except where the producer’s fail-closed behavior is the point.
- **AI concurrent flush and SSH/WSL `ok: false` payload redaction** until the product/spec decisions below.

## Product / spec decisions required before testing

1. **Post-mutation refresh failure (items 1, 3, 5):** whether the store must keep a “committed, reload required” signal (as target store now does via `requiresTargetReload` in `src/stores/targetStore.ts:79-87`, `:104`) versus today’s “set `error` and rethrow,” which callers can misread as mutation-not-done. Tests should lock the chosen contract, not the misleading status quo, if it violates `.trellis/spec/frontend/async-error-feedback.md:11-18`.
2. **Metadata generation gating:** `.trellis/spec/frontend/job-correlation-cancellation.md` is scoped to long-running jobs with `jobId`. `loadCentralSkills` already generation-gates; metadata/install slices do not. Decide whether `resetForTargetChange` must isolate in-flight create/accept/install refreshes before writing stale-write tests.
3. **AI settings overlapping flush:** `aiSaveSequence` ignores stale Zustand writes but does not prevent an older `set_ai_api_key` / `set_settings` from hitting the backend after a newer flush (`src/stores/settingsStore.aiSlice.ts:401-460`). Latest-edit-wins needs queue/mutex/coalesce semantics. Do not pick a mechanism in tests.
4. **SSH/WSL connection-test success payloads:** `SshTargetTestResult` / `WslTargetTestResult` include `message` (`src/types/credentials.ts:76-83`, `:124-128`). `testSshTarget` / `testWslTarget` return invoke results without storing them (`src/stores/targetStore.ts:210-221`, `:268-278`). Redacting `ok: false` inside an IPC **success** is not defined by the current redaction spec. Do not change or snapshot that contract here.
5. **`generate-latest-json.mjs` fail-closed:** whether duplicate NSIS assets and empty `.sig` files must throw at generation time (preflight already throws) or remain producer-lenient. Tests that encode last-match / empty-trim as desired behavior would freeze a known gap.

## Files Found

| File Path | Description |
|---|---|
| `src/stores/centralSkillsStore.metadataSlice.ts` | Repository/tag/AI-review mutations; empty unassign; unguarded review load |
| `src/stores/centralSkillsStore.installSlice.ts` | Install/delete/toggle mutate-then-refresh in one try |
| `src/stores/centralSkillsStore.listSlice.ts` | Generation-gated load; thin Central location-change wrappers |
| `src/stores/centralSkillsStore.updateSlice.ts` | Portable import (round 1 tested); `resetForTargetChange` bumps generation |
| `src/stores/settingsStore.ts` | GitHub PAT save/clear/test |
| `src/stores/settingsStore.aiSlice.ts` | AI flush sequence (partial-save tested; concurrent flush not) |
| `src/stores/targetStore.ts` | Mutation/secret tests exist; connection-test payload passthrough |
| `src/stores/updateCenterStore.ts` | Apply then inventory reload |
| `scripts/release/generate-latest-json.mjs` | Updater `latest.json` producer |
| `scripts/release/prepare-release-body.mjs` | Public release-body producer |
| `scripts/release/release-preflight.mjs` | Downstream metadata consumer (tested) |
| `src/test/stores/centralSkillsStore.test.ts` | Dense Central coverage; metadata create/review still missing |
| `src/test/stores/settingsStore.test.ts` | AI failure tested; PAT success-only |
| `src/test/stores/targetStore.test.ts` | Round 1 target failure/secret tests |
| `src/test/stores/updateCenterStore.test.ts` | Apply jobId + retry; no post-apply inventory failure |
| `src/test/scripts/releasePreflight.test.ts` | Consumer validation including duplicate NSIS |
| `src/test/contracts/releaseWorkflowContract.test.ts` | Static source contract for generate-latest-json |
| `src/test/runtime/ipc.test.ts` | Dense adapter coverage — do not pad |

### Code Patterns

Shared mutation+refresh anti-pattern (same `try`, refresh failure indistinguishable from mutation failure):

```35:58:src/stores/centralSkillsStore.metadataSlice.ts
    createRepository: async (name) => {
      // ...
        const repository = await invoke<SkillRepository>(
          "create_or_update_skill_repository",
          { name, sourceType: "manual" },
        );
        const repositories = await invoke<SkillRepositoryWithStats[]>(
          "get_skill_repositories",
        );
        set({ repositories, isMetadataUpdating: false });
        return repository;
      } catch (err) {
        set({ error: String(err), isMetadataUpdating: false });
        throw err;
      }
```

Empty-input no-op that is never asserted:

```123:125:src/stores/centralSkillsStore.metadataSlice.ts
    unassignSkillTags: async (skillId, tagIds) => {
      if (tagIds.length === 0) return;
```

Producer duplicate/empty-signature leniency vs preflight fail-closed:

```45:60:scripts/release/generate-latest-json.mjs
export function readSignature(assetPath) {
  const sigPath = `${assetPath}.sig`;
  if (!fs.existsSync(sigPath)) {
    throw new Error(`Updater signature not found: ${sigPath}`);
  }
  return fs.readFileSync(sigPath, "utf8").trim();
}
export function findAsset(assetDir, pattern, label) {
  const files = fs.readdirSync(assetDir).filter((file) => pattern.test(file));
  files.sort();
  const asset = files.at(-1);
```

### Related Specs

- `.trellis/spec/frontend/job-correlation-cancellation.md` — stale result / jobId; scoped to long-running jobs; generation-gating for short metadata/install refreshes needs a decision.
- `.trellis/spec/frontend/renderer-authority-boundary.md` — PAT/API key write-only; configured state must not reveal plaintext.
- `.trellis/spec/frontend/async-error-feedback.md` — store loading/error/rethrow; do not misreport a later step as the primary mutation.
- `.trellis/spec/frontend/ipc-adapter.md` — command-routed IPC test seam; do not add adapter tests for coverage.
- `.trellis/spec/quality/test-suite-layout.md` — Vitest include `src/test/**/*.test.{ts,tsx}`; new script tests belong there.
- `.trellis/spec/quality/ci-quality-gate.md:328-331` — release script Vitest set; after producer tests, include them in that focused command then `just ci`.

## Caveats / Not Found

- `python ./.trellis/scripts/task.py current --source` reported no active task (task status is `planning`). Output was written to the path given in the query: `.trellis/tasks/08-31-risk-driven-test-coverage-r2/research/`.
- No coverage provider or threshold is configured (`vite.config.ts:105-110`). No line/branch percentages are stated.
- Tests were not executed during this read-only research. Mapping is from source inspection.
- Backend/database transaction coverage is out of scope here. Round 1 already added Rust tests for project install/uninstall FS/DB consistency and target CRUD rollback; this report does not re-open those.
- `collectionStore` mutate+reload and `projectsStore` install/uninstall post-`get_project_skills` failure are real but lower priority than the five ranked items; they are listed under What NOT to test unless HIGH work finishes early.
- `localRemoteSyncStore.applySync` failure (loading/error) is a thin single-invoke catch (`src/stores/localRemoteSyncStore.ts:54-66`) with preview-error already tested; not ranked.
