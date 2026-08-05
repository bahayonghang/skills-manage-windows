# Test Suite Layout And Discovery

## 1. Scope / Trigger

本约定适用于新增、移动或重命名前端测试与测试 helper，以及修改 Vitest include/setup、`pnpm test:serial` 或 Rust `src-tauri/tests` fixture。目标是让测试按生产代码归属可发现，并保证正常与串行两条发现路径不会漂移。

## 2. Signatures

```text
Vitest include:
  src/test/**/*.test.{js,jsx,ts,tsx}
  src/test/**/*.spec.{js,jsx,ts,tsx}

Vitest setup:
  src/test/support/setup.ts

collectTestFiles(rootDir?: string, baseDir?: string): string[]
  -> recursive repository-relative paths
  -> forward-slash normalized
  -> deterministic global sort
```

Rust integration fixture boundary：

```text
src-tauri/tests/common/mod.rs
  fresh_db()
  seed_central_skill(pool, canonical_dir, skill_id, name)
```

GitHub archive redirect regression ownership：

```text
src-tauri/src/services/github_import/tests.rs
  production codeload + numeric API validators and hostile matrices
  local 302 -> 200 and trusted 301 -> 302 -> 200 transport/auth fixtures
  mirror provenance, request-count, parser-normalization, and Location rejection

src-tauri/src/commands/skill_update_inventory.rs tests
  static IPC envelope + Operation Log code/phase

src/test/stores/updateCenterStore.test.ts
src/test/lib/backendError.test.ts
src/test/runtime/ipc.test.ts
src/test/contracts/i18nLocales.test.ts
  code preservation + recorder + bilingual rendering/parity

src-tauri/src/services/central_updates/inventory/tests.rs
  all-scope classification + unsupported persistence/reload + invalid source_path
  no-request behavior + inventory transaction rollback + baseline isolation

src-tauri/src/services/scanner/tests.rs
  missing/unreadable/successfully-empty Central root reconciliation

src-tauri/src/services/scanner/ssh_batch.rs tests
  ROOT_OK/ROOT_UNREADABLE protocol and readable/searchable root probe

src/test/components/central/updateCenter/UnsupportedTabPanel.test.tsx
src/test/components/central/updateCenter/updateCenterDecisionAggregation.test.ts
src/test/components/central/UpdateCheckModeDialog.test.tsx
src/test/pages/CentralSkillsView.updates-and-search.test.tsx
  unsupported rendering/preferred tab + tab counts + skill scope versus repository progress
```

## 3. Contracts

前端测试按主要生产代码归属放置：

| Test path | Production ownership |
| --- | --- |
| `components/<domain>/` | `src/components/<domain>/` |
| `pages/` | 页面与页面级 workflow |
| `stores/` | Zustand stores |
| `lib/` | 纯函数、view model 与通用库 |
| `hooks/` | React hooks |
| `runtime/` | IPC/runtime/logging adapter |
| `fixtures/` | 浏览器 fixture 行为 |
| `app/` | 根应用组合 |
| `contracts/` | CI、i18n、字体、主题、类型覆盖等静态契约 |
| `scripts/` | 仓库脚本 |
| `support/` | setup、IPC mock、平台替身与测试数据 |

- 普通测试不得重新堆回 `src/test` 顶层，也不得另造与生产目录竞争的产品 taxonomy。
- `src` 内生产 import 优先使用 `@/*`；同组 test harness 可用短相对路径。
- `scripts/run-vitest-sequential.mjs` 被 import 时不得执行测试，只在 CLI 直接运行时进入逐文件 runner。
- 显式 `pnpm test:serial -- <nested-path>` 与无参数递归发现都必须工作。
- `src-tauri/tests` 无法访问 `#[cfg(test)] test_support`；两个以上 integration crates 共用的数据库/skill fixture 放入 `tests/common/mod.rs`，单文件专用替身留在拥有者内。

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| 串行发现器只读取顶层 | `runVitestSequential.test.ts` 的嵌套 fixture 失败 |
| helper/非测试文件进入目录 | 发现器过滤，不进入返回清单 |
| Windows 路径进入清单 | 输出统一为 `/`，排序在规范化后稳定 |
| setup 移动但 Vite 路径未更新 | 定向测试停止验收，先修复 `setupFiles` |
| 重排后原测试 basename 缺失 | 视为意外删除，不得用降低基线通过 |
| integration crates 复制 connect/init fixture | 提取到 `tests/common/mod.rs` 并更新 backend test-support spec |
| 为外部测试扩大 `pub(crate)` | 拒绝；改测真实 public boundary 或保留模块内测试 |
| archive fixture 需要本地 HTTP，但生产只允许 codeload HTTPS | 使用 `#[cfg(test)]` private endpoint policy；生产 authority 不可配置 |
| redirect test only checks final bytes | 同时断言逐跳 request、trusted-direct provenance、Bearer 隔离、Location 拒绝与三请求上限 |
| validator 只检查 `Url::path_segments()` | parse 前拒绝 raw `\`、`%2f`、`%5c`、userinfo 与 dot segments，覆盖 parser normalization 绕过 |
| numeric canonicalization 仅凭 Location host 授权 | 必须从 initial response 显式携带 trusted-direct provenance；mirror 的相同 301 仍拒绝 |
| 本地 HTTP/1.1 fixture 每次 accept 后关闭 socket，但响应未声明关闭 | 响应加 `Connection: close`，避免 reqwest 在并行测试中复用已关闭连接 |
| all-scope test only asserts repository progress | 同时断言每个 scope skill 的分类、unsupported reload 和 baseline 不变 |
| scanner keep set is empty because Central root disappeared | 必须有缺根/不可读保留回归；只有成功扫描的空根可以证明 stale 删除 |
| 过滤命令返回 0 tests | 视为无效验证；改用完整模块限定名或完整测试名重跑 |

## 5. Good / Base / Bad Cases

- Good：新增 store 测试进入 `src/test/stores/`，Vitest 与串行脚本都自动发现。
- Base：只运行一个嵌套测试文件，`pnpm test:serial -- src/test/lib/path.test.ts` 成功。
- Bad：在 `src/test` 顶层新增测试，靠文件名前缀维持人工分组。
- Bad：Vitest 使用递归 glob，但串行脚本用单层 `readdirSync`，导致排障路径静默少跑测试。

## 6. Tests Required

- `pnpm exec vitest run src/test/scripts/runVitestSequential.test.ts`
  - 断言嵌套 `.test` / `.spec` 被发现、helper 被过滤、路径规范化并确定排序。
- `pnpm test:serial -- src/test/scripts/runVitestSequential.test.ts`
  - 断言显式嵌套路径可执行且脚本 import 无副作用。
- `pnpm exec vitest list --filesOnly`
  - 与移动前基线逐 basename 比较；新增测试单独列出，原测试不得减少。
- Rust integration fixture 改动：分别运行每个受影响的 `cargo test --test <name> --locked`。
- Archive redirect：`cargo test --manifest-path src-tauri/Cargo.toml --locked archive_redirect -- --nocapture`，
  覆盖 direct/mirror 302、trusted numeric 301、hostile parser shapes、最多三请求与
  Bearer scope；并保留既有 no-redirect、mirror auth isolation、inventory refresh 和预算定向测试。
- Frontend coded error：定向运行 Update Center store、backend error、runtime IPC 和 i18n parity 测试。
- 全技能 inventory：运行 `services::central_updates::inventory::tests::refresh_`，确认测试数
  非零，并覆盖 unsupported/reload/rollback/baseline。
- Scanner Central：分别运行 missing root、CentralRootRead 和 successfully-empty root 的完整
  测试名；SSH protocol 运行 `services::scanner::ssh_batch::tests`。过滤后 0 tests 不计入证据。
- 最终运行 `just ci`。

## 7. Wrong vs Correct

### Wrong

```js
const tests = readdirSync(testDir)
  .filter((name) => name.endsWith(".test.ts"));
```

这会在目录分类后静默遗漏嵌套测试。

### Correct

```js
const tests = collectTestFiles(testDir, repoRoot);
// src/test/lib/path.test.ts
// src/test/stores/skillStore.test.ts
```

发现逻辑递归、路径跨平台稳定，并由聚焦回归测试锁定。
