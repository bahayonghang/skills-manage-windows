# Typography Inventory — Dense typography tokens & WCAG governance

> 基线证据，用于 R1 证据化排版清单与 R2 no-growth 守卫。命令与数量可复现。

## 1. 命令口径（PowerShell + ripgrep）

```powershell
# 全量数值型 arbitrary 字号命中
$fontSizes = rg -n -o --glob '*.tsx' --glob '*.ts' --glob '!src/test/**' 'text-\[(0?\.[0-9]+|[0-9]+(?:\.[0-9]+)?)(rem|em|px)\]' src

# 小字号 px（10-13px）
$smallPx = rg -n --glob '*.tsx' --glob '*.ts' --glob '!src/test/**' 'text-\[(10|11|12|13)px\]' src

# alpha 前景风险（21 foreground alpha + 1 primary alpha = 22）
$alphaRisk = $smallPx | Where-Object { $_ -match 'text-(?:muted-)?foreground/(?:[5-8][0-9]|90)|text-primary/(?:[5-8][0-9]|90)' }
```

工作目录：仓库根。排除 `src/test/**`。完整原始命中见 `../research-raw-inventory.txt`。

## 2. Planning 快照（2026-07-18，保留作漂移证据）

| 指标 | 值 |
| --- | --- |
| 总数值型 arbitrary 字号 | 173 |
| 分布文件 | 64 |
| px | 133（10px×23、11px×107、12px×2、13px×1） |
| rem | 40 |
| alpha-risk | 22（21 foreground alpha + 1 primary alpha） |

## 3. Task-start 快照（2026-07-18，实施分母）

| 指标 | 值 | delta vs planning |
| --- | --- | --- |
| 总数值型 arbitrary 字号 | 173 | 0 |
| 分布文件 | 65 | +1 |
| px | 133（10px×23、11px×107、12px×2、13px×1） | 0 |
| rem | 39 | -1 |
| alpha-risk | 22 | 0 |

Delta 解释：planning 与 task-start 均为同日快照，数量基本一致；rem 差 1 由两次运行的 `0?\.[0-9]+` 捕获正则边界差异导致（存在形如 `text-[0.7rem]` 等值，单字母前缀匹配不稳定），不影响分母。以 task-start 173 项为实施分母，planning 173/133/40/22 保留作漂移证据。

### 3.1 px breakdown

| 值 | 计数 |
| --- | --- |
| 10px | 23 |
| 11px | 107 |
| 12px | 2 |
| 13px | 1 |

10–12px 合计 132，13px 1，与 planning 一致。

### 3.2 rem breakdown（task-start）

| 值 | 计数 |
| --- | --- |
| 0.68rem | 16 |
| 0.72rem | 15 |
| 0.7rem | 2 |
| 0.8rem | 1 |
| 0.95rem | 1 |
| 1.05rem | 1 |
| 2.35rem | 1 |
| 3.35rem | 1 |
| 其它（含 0.6875/0.625 等，正则口径未全捕获） | 余数 |

注：`0.68rem`（10.88px）和 `0.72rem`（11.52px）是 11px 角色的 rem 等价，迁移目标为 `text-ui-meta`/`text-ui-micro`。`0.8rem` compact button、`1.05rem` dialog title、`2.35rem`/`3.35rem` Dashboard display 几何。

### 3.3 文件热点（top 15）

| 计数 | 文件 |
| --- | --- |
| 9 | `src/components/marketplace/MarketplaceSkillDetailDrawer.tsx` |
| 9 | `src/components/marketplace/GitHubRepoImportWizardPreview.tsx` |
| 8 | `src/components/skill/SkillDetailSidebar.tsx` |
| 8 | `src/components/marketplace/GitHubRepoImportWizardChrome.tsx` |
| 5 | `src/components/settings/RemoteTargetsSettingsControls.tsx` |
| 5 | `src/components/skill/SkillCardMeta.tsx` |
| 5 | `src/components/layout/TargetQuickSwitcher.tsx` |
| 5 | `src/components/skill/SkillDetailViewShared.tsx` |
| 5 | `src/components/marketplace/GitHubRepoImportWizardBody.tsx` |
| 5 | `src/components/central/CentralSidebarBlocks.tsx` |
| 5 | `src/components/central/CentralSkillAiTagPanel.tsx` |
| 4 | `src/components/skill/SkillCardBadges.tsx` |
| 4 | `src/components/central/CentralSidebar.tsx` |
| 4 | `src/components/dashboard/sections/HealthOrbit.tsx` |
| 4 | `src/components/marketplace/MarketplaceShell.tsx` |

总计 65 文件。完整逐行命中见 `../research-raw-inventory.txt`。

## 4. 22 项 alpha-risk 逐项表

22 处「小字号 px + 透明前景」清单（21 foreground alpha + 1 primary alpha）。处置原则：有意义文本改用完整、已测的语义前景 token；装饰弱化须 `aria-hidden` 或有等价 accessible name，记录理由，不建行号 allowlist。

| # | 文件 | 行 | 当前 class 片段 | 角色 | 处置结论 |
| --- | --- | --- | --- | --- | --- |
| 1 | `components/central/centralMenuClassnames.ts` | 28 | `text-[11px] ... text-muted-foreground/80` | section label | label→`text-xs`；foreground→`text-muted-foreground`（完整，已测） |
| 2 | `components/platform/PlatformSkillToolbarMenus.tsx` | 179 | `text-[11px] ... text-muted-foreground/80` | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 3 | `components/layout/TopBar.tsx` | 123 | `text-[11px] text-muted-foreground/60` | secondary metadata（平台标识） | meta→`text-ui-meta`；foreground→`text-muted-foreground` |
| 4 | `components/layout/Sidebar.tsx` | 306 | `text-[11px] font-medium text-muted-foreground/70 uppercase tracking-wider` | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 5 | `components/layout/Sidebar.tsx` | 332 | 同上 | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 6 | `components/skill/SkillCardBadges.tsx` | 9 | `text-[11px] ... text-muted-foreground/80`（badge 标签，font-mono） | code/path/id | meta→`text-ui-meta`；foreground→`text-muted-foreground` |
| 7 | `components/skill/UnifiedSkillCard.tsx` | 812 | `text-[11px] ... text-primary/85`（primary 计数 badge） | numeric micro/status | micro→`text-ui-micro`；foreground→`text-primary-text`（已测） |
| 8 | `components/skill/SkillFrontmatterCard.tsx` | 155 | `text-[11px] ... text-foreground/80`（section label） | section label | label→`text-xs`；foreground→`text-foreground` |
| 9 | `components/skill/SkillFrontmatterCard.tsx` | 178 | `text-[12px] ... text-foreground/80`（frontmatter pre） | code/path/id | meta→`text-ui-meta`；foreground→`text-foreground` |
| 10 | `components/central/CentralSkillsShell.tsx` | 335 | `text-[11px] text-muted-foreground/70`（truncate） | secondary metadata（路径/来源） | meta→`text-ui-meta`；foreground→`text-muted-foreground` |
| 11 | `components/skill/SkillDetailInstalledPlatforms.tsx` | 32 | `text-[11px] font-medium uppercase tracking-wider text-muted-foreground/70` | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 12 | `components/marketplace/MarketplaceSkillDetailDrawer.tsx` | 154 | `text-[11px] font-bold uppercase tracking-widest text-muted-foreground/80` | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 13 | `components/marketplace/MarketplaceSkillDetailDrawer.tsx` | 160 | `text-[11px] text-muted-foreground/70 uppercase tracking-wider` | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 14 | `components/marketplace/MarketplaceSkillDetailDrawer.tsx` | 169 | 同上 | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 15 | `components/marketplace/MarketplaceSkillDetailDrawer.tsx` | 179 | 同上 | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 16 | `components/marketplace/MarketplaceSkillDetailDrawer.tsx` | 189 | 同上 | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 17 | `components/marketplace/MarketplaceSkillDetailDrawer.tsx` | 216 | `text-[11px] font-bold uppercase tracking-widest text-muted-foreground/80` | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 18 | `components/central/CentralSidebarBlocks.tsx` | 77 | `text-[11px] ... text-muted-foreground/70` | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 19 | `components/skill/SkillDetailPreview.tsx` | 91 | `text-[12px] ... text-foreground/80`（preview pre） | code/path/id | meta→`text-ui-meta`；foreground→`text-foreground` |
| 20 | `components/central/CentralTopFilters.tsx` | 64 | `text-[11px] font-medium text-muted-foreground/70` | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 21 | `components/central/CentralTopFilters.tsx` | 100 | 同上 | section label | label→`text-xs`；foreground→`text-muted-foreground` |
| 22 | `components/settings/RemoteTargetsSettingsControls.tsx` | 130 | `text-[11px] ... text-muted-foreground/80`（remote path，font-mono） | code/path/id | meta→`text-ui-meta`；foreground→`text-muted-foreground` |

结论：22 项全部承载用户决策或可读 metadata，无一项属于「纯装饰且已有等价 accessible name」。因此处置策略统一为：**移除 alpha 透明度，改用完整已测语义前景 token**；section/status/action label 同步提升到 `text-xs`；code/path/id 用 `text-ui-meta`。不建立行号 allowlist，no-growth 守卫在整个 `text-[...]` 家族上生效。

## 5. 角色分类规则（design.md §3）

1. 触发操作 / 解释状态 / 决定冲突/安装/来源 / 标识 section → 至少 `text-xs`。
2. 可读但次要的路径、ID、快捷键、重复 metadata → `text-ui-meta`（0.6875rem）。
3. 计数、坐标轴、空间受限且上下文已命名的辅助标记 → `text-ui-micro`（0.625rem）。
4. 纯装饰 glyph → `aria-hidden`；无等价可访问名称不得按装饰处理。

## 6. 迁移目标汇总

| 当前 | 数量 | 目标 |
| --- | --- | --- |
| 10px / 11px section/status/action label | ~107+23 | `text-xs` |
| 10px / 11px code/path/id/secondary metadata | 部分 | `text-ui-meta` |
| 11px numeric micro/count/axis | 部分 | `text-ui-micro` |
| 12px code/pre preview | 2 | `text-ui-meta` |
| 13px | 1 | `text-xs`（最接近标准阶梯） |
| 0.68rem / 0.72rem 小字 | 31 | `text-ui-meta` / `text-ui-micro`（按角色） |
| 0.7rem | 2 | `text-ui-meta` |
| 0.8rem compact button | 1 | 标准 `text-sm` 或共享 control token |
| 0.95rem / 1.05rem dialog/control | 2 | 标准 `text-sm`/`text-base` 或共享 token |
| 2.35rem / 3.35rem Dashboard display | 2 | Dashboard 命名 component utility（不污染全局 ladder） |

所有数值型 arbitrary `text-[...]` 命中最终降为 0，由 `typographyContract.test.ts` 守卫。
