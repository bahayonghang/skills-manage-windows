# Implementation Plan

## Implementation Traceability

| Requirement | Steps | Acceptance evidence |
| --- | --- | --- |
| R1 | Step 0 | AC1, AC2 |
| R2 | Step 0 | AC3 |
| R3 | Steps 1-2 | AC4, AC5, AC6 |
| R4 | Steps 1, 3 | AC7 |
| R5 | Steps 1, 3 | AC8 |
| R6 | Steps 1, 4 | AC9 |
| R7 | Steps 1, 4 | AC10 |
| R8 | Overall Verification | AC11 |

## Step 0 — Fail-closed research gate [R1, R2]

Files/evidence: `package.json`, `pnpm-lock.yaml`, `.trellis/tasks/09-02-windows-release-signing/research/tauri-windows-bundle-phase-evidence.md`.

1. Confirm the pinned runtime and repository-local CLI:
   - `node --version` (must satisfy `26.x`)
   - `pnpm --version` (must equal `10.34.5`)
   - `pnpm exec tauri --version` (must resolve the lockfile CLI)
   - `pnpm exec tauri --help`
   - `pnpm exec tauri build --help`
2. Record only capabilities printed by this CLI. Construct a production-credential-free disposable rehearsal from those capabilities; do not assume a `--dry-run` flag exists.
3. Capture pre-bundle/post-sign/bundler-input paths and SHA-256 digests. Prove the bundler consumes the post-sign digest.
4. If any proof is absent or ambiguous, stop before editing `.github/workflows/release-desktop.yml`, mark AC2/AC3 honestly, and return the task for a scope decision.

Rollback point: research-only change; discard the disposable artifact directory. No source/workflow mutation has occurred.

## Step 1 — Lock the failing contracts [R3-R7]

Files/symbols: `src/test/contracts/releaseWorkflowContract.test.ts` (`release workflow contract`), `src/test/scripts/releasePreflight.test.ts`, `src/test/scripts/releaseSigningState.test.ts`, `src/test/scripts/releaseMetadataGeneration.test.ts`.

- Add parsed YAML assertions for the six R3 stages, their strict order and fail-fast dependency.
- Assert `TAURI_SIGNING_PRIVATE_KEY` and password have exactly one workflow step consumer.
- Assert non-signing build/bundle jobs cannot acquire `id-token: write`.
- Add final-byte digest/metadata drift tests without changing current asset names.

Directed verification:

```powershell
pnpm vitest run src/test/contracts/releaseWorkflowContract.test.ts src/test/scripts/releasePreflight.test.ts src/test/scripts/releaseSigningState.test.ts src/test/scripts/releaseMetadataGeneration.test.ts
pnpm typecheck
```

Expected intermediate result: new assertions fail against the old workflow for REL-001/REL-002 and existing release invariants remain green.

Rollback point: tests-only commit can be reverted without touching the workflow.

## Step 2 — Apply the proven phase split [R3]

File/symbols: `.github/workflows/release-desktop.yml`, `jobs.build-windows` and, only if required for step-level OIDC isolation, a narrowly named Authenticode signing job.

- Translate the exact Step 0 CLI evidence into compile and bundle steps; copy no hypothetical command from this plan.
- Make Authenticode of the application exe a hard predecessor of both NSIS and MSI bundling.
- Make installer Authenticode a hard successor of bundling.
- Keep exactly one application exe, one NSIS and one MSI at each boundary; stop on missing/duplicate artifacts.

Directed verification:

```powershell
pnpm vitest run src/test/contracts/releaseWorkflowContract.test.ts
pnpm exec tauri build --help
```

The second command is a semantic cross-check; no Windows bundle is declared verified until the controlled runner step.

Rollback point: revert the workflow phase-split unit together with its ordering assertions; REL-001 becomes open again.

## Step 3 — Minimize credentials and permissions [R4, R5]

File/symbols: `.github/workflows/release-desktop.yml`, updater secret validation/signing steps, `permissions`, Azure login/action boundary.

- Remove updater private key/password from validation and build environments; validate and consume them only at final updater signing.
- Move `id-token: write` to the smallest GitHub-supported Authenticode job boundary. Transfer only digest-pinned artifacts if a dedicated job is necessary.
- Retain `contents: read` everywhere else.

Directed verification:

```powershell
pnpm vitest run src/test/contracts/releaseWorkflowContract.test.ts
rtk rg -n "TAURI_SIGNING_PRIVATE_KEY|TAURI_SIGNING_PRIVATE_KEY_PASSWORD|id-token: write" .github/workflows/release-desktop.yml
```

Rollback point: credential/permission boundary is one independent workflow/test unit; reverting it reopens REL-002 but does not alter asset formats.

## Step 4 — Regenerate evidence only from final bytes [R6, R7]

Files/symbols: `.github/workflows/release-desktop.yml` `Prepare Windows assets`; existing `scripts/release/generate-latest-json.mjs`, `release-preflight.mjs`, `release-signing-state.mjs`; `docs/agents/git-and-release.md`.

- Run updater signing after installer Authenticode and before copying assets/metadata.
- Generate `.sig`, ZIP, `latest.json`, checksums and inventory from the final bytes only.
- Preserve all current names and downstream aggregate/publish paths.
- Document only the commands proven in Step 0.

Directed verification:

```powershell
pnpm vitest run src/test/scripts/releasePreflight.test.ts src/test/scripts/releaseSigningState.test.ts src/test/scripts/releaseMetadataGeneration.test.ts src/test/contracts/releaseWorkflowContract.test.ts
pnpm typecheck
pnpm lint
```

Rollback point: evidence-generation change is reverted with its tests; never reuse pre-sign checksums after rollback.

## Overall Verification

```powershell
pnpm vitest run src/test/contracts/releaseWorkflowContract.test.ts src/test/scripts/releasePreflight.test.ts src/test/scripts/releaseSigningState.test.ts src/test/scripts/releaseMetadataGeneration.test.ts
pnpm typecheck
pnpm lint
just ci
```

After local gates, run a separately authorized `workflow_dispatch` rehearsal on `windows-2022` using the frozen SHA. Inspect NSIS and MSI contents and retain per-file signer/timestamp/digest evidence. Do not push a tag, publish a release or expose environment values.

## External Evidence and Final Rollback

- Local CLI help/rehearsal proves capability, not real Authenticode trust.
- Only a controlled signing runner can close AC5 and AC6; formal timestamp/identity, GitHub environment approval and publish behavior remain `UNVERIFIED` until observed.
- If controlled runner evidence disagrees with Step 0, fail closed and revert the workflow/test unit to the pre-task state; do not retain a partially changed signing order.

本任务保持 `planning`；本规划阶段不执行发布、远程写入或产品代码修改。
