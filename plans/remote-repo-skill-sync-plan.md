# 远程 Repo Skills 增删同步实施计划

## 目标

解决 `TODO.md` 最后一项：当 GitHub 远程 repo 中的 skills 发生新增或删除时，Central Skills 的“检查更新”不只比较已安装 skill 内容，还能发现 repo 级别的 skill 增删，并提供安全的同步入口。

## 当前状态与证据

- `TODO.md` 最后一项仍未完成：`解决当repo中添加或者删除skills带来的检查更新问题`。
- 当前 `check_central_skill_updates` 只接收可选 `skill_ids`，通过 `load_selected_central_skills()` 读取本地 Central 已存在的 skills，然后逐个检查远端内容；因此它天然看不到“远端新增但本地不存在”的 skill。
- 远端删除已具备基础能力：`load_remote_skill_content()` 找不到 candidate / `SKILL.md` 时会返回 `RemoteMissing`，随后写入 `skill_update_states.status = remote_missing`。
- 前端已有 remote-missing 处理链：检查更新后统计 `remote_missing`，打开删除预览，用户可以“保留本地”或“删除本地”。
- GitHub 导入链路已有可复用能力：repo preview/import 会发现远端 candidates、处理重复 ID、写入 Central 目录、`skills` 表、`skill_repository_members` 关系。
- Central 删除链路已有可复用能力：本地与 SSH 都有 batch delete preview/delete，且会保护 Central root、只删除受管副本。

## 设计决策

### 1. 不把“远端新增”塞进 `skill_update_states`

`skill_update_states` 以 `skill_id` 为主键，语义是“已存在 Central skill 的更新状态”。远端新增 skill 尚无本地 `skills` 行，强行写入会让状态表与技能列表脱节，也容易与现有 update badge/filter 冲突。

采用新的 transient repo sync preview 返回新增项：

```ts
interface CentralRepositorySyncPreview {
  states: CentralSkillUpdateState[];      // 现有 skills 的 up_to_date/update_available/remote_missing/error
  remoteAdded: CentralRemoteAddedSkill[]; // 远端存在、本地 repo 成员不存在
  remoteMissing: CentralSkillUpdateState[];
  repositories: CentralRepositorySyncSummary[];
  failedRepositories: CentralRepositorySyncFailure[];
}
```

### 2. 保留现有 skill-only 检查命令，新增 repo-sync 检查命令

不破坏现有 `check_central_skill_updates(skillIds?) -> Vec<SkillUpdateState>`，新增 command / store method：

```rust
check_central_repository_sync(repository_ids: Vec<String>, skill_ids: Option<Vec<String>>)
```

- `skill_ids`：仍用于已存在 skills 的内容更新检查。
- `repository_ids`：用于远端 candidate set 与本地 repository members 的 diff，发现新增。
- 返回 `CentralRepositorySyncPreview`，其中 `states` 仍会 upsert 到 `skill_update_states`；`remoteAdded` 不落库，等待用户确认导入。

### 3. “检查更新”只检测和预览，不自动删除

删除 Central skill 是破坏性操作。即使远端删除，也必须继续沿用现有确认模型：

- 默认不自动删除 Central skill。
- 用户可以选择：保留本地（detach remote source）或删除本地 Central 副本。
- 删除时继续复用 `preview_delete_central_skills` / `delete_central_skills`，包括平台 copy/symlink/native 处理。

新增 skill 的导入也是显式确认：

- 无冲突新增项可默认选中。
- 有 skill ID 冲突的新增项默认跳过，用户可选择 overwrite / rename / skip。
- 若同一次 sync 中“远端删除旧 skill + 远端新增同 ID skill”，apply 顺序应先执行删除选择，再执行导入选择，避免 ID 冲突误判。

### 4. Repo scope 明确化

前端检查按钮按场景选择检查模式：

| UI 场景 | 行为 |
|---|---|
| 当前 viewState 选中了单个 repo 且未手动多选 skills | repo-sync：检查该 repo 全部本地成员 + 发现远端新增 |
| Check all | repo-sync：检查全部 Central skills + 全部 GitHub repository 的远端新增 |
| 手动选择了若干 skills | skill-only：只检查所选 skills，不额外扫 repo 新增，避免用户以为只选部分却触发 repo 级同步 |
| 非 repo 过滤的 current results | skill-only：只检查当前结果，避免搜索/tag 过滤导致隐式 repo 级变更 |

后续如需要，可在更多菜单加“同步当前仓库”入口；首版只改主检查按钮逻辑即可。

## 后端实施计划

### 阶段 1：补齐 repo member 查询能力

修改范围：

- `src-tauri/src/db/repos/repositories_repo.rs`
  - 新增查询：按 repository ids 返回 Central skill members，包含 `skill_id`、`source_path`、repository metadata。
  - 可新增结构体，例如 `SkillRepositoryMember` 或内部 DTO。
  - 保持 `LOCAL_UNKNOWN_REPOSITORY_ID` 不参与 repo sync。

验收：

- 能按 repo 查到全部 Central skills，而不是只查当前前端筛选结果。
- 无 source_path 的 member 不参与 addition diff，但仍可作为 skill update unsupported 状态。

### 阶段 2：新增 repo-sync preview 类型与核心 diff helper

建议新建或扩展：

- `src-tauri/src/commands/central_updates.rs`（若保持集中）
- 或新建 `src-tauri/src/commands/central_repository_sync.rs`（更清晰，需在 `commands/mod.rs` 和 `lib.rs` 注册）

核心逻辑：

1. 校验 `repository_ids`：只处理 `source_type == github` 且非 unknown 的 repo。
2. 加载每个 repo 的本地 Central members：`local_source_paths = {source_path}`。
3. 复用现有 GitHub auth、client、snapshot cache：避免同一轮重复下载 repo snapshot。
4. 对已存在 skills 继续复用 `prepare_skill_updates()` + `load_remote_skill_content()`，生成并 upsert `SkillUpdateState`。
5. 对每个 GitHub repo snapshot 调用 `inspect_repo_skill_candidates_from_snapshot_at_path()` 或同等 helper，拿到远端 valid candidates。
6. `remoteAdded = remote_candidates where candidate.source_path not in local_source_paths`。
7. 对新增 candidates 复用 `build_preview_skills()` 风格的冲突检测，返回 `GitHubSkillPreview` 或新增包装类型。
8. invalid candidates 作为 repo sync warning 返回，不阻断整个 repo，除非 repo snapshot 下载失败。

注意：

- 对已有历史 repo 没有“导入时 source base path”字段；首版按 repository 全量可发现 candidates 与本地 source_path 做 diff。这会把曾经手动跳过的 repo skills 也视为可新增，这是可解释且安全的，因为导入仍需确认。
- 如果后续用户觉得太吵，再加 ignored additions 或 repository source base path 持久化。

### 阶段 3：新增 apply sync command

建议新增：

```rust
apply_central_repository_sync(decisions: CentralRepositorySyncDecisions)
```

或拆成前端编排两类既有命令：

- 删除/保留：继续使用现有 `keep_remote_missing_central_skills`、`delete_central_skills`。
- 新增导入：新增专用 `import_central_repository_added_skills`，按 repo 分组调用已有 GitHub import service。

推荐后端专用导入命令，原因：

- 前端不需要重新拼 repo URL / branch / source path 细节。
- local 与 SSH import 差异可在后端集中路由。
- 可在导入完成后立即刷新新增 skill 的 update state 为 `up_to_date`。

导入实现复用：

- Local：`import_github_repo_skills_partially_with_auth()` 或抽取更底层的 snapshot import helper。
- SSH：`import_github_repo_skills_ssh_with_auth()`；无 preview workspace 时允许重新创建 remote workspace。
- 冲突策略继续使用 `DuplicateResolution`：overwrite / skip / rename。

Apply 顺序：

1. validate all decisions still match latest preview shape as much as possible。
2. keep remote-missing（detach source）选择。
3. delete remote-missing 选择。
4. import remote-added selections。
5. refresh/imported skill update states（至少可对 imported ids 调一次检查，或在 import 成功后写 up_to_date）。
6. 返回 added/deleted/kept/failed 明细。

### 阶段 4：事件进度与取消

首版可复用现有 `central-update:progress`：

- phase 保持 `checking` / `updating`。
- 新增 status 可仅用于 job 文案：`remote_added` 不进入 `CentralSkillUpdateStatus`，避免污染 skill state filter。
- 如果实现 apply command，可发 `phase = updating`，`skill_id` 对新增项用 proposed id，`skill_name` 用 candidate name。

如需更准确，可后续新增 `central-repo-sync:progress`，但首版不建议增加事件面。

## 前端实施计划

### 阶段 5：类型与 store

修改范围：

- `src/types/index.ts`
  - 新增 `CentralRemoteAddedSkill`、`CentralRepositorySyncPreview`、`CentralRepositorySyncApplyResult` 等小型类型。
  - 不扩展 `CentralSkillUpdateStatus` 为 `remote_added`，避免影响现有 update filter/grouping。
- `src/stores/centralSkillsStore.types.ts`
  - 新增 `checkRepositorySync(repositoryIds, skillIds?)`。
  - 新增 `applyRepositorySync(decisions)` 或 `importRemoteAddedSkills(...)`。
- `src/stores/centralSkillsStore.updateSlice.ts`
  - 调用新 IPC 后合并 `states` 到 `updateStatuses`。
  - apply 后刷新 `skills`、`repositories`、`tags`、`updateStates`。

### 阶段 6：检查按钮 scope 计算

修改范围：

- `src/pages/centralSkillsCheckButton.ts`
  - 返回 `mode: "skill" | "repository-sync"`、`repositoryIds?: string[]`。
  - 单 repo filter 且无手动选中时进入 repository-sync。
  - all scope 可传入全部 GitHub repo ids（排除 unknown/manual）。
- `src/pages/CentralSkillsView.tsx`
  - `checkButton.onClick` 根据 mode 调用 `handleCheckUpdates` 或新 `handleCheckRepositorySync`。

### 阶段 7：Repo Sync Dialog

新增或扩展组件：

- `src/components/central/CentralRepositorySyncDialog.tsx`
  - Section A：远端新增 skills（默认选中无冲突项；冲突项显示 overwrite / rename / skip）。
  - Section B：远端删除 skills（复用 remote missing 预览模型：keep/delete + copy installs）。
  - Footer：`应用选择`。
- 或者先复用现有 `RemoteMissingSkillsDialog` 并新增 `RemoteAddedSkillsDialog`，由 workflow 顺序打开。首选单 dialog，用户能一次处理 repo 增删。

i18n：

- `src/i18n/locales/zh.json` / `en.json`
  - 新增远端新增、同步预览、应用成功/部分失败、冲突选择等文案。

### 阶段 8：Workflow 编排

修改范围：

- `src/pages/centralSkillsUpdateWorkflow.ts`
  - 新增 `handleCheckRepositorySync()`。
  - 统计 toast 增加 `remoteAdded`。
  - 若有 `update_available`，先打开现有 update confirm；关闭后再打开 repo sync dialog。
  - 若无内容更新，直接打开 repo sync dialog。
  - Apply 成功后清理 selection 中已删除的 skill ids，并刷新 counts。
- `src/components/central/CentralSkillDialogs.tsx`
  - 挂载新 dialog。

## 测试计划

### Rust 单元/集成测试

- `src-tauri/src/commands/central_updates/tests.rs` 或新 sync tests：
  - repo sync 能发现远端新增 candidate。
  - repo sync 能同时返回 `update_available` 与 `remote_missing`。
  - repo sync 不为 unknown/manual repo 做新增扫描。
  - 新增 candidate 与已有 Central skill id 冲突时返回 conflict。
  - apply 顺序：先删除 selected remote-missing，再导入同 ID remote-added。
- `src-tauri/src/services/github_import/tests.rs`：
  - 新增导入命令复用 duplicate resolution。
  - local partial import 的失败隔离保持不回退。
- `src-tauri/src/services/central_skills/tests.rs`：
  - 删除预览/删除仍保护 Central root 与 copy-only selection。

### 前端测试

- `src/test/centralSkillsStore.test.ts`
  - `checkRepositorySync` 调用新 IPC 并合并 states。
  - apply 后刷新 skills/repositories/update states。
- `src/test/CentralSkillsView.updates-and-search.test.tsx`
  - 单 repo filter 下点击检查更新走 repository-sync，并显示远端新增 dialog。
  - 手动选择 skills 后点击检查仍走 skill-only。
  - remote-added + remote-missing 同时存在时，确认顺序符合预期。
- `src/test/GitHubRepoImportWizard.test.tsx` 或新增 dialog test：
  - 冲突新增项默认 skip，无冲突默认 import。
  - rename 输入参与 apply payload。

## 验证命令

```powershell
pnpm typecheck
pnpm lint
node .\node_modules\vitest\vitest.mjs run --maxWorkers=1 src/test/centralSkillsStore.test.ts src/test/CentralSkillsView.updates-and-search.test.tsx
$env:CARGO_TARGET_DIR='D:\Documents\Code\Agents\skills-manage-windows\target-codex'; cargo test --manifest-path src-tauri/Cargo.toml central_updates
$env:CARGO_TARGET_DIR='D:\Documents\Code\Agents\skills-manage-windows\target-codex'; cargo test --manifest-path src-tauri/Cargo.toml github_import
$env:CARGO_TARGET_DIR='D:\Documents\Code\Agents\skills-manage-windows\target-codex'; cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
$env:CARGO_TARGET_DIR='D:\Documents\Code\Agents\skills-manage-windows\target-codex'; just ci
```

如涉及打包链路，不止跑前端；本计划目前不改打包，最终验收以 `just ci` 为主。

## 风险与开放点

- 首版按 repo 全量扫描新增 candidates，会重新提示此前用户手动跳过的 skills；安全但可能偏吵。若实际体验差，再加 ignored additions 或 repo source base path。
- SSH apply 新增可能重新下载 remote workspace，速度比直接复用 check snapshot 慢；优先保正确性，后续再优化缓存/复用。
- 不能自动删除 remote_missing：这是数据损失边界，必须保留确认。
- `src/types/index.ts` 有 size budget 风险，新增类型要短小，必要时拆到更窄文件但保持导出兼容。

## 完成条件

- 检查单个 GitHub repo 时，能同时列出：可更新、远端删除、远端新增。
- 用户应用同步后：选中的远端删除从 Central 删除或 detach；选中的远端新增进入 Central，并建立 repository assignment。
- 手动选中 skills 的检查行为不被 repo-sync 意外扩大。
- 本地与 SSH target 均走受管路径，删除不越过 Central root，导入不绕过既有 GitHub import 校验。
- `TODO.md` 最后一项在实现并通过验证后可勾选。

## 实施状态（2026-05-18）

- 状态：implemented_verified。
- 已实现：repo-sync preview/apply command、repo member diff、remote-added transient preview、remote-missing 安全确认、前端 store/workflow/dialog/i18n、定向回归测试。
- 模块边界：repo-sync 后端实现已拆到 `src-tauri/src/commands/central_updates/repository_sync.rs`，避免 `central_updates.rs` 再次超过 frozen size budget。
- 最终验证：`pnpm typecheck`、`pnpm lint`、`pnpm sizecheck`、定向 Vitest、`cargo test central_updates --lib`、`cargo clippy -- -D warnings`、`just ci` 均通过。
- TODO：`TODO.md` 对应项已勾选；`添加 wsl 支持` 仍是独立未完成事项。
