# scripts/ 现状盘点

盘点日期：2026-08-17。范围：仓库根 `scripts/`，不含 `.trellis/scripts/`，也不含 Skill 协议里的 `scripts/` 目录。

## 1. 目录现状

`scripts/` 当前是一层平铺，共 23 个文件：22 个 `.mjs`，1 个 `generate_icon.py`。没有子目录。

`code_map.md` 已把该目录写成五类职责：Version、CI、docs、build、release。文件名没有按这五类分组。

## 2. 文件职责

| 文件 | 职责 | 仓库根解析 | 内部依赖 |
| --- | --- | --- | --- |
| `sync-version.mjs` | 以 `package.json` 为源，写/检查 Tauri 与 Cargo 版本 | `dirname` × 2 | 无 |
| `doctor.mjs` | 只读工具链诊断 | `dirname` × 2 | 无 |
| `run-ci.mjs` | CI lane 编排；再调 pnpm/cargo | `dirname` × 2 | 无（通过 pnpm 间接调其它脚本） |
| `run-vitest-sequential.mjs` | 按文件串行跑 Vitest | `dirname` × 2 | 无 |
| `check-capability-drift.mjs` | IPC capability 合同漂移 | `dirname` × 2 | 无 |
| `check-dependency-audit.mjs` | npm/cargo advisory 门禁 | `dirname` × 2 | 无 |
| `check-rust-entrypoints.mjs` | Cargo bin/default-run 合同 | `dirname` × 2 | 无 |
| `check-size-budget.mjs` | 生产源 800 行上限 | `dirname` × 2 | 无 |
| `dev-server.mjs` | Vite 开发服务器包装，端口 24200 | `dirname` × 2 | 无 |
| `build.mjs` | 当前平台 Tauri bundle，复制到 `outputs/` | `dirname` × 2 | 无 |
| `install.mjs` | Windows NSIS 被动安装 `outputs/` 最新包 | `dirname` × 2 | 无 |
| `generate_icon.py` | 打印 icon 重建命令；不画图、不覆盖 master | `parents[1]` | 无 |
| `build-ipc-dict.mjs` | 扫描 `#[tauri::command]`，写 IPC 字典 | `URL('.')` + `..` | `./generated-doc-file.mjs` |
| `build-schema-table.mjs` | 扫描 schema `CREATE TABLE`，写字段表 | `URL('.')` + `..` | `./generated-doc-file.mjs` |
| `generated-doc-file.mjs` | 生成文档写/校验 helper | 不解析仓库根 | 无 |
| `generate-latest-json.mjs` | 写 updater `latest.json` | 不解析仓库根 | 无 |
| `prepare-release-body.mjs` | 生成 GitHub Release body | 不解析仓库根 | 无 |
| `release-artifacts.mjs` | 期望资产清单与 checksum | 不解析仓库根 | 无 |
| `release-context.mjs` | 冻结 tag/version/SHA | 不解析仓库根 | 无 |
| `release-draft-state.mjs` | 校验 draft/public Release 状态 | 不解析仓库根 | `./release-artifacts.mjs` |
| `release-preflight.mjs` | 发布前签名与资产预检 | 不解析仓库根 | `./release-signing-state.mjs` |
| `release-signing-state.mjs` | Authenticode / updater 签名状态 | 不解析仓库根 | 无 |

`run-ci.mjs` 不直接 `import` 其它脚本。它通过 `pnpm version:check`、`pnpm docs:gen:check`、`pnpm capabilitycheck` 等 npm script 间接调用它们。

## 3. 按职责的自然分组

依据调用关系与文件名前缀，当前 23 个文件落在四组：

### check / ci（8）

`run-ci.mjs`、`run-vitest-sequential.mjs`、`doctor.mjs`、`check-capability-drift.mjs`、`check-dependency-audit.mjs`、`check-rust-entrypoints.mjs`、`check-size-budget.mjs`、`sync-version.mjs`

`sync-version.mjs` 也是 `just build` 的前置。它写文件，不是只读检查。放在 check 组是因为 CI 与本地 gate 都要经过它。

### build（4）

`build.mjs`、`install.mjs`、`dev-server.mjs`、`generate_icon.py`

### docs（3）

`build-ipc-dict.mjs`、`build-schema-table.mjs`、`generated-doc-file.mjs`

### release（7）

`generate-latest-json.mjs`、`prepare-release-body.mjs`、`release-artifacts.mjs`、`release-context.mjs`、`release-draft-state.mjs`、`release-preflight.mjs`、`release-signing-state.mjs`

内部 `import` 只发生在同一组内。跨组没有 ESM 依赖。因此一组一个子目录不会拆开现有模块边界。

## 4. 调用点

### 4.1 package.json（12 条 npm script）

| npm script | 当前命令 |
| --- | --- |
| `dev` | `node scripts/dev-server.mjs` |
| `entrypointcheck` | `node scripts/check-rust-entrypoints.mjs` |
| `test:serial` | `node scripts/run-vitest-sequential.mjs` |
| `version:check` | `node scripts/sync-version.mjs --check` |
| `sizecheck` | `node scripts/check-size-budget.mjs` |
| `capabilitycheck` | `node scripts/check-capability-drift.mjs` |
| `audit:dependencies` | `node scripts/check-dependency-audit.mjs` |
| `doctor` | `node scripts/doctor.mjs` |
| `release:preflight` | `node scripts/release-preflight.mjs` |
| `docs:gen` | `node scripts/build-ipc-dict.mjs && node scripts/build-schema-table.mjs` |
| `docs:gen:check` | 同上，带 `--check` |

`docs:dev` / `docs:build` 只经过上述 npm script，不直接写路径。

### 4.2 justfile

| recipe | 当前命令 |
| --- | --- |
| `sync-version` | `node scripts/sync-version.mjs` |
| `doctor` | `node scripts/doctor.mjs` |
| `check` | `node scripts/run-ci.mjs --lane quick` |
| `ci` | `node scripts/run-ci.mjs` |
| `build` | `node scripts/build.mjs` |
| `_install_windows` / `_install_unsupported` | `node scripts/install.mjs` |

### 4.3 GitHub Actions

`.github/workflows/ci.yml`：

- `node scripts/run-ci.mjs --lane common`
- `node scripts/run-ci.mjs --lane rust-platform`（Windows / Linux / macOS 各一次）
- `node scripts/sync-version.mjs --check`（三个手动 smoke package job）

`.github/workflows/release-desktop.yml`：

- `node scripts/release-context.mjs ... --resolve-only`
- `node scripts/release-context.mjs ...`
- `node scripts/sync-version.mjs --check`（Windows / macOS / Linux build）
- `node scripts/generate-latest-json.mjs ...`
- `node scripts/release-preflight.mjs ...`（install smoke + aggregate）
- `node scripts/release-artifacts.mjs ...`（写 checksum + `--verify`，publish 后再验一次）
- `node scripts/prepare-release-body.mjs ...`
- `node scripts/release-draft-state.mjs ...`（draft 与 public 各一次）

`.github/workflows/docs.yml` 只跑 `pnpm docs:build`，不直接写 `scripts/` 路径。

### 4.4 测试里的硬编码路径

`src/test/scripts/`（11 个文件）用 `import("../../../scripts/<file>.mjs")` 加载脚本。`runCi.test.ts` 还用 `spawnSync(process.execPath, ["scripts/run-ci.mjs", ...])` 做 CLI 冒烟。`releaseContext.test.ts` 与 `releaseWorkflowContract.test.ts` 用 `readFileSync("scripts/...")` 读源码。

合同测试把完整命令字符串钉死：

- `developerExperienceContract.test.ts`：`package.json.scripts.doctor === "node scripts/doctor.mjs"`；justfile 正则匹配 `scripts/doctor.mjs`、`scripts/run-ci.mjs`
- `syncVersion.test.ts`：`version:check === "node scripts/sync-version.mjs --check"`；justfile 正则匹配 `scripts/sync-version.mjs`、`scripts/run-ci.mjs`
- `ciWorkflowContract.test.ts`：`step.run === "node scripts/run-ci.mjs --lane common|rust-platform"`
- `capabilityDrift.test.ts` / `dependencyAuditContract.test.ts`：动态 import 旧路径

`.trellis/spec/quality/test-suite-layout.md` 规定 `src/test/scripts/` 归属仓库脚本。测试文件本身不必随脚本子目录再拆一层。

### 4.5 现行文档与规格

必须与实现同步的路径引用：

- `.trellis/spec/quality/ci-quality-gate.md`：`node scripts/run-ci.mjs`、`node scripts/sync-version.mjs`、`node scripts/check-dependency-audit.mjs`、`node scripts/doctor.mjs`
- `docs/reference/cli-just.md` 与中文版
- `docs/reference/release-process.md` 与中文版
- `docs/architecture/ipc-commands.md` 与中文版
- `docs/architecture/data-model.md` 与中文版
- `code_map.md` 搜索锚点

`README.md` / `README_CN.md` 只写 `scripts/` 目录角色，不钉具体文件名。

以下命中是 Skill 协议或历史记录，不是仓库脚本调用：

- `docs/reference/skill-protocol.md` 与中文版、`docs/blog/2026-04-09-research.md`、`docs/research-report.md`：Skill 目录里的可选 `scripts/`
- `docs/reports/skills-manage-windows-optimization-plan.md`、`docs/vitepress-plan.md`：历史计划
- `src-tauri` 测试里的 `scripts/run.py` / `scripts/run.ps1`：Skill 夹具路径

仓库外调用方未发现。没有发布 npm 包导出这些脚本。

## 5. 搬迁后会立刻坏掉的机制

### 5.1 仓库根解析写死了一层目录

14 个脚本用 `dirname(dirname(fileURLToPath(import.meta.url)))`，假定文件位于 `scripts/<name>.mjs`。

`build-ipc-dict.mjs` 与 `build-schema-table.mjs` 用 `new URL('.', import.meta.url)` 再 `resolve(..., '..')`，同样假定脚本目录的上一级是仓库根。

`generate_icon.py` 用 `Path(__file__).resolve().parents[1]`，同样假定一层。

移到 `scripts/<group>/<name>` 后，这些脚本会把 `scripts/` 当成仓库根，随后去读 `scripts/package.json`、`scripts/src-tauri/...`，表现为静默找错路径或立即失败。

release 组多数脚本用 `process.cwd()` 与相对路径，搬迁后只要工作目录仍是仓库根，行为不变。

### 5.2 合同测试会红

只要 `package.json`、`justfile`、workflow `run:` 字符串与测试期望不一致，`just check` / `just ci` 的 common lane 会失败。路径更新必须与合同测试同一提交。

### 5.3 Usage 字符串

若干脚本的 `--help` / 错误信息写死 `node scripts/<file>.mjs`。搬迁后若不改，诊断会指向不存在的路径。

## 6. 兼容垫片评估

旧路径留一层 `export * from './check/run-ci.mjs'` 再导出：

- `scripts/` 根仍会有约 22 个垫片文件，分类效果被抵消。
- 合同测试若继续钉旧路径，新路径不会成为规范。
- 仓库外没有调用方需要过渡期。

结论：不建议长期垫片。一次改完所有仓内调用点。

## 7. 候选目录方案

2026-08-18 已选定方案 A。

### 方案 A（已选定）：四组

```text
scripts/build/    build.mjs install.mjs dev-server.mjs generate_icon.py
scripts/check/    run-ci.mjs run-vitest-sequential.mjs doctor.mjs
                  sync-version.mjs check-*.mjs
scripts/docs/     build-ipc-dict.mjs build-schema-table.mjs generated-doc-file.mjs
scripts/release/  generate-latest-json.mjs prepare-release-body.mjs release-*.mjs
```

与现有文件名前缀和调用簇一致。内部 import 不跨目录。

### 方案 B：五组，对齐 code_map 用词

在方案 A 上把 `sync-version.mjs` 单独放到 `scripts/version/`。语义更准，但会多一个只含单文件的目录。

### 方案 C：三组

`scripts/ci/`（含 build/dev/doctor/sync-version）、`scripts/docs/`、`scripts/release/`。目录更少，但 `ci` 会混进打包与开发服务器。

三种方案都需要：

- 把仓库根解析改成与嵌套深度无关（按 `package.json` 上溯，或集中一个 `scripts/lib/repo-root.mjs`）
- 同步改 package.json、justfile、workflows、测试、现行文档与 `ci-quality-gate.md`

## 8. 验收时必须跑通的检查

- 合同测试：`src/test/contracts/{ciWorkflow,developerExperience,releaseWorkflow,capabilityDrift,dependencyAudit}Contract.test.ts` 与 `src/test/scripts/*.test.ts`
- `pnpm version:check`、`pnpm doctor`、`pnpm docs:gen:check`
- `just check`；完成前 `just ci`

Windows 打包与 GitHub Release 工作流本任务不改行为，只改脚本路径。发布链路的实机验证不在本机完成，除非用户另外授权跑 rehearsal。
