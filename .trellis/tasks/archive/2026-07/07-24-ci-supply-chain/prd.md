# CI 跨平台矩阵与供应链加固

## Goal

让 pull request 和集成分支在发布前暴露 Unix 平台源码问题，并让第三方
GitHub Actions 与双生态生产依赖具备可审计、可阻断、可过期的供应链门禁。
本任务闭环父任务中的 P2-04、P2-06 与 QW-08，同时保持 Windows x64 为首要
平台，且不改变现有 required check `just-ci` 的名称与 Windows runner 语义。

## Background

- `.github/workflows/ci.yml:27-71` 的 `just-ci` 仅运行于 `windows-2022`；
  Linux/macOS job 位于手动打包路径（`:73-239`），PR/push 不执行。
- `.github/workflows/ci.yml`、`release-desktop.yml`、`docs.yml` 共使用 8 种
  外部 Action，当前均引用可移动的 `@vN` 或 `@stable`。
- `src/test/contracts/ciWorkflowContract.test.ts:55` 已保护 `just-ci` 名称与完整
  Windows 质量链；`.trellis/spec/quality/ci-quality-gate.md:7` 将 Windows-first、
  routine source validation 与手动/release package smoke 分开定义。
- 2026-07-28 的 `pnpm audit --prod --json` 基线为 9 high、15 moderate、5 low。
  高危主要来自误列为生产依赖的 `shadcn`、`react-router` 与 `postcss` 传递链。
- 同日 `cargo audit --json` 返回 7 个 vulnerability；其中 6 个可通过
  `plist/quick-xml`、`quinn-proto`、`rustls-webpki` 定向更新修复，`rsa` 来自
  SQLx 默认启用但本项目未使用的 MySQL 适配器。
- 当前仓库没有 `.github/dependabot.yml`，也没有双生态 dependency audit job。

## Requirements

1. **跨平台源码门禁**：PR 到 `main`、push 到 `main/dev`、手动触发及 reusable
   workflow 都运行 Ubuntu 22.04 与 macOS 14 源码验证；Windows `just-ci` 的
   job id、显示名、runner 和完整质量链保持不变。Linux 可复用现有 Tauri 系统
   依赖安装段；routine PR/push 不构建安装包。
2. **Action 不可变引用**：三个 workflow 的所有外部 `uses:` 固定到已核验的
   40 位 commit SHA，并保留版本注释；仓库内 `./.github/workflows/...` 调用豁免。
   新增 weekly `github-actions` Dependabot 配置。
3. **双生态审计**：独立 blocking job 运行 `pnpm audit --prod --json` 与固定版本
   `cargo-audit`。JS 的 high/critical 和 Rust 的 vulnerability 若未被精确例外覆盖
   必须失败；JS moderate/low 与 Rust informational warnings 输出但不阻断。
4. **Fail-closed 例外**：例外记录必须包含 `ecosystem`、advisory ID、`owner`、
   `reason`、ISO date `expires`。未知字段形状、重复 ID、过期日期、未使用例外、
   跨生态匹配或宽泛 package ignore 都必须失败。例外只能匹配单一 advisory。
5. **先修复再例外**：将仅用于开发的 `shadcn` 移到 devDependencies，升级
   `react-router-dom` 与 `@lobehub/icons` 到兼容修复版本；关闭 SQLx 默认 features，
   只保留 SQLite/runtime/macros，并定向更新三组可修复 Rust 传递包。只允许为
   上游尚无稳定修复且本应用不使用受影响 RSC 模式的精确 React Router advisory，
   以及 `tauri-plugin-sql 2.4.0` 内部无条件启用、暂无修复版本的 SQLx/RSA advisory
   建立短期例外；两者均不得扩展为 package-wide ignore。
6. **可测试契约**：Vitest fixture 覆盖未知 high 阻断、精确有效例外放行、过期
   例外阻断、未使用例外阻断、Rust vulnerability 阻断及全 workflow SHA pin。
   更新 CI contract、`CONTRIBUTING.md` 与质量 spec。
7. **范围与兼容性**：不修改分支保护远端设置，不 push；不扩大到 CodeQL、secret
   scanning、SBOM、provenance 或日常多平台 bundle。发布 workflow 的 frozen SHA、
   Windows 签名/更新器产物与 draft publication 顺序不得改变。

## Acceptance Criteria

- [ ] `just-ci` 仍为 `windows-2022` 上稳定且完整的 required context。
- [ ] CI contract 证明 Ubuntu 22.04/macOS 14 source-validation 在 PR/push/reusable
      路径无 warning/continue-on-error guard，并证明三类 package smoke 仍仅手动触发。
- [ ] 所有外部 Action 均为 40 位 SHA，仓库存在 weekly github-actions Dependabot。
- [ ] 双生态实时审计通过；生产 JS high/critical 与 Rust vulnerability 为零，或仅有
      合法且未过期的精确例外。
- [ ] fixture 证明未知 advisory、过期/畸形/未使用例外都会使审计器非零退出。
- [ ] `pnpm typecheck`、`pnpm lint`、相关 Vitest、Rust fmt/clippy/test 与 `just ci`
      全部通过。
- [ ] `CONTRIBUTING.md` 和 `.trellis/spec/quality/ci-quality-gate.md` 与新门禁一致。

## Out Of Scope

- CodeQL、secret scan、SBOM、attestation/provenance（父审计长期项 L-03）。
- routine PR 的 MSI/NSIS/DMG/AppImage 打包或签名。
- 远端 branch protection/ruleset 变更和首次 GitHub-hosted runner 实跑；本任务只提交
  可由契约测试验证的 workflow 配置，远端 required-check 调整需在 push 后另行授权。

## Risks And Deferred Evidence

- GitHub-hosted Linux/macOS 首次运行只能在 push 后获得；本地以 workflow contract、
  Windows `just ci` 与可运行的跨平台命令结构作为关闭证据，不将缺失的远端运行冒充通过。
- Action SHA 会随上游发布而陈旧，但 Dependabot 负责提出可审查更新，不能恢复 movable tag。
- React Router 当前稳定线没有修复 RSC CSRF advisory；`tauri-plugin-sql 2.4.0`
  内部仍启用 SQLx/RSA 默认闭包。两条短期例外都必须说明实际不可达/未使用语义，并设置
  不晚于 2026-08-11 的到期日。
