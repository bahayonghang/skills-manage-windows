# Implementation Plan

## Implementation Traceability

| Requirement | Steps | Acceptance evidence |
| --- | --- | --- |
| R1 | Steps 3-4 | AC1, AC8 |
| R2 | Steps 3, 5 | AC2, AC9 |
| R3 | Steps 1, 3 | AC3, AC14 |
| R4 | Steps 2-3 | AC4, AC15 |
| R5 | Steps 2, 4-5 | AC5 |
| R6 | Step 4 | AC6 |
| R7 | Step 5 | AC7 |
| R8 | Steps 2, 4-5 | AC10 |
| R9 | Step 1 | AC11 |
| R10 | Step 1 | AC12 |
| R11 | Step 1 | AC13 |
| R12 | Step 6 and Overall Verification | AC16 |

## Step 1 — Pin and bound release-context before first use [R3, R9, R10, R11]

Files/symbols: `.github/workflows/release-desktop.yml` `jobs.release-context`; `scripts/release/release-context.mjs` `run`; `src/test/contracts/releaseWorkflowContract.test.ts` `release workflow contract`; `src/test/scripts/releaseContext.test.ts`.

- Add SHA-pinned Node setup with `node-version: "26"` before the first `node scripts/release/release-context.mjs --resolve-only` call.
- Add a `node --version` assertion derived from `package.json#engines.node` before that call.
- Add SHA-pinned Rust setup with `toolchain: "1.98.0"` and a `rustc --version` assertion before the full resolver can call `cargo metadata`.
- Parse step indexes and setup inputs in the contract test; do not rely on source substring order.
- Add a fixed subprocess timeout and bounded captured output to the existing `run` helper; preserve injectable `exec` behavior and release-context outputs.
- Add a timeout fixture that proves a hung resolver child reports the resolver stage without waiting for the outer job timeout.

Directed verification:

```powershell
pnpm vitest run src/test/contracts/releaseWorkflowContract.test.ts src/test/scripts/releaseContext.test.ts
pnpm typecheck
```

Expected intermediate proof: AC3 and AC11-AC14 pass; release-context output keys and existing resolver behavior are unchanged.

Rollback point RP1: revert setup/order and matching contract assertions as one unit; REL-004 becomes open again.

## Step 2 — Add bounded process ownership [R4, R5, R8]

File/symbols: new `scripts/release/windows-installer-smoke.ps1` functions `Invoke-BoundedProcess`, `Invoke-TimeoutFixture`, `Invoke-InstallerCase`.

- Implement process-handle waiting with a numeric deadline, exit-code capture and process-tree termination on timeout.
- Make `Invoke-InstallerCase` the single owner of application stop, uninstaller execution and residue cleanup through `try/finally`.
- Emit redacted stage records; never print an environment dump or credential-bearing command line.
- Add script fixture mode that starts a harmless deliberate hang and proves timeout/kill/cleanup without installing SkillPort.

Directed verification on Windows PowerShell:

```powershell
pwsh -NoProfile -File scripts/release/windows-installer-smoke.ps1 -Fixture timeout
```

Expected intermediate proof: fixture exits non-zero for the intended timeout and reports successful process-tree cleanup. The exact script parameter contract is frozen in its header/help and workflow contract before wiring real artifacts.

Rollback point RP2: revert the helper/fixture before workflow wiring; no install state should exist.

## Step 3 — Wire deterministic job limits and two cases [R1-R4]

Files/symbols: `.github/workflows/release-desktop.yml` `jobs.release-context.timeout-minutes`, `jobs.windows-install-smoke.timeout-minutes`, `jobs.windows-install-smoke.strategy.matrix`, installer helper step; `src/test/contracts/releaseWorkflowContract.test.ts`.

- Give release-context and installer smoke explicit outer `timeout-minutes` values greater than their summed inner deadlines.
- Define distinct `nsis` and `msi` matrix cases; both consume final assets and signing evidence from `windows-release-signing`.
- Pass each case a unique `$RUNNER_TEMP` install root.
- Keep aggregate dependent on the whole matrix job so either case failure blocks publish.

Directed verification:

```powershell
pnpm vitest run src/test/contracts/releaseWorkflowContract.test.ts
pnpm typecheck
```

Rollback point RP3a: revert matrix wiring while retaining the independently tested helper; QUAL-002 remains open.

## Step 4 — Implement NSIS lifecycle evidence [R1, R5, R6, R8]

Files/symbols: `scripts/release/windows-installer-smoke.ps1` NSIS branch, `.github/workflows/release-desktop.yml` NSIS matrix entry.

- Use the currently supported NSIS silent install into the unique case root.
- Resolve exactly one installed `skillport.exe`; validate `Get-AuthenticodeSignature` status, subject policy, timestamp certificate and file version.
- Launch with bounded supervision, stop explicitly, run the native uninstaller with bounded supervision and assert executable residue is absent.

Directed verification on the controlled Windows rehearsal artifact:

```powershell
pwsh -NoProfile -File scripts/release/windows-installer-smoke.ps1 -InstallerKind nsis -ArtifactPath <final-nsis-path> -ExpectedVersion <release-version> -InstallRoot <unique-temp-root>
```

Angle-bracket values are workflow inputs, not literal commands. This step is not considered verified on a non-Windows host.

Rollback point RP3b: revert only the NSIS case wiring/helper branch; do not report AC1/AC6/AC8 complete.

## Step 5 — Implement MSI lifecycle evidence [R2, R5, R7, R8]

Files/symbols: `scripts/release/windows-installer-smoke.ps1` MSI branch, `.github/workflows/release-desktop.yml` MSI matrix entry.

- Read the built MSI metadata to obtain the actual product code and verify the supported install-directory property before installation; fail closed if either is ambiguous.
- Run bounded `msiexec` quiet/no-restart install and product-code uninstall.
- Apply the same installed-exe signature/version, launch/stop and residue assertions as NSIS, using only the MSI case root.

Directed verification on the controlled Windows rehearsal artifact:

```powershell
pwsh -NoProfile -File scripts/release/windows-installer-smoke.ps1 -InstallerKind msi -ArtifactPath <final-msi-path> -ExpectedVersion <release-version> -InstallRoot <unique-temp-root>
```

Rollback point RP3c: revert only the MSI case. NSIS coverage may remain, but QUAL-002 stays open and aggregate must not claim the task complete.

## Step 6 — Document the verified contract [R12]

File: `docs/agents/git-and-release.md`.

- Document Node/Rust setup order, two installer cases, deadlines, phase logs and cleanup ownership.
- Mark actual controlled-runner observations separately from user-machine/production evidence.

Directed verification:

```powershell
pnpm docs:gen:check
pnpm docs:build
```

Rollback point RP4: revert documentation with the corresponding behavior change.

## Overall Verification

```powershell
pnpm vitest run src/test/contracts/releaseWorkflowContract.test.ts src/test/scripts/releaseContext.test.ts
pnpm typecheck
pnpm lint
pnpm docs:gen:check
just ci
```

Then, under separate remote-run authorization, run a `workflow_dispatch` rehearsal at a frozen SHA on `windows-2022`. Require both matrix cases and aggregate to pass; retain artifact digest, installed path, signer subject, timestamp-present boolean, version, per-stage outcome and cleanup outcome. Do not publish a draft or tag.

## External Evidence and Final Rollback

- Local tests prove setup/order contracts; they do not prove GitHub runner images or installer behavior.
- Controlled Windows rehearsal proves only that runner and those exact assets. AV, enterprise policy and real user-machine compatibility remain `UNVERIFIED`.
- On any partial failure, preserve redacted logs, execute `finally` cleanup, and revert only the last RP unit. Do not weaken timeouts/signature assertions to obtain green results.

## Execution record (2026-09-03)

Local parent-verified:

- `pnpm vitest run src/test/contracts/releaseWorkflowContract.test.ts src/test/scripts/releaseContext.test.ts src/test/scripts/releaseSigningState.test.ts src/test/scripts/releasePreflight.test.ts` — 30 passed
- `pnpm typecheck` — passed
- `pwsh -NoProfile -File scripts/release/windows-installer-smoke.ps1 -Fixture timeout` — exit 1, `{"stage":"fixture-timeout","outcome":"timeout","timedOut":true,"cleanupOutcome":"ok"}`
- `just ci` — passed (`JUST_CI_INSTALLER EXIT=0`)

Still UNVERIFIED: AC1, AC2, AC5–AC9 (windows-2022 NSIS/MSI lifecycle with final assets); inner-exe-before-bundle (REL-001); user-machine compatibility. REL-001/REL-002 remain open (fail-closed).

本任务在实现后归档；不执行正式发布、远程写入或 push。
