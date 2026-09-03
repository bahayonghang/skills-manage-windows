# Windows 发布签名与密钥边界

## Goal

使 Windows 发布流水线在当前仓库锁定的 Tauri CLI 能证明分段语义的前提下，遵守“应用二进制签名 → 打包 → 安装器签名 → updater 签名与元数据”的顺序，并把 updater 私钥缩到最小步骤边界。若本地 CLI 不能证明 bundler 会消费预签名应用 exe，则 fail closed 返回规划，不猜测命令、不改变签名链。

## Findings

- `REL-001`（High / M）：`.github/workflows/release-desktop.yml:156-168,201-224` 先生成 NSIS/MSI，再对外部 exe 与安装器签名；已封装进安装器的内部 exe 可能未签名。
- `REL-002`（High / S-M）：`.github/workflows/release-desktop.yml:97-104,156-168` 把 updater 私钥/密码暴露给整个 `pnpm tauri build`，即使该阶段禁用了 updater artifacts；`build-windows` 还持有 job 级 `id-token: write`。

## Requirements

- R1：**[REL-001] CLI 研究门禁。** 在改工作流前，必须用 Node 26、pnpm 10.34.5、`pnpm-lock.yaml` 锁定的 `@tauri-apps/cli` 运行本地 `--version`/`--help`，并用无生产凭据、可丢弃产物的 rehearsal 证明可独立得到应用 exe、对它签名后再让 bundler 消费同一字节。
- R2：**[REL-001] Fail closed。** 若 `--help` 与 rehearsal 不能唯一证明 compile/bundle 分段和 bundler 输入身份，任务必须停在研究结果，不得杜撰 CLI flag、手工替换包内文件或引入自制 bundler。
- R3：**[REL-001] 签名顺序。** 只有 R1 通过后，工作流才可按 `compile → Authenticode app exe → bundle NSIS/MSI → Authenticode installers → updater sign → metadata/inventory` 串行执行。
- R4：**[REL-002] Updater secret 边界。** `TAURI_SIGNING_PRIVATE_KEY` 与其密码只可注入对最终 NSIS 字节执行 updater 签名的单一步骤。
- R5：**[REL-002] Authenticode 权限边界。** OIDC 写权限与 Azure 身份只可存在于实际 Authenticode 签名边界；普通 compile、bundle、验证和资产整理边界保持 `contents: read`。
- R6：**[REL-001] 最终字节证据。** checksum、`.sig`、`latest.json`、签名状态与资产 inventory 必须从全部签名完成后的最终发布字节生成。
- R7：**[REL-001] 资产兼容。** 现有 NSIS、MSI、ZIP、`.sig`、`latest.json` 文件名、版本、release-context 输出和 publish/aggregate 消费契约保持不变。
- R8：**[REL-001, REL-002] 证据边界。** 静态契约与无凭据 rehearsal 不能宣称真实证书、包内签名或正式发布已验证；这些结果在受控 Windows signing runner 验证前保持 `UNVERIFIED`。

## Acceptance Criteria

- [x] AC1（R1）：任务 `research/` 中记录实际 `node --version`、`pnpm --version`、本地 Tauri CLI 版本、`build --help` 摘要和锁文件解析结果。
- [ ] AC2（R1）：无生产凭据 rehearsal 产出可复算证据，证明 bundler 读取的应用 exe 与 Authenticode 前置步骤输出的是同一路径且 digest 相同。 **FAIL** — 见 `research/tauri-windows-bundle-phase-evidence.md`（`patch_binary`；包内 digest 未证明）。
- [x] AC3（R2）：当 CLI 不提供可证明的分段语义时，研究门禁以非零结果停止，`.github/workflows/release-desktop.yml` 保持未修改。 2026-09-03 用户确认范围 1。
- [ ] AC4（R3）：解析后的 workflow contract 证明六个阶段严格按 R3 顺序出现，且任一前置阶段失败都会阻止后续阶段。 **N/A** — R1 未通过，未改 workflow。
- [ ] AC5（R3）：受控 Windows rehearsal 对 NSIS 包内 `skillport.exe` 给出有效 Authenticode 证据。 **UNVERIFIED / N/A**
- [ ] AC6（R3）：受控 Windows rehearsal 对 MSI 包内 `skillport.exe` 给出有效 Authenticode 证据。 **UNVERIFIED / N/A**
- [ ] AC7（R4）：静态契约证明 updater 私钥和密码只出现在最终 NSIS updater 签名步骤的 `env`。 **N/A** — 未授权 REL-002 工作流改动。
- [ ] AC8（R5）：静态契约证明普通构建步骤和不执行 Authenticode 的 job 没有 `id-token: write`。 **N/A**
- [ ] AC9（R6）：测试证明任一签名后资产字节变化都会使既有 checksum/metadata 验证失败并要求重新生成。 **N/A**
- [ ] AC10（R7）：release artifact contract 证明最终资产集合、名称、版本关联与现有 aggregate/publish 输入兼容。 **N/A** — 资产契约未改。
- [x] AC11（R8）：本地测试结果把真实证书、正式发布和真实安装器包内签名明确报告为 `UNVERIFIED`，不以 fixture 代替。
- [ ] AC12（R1, R2, R3, R4, R5, R6, R7）：定向 release contract 测试、`pnpm typecheck` 与最终 `just ci` 通过。 **N/A** — 无产品/工作流 diff；不把未改动的 `just ci` 当成 REL-001 证据。

## Scope Decision (2026-09-03)

用户选择范围 **1**：fail closed，不修改 `.github/workflows/release-desktop.yml`，REL-001 与 REL-002 在父任务 ledger 保持 **open**。不实施范围 2（只收 REL-002）或范围 3（自制 bundler / 包内替换 exe）。本子任务以 R2/AC3 收口，不宣称 finding 已修复。

## Out of Scope

- 轮换或采购证书、密钥。
- 实际推送 tag、创建 GitHub Release 或发布资产。
- 在 R1 未通过时新增依赖、维护自制打包器或改变安装器技术栈。
- 安装/卸载行为矩阵；由 `windows-installer-verification` 承接。
