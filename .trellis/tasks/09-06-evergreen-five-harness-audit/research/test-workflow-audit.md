# 测试与工作流审计

日期：2026-09-06。基线：`dev` / `a81b7c2d`。本报告只读审查当前代码、执行已有测试并形成批准前计划；未改产品代码、现有测试、工作流或 harness，未主动安装依赖、提交、归档、触发远程 run。doctor自身触发的自动bootstrap尝试及用户缓存副作用单独记录在下文。

## 结论

当前产品逻辑未发现可复现的测试失败：Windows 本地 Vitest 178 文件、2057 项通过，Rust 1553 项通过；当前 tracked tree 的最近一次 GitHub CI 五条 required lanes 全绿。需要修复的是两个 P1 门禁/诊断缺口和一个 P2 交互覆盖缺口：`doctor` 的 pnpm 探测会触发自动 bootstrap 并超时，Trellis Python 安全测试未进入 required CI 且新 clone 可能静默跳过 hook 合同，Central 当前筛选到批量安装的唯一串联用例仍为 `it.skip`。

本机未能执行 canonical `node scripts/check/run-ci.mjs` / `just ci`，原因是已安装 pnpm 12.3.4 会尝试自动获取项目锁定的 10.34.5 并在探测超时；直接调用已有 `node_modules` CLI 的通过结果不能顶替 canonical gate。Windows 安装器、真实 WebView、provider、SSH/WSL、签名、更新元数据及当前 release 端到端仍为 **UNVERIFIED**。

## 运行证据

| Surface | 结果 | 判定 |
| --- | --- | --- |
| `just doctor` | exit 1；Node 26.7.0、Rust/Cargo 1.98.0、just/Git/MSVC/Tauri 正常；pnpm probe `ETIMEDOUT` | 工具链/诊断阻断，不是产品测试失败；见 [`test-logs/doctor.md`](./test-logs/doctor.md) |
| Frontend Vitest | 178 文件；2057 passed / 1 skipped；44.37 s | 逻辑回归通过，有一个明确覆盖缺口；见 [`test-logs/frontend-tests.md`](./test-logs/frontend-tests.md) |
| TypeScript、ESLint、version/docs/capability/size/entrypoint | 全部 exit 0 | 当前本地静态与生成物合同通过 |
| Vite renderer / VitePress docs build | 全部 exit 0 | 本地构建通过，不证明原生 bundle |
| Rust fmt / Clippy / IPC codegen | 全部 exit 0 | 当前 Windows 编译/静态合同通过 |
| Rust locked tests | 1553 passed / 7 ignored；107.22 s | 当前 Windows Rust 测试通过；见 [`test-logs/rust-tests.md`](./test-logs/rust-tests.md) |
| Trellis Python tests | 31 passed / 4 skipped；6.342 s | Windows 路径/进程及 Claude/Codex hook 测试通过；POSIX symlink/process-group 四项缺证；见 [`test-logs/trellis-python-tests.md`](./test-logs/trellis-python-tests.md) |
| GitHub Actions | 最近覆盖 tracked tree 字节的 CI 五条 required lanes 成功 | hosted Windows/Linux/macOS 与 supply-chain 当前通过；见 [`test-logs/github-workflows.md`](./test-logs/github-workflows.md) |

## 分级发现与最小改造

### ENV-001 · P1 · `doctor` 的 pnpm 探测违反只读诊断合同，并阻断 canonical 本地门禁

**证据与根因。** `scripts/check/doctor.mjs:44-54` 在项目 cwd、继承全部环境运行命令，并统一使用 5 秒超时。本机 `pnpm` 是 Scoop 12.3.4，而 `package.json` 锁定 10.34.5；pnpm 在项目目录自动尝试获取锁定版本，产生用户级 engine store/lock，随后 doctor 只显示 `spawnSync pnpm ETIMEDOUT`。子进程 A/B 设置已验证的 `pnpm_config_pm_on_fail=ignore` 后，`pnpm --version` 立即返回 12.3.4，doctor 也立即、正确报告版本错配。因此根因是 pnpm 12 自动项目版本 bootstrap 与通用 5 秒探针相互作用；不是测试套件或产品逻辑挂起。

**最小改造。** 只在 pnpm version probe 的子环境注入已实测的 `pnpm_config_pm_on_fail=ignore`，保留 10.34.5 pin、版本错配、不可执行和超时分类。不要修改父进程/全局配置、调大通用超时、让 pnpm 12 代替 pin，或添加多套版本管理兼容层。修改范围限 `scripts/check/doctor.mjs` 及其现有测试；规划见 `09-06-doctor-pnpm-readonly`。

**可判定回归。** 夹具分别模拟匹配、错配、缺失、超时与敏感 stderr；用隔离 cache/cwd 证明错配快速返回且无包获取/新缓存，父环境和仓库字节不变。有可用 pnpm 10.34.5 后再运行 canonical `doctor`、`run-ci.mjs` 与 `just ci`；在此之前保持 **BLOCKED**，不把 direct CLI 结果改写成 gate pass。

### HARNESS-001 · P1 · Trellis Python 安全/会话测试不在 required CI，新 clone 可静默少测 hooks

**证据与根因。** `scripts/check/run-ci.mjs:32-62` 的 common/rust-platform lanes 只有 pnpm/cargo，没有 Python；`.github/workflows/ci.yml:79-80,110-112,157-158` 只调用这些 lanes。`.trellis/scripts/tests/test_runtime_resilience.py:54-65` 直接加载被 `.gitignore` 排除的 `.codex/.claude` hooks，`:304-314` 在两文件缺失时 `skipTest`。当前开发机有被忽略 hooks，所以本轮 31 项执行并通过；HEAD 不含这些源时，新 clone 既不能重建定制 hook，又可能以 skip 绿色结束。全新 checkout 尚未实跑，结果为 **UNVERIFIED**，但源码级覆盖缺口已确定。

**最小改造。** 复用 pinned Trellis bootstrap 和现有 `rust-platform` lane：精确提供两个定制 inject hook 的受控来源，把 Python unittest 放进每个已有 hosted OS 的 rust-platform 执行一次，使必要 hook 缺失明确失败。无需新增测试框架或 CI lane；Grok/Kimi/OMP 的静态 conformance 依各工具真实 discovery smoke 加现有 tests，不能用文本解析器模拟 harness。规划见 `09-06-harness-bootstrap-and-gates`。

**可判定回归。** 在不读取当前被忽略目录的临时 Git checkout 中运行 pinned bootstrap；正常夹具通过，删除必要 hook 后同一测试非零；`runCi.test.ts` 与 `ciWorkflowContract.test.ts` 证明 Python 步骤进入既有 lane、三主机各一次且失败向 required aggregate 传播。Windows 本轮通过不替代 Linux/macOS POSIX symlink/process-group 四项 hosted 结果。

### TEST-001 · P2 · Central 筛选到批量安装的当前交互链没有执行中的回归

**证据与根因。** `src/test/pages/CentralSkillsView.repositories-and-installs.test.tsx:541-544` 明确跳过原链路，因为 `installed-filter-*` 已被 ToolbarViewMenu、卡片 checkbox 与 BulkActionBar 取代；余下测试分别覆盖控件或批处理结果，但没有从当前已安装筛选开始，选择过滤结果并断言最终 `batchInstallSkills` 参数。2057 项通过不能覆盖这一个串联缺口。这是测试随 UI 演进未重写，不是已复现的产品故障。

**最小改造。** 仅重写 `:541` 这一用例与必要同域 fixture：经 `central-toolbar-view-installed-*` 选项筛选，点击当前卡片 checkbox 和 BulkActionBar，断言被筛掉 skill 不进入调用，并精确匹配所选 skill、目标 agent 与安装方式。不得恢复旧 UI、改生产代码、引入依赖或新测试框架。规划见 `09-06-central-bulk-install-regression`。

**可判定回归。** 目标测试先单跑，再执行 typecheck 与全 Vitest；原 `it.skip` 消失且总数变为全执行。若该串联暴露真实产品问题，返回强模型裁决范围，不能降低断言。

### EVIDENCE-001 · P2 · 当前 release / Windows 安装体验没有近期端到端证据

最近的 Release Desktop 失败属于历史进程夹具竞态，当前源码已经修复，当前 macOS CI 也通过；详见下节和 GitHub 日志。最近 30 个 release runs 中没有覆盖 2026-09-03 后 workflow/tree 的 Release Desktop run。现阶段只保留 **UNVERIFIED**：不得把 hosted CI、`vite build` 或测试夹具写成 Windows installer、Authenticode、updater `.sig` / `latest.json`、发布权限或人工安装体验通过。REL-001/REL-002 继续遵守既有 wontfix 合同，本审计不据此重开实施项。

## 历史失败的当前对应关系

- 当前成功基线是 GitHub CI [`33710069460`](https://github.com/bahayonghang/skills-manage-windows/actions/runs/33710069460)。其 PR head `9aca456d` 是当前 promotion merge 的第二父提交，且到 HEAD 的文件 diff 为空；`.github/workflows/ci.yml:260-290` 当前仍按五条 required lanes fail closed。
- 历史 CI [`33171369538`](https://github.com/bahayonghang/skills-manage-windows/actions/runs/33171369538) 的三项 `skills_cli` 失败来自宿主 well-known `npx-cli.js` 泄入 missing-npx 夹具。当前修复接缝在 `src-tauri/src/services/skills_cli/argv.rs:207-232,270-299`，三个隔离测试在 `src-tauri/src/services/skills_cli/tests.rs:495-524,1013-1028` 注入空 roots，`:1031-1046` 保留生产 fallback 正例。
- 同一历史 CI 的供应链失败已反映为当前 `package.json:68` / `pnpm-lock.yaml:80-82,5110-5118` 的 React Router 7.18.3、`src-tauri/Cargo.lock:1923-1927` 的 h2 0.4.19；`security/dependency-audit-exceptions.json:1-16` 只保留两条有 owner/reason/expiry 的 Cargo 例外。它们不是当前整改项。
- 历史 Release Desktop [`31308665794`](https://github.com/bahayonghang/skills-manage-windows/actions/runs/31308665794) 的 macOS `TerminationFailed(OutputLimit, EPERM)` 是输出溢出后夹具自然退出与 process-group terminate 的竞态。当前错误/回收路径在 `src-tauri/src/targets/runner.rs:345-375`，直接修复在 `:478-498` 让 large-output fixture 保持存活，断言在 `:615-655`。后续 `:535-570` 的 timeout fairness 稳定化是另一测试，不应冒充这次失败的直接修复。

这些历史链路有直接 run URL、日志结论和当前源文件对应关系；当前本地 1553 Rust tests 与 hosted 五 lane 通过，所以不得只凭旧日志或修复 commit hash 重新立项。

## 五套 harness 与模型分工

工具接线事实与官方来源详见 [`harness-alignment.md`](./harness-alignment.md)。测试实施按风险分工，不把工具品牌等同模型能力：

| 工具 | 当前测试用途 | 模型职责 |
| --- | --- | --- |
| Claude Code | 受控 hook/context smoke；规则 import 后有效上下文核对 | 强推理模型参与会话所有权、权限边界和 CI fail-closed 独立审查；不让 research 的 Write/Bash 自动扩大范围 |
| Codex | 受控 hook/context smoke；Trellis Python 主验收 | 强推理模型拥有 ENV-001/HARNESS-001 设计与最终 cross-layer review；确定断言后的 fixture/机械编辑可下放 |
| Grok Build | `inspect`/真实 agent discovery 与现有测试，不新增静态模拟框架 | 便宜模型可收集可复现输出；规则双加载、capability 与越界结果由强模型判读 |
| Kimi Code | 真实 agent discovery；Central 单用例实施候选 | TEST-001 可交便宜模型严格按一个测试文件执行，强模型复核筛选语义、调用参数与无生产改动；过时能力声明由 rules child处理 |
| OMP | 真实 task-agent discovery/session smoke 与现有测试 | 便宜模型可收集无敏感数据 smoke；`pi/task` 解析、审批/权限与模型边界无证据时交强模型判读并保持 **UNVERIFIED** |

跨会话任务所有权缺陷 SES-001（见 [`project-and-session-audit.md`](./project-and-session-audit.md)）直接关系到五工具任务归属，`09-06-harness-session-isolation` 必须由强推理模型实施并由另一强模型审查。`09-06-harness-rules-and-handoff` 最后消费所有 children 的实际验收证据再更新项目说明；便宜模型只能做明确文件/断言内的机械工作，不能决定 source of truth、权限、fallback、凭据、发布或全局记忆语义。

## 建议执行顺序

1. `09-06-doctor-pnpm-readonly`：先恢复无副作用的可判定诊断；本机缺 pinned pnpm 时仍单独保持门禁 BLOCKED。
2. `09-06-harness-session-isolation`：修复共享任务所有权，避免后续五工具 smoke 借用或清理他人会话。
3. `09-06-harness-bootstrap-and-gates`：精确接线来源并把 Python tests 接入现有跨平台 lane；依工具真实 discovery smoke 验收 Grok/Kimi/OMP。
4. `09-06-central-bulk-install-regression`：便宜模型只重写已定位的一个 skip，强模型审查后跑目标/全量前端门禁。
5. `09-06-harness-rules-and-handoff`：在前四项通过后统一规则入口、事实说明和已批准知识回写；未验证内容继续标注，不写入稳定能力承诺。

每一步先跑最小相关测试，再按风险扩到全量；不因 file count 新建框架，不安装未授权工具，不触发 provider/release/远程写入。实施、提交、归档、合并与 push 仍是分别授权的动作。
