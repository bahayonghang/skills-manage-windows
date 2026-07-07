# 技术设计：平台视图安装来源快速导航

> 复杂度评级：中（多文件前端改动：视图模型 + 新组件 + 页面布局 + i18n + 测试）。纯前端增量，无 IPC / 后端改动。

## 1. 分类模型（唯一判定点，进视图模型层）

所有判定与聚合放在 `src/lib/platformSkillViewModel.ts`（纯函数、可测），组件层不写分类逻辑。

```ts
export type PlatformOriginFilter =
  | { kind: "all" }
  | { kind: "standalone" }
  | { kind: "central"; repoKey?: string };
// repoKey 缺省 = 整个 SkillPort 组；"unassigned" = 仓库未指派；`repo:<id>` = 指定仓库

/** 与 SkillCardBadges.SourceIndicator 同语义：symlink 即中央链接 */
export function getPlatformSkillOrigin(skill: ScannedSkill): "central" | "standalone";

/** central 组内的仓库桶 key：repository 存在且 !is_unknown → `repo:${repository.id}`；否则 "unassigned" */
export function getPlatformOriginRepoKey(skill: ScannedSkill): string;

export interface PlatformOriginNavModel {
  total: number;
  centralCount: number;
  standaloneCount: number;
  /** 仅统计 origin=central 的行；label 取 repository.name || `${owner}/${repo}` || id（与 getPlatformRepositoryGroupInfo 回退顺序一致）；按 label 排序 */
  repos: Array<{ key: string; label: string; count: number }>;
  /** symlink 且仓库未指派的行数；>0 时组件在 repos 末尾渲染「未指派来源」子项 */
  unassignedCentralCount: number;
}
export function derivePlatformOriginNav(
  skills: readonly ScannedSkill[]
): PlatformOriginNavModel;
```

**判定规则依据**（调研结论，详见 prd Confirmed Facts）：
- `installed_at` 是"首次扫描到"时间，扫描器对手放技能同样写 `skill_installations` 行，不可用；
- `link_type === "symlink"` 是唯一可靠信号，且与卡片 footer「中央技能库/独立安装」逐字同语义，用户可在界面自洽验证；
- 不复用 `getPlatformRepositoryGroupInfo` 做 nav 聚合（它把 plugin/local/unknown 混为并列桶，语义是"按仓库分组"视图的，与本导航的两层结构不同），但 repo label 回退顺序保持一致。

## 2. 管线接入（derivePlatformSkillRows）

现管线：`skills → tab过滤(sourceFilter) → sourceFilteredSkills → 搜索 → filteredSkills → 排序 → sortedSkills → 分组 → groups`。

改动：`DerivePlatformSkillRowsInput` 增加 `originFilter: PlatformOriginFilter`，在 tab 过滤之后、搜索之前 retain：

```
skills → tab过滤 → sourceFilteredSkills（保持=tab后、origin前，供 nav 计数与既有空态）
       → origin过滤 → originFilteredSkills（新增输出，供新空态判断）
       → 搜索 → filteredSkills → 排序 → sortedSkills → 分组 → groups
```

- `matchOriginFilter(skill, filter)`：`all` → true；`standalone` → origin=standalone；`central` 无 repoKey → origin=central；`central`+repoKey → origin=central 且 `getPlatformOriginRepoKey(skill) === repoKey`。
- nav 计数在 PlatformView 中 `useMemo(() => derivePlatformOriginNav(platformRows.sourceFilteredSkills), …)` —— 满足 prd 计数口径（随 tab 变、不随搜索变）。

## 3. UI 组件与布局

**新组件** `src/components/platform/PlatformOriginNav.tsx`：

```ts
interface PlatformOriginNavProps {
  model: PlatformOriginNavModel;
  value: PlatformOriginFilter;
  onChange: (filter: PlatformOriginFilter) => void;
  className?: string;
}
```

- 结构：`<nav aria-label={t("platform.originNav.label")}>` 内一列按钮：
  - 全部 `(total)`
  - SkillPort 安装 `(centralCount)` —— 其下缩进渲染 repo 子项（`pl-6` 级缩进，label truncate + `title`，计数右对齐 `tabular-nums`），`unassignedCentralCount > 0` 时末尾加「未指派来源」子项
  - 独立安装 `(standaloneCount)`
- 选中态样式对齐 claude 来源 tab：`bg-primary/15 text-foreground font-medium`，未选中 `text-muted-foreground hover:bg-muted/40`；选中项 `aria-current="true"`；焦点环用项目共享 focus-visible 工具类。
- 点击「SkillPort 安装」父项 = `{kind:"central"}`（不带 repoKey，选中父项时子项不联动高亮）；点击子项 = `{kind:"central", repoKey}`。
- 空模型（total=0）时导航仍渲染骨架三段但计数为 0（页面此时本就走"平台无技能"空态）。

**PlatformView 布局改动**（`src/pages/PlatformView.tsx`）：

```tsx
// 现：<div ref={contentRef} className="flex-1 overflow-auto p-6">…</div>
// 改：
<div className="flex flex-1 min-h-0">
  <PlatformOriginNav className="w-56 shrink-0 border-r border-border overflow-y-auto p-3" … />
  <div ref={contentRef} className="flex-1 overflow-auto p-6">…（原内容不动）</div>
</div>
```

- `contentRef` 语义不变，`VirtualizedGrid` 的 `scrollContainerRef` 不受影响；卡片区变窄后 `minColumnWidth=420 / maxColumns=2` 自适应为 1~2 列，无需调整。
- 导航栏始终可见（桌面应用窗口宽度足够；不做响应式折叠——简单优先，窄窗口下卡片区自动降为单列）。

## 4. 状态与空态

- `const [originFilter, setOriginFilter] = useState<PlatformOriginFilter>({ kind: "all" })`；在既有 `useEffect(() => setSourceFilter("all"), [agentId])` 同处一并重置。
- 空态分支顺序（在现有链上插入一级）：
  1. `skills.length === 0` → 平台无技能（既有）
  2. `sourceFilteredSkills.length === 0` → claude tab 空态（既有）
  3. **新增** `originFilteredSkills.length === 0` → 「该来源下暂无技能」+「清除筛选」按钮（onClick 重置 originFilter 为 all）
  4. `filteredSkills.length === 0` → 搜索无匹配（既有）

## 5. i18n key 规划（zh / en 同步新增）

| key | zh | en |
| --- | --- | --- |
| `platform.originNav.label` | 安装来源导航 | Install origin navigation |
| `platform.originNav.all` | 全部 | All |
| `platform.originNav.central` | SkillPort 安装 | Installed via SkillPort |
| `platform.originNav.standalone` | 独立安装 | Standalone |
| `platform.originNav.unassigned` | 未指派来源 | Unassigned source |
| `platform.originNav.emptyFiltered` | 该来源下暂无技能 | No skills for this origin |
| `platform.originNav.clearFilter` | 清除筛选 | Clear filter |

说明：「独立安装」复用 `platform.sourceStandalone` 的既有措辞（不复用其 key，避免语境耦合）；「SkillPort 安装」按用户心智命名，鼠标悬停 `title` 可标注"符号链接到中央技能库"以衔接卡片徽标词汇（`platform.sourceCentral`）。en 文案终稿以 en.json 既有语气为准。

## 6. 测试设计

- `src/test/platformSkillViewModel.test.ts` 新增：
  - `getPlatformSkillOrigin`：symlink → central；copy/native → standalone。
  - `derivePlatformOriginNav`：混合数据下 total/centralCount/standaloneCount 守恒；repo 桶按 label 排序；`is_unknown` 仓库与无 repository 都进 unassignedCentralCount；standalone 行的 repository（如 handoff 场景：copy + repo 徽标）**不**计入 repos。
  - `derivePlatformSkillRows`：originFilter 各分支过滤正确；与 searchQuery、claude sourceFilter 组合时管线顺序正确（sourceFilteredSkills 不含 origin 过滤、filteredSkills 含）。
- `src/test/PlatformView.test.tsx` 新增（沿用既有 mock 手法）：
  - 导航渲染三段与计数；
  - 点击 repo 子项后卡片列表只剩该 repo 的 symlink 技能；
  - 切换 agentId 路由后选中态重置为「全部」；
  - 过滤出零结果时展示空态与「清除筛选」，点击后恢复。

## 7. 兼容性 / 回滚

- 纯前端增量：不改 IPC 契约、不改 store、不改后端；`derivePlatformSkillRows` 新入参对既有调用点是必填新增（仅 PlatformView 一处调用 + 测试），编译期兜底。
- 回滚 = revert 该前端 commit，无数据/迁移残留。

## 8. 已裁决的取舍（防返工）

| 取舍点 | 决定 | 理由 |
| --- | --- | --- |
| SkillPort copy 安装归类 | 归「独立安装」 | 落盘后与手放不可区分（扫描器覆盖写 si 行）；与卡片徽标一致，界面自洽 |
| 用 `installed_at` 判定 | 否 | 它是"首次扫描到"时间，手放技能也有 |
| 复用 `getPlatformRepositoryGroupInfo` 聚合 | 否（仅对齐 label 回退顺序） | 它的 plugin/local/unknown 并列桶语义服务于"按仓库分组"，与两层导航结构不同 |
| 响应式折叠导航 | 不做 | 桌面应用；卡片区已能降为单列；简单优先 |
| 后端加 origin 标记 | 超出本任务 | 需要动安装链路与 schema，价值待验证，留扩展点 |
