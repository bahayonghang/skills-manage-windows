# Design: scripts subfolder reorg

## Scope And Boundaries

本设计只改仓库根 `scripts/` 的目录布局、仓库根解析，以及仓内对这些入口路径的引用。

不改：CI lane 步骤、release 资产合同、脚本算法、`.trellis/scripts/`、Skill 协议中的 `scripts/`。

## Target Layout

```text
scripts/
  lib/repo-root.mjs
  build/build.mjs
  build/install.mjs
  build/dev-server.mjs
  build/generate_icon.py
  check/run-ci.mjs
  check/run-vitest-sequential.mjs
  check/doctor.mjs
  check/sync-version.mjs
  check/check-capability-drift.mjs
  check/check-dependency-audit.mjs
  check/check-rust-entrypoints.mjs
  check/check-size-budget.mjs
  docs/build-ipc-dict.mjs
  docs/build-schema-table.mjs
  docs/generated-doc-file.mjs
  release/generate-latest-json.mjs
  release/prepare-release-body.mjs
  release/release-artifacts.mjs
  release/release-context.mjs
  release/release-draft-state.mjs
  release/release-preflight.mjs
  release/release-signing-state.mjs
```

`scripts/` 根目录不得再放业务入口。`lib/` 只放共享解析 helper。

## Repo Root Resolution

现状把脚本位置写死为 `scripts/<file>`：

```text
dirname(dirname(fileURLToPath(import.meta.url)))
new URL('.', import.meta.url) + resolve(.., '..')
Path(__file__).resolve().parents[1]
```

搬到 `scripts/<group>/<file>` 后，上述计算会把 `scripts/` 当成仓库根。

新增 `scripts/lib/repo-root.mjs`：

```text
resolveRepoRoot(fromUrl = import.meta.url)
  -> start at dirname(fileURLToPath(fromUrl))
  -> walk parents until package.json exists
  -> require that package.json.name === "skillport"
  -> otherwise keep walking; fail if filesystem root is reached
```

调用约定：每个需要仓库根的 `.mjs` 写

```js
import { resolveRepoRoot } from "../lib/repo-root.mjs";
const repoRoot = resolveRepoRoot(import.meta.url);
```

不要在 helper 内部用 helper 自己的 `import.meta.url` 当起点后再写死 `../..`。起点必须是调用方文件。

`generate_icon.py` 不引入 Node。它从 `__file__` 向上查找含 `"name": "skillport"` 的 `package.json`。

release 组脚本当前用 `process.cwd()`，不解析仓库根。搬迁后保持 cwd 语义，不强制改用 helper。

## Internal Imports

组内相对路径保持不变：

| 调用方 | 保持 |
| --- | --- |
| `scripts/docs/build-ipc-dict.mjs` | `./generated-doc-file.mjs` |
| `scripts/docs/build-schema-table.mjs` | `./generated-doc-file.mjs` |
| `scripts/release/release-preflight.mjs` | `./release-signing-state.mjs` |
| `scripts/release/release-draft-state.mjs` | `./release-artifacts.mjs` |

禁止跨组 `import`。`run-ci.mjs` 继续通过 pnpm script 间接调用其它检查。

## Caller Path Map

入口文件名不变，只加一层目录。

| 旧路径 | 新路径 |
| --- | --- |
| `scripts/dev-server.mjs` | `scripts/build/dev-server.mjs` |
| `scripts/build.mjs` | `scripts/build/build.mjs` |
| `scripts/install.mjs` | `scripts/build/install.mjs` |
| `scripts/generate_icon.py` | `scripts/build/generate_icon.py` |
| `scripts/run-ci.mjs` | `scripts/check/run-ci.mjs` |
| `scripts/run-vitest-sequential.mjs` | `scripts/check/run-vitest-sequential.mjs` |
| `scripts/doctor.mjs` | `scripts/check/doctor.mjs` |
| `scripts/sync-version.mjs` | `scripts/check/sync-version.mjs` |
| `scripts/check-capability-drift.mjs` | `scripts/check/check-capability-drift.mjs` |
| `scripts/check-dependency-audit.mjs` | `scripts/check/check-dependency-audit.mjs` |
| `scripts/check-rust-entrypoints.mjs` | `scripts/check/check-rust-entrypoints.mjs` |
| `scripts/check-size-budget.mjs` | `scripts/check/check-size-budget.mjs` |
| `scripts/build-ipc-dict.mjs` | `scripts/docs/build-ipc-dict.mjs` |
| `scripts/build-schema-table.mjs` | `scripts/docs/build-schema-table.mjs` |
| `scripts/generated-doc-file.mjs` | `scripts/docs/generated-doc-file.mjs` |
| `scripts/generate-latest-json.mjs` | `scripts/release/generate-latest-json.mjs` |
| `scripts/prepare-release-body.mjs` | `scripts/release/prepare-release-body.mjs` |
| `scripts/release-*.mjs` | `scripts/release/release-*.mjs` |

`just` recipe 名称、npm script 名称、CI job 名称保持不变。

## Tests

`src/test/scripts/` 留在原处。动态 `import` 与 `readFileSync` 改为新路径，例如：

```text
../../../scripts/check/run-ci.mjs
../../../scripts/docs/build-ipc-dict.mjs
```

`runCi.test.ts` 的 `spawnSync(process.execPath, ["scripts/run-ci.mjs", ...])` 改为 `scripts/check/run-ci.mjs`。

合同测试中的完整命令字符串一并改写：

- `developerExperienceContract.test.ts`
- `syncVersion.test.ts`
- `ciWorkflowContract.test.ts`
- `releaseWorkflowContract.test.ts`
- `capabilityDrift.test.ts`
- `dependencyAuditContract.test.ts`

在 `developerExperienceContract` 增加一条布局断言：`scripts/` 根目录条目仅为 `build`、`check`、`docs`、`release`、`lib`。

Usage / 文件头注释里的 `node scripts/<file>.mjs` 改为新路径。

## Compatibility

不保留旧路径垫片。搬迁与调用点更新同一提交完成。

## Rollback

还原该提交即可。无数据库迁移，无 IPC 变更，无生成物格式变更。

## Risks

- 漏改 workflow 或合同测试中的一条字符串，common lane 会红。实现后用仓库搜索核对旧入口路径。
- 仓库根 helper 若在遇到任意 `package.json` 时停下，可能误停在未来新增的嵌套包。用 `name === "skillport"` 避免。
- `docs:gen` 的仓库根若算错，会把生成物写到 `scripts/docs/architecture/_generated/`。`pnpm docs:gen:check` 会发现漂移或找不到既有生成物。
