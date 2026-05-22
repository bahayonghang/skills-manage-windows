# 更新机制 + 去重统一改造计划（v1）

> 对应 `TODO.md` 最后一项「优化更新机制」。延续 `remote-repo-skill-sync-plan.md` 的工作，重新梳理「检查更新」与各种「去重 / 同步 / 删除」分支的关系。

## 1. 背景与问题

当前「检查更新」+「去重」相关链路存在四类问题：

| 类别 | 说明 |
|------|------|
| 概念混淆 | "检查更新"一个按钮承担 detect content drift / detect remote add / detect remote delete 三件事；"去重"承担 4 种语义（plugin readonly 重复 / remote_missing 删除 / import 冲突 / 多平台不一致） |
| 入口分散 | 检查按钮根据 filter 隐性切换 skill / repository-sync mode；平台去重仅在 `/platform/:agentId` 出现；删除决策散布在 4 个 dialog |
| 状态割裂 | `remote_added` 不持久化、关 dialog 即丢；`skill_update_states.status` 是裸字符串常量；`keep_remote_missing` 与 `apply_central_repository_sync._impl` 双轨复用 |
| UX 问题 | 一次"检查更新"可能连弹两个 dialog（UpdateConfirm → Missing/RepoSync）；卡片 badge 只对 update_available 友好；信息密度过高 |

详细现状摸底见 `findings.md`，业界调研对比见同文件「业界更新机制参考」一节。

## 2. 设计目标

```text
┌──────┬──────────────────────────────────────────────────────────────────┐
│ 目标 │ 描述                                                             │
├──────┼──────────────────────────────────────────────────────────────────┤
│ G1   │ 「检查」纯只读，不写盘；「应用」单一动词，按用户决策落库         │
│ G2   │ 五类变更分桶统一展示：更新 / 新增 / 删除 / 平台冗余 / 失链孤儿   │
│ G3   │ 单一「更新中心」入口；旧的三个 dialog 折叠为一个 Tabbed dialog   │
│ G4   │ 检查按钮的 scope/mode 显式可见，不再自动猜测                     │
│ G5   │ 状态可持久化、可清空、有 last_synced_at                          │
│ G6   │ 渐进迁移：旧 command 保留兼容期，UI 先切新接口                   │
└──────┴──────────────────────────────────────────────────────────────────┘
```

## 3. 整体抽象

学 Homebrew 的四动词，但范围限制在第一版只落地 `refresh` 与 `apply`，`repair` 与 `cleanup` 留位：

```text
┌────────┬────────────────────────────────────────┬──────────────────────┐
│ 动词   │ 职责                                   │ 状态                 │
├────────┼────────────────────────────────────────┼──────────────────────┤
│ refresh│ 拉取远端 snapshot，对比本地，写入临时  │ 本次实现             │
│        │ 检查报告（持久化但可清空）             │                      │
│ apply  │ 按用户勾选执行 update / import /       │ 本次实现             │
│        │ keep / delete / skip / unskip          │                      │
│ repair │ 修复跨平台不一致 / 孤儿 symlink        │ 留位，后续           │
│ cleanup│ 平台 plugin readonly vs writable 去重 │ 整合到「更新中心」   │
└────────┴────────────────────────────────────────┴──────────────────────┘
```

五类变更分桶：

```text
┌──────────────┬────────────────────────────────────────────────────────┐
│ 桶           │ 触发条件                                               │
├──────────────┼────────────────────────────────────────────────────────┤
│ 可更新       │ 远端 commit 推进、SKILL.md 内容变化                    │
│ 远端新增     │ 远端 repo 有 candidate、本地 repository_member 无       │
│ 远端删除     │ 本地 skill 仍有 source_path，远端 candidate 已消失      │
│ 平台冗余     │ 同平台同 id：plugin readonly 副本 + 可写副本并存        │
│ 失链孤儿     │ 平台 symlink target 不存在 / 中央 canonical_path 缺失  │
└──────────────┴────────────────────────────────────────────────────────┘
```

## 4. 后端改造

### 4.1 数据模型

#### 4.1.1 `SkillUpdateStatus` 升级为 enum

把 `central_updates.rs` 顶部 6 个 `const STATUS_*` 改成 `enum SkillUpdateStatus`，序列化为小写下划线字符串保持向后兼容：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillUpdateStatus {
    UpToDate,
    UpdateAvailable,
    Unsupported,
    RemoteMissing,
    Error,
    Cancelled,
}
```

DB schema 保持 TEXT，不要破坏迁移。读取时 `FromStr`，写入时 `to_string`。所有比较位置统一使用 enum，砍掉硬编码字符串散落。

#### 4.1.2 新增 `skill_repository_pending_additions` 表

持久化 transient 的 `remote_added`，让用户关闭"更新中心"也不丢检查结果：

```sql
CREATE TABLE skill_repository_pending_additions (
    repository_id TEXT NOT NULL,
    source_path TEXT NOT NULL,
    skill_id TEXT NOT NULL,
    skill_name TEXT NOT NULL,
    conflict_existing_skill_id TEXT,
    discovered_at TEXT NOT NULL,
    PRIMARY KEY (repository_id, source_path)
);
```

迁移规则：
- `refresh` 写入；用户在"更新中心"勾选 import / skip / unskip 之后由 `apply` 删除对应行
- 不与 `skill_repository_sync_skips` 合并：前者是"待处理"，后者是"已永久跳过"

#### 4.1.3 `skill_repositories` 增加 `last_synced_at`

记录 repo 级 sync 时间，UI 可显示「上次刷新 5 分钟前」：

```sql
ALTER TABLE skill_repositories ADD COLUMN last_synced_at TEXT;
```

### 4.2 新增命令（与旧命令并存）

不破坏现有 `check_central_skill_updates` / `check_central_repository_sync` / `apply_central_repository_sync` / `keep_remote_missing_central_skills` / `update_central_skills`，新增 4 个命令作为新前端入口：

```rust
#[tauri::command]
pub async fn refresh_skill_update_inventory(
    scope: SkillRefreshScope,
) -> Result<SkillUpdateInventory, String>;

#[tauri::command]
pub async fn apply_skill_update_decisions(
    decisions: SkillUpdateDecisions,
) -> Result<SkillUpdateApplyResult, String>;

#[tauri::command]
pub async fn clear_skill_update_inventory(
    scope: Option<SkillRefreshScope>,
) -> Result<(), String>;

#[tauri::command]
pub async fn get_skill_update_inventory() -> Result<SkillUpdateInventory, String>;
```

类型草稿：

```rust
#[serde(rename_all = "camelCase")]
pub struct SkillRefreshScope {
    pub kind: SkillRefreshScopeKind, // "all" | "skills" | "repositories"
    pub skill_ids: Option<Vec<String>>,
    pub repository_ids: Option<Vec<String>>,
}

#[serde(rename_all = "camelCase")]
pub struct SkillUpdateInventory {
    pub updatable: Vec<UpdatableSkill>,
    pub remote_added: Vec<RemoteAddedSkill>,
    pub remote_missing: Vec<RemoteMissingSkill>,
    pub platform_duplicates: Vec<PlatformDuplicateGroup>,
    pub orphans: Vec<OrphanSkillEntry>,
    pub failed_repositories: Vec<FailedRepository>,
    pub generated_at: String,
}

#[serde(rename_all = "camelCase")]
pub struct SkillUpdateDecisions {
    pub updates: Vec<String>,                          // skill_ids to update
    pub keep_missing: Vec<String>,                     // detach but keep local
    pub delete_missing: Vec<BatchDeleteCentralSkillRequest>,
    pub import_additions: Vec<CentralRepositoryAddedSkillSelection>,
    pub skip_additions: Vec<CentralRepositoryAdditionSkipRequest>,
    pub unskip_additions: Vec<CentralRepositoryAdditionUnskipRequest>,
    pub remove_platform_duplicates: Vec<PlatformDuplicateRemoval>,
}
```

实现要点：
- `refresh` 内部统一调用既有 `prepare_skill_updates()` + `collect_remote_added_skills()` + `collect_platform_duplicates_for_central()`，把原本两条 99% 重复的检查路径合并成一个 helper。
- `refresh` **不写** `skill_update_states`；写到新表 `skill_update_inventory_*` 或缓存到 AppState（短期方案：复用 `skill_update_states` 但加一个 `inventory_dirty` 标志，等用户关闭 dialog 后由 `clear_skill_update_inventory` 清掉 dirty 行）。**推荐做新表**。
- `apply` 拆成五个内部步骤，复用现有 `keep_remote_missing_central_skills_impl`、`delete_central_skills_impl`、`update_central_skills_impl`、`import_github_repo_skills_with_auth`，加一个 `remove_platform_duplicates_impl`（前端 `findDuplicatePlatformSkillGroups` 等价 Rust 实现）。

### 4.3 平台冗余的后端化

当前平台去重完全在前端 `findDuplicatePlatformSkillGroups()` 里算。要进入「更新中心」需要后端能扫，给个新命令：

```rust
#[tauri::command]
pub async fn scan_platform_duplicate_skills(
    agent_ids: Option<Vec<String>>,
) -> Result<Vec<PlatformDuplicateGroup>, String>;
```

实现复用 `scan_skills_for_agent` 然后做同 ID 分组（plugin readonly + writable 并存）。

### 4.4 事件

保留 `central://skill-update-progress`，phase 增加：

| phase | 说明 |
|-------|------|
| `refreshing` | 替换原 `checking` 在新 command 内的语义 |
| `applying` | 替换原 `updating` 在新 command 内的语义 |
| `checking` / `updating` | 旧 command 保留以维持兼容 |

不新增 event namespace，减少前端订阅复杂度。

### 4.5 渐进迁移

```text
┌────────┬─────────────────────────────────────────────────────────────┐
│ 阶段   │ 动作                                                        │
├────────┼─────────────────────────────────────────────────────────────┤
│ B1     │ status enum 化（独立 PR，零功能变更，全测试通过）           │
│ B2     │ 新表 `skill_repository_pending_additions`、新字段           │
│        │ `last_synced_at`、新命令的 happy path（无前端切换）         │
│ B3     │ `refresh` + `apply` 完整测试覆盖；旧 command 标记 deprecated│
│ B4     │ 前端切换到新命令（见第 5 节）                               │
│ B5     │ 一个 release 后删除旧 command（与 changelog 配套）          │
└────────┴─────────────────────────────────────────────────────────────┘
```

## 5. UI 改造

### 5.1 「更新中心」单 Dialog（核心改动）

新建 `src/components/central/UpdateCenterDialog.tsx`，取代以下三个 dialog 的混合调用：

- `CentralUpdateConfirmDialog`
- `RemoteMissingSkillsDialog`
- `CentralRepositorySyncDialog`

结构：

```text
┌─────────────────────────────────────────────────────────────────┐
│  更新中心                                          [刷新][关闭] │
├─────────────────────────────────────────────────────────────────┤
│  Tab: [可更新 N] [新增 N] [删除 N] [平台冗余 N] [失链 N]        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   [active tab 内容：per-item 三态决策行]                        │
│                                                                 │
│   每行：技能名 / 来源 / 当前状态 / 建议动作 / 用户选择          │
│   行内三态按钮：保留 / 应用 / 跳过                              │
│   全选按钮 + 计数 chip                                          │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ Footer: [清空检查结果] [取消] [应用 (N 项)]                     │
└─────────────────────────────────────────────────────────────────┘
```

每个 Tab 内的决策颗粒度沿用现有 dialog 的设计，但把动作语义统一到三态：

```text
┌──────────────┬──────────────────┬──────────────────┬─────────────┐
│ Tab          │ 应用              │ 保留              │ 跳过         │
├──────────────┼──────────────────┼──────────────────┼─────────────┤
│ 可更新       │ 拉取并覆盖       │ 不动              │ —           │
│ 远端新增     │ 导入到中央        │ —                 │ 永久跳过    │
│ 远端删除     │ 删除中央副本     │ Detach 源（默认） │ —           │
│ 平台冗余     │ 删除可写副本     │ 不动              │ —           │
│ 失链孤儿     │ 删除失链        │ 不动              │ —           │
└──────────────┴──────────────────┴──────────────────┴─────────────┘
```

实现要点：
- 一个 controlled component，状态来源 `useUpdateCenterStore` 或新 store slice。
- 用 `Tabs` 组件，未触发 fetch 的 Tab 标题灰色，已发现条目的 Tab 高亮。
- 每行用现有 `BatchDeleteCentralSkillRequest` / `CentralRepositoryAddedSkillSelection` 的子结构，避免重新造类型。
- Footer 的「应用」按钮聚合所有 Tab 的决策，调用 `apply_skill_update_decisions`。
- 「清空检查结果」对应 `clear_skill_update_inventory`，下次重新触发 refresh 才有数据。

### 5.2 入口收敛

```text
┌─────────────────────┬──────────────────────────────────────────────────┐
│ 入口                │ 新行为                                           │
├─────────────────────┼──────────────────────────────────────────────────┤
│ CentralSkillsShell  │ 「刷新」按钮 + scope dropdown：当前结果 / 当前   │
│ 顶部工具栏          │ 仓库 / 全部。点击触发 refresh，结果存入 store，   │
│                     │ 自动打开 UpdateCenterDialog                     │
│ UnifiedSkillCard    │ 卡片上保留 "更新此技能" 单条快捷动作，等价      │
│                     │ refresh(skill) → apply(updates=[id])，免开 dialog│
│ SkillDetailDrawer   │ 同上单条更新                                    │
│ PlatformView        │ "扫描重复" 按钮保留（独立场景），但触发后跳到   │
│                     │ UpdateCenterDialog 的 "平台冗余" Tab            │
│ AppShell / 右上角   │ Badge：未处理 inventory 总数（来自               │
│                     │ get_skill_update_inventory，启动时拉一次）       │
└─────────────────────┴──────────────────────────────────────────────────┘
```

旧的 3 个 dialog 组件保留在仓库内 6 个月作为 fallback，标记 `@deprecated`，CSS class 加 `deprecated-` 前缀。

### 5.3 卡片 badge 体系

`UnifiedSkillCard` 扩展 status chip：

```text
┌────────────┬───────────────┬──────────────────────────────────────────┐
│ 状态        │ Chip 颜色      │ 含义                                     │
├────────────┼───────────────┼──────────────────────────────────────────┤
│ Update     │ 蓝色 (primary)│ 远端有内容更新                            │
│ Added      │ 紫色          │ 这个 skill 是本次 refresh 的远端新增      │
│ Missing    │ 红色          │ 源已删除                                  │
│ Duplicate  │ 橙色          │ 当前平台有同 ID 冗余                      │
│ Orphan     │ 灰色          │ symlink 失链                              │
└────────────┴───────────────┴──────────────────────────────────────────┘
```

点击 chip 打开 UpdateCenterDialog 并切到对应 Tab，预选当前 skill。

### 5.4 Store / 类型变更

新增 `src/stores/updateCenterStore.ts`（或并入 `centralSkillsStore.updateSlice.ts`，按文件 size budget 选择）：

```ts
interface UpdateCenterState {
  inventory: SkillUpdateInventory | null;
  isRefreshing: boolean;
  isApplying: boolean;
  lastRefreshedAt: string | null;
  isDialogOpen: boolean;
  activeTab: UpdateCenterTab;
  refresh(scope: SkillRefreshScope): Promise<void>;
  apply(decisions: SkillUpdateDecisions): Promise<SkillUpdateApplyResult>;
  clear(scope?: SkillRefreshScope): Promise<void>;
  openDialog(tab?: UpdateCenterTab): void;
  closeDialog(): void;
}
```

类型放到 `src/types/skillUpdateInventory.ts`，避免 `src/types/index.ts` 继续膨胀（旧 plan 也提醒过 size budget）。

### 5.5 i18n

新增命名空间 `central.updateCenter.*`，旧 key 保留兼容期：

```text
central.updateCenter.title                "更新中心"
central.updateCenter.refreshButton        "刷新"
central.updateCenter.scopeAll             "全部"
central.updateCenter.scopeRepository      "当前仓库"
central.updateCenter.scopeCurrent         "当前结果"
central.updateCenter.tabs.updates         "可更新 ({count})"
central.updateCenter.tabs.added           "新增 ({count})"
central.updateCenter.tabs.missing         "已删除 ({count})"
central.updateCenter.tabs.duplicates      "平台冗余 ({count})"
central.updateCenter.tabs.orphans         "失链 ({count})"
central.updateCenter.actions.apply        "应用"
central.updateCenter.actions.keep         "保留"
central.updateCenter.actions.skip         "跳过"
central.updateCenter.applyAll             "应用全部 ({count})"
central.updateCenter.clearInventory       "清空检查结果"
central.updateCenter.emptyAllClean        "所有内容都是最新的"
central.updateCenter.lastRefreshedAt      "上次刷新: {time}"
```

中英两套都加。旧的 `central.updateCheck*` / `central.remoteMissing*` / `central.repositorySync*` key 在新 UI 不再被使用，半年后批量删除。

## 6. 渐进实施顺序

```text
┌────┬──────────────────────────────────────────────────────────────────┐
│ 阶 │ 内容                                                              │
├────┼──────────────────────────────────────────────────────────────────┤
│ P1 │ 后端 status enum 化（仅重构，零行为变更，全测试通过）             │
│ P2 │ 后端新表 + 新字段 + 4 个新命令 happy path 实现                    │
│ P3 │ 后端新命令完整覆盖（含失败、取消、冲突、SSH target）              │
│ P4 │ 前端：新 store + 新类型 + UpdateCenterDialog 骨架（无 i18n 完善） │
│ P5 │ 前端：refresh / apply 接通；卡片 badge 体系；scope dropdown       │
│ P6 │ 前端：把 `UnifiedSkillCard` 单条更新切到新接口；旧 dialog 标 dep   │
│ P7 │ 前端：PlatformView "扫描重复" 跳转到 UpdateCenter 对应 Tab        │
│ P8 │ i18n 完善 + 文档（`docs/guide/update-center.md`）+ release notes  │
│ P9 │ 老 dialog 与老命令打 `@deprecated`，规划下个 release 删除         │
└────┴──────────────────────────────────────────────────────────────────┘
```

不要试图一次性 PR 完成，按 P1-P9 拆 PR。P1-P3 可以与 P4-P7 并行。

## 7. 测试与验证

### 7.1 Rust

新增 `src-tauri/src/commands/skill_update_inventory/tests.rs`：

- refresh 在无变更时返回全空 inventory，`generated_at` 更新
- refresh 在 remote_added + remote_missing + update_available 共存时返回全部桶
- apply 顺序：先 keep_missing → delete_missing → import_additions → updates → remove_duplicates
- apply 单步失败不回滚其他步骤（partial success 语义保持现状）
- clear 只清掉 inventory，不动 skills / skill_update_states
- SSH target 路径走 `*_remote_impl`，回归不破坏
- 取消信号在 refresh 中段生效

### 7.2 前端

- `src/test/updateCenterStore.test.ts`：refresh / apply / clear 三态
- `src/test/UpdateCenterDialog.test.tsx`：
  - 五个 Tab 切换、计数 chip 与 inventory 长度一致
  - 全选 → 应用 → 调用 apply 时 payload 正确
  - 单 Tab 内 per-item 三态切换
  - 空状态文案
- `src/test/CentralSkillsView.update-center.test.tsx`：替换原 `CentralSkillsView.updates-and-search.test.tsx` 的更新流程断言
- `src/test/PlatformView.duplicate-redirect.test.tsx`：扫描重复 → 跳转到 UpdateCenter Duplicate Tab

### 7.3 端到端

`pnpm typecheck && pnpm lint && pnpm sizecheck && pnpm test && cargo test --manifest-path src-tauri/Cargo.toml && cargo clippy -- -D warnings && just ci`

### 7.4 验收用例

```text
┌─────┬──────────────────────────────────────────────────────────────┐
│ U1  │ 用户点刷新（默认 scope=全部）→ 单次弹出 UpdateCenter，五桶  │
│     │ 计数正确，关闭再打开数据仍在                                 │
│ U2  │ 单 repo 过滤下点刷新 → scope 自动选当前仓库，仅扫该 repo    │
│ U3  │ 卡片 update badge 点击 → 打开 UpdateCenter 直接定位该行     │
│ U4  │ Apply 5 类决策混合操作一次成功，刷新数据                    │
│ U5  │ 平台冗余 Tab 中删除可写副本，PlatformView 列表立即同步       │
│ U6  │ 远端 repo 同时 add + delete + content change 三类，apply 顺  │
│     │ 序正确，无 ID 冲突                                            │
│ U7  │ 取消 refresh 不留半截 inventory，scope=skills 时不动其他桶   │
└─────┴──────────────────────────────────────────────────────────────┘
```

## 8. 风险与开放点

- **inventory 表 vs 内存缓存**：新表持久化简单但增加迁移成本；内存缓存重启即丢但更快。第一版选**新表**（理由：用户期望"关 dialog 不丢"）。
- **平台冗余跨页面**：后端 `scan_platform_duplicate_skills` 全平台扫一次可能慢；可分批 lazy load 每个 Tab。
- **size budget**：`src/types/index.ts` 与 `CentralRepositorySyncDialog.tsx` 已经接近上限。新增的 `UpdateCenterDialog.tsx` 必须模块化拆分（每 Tab 一个子组件）。
- **测试迁移成本**：现有 `RemoteMissingSkillsDialog` / `CentralRepositorySyncDialog` 的测试要么改成测旧 dialog（半年后删）要么直接切到新 dialog。建议**保留旧测试到下个 release**，新测试并行写。
- **去重语义教育**：用户可能问"平台冗余和远端删除有什么区别"——文档需要给出明确例子。
- **`auto-check` / `auto-apply` 双开关**：第一版不做，按 VS Code 经验是后续易加的增量。

## 9. 完成条件

- [ ] 后端 `refresh_skill_update_inventory` / `apply_skill_update_decisions` / `clear_skill_update_inventory` / `scan_platform_duplicate_skills` 全部实现并测试通过
- [ ] 数据库迁移：status enum、`skill_repository_pending_additions` 表、`last_synced_at` 字段
- [ ] 前端 UpdateCenterDialog 上线，覆盖五类变更分桶
- [ ] CentralSkillsShell 顶栏 scope dropdown + 刷新按钮
- [ ] UnifiedSkillCard badge 体系（5 种）
- [ ] PlatformView 扫描重复跳转到 UpdateCenter
- [ ] 旧 3 个 dialog + 5 个旧 command 全部标 `@deprecated`
- [ ] 文档 `docs/guide/update-center.md` 与 release notes
- [ ] TODO.md 「优化更新机制」项勾选

## 10. 关联文档

- `findings.md` — 现状摸底与业界调研详尽数据
- `plans/remote-repo-skill-sync-plan.md` — 上一次 repo-sync 实现，是本次合并 + 重构的起点
- `task_plan.md` — 本次工作的阶段与决策记录
