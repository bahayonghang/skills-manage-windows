# Implementation Plan

## Completion Contract

GitHub 导入 Preview 对每个候选返回与实际 sourcePath 内容边界一致的文件清单；向导以 `SKILL.md / 文件树 / AI 导入摘要` 三个 tab 展示，文件树在 1、177、650 乃至 archive 上限规模下保持可核对和有界渲染。Result、selection、持久化和实际复制逻辑不变。

## Ordered Steps

### 1. Add Preview File DTO Without Broadening Shared Callers

- 在 `src-tauri/src/services/github_import/types.rs` 新增 `GitHubSkillPreviewFile { path, byte_len }`，并给 `GitHubSkillPreview` 增加 optional `files`。
- 在 `src-tauri/src/services/github_import/mod.rs` 只导出前端 IPC 所需类型；`build_preview_skills` 继续设置 `files: None`，保证 CLI、skills.sh、Marketplace 与 Central sync 不自动枚举文件。
- 在 `src/types/index.ts` 增加 camelCase 镜像。
- 更新直接构造 `GitHubSkillPreview` 的 Rust 测试/fixture；用序列化测试证明 `None` 被省略、`Some` 使用 `path/byteLen`，且 import selection/result 没有该字段。

Verify:

```powershell
pnpm typecheck
cargo test --manifest-path src-tauri/Cargo.toml github_import --lib
```

### 2. Populate Local Preview From The Existing Snapshot

- 调整 `preview_github_repo_import_with_auth`，保留本次下载的 `GitHubRepoSnapshot` 并从同一 snapshot 构建候选与文件清单；不得追加第二次下载。
- 增加小型后端 helper，把 snapshot repo paths 通过 `repo_file_relative_to_source` 映射成稳定排序的 DTO 条目并汇总字节数；可在内部复用 `collect_snapshot_source_files`，但不复制 root / nested 分支。
- 验证每个有效 preview 至少含相对路径 `SKILL.md`；缺失或清单构建失败时返回 typed GitHub import error。
- 保持 `fetch_repo_skill_candidates_from_source` 的 Marketplace 调用契约不变。
- 添加 root 完整树、nested 精确子树、兄弟排除、稳定排序和字节数测试。

Verify:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml github_import --lib
```

### 3. Populate SSH / WSL Preview From One Workspace Inventory

- 在现有 remote workspace/source ownership 内增加一次递归文件 inventory helper，经 `ConnectedRemoteTarget::run_script` 返回 repo-relative path + byte size。
- 单次枚举整个 workspace，再用共享 `repo_file_relative_to_source` 为所有候选切分；禁止 candidate × remote round-trip。
- 对输出解析、稳定排序、文件预算、非法/空路径和缺失 `SKILL.md` fail closed。
- 保持 `previewWorkspaceId` 注册、TTL、重用、失败清理和 `remote_skill_source_dir` 导入路径不变。
- 添加 remote output parser、命令参数传递、root/nested 等价、multiple candidate 单次 inventory 与失败清理测试；不依赖真实 SSH。

Verify:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml github_import --lib
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

### 4. Build A Shared Pure Tree Model

- 检查并扩展 `src/lib/fileTree.ts`：抽取从规范化文件路径构建目录树的纯 helper，复用现有目录优先排序；skills.sh adapter 保持行为不变。
- 为 preview files 派生目录、递归文件数、聚合大小、visible rows 与默认展开集合。路径校验失败返回显式错误状态，不静默丢条目。
- 单独增加 helper 测试：single file、root directories、deep nesting、同名层级、排序、统计、默认展开、折叠后的 visible rows 与 20,000 个根级文件。
- 不把 UI 状态或 i18n 放进 pure helper。

Verify:

```powershell
pnpm exec vitest run src/test/fileTree.test.ts
pnpm typecheck
```

### 5. Add The Files Tab As A Focused Component

- 将 `DetailTab` 扩展为 `overview | files | ai`；把 `overview` 的用户可见 label 改为 `SKILL.md`。
- 新建小型 `GitHubImportFileTree` 组件，避免继续放大 `GitHubRepoImportWizardPreview.tsx`；复用 Lucide icons、主题 token、focus 样式和仓库 `VirtualizedList`，不新增依赖。
- 显示 preview snapshot 提示、文件/目录/总大小、rename-aware 根名、目录 disclosure、文件大小和独立滚动视口。
- 文件节点保持静态文本；不增加任意文件读取、hover 工具栏、搜索、文件选择或 expand-all。
- skill 切换/re-preview 时重置 tree scroll；tab 选择保持现有行为。缺失 `files` 显示契约错误并阻止 Review import，而不是空树。
- 同步 `src/i18n/locales/en.json`、`src/i18n/locales/zh.json`，注意单复数和长文案不溢出。

Verify:

```powershell
pnpm exec vitest run src/test/GitHubRepoImportWizard.test.tsx src/test/CentralSkillsView.github-import-preview.test.tsx
pnpm typecheck
pnpm lint
```

### 6. Lock UX, Compatibility, And Accessibility Regressions

- 更新 wizard fixtures，为 Preview 场景提供文件清单；保留非向导 `GitHubSkillPreview` fixture 不带 `files` 的兼容测试。
- 覆盖 tab 顺序、single-file、root/nested、展开/折叠、rename 根名、技能切换、re-preview、缺失清单、browser fixture 和大树有界 DOM。
- 使用 keyboard events 验证目录按钮 Enter/Space、`aria-expanded`、focus-visible 可达；文件行不得出现在 button 查询中。
- 保留 selection payload 的精确断言，证明 `files` 不进入 `GitHubSkillImportSelection`。
- 检查 light/dark 主题语义 token，不写死颜色，不用颜色单独区分 file/folder。

Verify:

```powershell
pnpm exec vitest run src/test/GitHubRepoImportWizard.test.tsx src/test/CentralSkillsView.github-import-preview.test.tsx src/test/SkillDetailFileTree.test.tsx
pnpm typecheck
pnpm lint
```

### 7. Update The Executable Contract And Run Full Gates

- 在 `.trellis/spec/backend/github-import-preview-contract.md` 增加 Preview File Manifest 场景，记录 DTO、边界、fail-closed、local/remote parity 与 display-only 约束。
- 检查是否需要在现有 frontend IPC fixture spec 增加一条 optional display-field fixture 约定；只有形成可复用跨任务规则时才更新，不为任务日志扩写 spec。
- 对所有本任务文件执行格式与 diff 检查，再跑聚合 gate。

Verify:

```powershell
pnpm exec vitest run src/test/GitHubRepoImportWizard.test.tsx src/test/CentralSkillsView.github-import-preview.test.tsx src/test/fileTree.test.ts src/test/SkillDetailFileTree.test.tsx
cargo test --manifest-path src-tauri/Cargo.toml github_import --lib
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
pnpm typecheck
pnpm lint
git diff --check
just ci
```

## Risk And Rollback Points

- **Shared DTO blast radius:** `GitHubSkillPreview` has CLI/skills.sh/Central sync consumers. Keep `files` optional and populate only in import preview commands; run their existing tests before completion.
- **Remote command output:** malformed delimiter/path parsing must fail preview and clean workspace. Keep parser pure and command invocation fake-testable.
- **Large tree rendering:** archive cap is 20,000 files. Build/flatten in memory once per selected skill and virtualize visible rows; assert bounded rendered row count.
- **Wizard size budget:** keep tree model and component in sibling files; do not inflate the already large preview component beyond repo sizecheck limits.
- **Preview/import drift:** copy must say preview snapshot. Commit pinning and Result verification are explicitly out of scope.
- **Dirty tree:** do not modify or revert the pre-existing `package.json`, `src-tauri/Cargo.lock`, `src-tauri/Cargo.toml`, or `src-tauri/tauri.conf.json` changes unless implementation proves a direct dependency, which this design does not expect.

Rollback is one coherent slice: remove optional DTO + local/remote population + Files tab/i18n/tests/spec scenario together. No database or filesystem migration is involved.

## Review Gate

Do not run `task.py start` until the user reviews `prd.md`, `design.md`, and `implement.md` and explicitly asks to begin implementation.

## Implementation Result

- Backend GitHub import preview now attaches a stable, byte-sized file manifest from the existing local snapshot or one remote workspace inventory. Missing or invalid manifests fail closed.
- The wizard now exposes `SKILL.md / File tree / AI import summary`, derives a virtualized read-only tree, follows rename decisions, and blocks Review when selected manifests are untrustworthy.
- Responsive verification covered 1440x900, 960x720, the Tauri minimum 900x600, and 640x800. Short viewports scroll the Preview workspace instead of clipping the detail pane; no horizontal overflow was observed.
- Keyboard verification expanded a nested directory with Enter, and dark/light theme screenshots showed visible focus, selection, and file/directory states.
- Final verification on 2026-07-15: GitHub import Rust tests `78/78`, affected frontend tests `20/20`, broader GitHub preview tests `58/58`, and `just ci` passed with 124 frontend test files (`1348` passed, `1` skipped) plus all `799` Rust tests.
