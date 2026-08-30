# Skills CLI 页面骨架与紧凑卡片网格 — 技术设计

## 1. 依赖、设计权威与所有权

- 硬前置：`08-26-backend-contract` 完成并合入 `dev`，提供 placement-aware inventory DTO、
  install targets、独立 runtime/inventory 错误及稳定 i18n error code。
- 规范性设计：`../08-26-skills-cli-redesign/research/design-contract.md`。
- `../08-26-skills-cli-redesign/research/skills-cli-redesign.dc.html` 仅作静态灵感；缺失 README、
  `support.js`、HTML no-op/CDN/模板事件不是实现输入或证据。
- 本任务拥有页面壳、纯视图模型、surface/content-width controller、Header/Toolbar/GroupHeader、
  install mount adapter、共享 action toast、skillsCli dense 卡片布局与 Dashboard census 挂载。
  后续子任务通过本文边界接线，不重建这些所有权。

## 2. 页面状态、surface 与错误矩阵

`SkillsCliView.tsx` 不直接调用 Tauri `invoke()`。它读取 `useSkillsCliStore`，并持有：

```ts
type SkillsCliGroupBy = "repo" | "platform" | "status" | "none";
type SkillsCliLayoutBand = "twoColumns" | "threeColumns" | "fourColumns";
type SkillsCliDrawerBand = "fullWidth" | "fixed460";

type SkillsCliActiveSurface =
  | null
  | { kind: "install" }
  | { kind: "detail"; skillName: string; focus: null | "links" }
  | { kind: "update" }
  | { kind: "uninstall"; skillNames: readonly string[] };

interface SkillsCliPageState {
  query: string;
  groupBy: SkillsCliGroupBy;
  platformFilter: string | null;
  unlinkedOnly: boolean;
  selectMode: boolean;
  collapsedGroupIds: ReadonlySet<string>;
  activeSurface: SkillsCliActiveSurface;
  contentWidthPx: number | null;
}
```

页面拥有 `openInstall/openDetail/openUpdate/openUninstall/closeSurface`。普通详情固定
`focus=null`，Manage Links 才传 `focus="links"`；`closeSurface` 清空整个对象，防止 focus 泄漏到下次。
后续 Dialog/Drawer 作为受控 surface 消费该状态。Base UI 保持 topmost Escape owner；页面根只在
`activeSurface === null && !event.defaultPrevented` 时于冒泡阶段清除 selection，不注册 window listener。

内容根用 `ResizeObserver` 测量自身宽度，纯函数派生：`<720=fullWidth`，否则 fixed460；
`<900=twoColumns`、`900–1179=threeColumns`、`>=1180=fourColumns`。JS band 供后续 surface 与测试消费；
CSS container query 才是网格渲染权威，二者用同一常量和边界测试防漂移。

### 2.1 安装表面的稳定 mount seam

`SkillsCliView` 只依赖 `SkillsCliInstallMount`，不直接 import 安装业务组件：

```ts
export interface SkillsCliInstallMountProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  returnFocusRef: RefObject<HTMLElement | null>;
  contentWidthPx: number | null;
}

export const SKILLS_CLI_INSTALL_SURFACE_AVAILABLE: boolean;
export function SkillsCliInstallMount(props: SkillsCliInstallMountProps): ReactNode;
```

page-shell 初始 adapter 导出 `available=false` 并返回 null，Header 的 Install disabled 条件包含
`!available`，因此没有可点击 no-op。`install-wizard` 在 page-shell 完成后创建独立
`SkillsCliInstallSurface` / store / view model，只修改 adapter 内部接线并把 available 设为 true；
它不编辑 `SkillsCliView`、Header、surface union 或其他 overlay adapter。adapter 把 open/change、
return-focus 和内容宽度原样透传，关闭仍由 page-owned `closeSurface` 复位。

错误/加载呈现：

| runtime | inventory | 页面行为 |
| --- | --- | --- |
| loading | 无数据 | 页头占位、12 张 aria-hidden skeleton、主区域 `aria-busy=true` |
| failed | success/stale | runtime error pill，Install disabled；网格/过滤仍可用 |
| success | failed + stale | 保留网格，显示本地化 inventory error 与 Retry |
| success/failed | failed + 无数据 | inventory error surface；不得显示 clean empty |
| refreshing | 已有数据 | 保留内容，刷新旋转且 Refresh disabled |
| success | empty | inventory empty；过滤空使用包含 query 的另一文案 |

`runtimeError`、`inventoryError`、`actionError` 保持三条 store 语义；本任务只呈现前两条。backend
rejection 经 `formatBackendError(error,t)`，不渲染原始路径/details。

## 3. 纯视图模型

`src/pages/skillsCliViewModel.ts` 提供：

```ts
interface SkillsCliCounts {
  installed: number;
  linked: number;
  unlinked: number;
  repositories: number;
}

interface SkillsCliBucket {
  id: string;
  labelKey: string;
  labelValue?: string;
  skillCount: number;
  managedLinkCount: number;
  skills: SkillsCliGlobalSkill[];
}

deriveSkillsCliCounts(skills, enabledTargetIds): SkillsCliCounts;
filterSkillsCli(skills, filters, enabledTargetIds): SkillsCliGlobalSkill[];
bucketSkillsCli(skills, groupBy, targets): SkillsCliBucket[];
deriveSkillsCliLayoutBands(contentWidthPx): {
  grid: SkillsCliLayoutBand;
  drawer: SkillsCliDrawerBand;
};
```

- 计数使用未过滤 inventory。linked 看 `managed_link`；unlinked 看 enabled target 的 `missing`；
  direct_copy 不计 linked，linked/unlinked 可同时命中，不能用差值。
- 搜索 name/source/canonical path，trim 后做 locale-independent case fold；平台 chip 只命中目标平台的
  managed_link/direct_copy；Unlinked only 只命中 enabled target 的 missing。
- Repository 桶首次顺序、unknown 末置；Platform 按 target 顺序，允许多桶并追加 unlinked；
  Status 至少 linked/unlinked/copy-or-conflict；None 为固定 all。id 不含翻译文本。

## 4. 呈现组件

### `SkillsCliHeader.tsx`

接收 counts、doctor/runtimeError、isRefreshing、onRefresh、onOpenInstall。Refresh/Install 使用原生 button；
runtime status 有 accessible name。刷新中防重复，错误不清空已有计数。

### `SkillsCliToolbar.tsx`

接收 filters/group/select props 与 `onExportAll?: () => void`、`isExporting?: boolean`。Toolbar 只呈现
Export all，不接 store、不选文件、不序列化；undefined/导出中时 disabled，提供后只调用一次且不传
filtered/selected 数据。`batch-actions` 接全量 snapshot controller，`backend-contract` 拥有文件 IPC；
Export selected 是批量栏的另一入口。

Group 和 chip 用 `aria-pressed`；搜索 clear 有 accessible name。所有视觉小于 40px 的图标控件以
pseudo-element 扩展热区并保留 focus-visible ring。

### `SkillsCliGroupHeader.tsx`

折叠 button 带 `aria-expanded`/`aria-controls`。Select all/Update all 是可选 callback，未接线时 disabled
或不渲染，不保留 no-op。sticky 背景使用主题 token。

### `skillsCliActionToast.tsx`

page-shell 提供 Skills CLI 所有子表面共用的轻量 helper：

```ts
export const SKILLS_CLI_ACTION_TOAST_ID = "skills-cli-action";
export const SKILLS_CLI_ACTION_TOAST_DURATION_MS = 2_800;
export type SkillsCliToastSemantic =
  | "success"
  | "error"
  | "destructiveSuccess"
  | "destructiveError";

showSkillsCliActionToast({ semantic, message }: {
  semantic: SkillsCliToastSemantic;
  message: string;
}): void;
```

helper 固定 sonner id/duration 和经评审的 lucide icon + status token；message 必须由调用 surface 先经 i18n
和需要时的 `formatBackendError` 生成。helper 不接受 Error/backend envelope，避免绕过错误脱敏；后续调用以
同 id 替换旧 toast。batch/install/detail/update 只消费该 API，不复制 duration/id/icon 映射。

### `skillsCliStore.addGlobal` mutation-only seam

page-shell 同步拥有现有 store action 的契约修复，不增加 `addGlobalMutation` 等第二入口：

```ts
addGlobal(input: SkillsCliAddInput): Promise<SkillsCliAddResult>;
```

action 只处理 selection/busy guard、job id、`skills_cli_add_global` invoke、当前 job 的 mutation 状态和
`actionError`。selection 写稳定 actionError 后 reject；busy reject 但不覆盖正在运行的状态；成功返回 result
并清理 preview；当前 job 的 backend failure 写 reviewed state value 后 rethrow；stale completion 不覆盖
新 target/job state。action 内禁止 `loadAll()`。

调用 surface 采用两段式控制流：

```ts
const result = await addGlobal(input); // 主 mutation；失败才显示 add error
showMutationSuccess(result);

await loadAll(); // 独立 follow-up；loadAll 保持 runtime/inventory independent settle
const refreshError = useSkillsCliStore.getState().inventoryError;
if (refreshError) {
  showRefreshWarning(formatBackendError(refreshError, t));
}
```

第二段 inventory 失败不改 `actionError`、不进入第一段 catch、不重放 add，也不撤回 result/成功语义。
doctor-only failure 由 runtime pill 表达，不触发 inventory refresh warning。当前页面安装入口先迁到此契约；
`SkillsCliInstallSurface` 后续通过 `SkillsCliInstallMount` 消费相同 action，不自建 wrapper action。

## 5. `UnifiedSkillCard` dense-row

不复用 168px `density="compact"`，不创建第二个卡片。在 `SkillsCliSkillCardProps` 增加专属字段：

```ts
interface SkillsCliSkillCardProps extends SkillCardCoreProps {
  variant: "skillsCli";
  layout: "denseRow";
  path?: string | null;
  placements: readonly SkillsCliPlacement[];
  checkbox?: SkillCardCheckbox;
  updateAvailable?: boolean;
  onDetail: MouseEventHandler<HTMLButtonElement>;
  onManageLinks?: () => void;
  onUninstall: () => void;
  isLoading?: boolean;
}
```

按 `skill-card-scenarios.md` 只改 skillsCli interface → `toModel` 单点映射 → 私有 model 渲染，
同步每场景最小正例和 `@ts-expect-error` 负例。

dense-row 用 `min-h-[76px] h-auto`，名称/状态、canonical path、placement 三行单行；font_scale=1.125
允许自然增高，禁止固定高度裁切。主详情是可聚焦 button；checkbox/Manage links/Uninstall 阻止冒泡。
最多显示 4 个 managed-link `PlatformIcon` 和 `+n`；copy/conflict/missing 用本地化状态表达。
动作同时支持 group-hover 与 group-focus-within；focus ring 放在不会被 overflow 裁切的位置。

## 6. 响应式契约

内容根声明命名 container，例如 `@container/skills-cli`；网格使用：

```text
grid grid-cols-2
@min-[900px]/skills-cli:grid-cols-3
@min-[1180px]/skills-cli:grid-cols-4
```

它们分别代表 `<900`、`900–1179`、`>=1180` 的**内容宽度**。不得使用 viewport `min-[...]`，
不得提供“container 不可用则降级”。Toolbar `flex-wrap`，页面不引入水平滚动。生产构建后检查 CSS
同时存在 900/1180px `@container`；jsdom class 断言不是完整证据。

## 7. `InventoryCensus` 迁移

- `InventoryCensus.tsx` 内部及路径不改。
- 从 Skills CLI 移除；Local `DashboardView` 挂独立 Skills CLI census 区块。
- Dashboard 只订阅 skillsCliStore 的 skills/targets/loadAll，非 Local 不触发 loader；不得订阅 central
  skills 重算 census，也不得覆盖 `dashboardCentralSummary`。
- 测试覆盖 Local、非 Local、loader failure 不污染中央 summary。

## 8. 测试与证据

- 纯函数：placement 混合计数、过滤/叠加、四分组、稳定 id、719/720/899/900/1179/1180 bands。
- surface：普通 detail focus、links focus、close reset、uninstall payload、Base UI/prevented Escape 守卫。
- install seam：unavailable disabled、available open/close、return-focus/contentWidth 透传，以及安装子任务不改页壳的文件契约。
- toast helper：稳定 id、2800ms、replacement、四 semantic icon/tone 与 string-only message API。
- mutation seam：add-only invoke、成功 result、failure rethrow、无隐式 refresh、stale job 不覆写。
- follow-up：mutation success + inventory refresh failure 保留成功、单独 warning、无 add retry/error。
- 组件：Header 成功/失败/refreshing；Toolbar pressed/clear/export disabled；GroupHeader；dense-row 高度、
  图标上限、keyboard、propagation、focus-within。
- 页面：独立错误轨道、stale+error、loading/empty/filter-empty、折叠、Dashboard 迁移。
- 响应式：命名 container 类 + Vite/Tailwind 生产 CSS。
- i18n/token：en/zh parity，无原型 hex/CDN/远程字体。

最后运行 focused Vitest、typecheck、lint、build、task validate、`just ci`。jsdom/源码不能证明真实布局；
Windows installer/WebView2 的断点、中文排版、hover/focus、键盘和视觉保真在实际运行前为 `UNVERIFIED`。

## 9. 回滚与并发

本任务先落稳定页面/surface/content-width/install-mount/toast/add-mutation contracts。batch/install/detail/update 子任务
依赖这些边界，不得并行重写页面壳、复制 toast helper 或另建卡片。回滚必须同时撤销 Dashboard census、
surface/mount/toast contract 与 skillsCli 类型扩展，并保持其他卡片场景和 central summary 不变。
