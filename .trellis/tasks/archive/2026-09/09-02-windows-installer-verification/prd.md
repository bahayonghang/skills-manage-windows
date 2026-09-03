# Windows 安装器与 release-context 验证矩阵

## Goal

让 NSIS 与 MSI 都具备独立、有限时、可清理的安装后验证，直接检查已安装应用的签名与版本，并确保冻结 release context 的第一条 Node/Cargo 调用已经由仓库固定的 Node 26 与 Rust 1.98.0 提供。

## Findings

- `QUAL-002`（Medium / M）：`.github/workflows/release-desktop.yml:263-265,288-336` 要求发布 MSI，但 smoke 只覆盖 NSIS。
- `REL-003`（Low / S）：`.github/workflows/release-desktop.yml:46-88,288-336` 的相关 job 与 `Start-Process -Wait` 没有明确 timeout，异常 resolver、installer 或应用可无限悬挂。
- `REL-004`（Low / S）：`.github/workflows/release-desktop.yml:46-88` 在任何 Node/Rust setup 前运行 `node scripts/release/release-context.mjs`；该脚本 `scripts/release/release-context.mjs:63-68` 又直接运行 `cargo metadata`，因此 release context 依赖 runner 漂移工具链。
- 当前 smoke 只启动应用，没有从真实安装位置验证 Authenticode 主体、时间戳与文件版本。

## Dependencies

- **D1：**不得用安装 smoke 掩盖未证明的 bundle 输入，也不得把已安装 exe 的 Authenticode 结果写成 REL-001/REL-002 已修复。
- **D1 status (2026-09-03)：** signing R1 **FAIL**，用户选择 fail-closed（范围 1）。本任务仍实施 QUAL-002 生命周期、REL-003 超时和 REL-004 工具链固定。Authenticode `Valid` 只在 `windows-signing.json` 已要求它时断言；rehearsal `authenticode=not-configured` 必须原样记录，禁止把未签名/未配置当成已签名。REL-001/REL-002 保持 open。

## Requirements

- R1：**[QUAL-002] NSIS 生命周期。** NSIS 必须用真实非交互安装/卸载路径独立验证 install、launch、stop、uninstall 和残留清理。
- R2：**[QUAL-002] MSI 生命周期。** MSI 必须用真实非交互安装/卸载路径独立验证 install、launch、stop、uninstall 和残留清理，不能外推 NSIS 结果。
- R3：**[REL-003] Resolver deadline。** release-context 启动的 `git`/`cargo` 子进程必须有显式 timeout 与阶段化错误，job 还必须有外层 deadline。
- R4：**[REL-003] Installer process deadline。** installer、应用和 uninstaller 每次等待必须有显式 timeout、退出码诊断和超时后的进程树终止。
- R5：**[REL-003] 必达清理。** 每个安装器 case 都在 `finally` 中停止应用、卸载并检查测试根；cleanup 失败是独立失败结果。
- R6：**[QUAL-002] NSIS 已安装 exe。** 从 NSIS case 的实际安装位置读取唯一 `skillport.exe`，核对文件版本，并读取 Authenticode 状态。若 signing evidence 为 publish/`Valid`，再核主体与时间戳；若为 rehearsal `not-configured`，必须记录该状态且不得报成已签名。
- R7：**[QUAL-002] MSI 已安装 exe。** 与 R6 相同，仅使用 MSI case 根；不得外推 NSIS 的签名结论。
- R8：**[REL-003] 日志。** 阶段化日志可区分 resolve/install/signature/version/launch/stop/uninstall/cleanup，且不输出凭据或完整环境。
- R9：**[REL-004] Node 固定。** `release-context` job 在第一次 `node` 调用前，以 SHA-pinned `actions/setup-node` 配置 `node-version: "26"`，并执行版本断言。
- R10：**[REL-004] Rust 固定。** `release-context` job 在任何可能进入 `cargo metadata` 的 resolver 调用前，以 SHA-pinned `dtolnay/rust-toolchain` 配置 `toolchain: "1.98.0"`，并执行版本断言。
- R11：**[REL-004] 确定性契约。** 解析 workflow 的测试必须证明 setup/版本断言的索引先于 resolver 调用，并证明 setup 值与 `package.json`、`rust-toolchain.toml` 一致。
- R12：**[QUAL-002, REL-003, REL-004] 证据边界。** fixture/静态契约不能代替受控 Windows runner 或真实用户机器兼容性；外部证据单独标为 `UNVERIFIED`。

## Acceptance Criteria

- [ ] AC1（R1）：隔离 Windows runner 上的 NSIS case 独立完成 install → verify → launch → stop → uninstall。 **UNVERIFIED**（无 windows-2022 rehearsal）
- [ ] AC2（R2）：隔离 Windows runner 上的 MSI case 独立完成 install → verify → launch → stop → uninstall。 **UNVERIFIED**
- [x] AC3（R3）：故意悬挂的 resolver fixture 在配置的 deadline 后以明确 timeout 阶段失败。
- [x] AC4（R4）：故意悬挂的 installer fixture 超时后没有遗留应用或安装器进程。
- [ ] AC5（R5）：任一中间断言失败时，`finally` 仍执行卸载并报告 cleanup 结果。 **UNVERIFIED**（真实 installer 失败路径未在 runner 上跑）
- [ ] AC6（R6）：NSIS 安装出的 exe 通过版本断言；Authenticode 按 signing evidence 策略断言（`Valid` 或显式 `not-configured`），不得把 REL-001 标成 fixed。 **UNVERIFIED**（策略已接线；无真实 NSIS 安装）
- [ ] AC7（R7）：MSI 安装出的 exe 通过版本断言；Authenticode 按同一策略独立断言，不得外推 NSIS。 **UNVERIFIED**
- [ ] AC8（R1）：NSIS 卸载后 case 安装根内不再存在 `skillport.exe`。 **UNVERIFIED**
- [ ] AC9（R2）：MSI 卸载后 case 安装根内不再存在 `skillport.exe`。 **UNVERIFIED**
- [x] AC10（R8）：失败 fixture 的日志只含阶段、退出码、timeout 和清理状态，不含 secret 值。
- [x] AC11（R9）：workflow contract 证明 Node 26 setup 与 `node --version` 断言均先于第一次 `release-context.mjs` 调用。
- [x] AC12（R10）：workflow contract 证明 Rust 1.98.0 setup 与 `rustc --version` 断言均先于第一次非 `--resolve-only` resolver 调用。
- [x] AC13（R11）：测试从 `package.json#engines.node` 和 `rust-toolchain.toml#toolchain.channel` 读取期望值，并拒绝 workflow setup 值漂移。
- [x] AC14（R3）：`release-context` job 具有确定的 `timeout-minutes`。
- [x] AC15（R4）：`windows-install-smoke` job 具有确定的 `timeout-minutes`。
- [x] AC16（R12）：release contract 与两个 installer smoke case 通过后，真实用户机器兼容性仍明确报告为 `UNVERIFIED`。
- [x] AC17（R1, R2, R3, R4, R5, R6, R7, R8, R9, R10, R11）：定向 release-context/workflow tests、`pnpm typecheck` 与最终 `just ci` 通过。 NSIS/MSI 真实 runner 生命周期仍为 AC1/AC2/AC5–AC9 UNVERIFIED。

## Out of Scope

- UI 端到端点击测试。
- 正式发布、远程资产上传或证书轮换。
- 以 runner 预装 Node/Rust 版本作为固定机制。
- 广泛重构所有平台 job 的工具链 setup；本任务只修复 `release-context` 与 Windows installer smoke 证据链。
