# Design

## Change List / Symbols

1. `.github/workflows/release-desktop.yml`
   - `jobs.release-context.steps`：在首次 Node 调用前加入现有 SHA-pinned `actions/setup-node`（Node 26）和版本断言；在完整 resolver 可能调用 Cargo 前加入现有 SHA-pinned `dtolnay/rust-toolchain`（Rust 1.98.0）和版本断言。
   - `jobs.release-context.timeout-minutes`、`jobs.windows-install-smoke.timeout-minutes`：提供 job 外层 deadline。
   - `jobs.windows-install-smoke.strategy.matrix` 或等价的两个明确 case：分别传入 `nsis`、`msi`，共享最终签名资产但独立结算。
   - `Verify signatures before installation` 与 installer step：保持 preflight，改为调用仓库内有界脚本。
2. `scripts/release/release-context.mjs`
   - 现有 `run`：为 `execFileSync` 增加固定 timeout 与有限输出；超时时报告 `release-context` 阶段，不改变 `resolveReleaseTag`、`resolveRehearsalRef` 和输出字段。
3. `scripts/release/windows-installer-smoke.ps1`（新建的单一最小 helper）
   - `Invoke-BoundedProcess`：启动 process handle，按 deadline 轮询，记录退出码，并在 timeout 时终止进程树。
   - `Resolve-InstalledSkillPort`：只在 case 专属安装根解析唯一 `skillport.exe`。
   - `Assert-InstalledExecutable`：检查 Authenticode `Valid`、subject policy、timestamp 和 file version。
   - `Invoke-InstallerCase`：以 `try/finally` 编排 install/verify/launch/stop/uninstall/cleanup；NSIS 与 MSI 分支只包含原生参数差异。
   - `Invoke-TimeoutFixture`：不接触真实安装状态，只证明 helper 的 timeout/kill/cleanup 行为。
4. `src/test/contracts/releaseWorkflowContract.test.ts`
   - 扩展 `release workflow contract`：解析 setup action SHA/参数/step 顺序、版本断言、job timeout、双 case 矩阵及 helper 调用。
   - 从 `package.json` 与 `rust-toolchain.toml` 读取版本 authority，不在测试中重复写另一套期望值。
5. `src/test/scripts/releaseContext.test.ts`
   - 保留 `resolveReleaseTag`/`resolveRehearsalRef`/`validateVersionSet` 单元测试；覆盖现有 `run` 的 timeout/有限输出行为，不建第二个 context resolver。
6. `docs/agents/git-and-release.md`
   - 记录两个 installer case、deadline、安装后签名证据和真实用户机器 `UNVERIFIED` 边界。

## Contract and Traceability

| Requirement | Mechanism | Proof |
| --- | --- | --- |
| R1 | explicit NSIS case with native install/uninstall path | AC1, AC8 |
| R2 | explicit MSI case with native install/uninstall path | AC2, AC9 |
| R3 | resolver subprocess timeout plus job timeout | AC3, AC14 |
| R4 | process deadline, tree termination and smoke job timeout | AC4, AC15 |
| R5 | one `try/finally` lifecycle owner per case | AC5 |
| R6 | validate exe from NSIS case root | AC6 |
| R7 | validate exe from MSI case root | AC7 |
| R8 | redacted phase records, not environment dumps | AC10 |
| R9 | pinned Node setup and assertion before first resolver | AC11 |
| R10 | pinned Rust setup and assertion before Cargo-capable resolver | AC12 |
| R11 | parsed order/version contract with authority files | AC13 |
| R12 | controlled-runner evidence is not user-machine evidence | AC16 |

## Release-context Toolchain Contract

The deterministic order is:

`checkout resolver → setup Node 26 → assert node 26.x → resolve frozen SHA (--resolve-only) → checkout frozen SHA → setup Rust 1.98.0 → assert rustc 1.98.0 → full release-context resolver`.

- `actions/setup-node` and `dtolnay/rust-toolchain` reuse the exact SHA-pinned action identities already present elsewhere in the same workflow.
- Node expected major is read from `package.json#engines.node`; Rust expected channel is read from `rust-toolchain.toml`.
- `--resolve-only` does not call Cargo today, but Rust must still precede the later full resolver that calls `cargo metadata`.
- Contract tests compare parsed step indexes and setup inputs; a runner's incidental `node`/`cargo` availability is never accepted as proof.

## Installer Matrix and Process Contract

| Installer | Install boundary | Installed exe evidence | Launch/stop | Uninstall boundary |
| --- | --- | --- | --- | --- |
| NSIS | current supported silent args into case root | unique path, signature, version | bounded process handle | native `uninstall.exe` with bounded wait |
| MSI | `msiexec` quiet/no-restart using metadata verified from the built MSI | unique path, signature, version | bounded process handle | product-code `msiexec /x` with bounded wait |

- Implementation first inspects the generated MSI Property table/product code on the controlled runner; it does not guess a product code or accept discovery outside the case root.
- Each command returns a structured stage result `{stage, outcome, exitCode?, timedOut, cleanupOutcome}`. Logs emit only this result plus artifact digest/install root; they never serialize environment variables.
- The expected signer subject is supplied by the signing evidence/policy already produced by `windows-release-signing`, not hardcoded as a second certificate authority.
- A case passes only if its own install, installed-exe assertions, launch/stop, uninstall and residue checks pass.

## Compatibility

- Preserve `windows-install-smoke` as an aggregate prerequisite and consume existing `desktop-release-windows` plus `windows-signing-evidence` artifacts.
- Preserve release-context outputs (`tag`, `version`, `sha`, `release_name`, `mode`) and frozen-SHA behavior.
- Preserve installer asset names and do not require UI automation, new test dependencies or a persistent machine-wide test installation.
- The two matrix cases may run in parallel only because each uses a unique runner/temp install root; they never share mutable installation state.

## Verification Boundary

- Linux/local Vitest proves workflow structure, setup ordering, version authority and timeout configuration.
- Windows timeout fixture proves helper supervision without claiming installer behavior.
- A controlled `windows-2022` rehearsal using final signed assets proves NSIS/MSI lifecycle and installed-exe evidence.
- Certificate trust beyond the runner, timestamp service continuity, Group Policy/AV interactions and real user-machine compatibility remain external `UNVERIFIED` evidence.

## Rollback

- **RP1:** release-context setup/order plus its contract tests are one independently revertible unit; outputs and resolver source remain unchanged.
- **RP2:** bounded PowerShell helper plus timeout fixture is one unit; it can be reverted before wiring either installer.
- **RP3:** NSIS wiring and MSI wiring are separate matrix cases. A failing MSI case may be reverted without weakening the existing NSIS case, but QUAL-002 stays open until MSI returns.
- **RP4:** docs update is reverted with the behavior it describes.
- Cleanup rollback never means leaving a test installation behind; failed cleanup is surfaced for runner teardown and the job remains failed.

## Considered but Not Chosen

- Trusting `windows-2022` preinstalled Node/Rust: rejected because it does not close REL-004.
- One NSIS smoke standing in for MSI: rejected because installer technologies have different install/uninstall semantics.
- `Start-Process -Wait` plus job timeout only: rejected because it cannot guarantee per-stage diagnosis or timely process-tree cleanup.
- Pester/new process library: rejected as a new dependency; a narrow PowerShell helper plus Windows fixture is sufficient.
