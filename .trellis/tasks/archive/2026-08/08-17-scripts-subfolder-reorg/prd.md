# Reorganize scripts into classified subfolders

## Goal

把仓库根 `scripts/` 从一层平铺改成按职责分子目录，并让 just、npm、GitHub Actions、测试与现行文档都指向新路径。开发者按目录就能找到 CI、文档生成、打包或发布脚本。

## Background

2026-08-17 盘点：`scripts/` 有 23 个文件（22 个 `.mjs` + `generate_icon.py`），没有子目录。调用方全部在本仓库内。清单与调用图见 `research/scripts-inventory.md`。

2026-08-18 选定方案 A：`build/` `check/` `docs/` `release/`。`sync-version.mjs` 放入 `check/`，因为它是 `just ci` / `just check` 的前置。

约束：

- 14 个脚本用 `dirname` × 2 解析仓库根；文档生成脚本与 `generate_icon.py` 也假定只嵌套一层。
- `package.json`、`justfile`、`.github/workflows/ci.yml`、`.github/workflows/release-desktop.yml` 直接写 `node scripts/<file>.mjs`。
- 合同测试把完整命令字符串钉死。
- 脚本之间的 ESM `import` 只发生在同一职责组内。
- `.trellis/scripts/` 与 Skill 目录中的 `scripts/` 不在本任务范围。

## Requirements

- **R1** `scripts/` 根目录只保留职责子目录与共享 helper 目录。业务脚本按方案 A 放置：
  - `scripts/build/`：`build.mjs`、`install.mjs`、`dev-server.mjs`、`generate_icon.py`
  - `scripts/check/`：`run-ci.mjs`、`run-vitest-sequential.mjs`、`doctor.mjs`、`sync-version.mjs`、`check-capability-drift.mjs`、`check-dependency-audit.mjs`、`check-rust-entrypoints.mjs`、`check-size-budget.mjs`
  - `scripts/docs/`：`build-ipc-dict.mjs`、`build-schema-table.mjs`、`generated-doc-file.mjs`
  - `scripts/release/`：`generate-latest-json.mjs`、`prepare-release-body.mjs`、`release-artifacts.mjs`、`release-context.mjs`、`release-draft-state.mjs`、`release-preflight.mjs`、`release-signing-state.mjs`
  - `scripts/lib/`：仅放仓库根解析 helper，不放业务入口。
- **R2** 同一职责组内现有 ESM `import` 必须继续有效。禁止拆开 `generated-doc-file`、`release-artifacts`、`release-signing-state` 这组同簇依赖。
- **R3** 脚本解析仓库根不得依赖固定的 `dirname` 次数。从脚本文件向上查找含 `package.json` 的目录，找到仓库根。
- **R4** 必须同步更新全部仓内调用点：`package.json` npm scripts、`justfile`、`.github/workflows/ci.yml`、`.github/workflows/release-desktop.yml`、`src/test/scripts/*`、相关合同测试、脚本自身 Usage 字符串。
- **R5** 必须同步更新现行操作文档与质量规格中的脚本路径：`docs/reference/cli-just.md` 及中文版、`docs/reference/release-process.md` 及中文版、`docs/architecture/ipc-commands.md` 及中文版、`docs/architecture/data-model.md` 及中文版、`code_map.md`、`.trellis/spec/quality/ci-quality-gate.md`。
- **R6** 脚本对外行为保持不变：CI lane、版本同步写/查、文档生成写/校验、doctor 只读、Windows `just install`、release preflight / artifacts / draft state 的参数与退出码都不改。本任务只改路径与仓库根解析。
- **R7** 不在旧路径保留 re-export 垫片。旧路径在同一提交中删除。
- **R8** 不改 `.trellis/scripts/`，不改 Skill 协议文档里作为技能资源目录的 `scripts/`，不改历史报告与计划文中的旧路径。
- **R9** 用户可见产品文案不在本任务范围。R5 列出的中英操作页路径必须一致。

## Acceptance Criteria

- [ ] AC1：`scripts/` 根下只有 `build/`、`check/`、`docs/`、`release/`、`lib/`。根目录没有业务 `.mjs` 或 `generate_icon.py`。R1 列出的每个文件都在对应子目录。
- [ ] AC2：`just doctor`、`just version-check`、`pnpm docs:gen:check` 从仓库根读到正确的源文件与生成物。
- [ ] AC3：合同测试与 `src/test/scripts/*` 全部指向新路径并通过。这些测试不再包含旧的 `scripts/<file>.mjs` 入口字符串。
- [ ] AC4：`.github/workflows/ci.yml` 与 `release-desktop.yml` 的 `run:` 命令指向新路径。`ciWorkflowContract` 与 `releaseWorkflowContract` 通过。
- [ ] AC5：R5 列出的现行文档与 `ci-quality-gate.md` 使用新路径；中英对应页路径一致。
- [ ] AC6：依赖仓库根的脚本在新目录深度下读取仓库根的 `package.json` 与 `src-tauri/`，不会去读 `scripts/package.json` 或 `scripts/src-tauri/`。
- [ ] AC7：`just check` 通过。实现完成前跑 `just ci`。

## Out of Scope

- 不改 CI lane 内容、检查顺序或失败语义。
- 不改 release 签名顺序、资产清单或 updater metadata 字段。
- 不重写脚本内部算法，除非仓库根解析或 Usage 路径必须改。
- 不整理 `.trellis/scripts/`。
- 不改 Skill 协议或夹具里的 `scripts/`。
- 不改历史 blog、optimization plan、vitepress-plan。
- 不跑真实 GitHub Release / 签名 rehearsal，除非用户另外授权。

## Technical Notes

- `.trellis/spec/quality/ci-quality-gate.md` 把 `node scripts/run-ci.mjs` 写成签名。改路径时必须改该规格。
- `src/test/scripts/` 按测试布局规格归属「仓库脚本」，不必按新子目录再拆测试文件夹。
- 仓库根解析的实现见 `design.md`：集中到 `scripts/lib/repo-root.mjs`，Python 脚本独立上溯。
