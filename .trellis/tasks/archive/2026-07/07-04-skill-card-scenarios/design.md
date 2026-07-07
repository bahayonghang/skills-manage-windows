# design: UnifiedSkillCard 显式场景 interface

> 依据 2026-07-05 代码勘查（调用点矩阵、死 prop 判定均为当日 grep 实测）。prd.md 定义 What/验收，本文定 How。

## 0. 结论摘要

| 决策点 | 裁决 |
| --- | --- |
| interface 形状 | **方案 A：顶层判别联合**。`UnifiedSkillCardProps = Central \| Platform \| Project \| Import \| Marketplace \| Collection`，判别字段 `variant`；调用点 `<UnifiedSkillCard variant="platform" …/>` |
| 场景数量 | **6 个**（PRD 例举 5 个 + `import`）。Obsidian 两处调用点是独立的第 6 簇（原代码注释里的 "discover variant"：isCentral/platformBadge/projectBadge/onInstallToCentral/onInstallToPlatform），塞进 `project` 会让该场景反向宽化 5 个 props，违背收窄目标。PRD 把形状裁决权交给 design，此处据实分簇 |
| 实现结构 | 现有约 40 prop 的 interface 降级为**模块私有 `SkillCardModel`**（不再导出）；新增 `toModel(props)` 按 variant 归一化；**渲染函数零改动**，视觉零回归由「渲染代码不动」结构性保证 |
| 隐式组合判定内化 | `hasActions`（10 回调 OR）、`hasCheckbox`、footer/platformIcons 切换等判定全部落在 model 之后的现有渲染逻辑里，调用方不再感知；`:256` 可点击分支见下行 |
| 死代码裁决 | `:256` 可点击分支 + `onClick` prop：生产 0 调用点、0 测试覆盖 → **删除**（连带删 zh/en 的 `platform.searchSkillLabel` i18n key）。`summaryLabel`、`isInstalled`：全仓无人传 → **删除**（`isInstalled` 恒 undefined，删除后 install 按钮渲染输出对现有调用者比特级不变） |
| skillId 归属 | 仅 `central` 场景保留 `skillId?`（inventory 徽章派生入口；生产 grid 模式经 `platformIcons.skillId` 兜底的现状保持，list 模式今天不传→无徽章，行为不变） |
| requiredness 规则 | 场景内「全部调用点恒传且值恒定义」的 prop → required；条件传/值可 undefined → optional。让类型面诚实反映场景契约 |
| 类型强制机制 | 判别联合 + TS 按判别式收窄的 excess property check。负面用例新文件 `src/test/unifiedSkillCardVariants.test.tsx`：对象字面量 + JSX 两种形态的 `@ts-expect-error`（带描述），由 `pnpm typecheck` 双向强制（未触发的 directive 会报 Unused 错误） |
| 硬指标 | 单场景可见 props 总数（含 core 与 variant）：central ≤ 24、其余场景 ≤ 17（现状：所有场景 40）。负面用例至少覆盖 4 组跨场景互斥对 |

## 1. 现状（2026-07-05 实测）

- `src/components/skill/UnifiedSkillCard.tsx` 717 行：1 必填 + 39 可选 props、11 动作回调（含 onClick）、3 嵌套配置对象。场景由 prop 组合隐式决定（`:256` `onClick && !hasActions && !hasCheckbox && !hasPlatformIcons`；`hasActions` = 10 回调 OR）。
- **生产 JSX 调用点 11 处 / 8 文件**（PRD 的"9 个"为评审时口径，本表为当日实测）：

| # | 调用点 | 场景簇 | 特征 props |
| --- | --- | --- | --- |
| 1 | `CentralGroupedSkillList.tsx:139` | central | builder 展开 + platformIcons + footer |
| 2 | `CentralSkillListContent.tsx:144`（list） | central | builder 展开（无 platformIcons/footer） |
| 3 | `CentralSkillListContent.tsx:149`（grid） | central | builder 展开 + platformIcons + footer |
| 4 | `PlatformView.tsx:496` | platform | sourceType/originKind/isReadOnly/checkbox?/onInstallTo?/onUninstallFromPlatform?/uninstallFromLabel/publisher/usageBadge/detailButtonRef/className |
| 5 | `ProjectsShell.tsx:520` | project | sourceType/originBadge/platformBadge/onUninstallFromPlatform/uninstallFromLabel |
| 6-7 | `ObsidianVaultView.tsx:392,433` | import | isCentral/platformBadge/projectBadge/onInstallToCentral/onInstallToPlatform/detailButtonRef（392 另有 className） |
| 8 | `MarketplaceShell.tsx:257`（推荐 Tab） | marketplace | tags/publisher/onDetail（无安装动作） |
| 9 | `MarketplaceShell.tsx:560`（skills.sh Tab） | marketplace | publisher/onInstall/installLabel/isLoading |
| 10 | `CollectionView.tsx:418` | collection | onDetail/onInstallTo/onRemove |
| 11 | `CollectionsListView.tsx:484` | collection | 同上 + detailButtonRef |

- central 场景已有唯一 props 构建器 `src/components/central/centralSkillCardProps.ts`（`buildCentralSkillCardProps`，也是 `UnifiedSkillCardProps` 类型的唯一外部 import 者）。
- 测试面：`src/test/UnifiedSkillCard.test.tsx`（13 用例，全部裸最小 props）；`marketplaceViewTestSupport.tsx` 以 `vi.mock` 替换卡片（运行时 mock，类型不受本次改动约束）；页面测试 PlatformView / ProjectsShell / CollectionView / CollectionsListView / CentralSkillsView.* 真实渲染卡片，构成视觉行为锁定面。
- 死 prop 实测：`onClick`（含 `:256` 分支与 `platform.searchSkillLabel` key，zh/en `:539`）、`summaryLabel`、`isInstalled` 在生产与测试中均无人传。

## 2. 方案对比（design-it-twice）

| 维度 | 方案 A：顶层判别联合 | 方案 B：场景对象 prop（core 顶层 + `central={…}` 互斥嵌套） | 方案 C：具名薄包装组件 ×6 |
| --- | --- | --- | --- |
| 调用点形状 | 现有扁平 props + 一行 `variant="…"`，迁移≈加一行 | 全部场景 props 下沉一层嵌套，11 处调用点整体改写 | `<CentralSkillCard …/>`，改 import + 标签名 |
| 互斥强制 | 判别联合原生：TS 按 variant 收窄后做 excess property check，错误信息直指非法 prop | 需 `{central: X; platform?: never; …}` never 样板 ×6²，错误信息晦涩 | 各组件独立 props，天然互斥 |
| depth/locality | 单入口单实现；组合判定内化进 toModel | 同左，但调用方多一层对象包装的认知税 | 表面 6 个模块；违反「唯一卡片实现」约束字面义（CLAUDE.md/CONTEXT.md），CLAUDE.md 明文「只用 UnifiedSkillCard」 |
| 迁移成本 | 最低（builder 改返回类型，调用点加 variant） | 最高（所有调用点重排 props） | 中（import 面全改），且约束不允许 |
| 裁决 | **采用** | 弃（样板 + 迁移成本，无额外收益） | 弃（约束排除） |

成员内形状的次级对比：central 的 5 个动作回调曾考虑聚合为 `actions: {…}` 对象（可再降顶层计数），弃——与其余场景扁平回调风格不一致，且 builder 是 central 唯一构造方，聚合收益仅体现在类型声明处，反而增加一层解包。

## 3. 目标 interface（核心契约）

```ts
// —— 公共 core（所有场景共享，4 个）——
interface SkillCardCoreProps {
  name: string;
  description?: string;
  aiSummary?: string | null;
  className?: string;
}

export interface CentralSkillCardProps extends SkillCardCoreProps {
  variant: "central";
  skillId?: string;
  checkbox: { checked: boolean; onChange: () => void };
  statusAccent?: "amber" | "red";
  statusChipLabel?: string;
  tags?: { key: string; label: string }[];
  publisher?: string;
  usageBadge?: number;
  updateStatus?: CentralSkillUpdateState & { isUpdating?: boolean };
  onDetail: MouseEventHandler<HTMLButtonElement>;
  onInstallTo: () => void;
  onUninstallFromPlatforms: () => void;
  onUpdateCentral: () => void;
  onDeleteFromCentral: () => void;
  detailButtonRef?: Ref<HTMLButtonElement>;
  editableTags?: { /* 形状不变 */ };
  density?: SkillCardDensity;
  platformIcons?: { /* 形状不变 */ };
  footer?: { repoName?: string; repoColor?: string };
}

export interface PlatformSkillCardProps extends SkillCardCoreProps {
  variant: "platform";
  sourceType: "symlink" | "copy" | "native";
  originKind: ClaudeSourceKind | null;
  isReadOnly: boolean;
  publisher?: string;
  usageBadge?: number;
  checkbox?: { checked: boolean; onChange: () => void };
  isLoading?: boolean;
  onDetail: MouseEventHandler<HTMLButtonElement>;
  onInstallTo?: () => void;
  onUninstallFromPlatform?: () => void;
  uninstallFromLabel: string;
  detailButtonRef?: Ref<HTMLButtonElement>;
}

export interface ProjectSkillCardProps extends SkillCardCoreProps {
  variant: "project";
  sourceType?: "symlink" | "copy" | "native";
  originBadge: { kind: string; label: string };
  platformBadge: { id: string; name: string };
  onUninstallFromPlatform: () => void;
  uninstallFromLabel: string;
  isLoading?: boolean;
}

export interface ImportSkillCardProps extends SkillCardCoreProps {
  variant: "import"; // Obsidian vault 导入候选（原 discover 场景簇）
  isCentral: boolean;
  platformBadge: { id: string; name: string };
  projectBadge?: string;
  onDetail: MouseEventHandler<HTMLButtonElement>;
  detailButtonRef?: Ref<HTMLButtonElement>;
  onInstallToCentral: () => void;
  onInstallToPlatform: () => void;
  isLoading?: boolean;
}

export interface MarketplaceSkillCardProps extends SkillCardCoreProps {
  variant: "marketplace";
  publisher?: string;
  tags?: { key: string; label: string }[];
  onDetail: MouseEventHandler<HTMLButtonElement>;
  onInstall?: () => void;
  installLabel?: string;
  isLoading?: boolean;
}

export interface CollectionSkillCardProps extends SkillCardCoreProps {
  variant: "collection";
  onDetail: MouseEventHandler<HTMLButtonElement>;
  detailButtonRef?: Ref<HTMLButtonElement>;
  onInstallTo: () => void;
  onRemove: () => void;
}

export type UnifiedSkillCardProps =
  | CentralSkillCardProps
  | PlatformSkillCardProps
  | ProjectSkillCardProps
  | ImportSkillCardProps
  | MarketplaceSkillCardProps
  | CollectionSkillCardProps;
```

- requiredness 以 §1 调用点矩阵「恒传且值恒定义」为准；实现中以 tsc 对 11 个调用点的实测微调 optional/required，规则不变、只紧不松。
- `SkillCardDensity`（含 `"default"` 旧别名归一化）原样保留，仅出现在 central 成员。

### 计数复核（硬指标）

| 场景 | 自有 props | + core 4 + variant | 现状 |
| --- | --- | --- | --- |
| central | 18 | **23** | 40 |
| platform | 12 | **17** | 40 |
| project | 6 | **11** | 40 |
| import | 8 | **13** | 40 |
| marketplace | 6 | **11** | 40 |
| collection | 4 | **9** | 40 |

central 是唯一全功能管理面（5 动作 + tags/footer/platformIcons 三个功能区），23 为本质复杂度；且所有成员内 props 均场景相关，跨场景漏出为 0。

## 4. 内部实现

```
UnifiedSkillCard.tsx
  ├─ 判别联合类型（§3，导出）
  ├─ SkillCardModel        // 原 40-prop interface 去掉 onClick/summaryLabel/isInstalled 后降为模块私有
  ├─ toModel(props): SkillCardModel   // 纯映射：switch (props.variant)，每分支只拷贝该场景合法字段
  └─ UnifiedSkillCardComponent(props) // 首行 const model = toModel(props)；其余渲染代码零改动
```

- `:256` 可点击分支整块删除（连带 `onClick`、`platform.searchSkillLabel` zh/en key）。
- `inventorySkillId = skillId ?? platformIcons?.skillId` 等派生逻辑原样保留在渲染侧（model 同名字段直通）。
- `memo` 包装、displayName、子组件（SkillCardMeta / UnifiedSkillCardFooter / CardTagEditor / CompactCardMoreMenu / InlineConfirmAction）接口零改动。

## 5. 迁移面（全部一次 flip，无兼容垫层）

| 动作 | 面 |
| --- | --- |
| `buildCentralSkillCardProps` 返回类型 → `CentralSkillCardProps`（返回对象加 `variant: "central"`；context 的 `density` 类型改引 `CentralSkillCardProps["density"]`） | 1 文件 |
| 11 处 JSX 调用点补 `variant="…"`（central 3 处经 builder 自动获得；grid 模式 spread + platformIcons/footer 附加写法保持） | 8 文件 |
| `UnifiedSkillCard.test.tsx` 13 用例按语义归入场景（AI 摘要/usageBadge → platform 或 marketplace；statusChip/editableTags/footer/updateStatus → central，文件内加 `centralBaseProps` 辅助收敛必填噪音） | 1 文件 |
| 新增 `src/test/unifiedSkillCardVariants.test.tsx`：每场景 1 个正例（可构造/可渲染）+ ≥4 组跨场景互斥负例（`@ts-expect-error` 带描述；对象字面量形态为主、附单行 JSX 形态各 1，双证 EPC 生效） | 新文件 |
| `marketplaceViewTestSupport.tsx` 的 vi.mock：运行时替身不受类型约束，仅核查其消费的 prop 名未被重命名（未被重命名，零改动预期） | 0-1 文件 |
| 删除 zh/en `platform.searchSkillLabel` | 2 文件 |

## 6. 验收数值（grep/命令复核表）

| 检查 | 目标 |
| --- | --- |
| `UnifiedSkillCard.tsx` 中 `onClick` / `summaryLabel` / `isInstalled` | 0 处 |
| 全仓 `searchSkillLabel` | 0 处 |
| 生产 JSX 调用点 `variant=`（含 builder 注入） | 11 处全覆盖 |
| `@ts-expect-error` 负面互斥用例 | ≥ 4 组，`pnpm typecheck` 绿（含 Unused directive 反向保证） |
| 场景成员 props 计数 | §3 表（central ≤ 24、其余 ≤ 17） |
| `pnpm test` / `pnpm typecheck` / `pnpm lint` | 全绿（页面测试 PlatformView / ProjectsShell / Collection× 2 / CentralSkillsView.* 构成视觉行为锁定面） |

## 7. 风险与对策

| 风险 | 对策 |
| --- | --- |
| 判别联合 + JSX spread（grid 模式 `{...buildCardProps()}` + 附加 props）类型边界 | builder 返回**具体成员类型**（非 union），spread 后附加同成员 props 合法；负面用例文件先行验证 |
| requiredness 收紧导致存量测试大量改动 | 测试文件内 `centralBaseProps` 等小辅助；页面测试传参来自真实调用点、天然满足 required |
| 删除死分支/死 prop 造成隐蔽回归 | 三者均实测 0 引用；全量 `pnpm test` + 页面测试锁定；i18n key 删除仅影响已死 aria-label |
| `@ts-expect-error` 位置错行导致误报/漏报 | 对象字面量形态（错误落在具体属性行，directive 紧邻其上）为主；tsc 的 Unused directive 错误提供反向保证 |
| ESLint `ban-ts-comment` | tseslint recommended 允许带描述的 `@ts-expect-error`，所有 directive 均带中文描述 |

## 8. 回滚与兼容

- 纯前端重构，无 IPC/DB/后端改动，无运行时迁移；分步提交（重构主体 / spec 文档），任一步 `git revert` 即净回滚。
- 对外行为唯一变化 = 删除三个实测无人使用的死 prop 与一个死渲染分支；所有在用路径渲染输出比特级不变。
