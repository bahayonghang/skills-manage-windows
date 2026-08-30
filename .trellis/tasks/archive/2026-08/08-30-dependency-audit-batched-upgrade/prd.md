# 全项目依赖审计与分批升级

## Goal

在不一次性混合所有变更的前提下，审计 SkillPort 当前受 Git 跟踪、参与构建或交付的依赖图，识别过期版本、安全公告、废弃 API、可兼容升级和 Breaking Change，并按风险从低到高逐批升级。每一批只有在完整本地质量门槛稳定通过后才能进入下一批。

## Background And Confirmed Facts

- 交付依赖的唯一受跟踪 manifest/lock/toolchain 集合是根 `package.json`、`pnpm-lock.yaml`、`pnpm-workspace.yaml`、`.node-version`、`rust-toolchain.toml`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 与根 `.github/workflows/*.yml` / `.github/dependabot.yml`。
- `ref/`、`.opencode/`、`docs/.vitepress/cache/`、`node_modules/` 与 `src-tauri/target/` 中虽存在第三方或生成的 manifest，但均不受 Git 跟踪，不属于 SkillPort 的构建、审计或交付依赖图；不得在本任务中升级。
- 当前直接依赖规模为 npm 生产依赖 28 个、开发依赖 22 个，Cargo 41 条直接声明（40 个唯一 crate），GitHub Actions 14 个唯一外部 Action；`pnpm-lock.yaml` 有约 1250 个 package entry，`cargo audit` 扫描 720 个 lock dependency。
- 2026-08-30 的完整 `pnpm audit` 报告 16 high、32 moderate、9 low；生产图为 0 high/critical、8 moderate、5 low。开发图 high 主要来自 `jsdom -> undici`、`vite` / `vitepress -> vite`、`shadcn` 与 `@typescript-eslint` 的传递依赖。
- `cargo audit` 报告 `RUSTSEC-2026-0235` (`rkyv 0.7.46`) 与 `RUSTSEC-2023-0071` (`rsa 0.9.10`) 两个 vulnerability，并有 24 个 informational warning；两个 vulnerability 当前在普通目标和 `--target all` 的 `cargo tree --invert` 中均无可达路径，只存在于 lock 图的可选闭包，现有精确例外截至 2026-11-30。
- `cargo update --dry-run` 会一次改变约 207 个 package，不能把无差别全量 lock refresh 当作低风险操作；必须按依赖族分组并审查 lock diff。
- 基线 `just ci` 已通过：Vitest 177 个文件、1966 passed / 1 skipped；Rust 主库 1468 passed / 7 ignored，其他 binary、integration 与 doc tests 全部通过；Clippy、类型检查、前端构建、IPC 生成检查和文档构建均通过。
- 基线没有编译器或 linter 可见的 deprecated API 诊断。源码也未使用 jest-dom 已知 deprecated matcher；废弃风险主要是上游版本线和即将移除的 API，而不是已确认的当前源码告警。
- `.trellis/spec/quality/ci-quality-gate.md` 的“Current exceptions”文字仍描述过期的 2026-08-11 React Router/RSA 例外，与实际例外文件不一致；本任务必须修复该依赖治理文档漂移。

## Requirements

### R1 — 审计边界

- 审计所有受跟踪且参与 SkillPort 构建、测试、打包、发布或供应链门槛的 npm、Cargo、工具链和 GitHub Actions 依赖。
- 将忽略的参考源码、缓存和构建输出列为明确排除项，不把第三方参考仓库误当作本项目依赖。

### R2 — 分类与证据

- 对发现项区分：已是最新、兼容范围内可升级、需要显式 manifest 变更、含 Breaking Change、废弃/无人维护、安全公告、仅 lock 图不可达、以及尚无稳定升级目标。
- 版本和公告结论必须注明扫描日期，并优先依据 npm/crates.io、RustSec、GitHub Advisory 与上游官方 changelog/release。
- 不把 lock-wide advisory 自动表述为已证实的运行时可利用风险；同时不得因当前不可达就从审计结果中隐藏。

### R3 — 计划先行

- 在用户明确批准最终规划摘要前，只允许修改本任务的 Trellis 规划/研究文件，不得修改产品 manifest、lockfile、源码、工作流或审计例外。
- 若批准后计划发生实质变化，必须重新展示更新后的规划摘要并再次获得批准。

### R4 — 风险递增批次

- 按 `implement.md` 的批次顺序执行；一个批次只包含可共同回滚、共同验证的依赖族。
- 同一批内先运行最小聚焦检查，再运行 `just ci` 与 `just audit`；两者均成功且工作树 diff 符合本批范围后，才能进入下一批。
- 不得为了减少批次数量，把工具链、Actions、数据库、凭据、网络、归档或测试运行时的 Breaking Change 混入普通 patch/minor 更新。

### R5 — 失败处理

- 任一检查失败时停止后续批次，定位为依赖行为变化、类型/API 迁移、fixture 假设、平台差异或基线无关故障。
- 修复必须留在失败批次的最小范围内；先重跑失败的聚焦检查，再完整重跑 `just ci` 与 `just audit`。
- 不得删除测试、降低断言、放宽 `-D warnings`、扩大 audit severity ignore 或加入无到期日例外来制造通过。
- 若确需新增或扩大安全例外，视为新的风险决策，暂停并请求用户批准。

### R6 — 跨生态一致性

- Tauri 核心、CLI 与 JS/Rust plugin 对应项按兼容组处理，保持 capability、IPC、生成文档和 Windows bundle 一致。
- Node、pnpm、Rust 与 GitHub Actions 中的版本锚点同步更新；Action 继续使用完整 40 字符 SHA。
- 任何 Tauri command 或数据库 schema 变化（预期不应发生）都必须运行 `pnpm docs:gen` 并提交生成物；无源码变化时生成物应保持不变。

### R7 — Breaking Change 隔离

- `@testing-library/jest-dom 7`、`jsdom 30`、`TypeScript 7`、`base64 0.23`、`sha2 0.11`、`zip 8`、`reqwest 0.13`、`keyring 4` 与 `sqlx 0.9` 分成可独立验证的子批次。
- `VitePress 2` alpha、pnpm 11 与尚未稳定的 Specta/Tauri-Specta v2 不直接采用；只记录为延后项。

### R8 — 证据边界

- 每批报告实际执行的命令、通过/失败/跳过结果以及剩余 advisory。
- 最终本地收口除 `just ci` / `just audit` 外运行一次 `just build` 并验证 Windows NSIS 产物存在。
- 本地成功不等于 GitHub hosted runner、Azure 签名、真实 OS credential store、生产 updater 或跨平台 bundle 已验证；这些必须明确标为 `UNVERIFIED`，且本任务不进行 push、PR、发布或安装。

## Acceptance Criteria

- [ ] `research/dependency-audit.md` 覆盖 npm、Cargo、工具链、Actions、安全公告、废弃 API 与排除边界，并带有当前日期和官方来源。
- [ ] 每个直接过期依赖被归入兼容升级、Breaking Change 或明确延后之一；不存在未分类的直接依赖。
- [ ] 计划按低到高风险分批，批次边界、聚焦检查、完整门槛、回滚点和停止条件明确。
- [ ] 每批实施后 `just ci` 退出码为 0，`just audit` 退出码为 0，且没有使用新的未批准安全例外。
- [ ] 生产 npm high/critical 保持为 0；完整开发图的 high advisory 应在可稳定修复范围内清零，不能稳定修复的项有上游与暴露边界说明。
- [ ] Cargo vulnerability 被移除，或继续由精确、未过期、与当前可达性证据一致的例外覆盖；informational warning 数量不因升级无说明地增加。
- [ ] 当前源码仍无 compiler/linter deprecated API 诊断；Breaking Change 所需迁移有聚焦回归覆盖。
- [ ] `.trellis/spec/quality/ci-quality-gate.md` 与实际 dependency audit exception 保持一致。
- [ ] 最终 `just build` 成功且 `outputs/` 中存在最新 Windows NSIS；安装、签名、hosted CI 与生产 updater 证据分别标注为未执行或 `UNVERIFIED`。

## Out Of Scope

- 升级或修改不受 Git 跟踪的 `ref/`、`.opencode/`、缓存、`node_modules/` 或 Cargo target 内容。
- 采用 VitePress 2 alpha、pnpm 11、未稳定的 Specta/Tauri-Specta v2，或为了消除低/中风险 lock 告警重写品牌图标系统。
- 修改业务功能、数据库 schema、IPC 契约或用户可见行为；只有依赖迁移直接要求的最小兼容修复在范围内。
- push、PR、GitHub Actions 远程执行、Azure/Authenticode、发布、安装器安装/卸载、真实凭据迁移与生产验证。
