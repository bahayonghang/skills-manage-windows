# Implementation Plan: Dense typography tokens and WCAG governance

## 1. Baseline and failing contracts

1. 任务启动时重跑 inventory，并在 `research/typography-inventory.md` 并列记录 planning 快照与 task-start delta。Planning 快照为 173 个数值型 arbitrary 字号、64 文件，其中 133 px（23x10px、107x11px、2x12px、1x13px）+ 40 rem；alpha-risk 为 22（21 foreground alpha + 1 primary alpha）。建议固化以下 PowerShell 口径：

```powershell
$fontSizes = rg -n -o --glob '*.tsx' --glob '*.ts' --glob '!src/test/**' 'text-\[(0?\.[0-9]+|[0-9]+(?:\.[0-9]+)?)(rem|em|px)\]' src
$smallPx = rg -n --glob '*.tsx' --glob '*.ts' --glob '!src/test/**' 'text-\[(10|11|12|13)px\]' src
$alphaRisk = $smallPx | Where-Object { $_ -match 'text-(?:muted-)?foreground/(?:[5-8][0-9]|90)|text-primary/(?:[5-8][0-9]|90)' }
```

2. 按 label/meta/code/status/micro/decorative 分类每个命中，并记录目标 utility；优先复核 GitHub import、Central、remote target 和状态/错误文本。
3. 新增 `src/test/typographyContract.test.ts`：读取生产 TS/TSX，先断言没有任何 `text-[...]` 并观察失败，再实现迁移；测试不得依赖行号或允许数值 allowlist，因当前生产没有必须保留的 arbitrary text color class。
4. 扩展 `fontContract.test.ts`，断言语义字号使用 rem、继承根级 `--font-scale`，且不新增 viewport 字号。
5. 扩展 `themeContrast.test.ts` 的 surface/alpha 合成覆盖；先为现有失败或未覆盖组合建立明确失败证据。

## 2. Token implementation

1. 在 `src/index.css` 的 Tailwind theme 边界增加 `text-ui-meta`（0.6875rem）和 `text-ui-micro`（0.625rem）。
2. 保持 token 只负责字号；保留组件现有 line-height、weight、tabular nums 与 font family。
3. 将 40 处 arbitrary rem 按语义迁移：小字进入标准/meta/micro，compact control/dialog 优先标准或共享 token，Dashboard display 几何使用命名 component utility，避免污染全局 type ladder。
4. 在前端 spec 记录 label=`text-xs`、meta、micro、deliberate display utility 的语义和全面禁用 `text-[...]` 的 no-growth 契约，并更新 `frontend/index.md`。

## 3. Migration order

1. 迁移 `UnifiedSkillCard`、`SkillCardMeta`、`SkillCardBadges`、footer 与 Central shell/sidebar/filter/menu；跑卡片和 Central 定向测试。
2. 迁移 GitHub import wizard chrome/body/preview/file tree；冲突、选择、状态、路径不得降为 micro，跑 Preview/Confirm/Result 回归。
3. 迁移 Sidebar/TopBar/TargetQuickSwitcher、RemoteTargets settings 与 platform toolbar；处理导航 label、凭据状态、错误与路径对比度。
4. 迁移 Marketplace、Projects、Usage/heatmap、AI settings 和剩余组件；heatmap 轴/计数可使用 micro，正文和状态不可。
5. 对技能详情仅做同尺寸 token 替换；运行现有 `SkillDetailView` / `SkillDetailFileTree` 回归，确认 2026-07-16 信息层级未改变。
6. 每个 slice 后重跑 inventory；最终生产 `text-[...]` 为 0，并检查没有用 arbitrary rem、CSS length 函数或动态 class 拼接绕过守卫。

## 4. Contrast remediation

1. 对 research 中 22 个 alpha-risk 项逐项确认 surface 与语义。
2. 有意义文本改用 `text-muted-foreground`、`text-foreground`、`text-primary-text` 或 semantic foreground 的完整 token。
3. 对确属装饰/冗余的弱化项确认 `aria-hidden` 或等价 accessible name；记录理由，不建立代码行 allowlist。
4. 跑六主题/14 accent contrast matrix；任何未测实际组合保持“missing evidence”，不能写成通过。

## 5. Scale, layout and virtualization checks

1. 增加 DOM/computed-style 测试，证明 micro/meta 在 0.875 / 1 / 1.125 下随根字号变化。
2. 用 >60 list 和 >40 grid fixtures 覆盖虚拟化路径，检查 card/row 分配高度、相邻边界与首末项可达。
3. 在 0.875、1、1.125 下检查 Central、GitHub import、Marketplace、Settings、Projects/Usage；视口为 900x600、1200x800、1440x900，并覆盖中文、长英文、长 Windows path、JetBrains/system body font。
4. 若仅 1.125 的固定 virtual item height 不足，先做 Central 局部的 scale-aware height 修正并补测试；不修改阈值/overscan，不无证据重写共享 virtualizer。
5. 保存默认 Scale=1 的代表性 before/after 截图，确认单位屏信息量和技能详情视觉层级没有显著回退。

## 6. Validation

```powershell
pnpm vitest run src/test/typographyContract.test.ts src/test/fontContract.test.ts src/test/themeContrast.test.ts src/test/displayFont.test.ts
pnpm vitest run src/test/UnifiedSkillCard.test.tsx src/test/CentralSkillsView.github-import-preview.test.tsx src/test/GitHubRepoImportWizard.test.tsx src/test/SkillDetailView.test.tsx src/test/SkillUsage.components.test.tsx
pnpm typecheck
pnpm lint
pnpm build
git diff --check
just ci
```

测试文件名以实施时仓库实际存在者为准；不存在时选取同一行为边界的现有 focused suite，不为满足命令名称创建空测试。

## 7. Risk files and rollback points

- `src/index.css`
- `src/components/skill/UnifiedSkillCard*.tsx`
- `src/components/skill/SkillCardMeta.tsx`
- `src/components/skill/SkillCardBadges.tsx`
- `src/components/central/CentralSkillListContent.tsx`（仅条件性高度适配）
- GitHub import / Marketplace / layout / settings / projects / usage 的当前 inventory 文件
- `src/test/typographyContract.test.ts`
- `src/test/themeContrast.test.ts`
- `.trellis/spec/frontend/`

建议提交形状：

1. inventory + failing contract tests + token。
2. Central/shared card + GitHub import 高风险迁移。
3. 其余 surface + contrast remediation。
4. 条件性 virtual height 修正 + visual evidence/spec。

每个 migration slice 都可独立回滚 class 变更；token 在所有消费者回滚后再删除。若虚拟化修正不稳定，回滚该条件 commit，但不能把已确认的重叠写成通过。
