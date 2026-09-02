# Design

## Change List / Symbols

1. `.trellis/tasks/09-02-windows-release-signing/research/tauri-windows-bundle-phase-evidence.md`
   - 记录锁定工具链版本、原始 `--help` 能力、无生产凭据 rehearsal 命令、产物路径/digest 和 R1 结论。
   - 这是任何 workflow 修改之前的硬门禁；不通过时本任务只提交研究结论。
2. `.github/workflows/release-desktop.yml`
   - `jobs.build-windows.permissions` 与 `jobs.build-windows.steps`：只有研究门禁通过后才按已证实的本地 CLI 语义拆分 compile/bundle；不在设计中预写尚未证实的 Tauri flag。
   - `Validate updater signing secrets` / `Build Windows bundles` / `Azure login for Artifact Signing` / `Sign Windows files with Azure Artifact Signing` / `Sign final NSIS updater bytes and record Windows signature state` / `Prepare Windows assets`：收窄 secret/permission，固定 R3 顺序，并保持最终资产名称。
3. `src/test/contracts/releaseWorkflowContract.test.ts`
   - 扩展 `release workflow contract`，按 step 名和解析后的 `env`/`permissions` 断言阶段顺序、依赖、secret 唯一注入点和最终资产消费者。
4. `src/test/scripts/releasePreflight.test.ts`、`src/test/scripts/releaseSigningState.test.ts`、`src/test/scripts/releaseMetadataGeneration.test.ts`
   - 在现有符号上补“签名后字节才是 metadata/inventory 输入”和 digest 漂移拒绝用例；优先复用 `validateSigningState`、release preflight 与 metadata helper，不建平行验证器。
5. `docs/agents/git-and-release.md`
   - 仅在 R1 通过并确定实际命令后，记录已经验证的签名阶段和 `UNVERIFIED` 边界。

## Contract and Traceability

| Requirement | Mechanism | Proof |
| --- | --- | --- |
| R1 | pinned local CLI help + production-credential-free disposable rehearsal | AC1, AC2 |
| R2 | workflow edit is conditional on a passing research artifact | AC3 |
| R3 | one ordered Windows build/signing sequence with fail-fast dependencies | AC4, AC5, AC6 |
| R4 | updater private key/password only on final updater signer step | AC7 |
| R5 | read-only default; OIDC only at Authenticode boundary | AC8 |
| R6 | asset preparation and metadata run after both signature families | AC9 |
| R7 | retain existing asset names and downstream inputs | AC10 |
| R8 | evidence report distinguishes fixture/local/controlled runner | AC11 |

The intended shape is `compile app exe → Authenticode app exe → bundle NSIS/MSI → Authenticode installers → updater sign final NSIS → inventory/checksum/latest.json`. It is a desired contract, not an assertion that Tauri CLI 2.11.4 already exposes the needed split.

## Research Gate

- Run only the repository-local CLI resolved by `pnpm exec`; record `package.json` range and `pnpm-lock.yaml` resolved version.
- Inspect `pnpm exec tauri --help` and `pnpm exec tauri build --help`. Any rehearsal command must be composed solely from flags shown by that installed CLI.
- The rehearsal uses no updater private key, no Azure/OIDC credential and a disposable artifact root. It must identify the application exe before bundling, apply a local non-production test signature or otherwise produce a distinguishable digest, and prove the bundle consumed that exact post-step file.
- If the installed CLI has no dry-run or no supported split, explicitly record that negative result. Absence of a dry-run must not be replaced by an invented `--dry-run` flag.
- A failed gate returns this task for redesign or an explicit scope decision; it never falls through to the current bundle-then-sign order under a new claim.

## Security and Data Contract

- `TAURI_UPDATER_PUBLIC_KEY` may remain non-secret build configuration; `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are absent from compile/bundle/verification environments.
- Azure variables are not secrets, but `id-token: write` and Azure login credentials are limited to the Authenticode execution boundary. If GitHub Actions cannot grant OIDC at step granularity, isolate that boundary in a dedicated signing job with digest-pinned artifacts rather than retaining permission on a general build job.
- Stage transfer accepts only the exact expected application exe, one NSIS and one MSI; digest evidence is regenerated after each byte-mutating signature and final metadata is generated last.

## Compatibility

- Preserve release modes (`rehearsal`/`publish`), frozen SHA/tag checks, required Windows asset names, `.sig` relationship, aggregate dependencies and publish read-back.
- Rehearsal may remain unsigned only where current policy explicitly permits `authenticode=not-configured`; it must not be reported as proof of AC5.
- No new CLI wrapper, signing provider, configuration surface or compatibility fallback is introduced.

## Verification Boundary

- Static/Vitest proof: ordering, permissions, secret placement, artifact naming and metadata inputs。
- Local pinned-CLI proof: only command capability and disposable bundler-input identity。
- Controlled Windows runner proof: actual package contents and Authenticode result for app exe/NSIS/MSI。
- Formal certificate trust, timestamp service availability, GitHub environment policy and production publish remain external evidence; report them separately as `UNVERIFIED` until observed。

## Rollback

- **RP1:** research artifact only. If R1 fails, stop here; no product/workflow rollback is needed.
- **RP2:** contract tests are committed with the workflow change as one atomic unit. Reverting that unit restores the prior workflow and tests without touching artifact formats.
- **RP3:** metadata/signing-state test changes are independently revertible because they preserve existing exported symbols and file formats.
- Never roll back by reordering back to bundle-then-sign while claiming REL-001 resolved; such a revert reopens the finding.

## Considered but Not Chosen

- Post-build replacement of `skillport.exe` inside NSIS/MSI: rejected because it invalidates installer structure/signatures and bypasses the real bundler contract.
- A guessed Tauri phase flag or undocumented target-directory copy: rejected because the current CLI semantics are unverified.
- A new custom Windows bundler/signing framework: rejected as scope expansion and unnecessary unless a separately approved design-size checkpoint authorizes it.
