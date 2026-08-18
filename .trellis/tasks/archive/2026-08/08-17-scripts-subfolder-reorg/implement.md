# Implement: scripts subfolder reorg

## Ordered checklist

1. 新增 `scripts/lib/repo-root.mjs`，按 `design.md` 从调用方 `import.meta.url` 上溯到 `package.json.name === "skillport"`。
2. 按 R1 用 `git mv` 把 23 个业务文件移进 `build/` `check/` `docs/` `release/`。组内相对 `import` 保持 `./`。
3. 需要仓库根的 `.mjs` 改为 `resolveRepoRoot(import.meta.url)`。`generate_icon.py` 独立上溯到同一 `package.json`。
4. 更新 Usage 字符串与文件头路径注释。
5. 更新 `package.json` 12 条 npm script，以及 `justfile` 中全部 `node scripts/...` 行。
6. 更新 `.github/workflows/ci.yml` 与 `release-desktop.yml` 的 `run:` 路径。
7. 更新 `src/test/scripts/*` 的 import / `readFileSync` / `spawnSync` 路径。
8. 更新合同测试中的完整命令字符串，并加上 `scripts/` 根目录只含五个条目的断言。
9. 更新 R5 文档与 `.trellis/spec/quality/ci-quality-gate.md`。中英对应页路径保持一致。
10. 搜索旧入口路径，确认残留只出现在本任务研究/规划、历史报告、Skill 协议或夹具中。
11. 跑下方验证命令。实现收尾跑 `just ci`。

## Validation commands

```bash
pnpm test -- src/test/scripts src/test/contracts/ciWorkflowContract.test.ts src/test/contracts/developerExperienceContract.test.ts src/test/contracts/releaseWorkflowContract.test.ts src/test/contracts/capabilityDrift.test.ts src/test/contracts/dependencyAuditContract.test.ts src/test/scripts/syncVersion.test.ts
just doctor
just version-check
pnpm docs:gen:check
just check
just ci
```

`just ci` 是完成门禁。发布 rehearsal 不在本任务验证范围。

## Old-path search

实现后搜索这些字面量，命中必须能解释为 Out of Scope 或任务文档：

```text
scripts/run-ci.mjs
scripts/sync-version.mjs
scripts/doctor.mjs
scripts/build.mjs
scripts/install.mjs
scripts/dev-server.mjs
scripts/build-ipc-dict.mjs
scripts/build-schema-table.mjs
scripts/generated-doc-file.mjs
scripts/generate-latest-json.mjs
scripts/prepare-release-body.mjs
scripts/release-artifacts.mjs
scripts/release-context.mjs
scripts/release-draft-state.mjs
scripts/release-preflight.mjs
scripts/release-signing-state.mjs
scripts/run-vitest-sequential.mjs
scripts/check-capability-drift.mjs
scripts/check-dependency-audit.mjs
scripts/check-rust-entrypoints.mjs
scripts/check-size-budget.mjs
scripts/generate_icon.py
```

允许残留：`.trellis/tasks/08-17-scripts-subfolder-reorg/`、`docs/reports/`、`docs/vitepress-plan.md`、Skill 协议/夹具中的技能资源 `scripts/`。

## Risky files

| 文件 | 风险 |
| --- | --- |
| `scripts/lib/repo-root.mjs` | 解析错会让所有检查去读错误树。必须用 `name === "skillport"`。 |
| `scripts/docs/build-ipc-dict.mjs` / `build-schema-table.mjs` | 仓库根错会写错生成物路径。 |
| `.github/workflows/release-desktop.yml` | 漏改一条 `node scripts/...` 会在发布 job 才失败。 |
| `.trellis/spec/quality/ci-quality-gate.md` | 规格与合同测试必须同时改。 |
| `src/test/contracts/developerExperienceContract.test.ts` | 钉死 justfile / package.json 命令字符串。 |

## Rollback

还原该提交。无迁移，无运行时状态。

## Follow-up (not this task)

- 不在本任务整理 `.trellis/scripts/`。
- 不把历史报告中的旧路径改写成新路径。
