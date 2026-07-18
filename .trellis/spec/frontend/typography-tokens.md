# 语义字号与 no-growth 排版契约

> 建立于 2026-07-18（任务 07-18-dense-typography-wcag）。高密度排版从散落固定 px 收敛为可缩放、可测试的语义层级，并阻止任意字号继续增长。字体 profile/fallback/Scale 选项仍由 [font-preferences.md](./font-preferences.md) 拥有，本规范只治理字号与前景角色。

## 1. Scope / Trigger

修改任何 `src/**/*.ts(x)` 中的字号 utility、新增小字标签/状态/计数、或在 `src/index.css` 的 `@theme inline` 边界增删语义字号 token 时适用。组件不得自行拼 `text-[...]` arbitrary 字号或 arbitrary 颜色，也不得用 inline `style` fontSize 绕过契约。

## 2. Semantic type ladder

| 角色 | Utility | 默认尺寸 | 用途 |
| --- | --- | ---: | --- |
| 正文 / 操作 | `text-sm` 或现有组件尺寸 | 0.875rem | 正文、主要控件、持续阅读的状态说明 |
| Label | `text-xs` | 0.75rem | section、status、action、table header、badge、filter group、dialog/control label 等决策相关标签 |
| UI meta | `text-ui-meta` | 0.6875rem | 次要 metadata、ID、路径、keycap、frontmatter/preview pre、紧凑辅助说明 |
| UI micro | `text-ui-micro` | 0.625rem | 紧凑数字计数、heatmap 坐标轴、冗余 chevron/step marker、`aria-hidden` 装饰 glyph |
| Dashboard display | `text-display-hero` / `text-display-hero-xl` / `text-display-score` | 2.35 / 3.25 / 3.35rem | Dashboard hero 标题与 score 大数，归 Dashboard surface 所有，不参与通用 type ladder |

规则：
- `section` / `status` / `action` label 至少 `text-xs`，不落入 `text-ui-meta` / `text-ui-micro`。
- `code` / `path` / `id` / 快捷键 / 重复 metadata 使用 `text-ui-meta`。
- `text-ui-micro` 仅用于紧凑计数、坐标轴或已有等价可访问名称的辅助标记；纯装饰 glyph 必须同时 `aria-hidden`。
- 不要新增 `text-ui-label` 同义 token：标准 `text-xs` 已表达该层级。不要把 11px 一对一替换为 meta；先按角色分类，真正的 label 提升到 `text-xs`，路径/ID/辅助信息才使用 meta。
- 状态重要性通过 weight、foreground、icon 和位置表达，不通过再造 10.5px/11.5px 值表达。

## 3. Scale contract

- 所有语义字号 token 在 `src/index.css` 的 `@theme inline` 边界用 `rem` 声明，只定义 `font-size`，不覆盖 `line-height`；`leading-*` 继续由拥有布局语义的组件控制。
- 根级 `font-size: calc(16px * var(--font-scale))`（0.875 / 1 / 1.125 三档）驱动全部 rem token；不得新增 viewport 驱动字号、运行时 DOM 扫描或组件内字号计算。
- 默认 Scale=1 时 `text-ui-meta`（11px）与 `text-ui-micro`（10px）保持迁移前几何，避免机械放大造成密度回退；只有从微型文本提升为真正 label/body 的角色才允许有依据地增大。

## 4. No-growth contract

- 生产 `src/**/*.ts(x)`（排除 `src/test/**`）禁止整个 `text-[...]` arbitrary 家族：包括 `text-[Npx]`、`text-[Nrem]`、`text-[Nem]`、`text-[calc(...)]`、`text-[clamp(...)]` 与 arbitrary `text-[#hex]` 颜色。
- 不保留按文件/行号或按数值的 allowlist；确需保留的 deliberate display 几何也必须使用命名语义 token（如 `text-display-*`），不得继续写 arbitrary class。
- 颜色必须通过 theme token / named utility 表达，不能借 arbitrary syntax 绕过字号或颜色契约。
- 守卫由 `src/test/typographyContract.test.ts` 在 CI 期读取源码实现，不进入生产 bundle，不增加运行时 observer / resize listener / 扫描逻辑。

## 5. Contrast contract

- 承载标签、状态、路径、错误、来源、操作或用户决策的小字，在实际 `background` / `card` / `popover` / `sidebar` / 状态 surface 上达到 WCAG 2.1 AA 普通文本 4.5:1。
- 有意义文本默认不使用 `/60` `/70` `/80` 透明前景色；`primary` 用于填充/强调，普通 accent 文本使用已测的 `text-primary-text`；`success` / `warning` / `info` / `destructive` 使用现有 semantic foreground，不新增硬编码组件色。
- `disabled` 或纯装饰例外必须同时具有不可操作语义/等价 label 并记录在任务 research 决策表；不建立按行号 allowlist。
- 测试基线为 `src/test/themeContrast.test.ts`（六主题×14 accent×核心前景/背景/card/popover/sidebar/muted/alpha 合成），新治理扩展该测试，不引入第二套颜色或字体模型。

## 6. Tests Required

- `src/test/typographyContract.test.ts`：扫描生产 TS/TSX，断言无任何 `text-[...]`、无 arbitrary size/color、`--text-ui-meta`/`--text-ui-micro` 存在、无 viewport 字号。
- `src/test/fontContract.test.ts`：token 在 `@theme inline` 用 rem 声明、不覆盖 line-height、阶梯 `micro < meta < xs`、由根 `--font-scale` 驱动。
- `src/test/themeContrast.test.ts`：六主题×14 accent 在 background/card/popover/sidebar/muted 上满足 AA；alpha 前景与真实 surface 合成后满足 AA。
- 改动后至少运行定向 Vitest、`pnpm typecheck`、`pnpm lint`、`pnpm build` 与 `just ci`。

## 7. Wrong vs Correct

```tsx
// Wrong: arbitrary px/rem 字号 + alpha 前景，绕过 token 与对比度契约
<span className="font-mono text-[11px] text-muted-foreground/80">{path}</span>

// Wrong: 用 arbitrary color 绕过颜色契约
<span className="text-[#a6adc8]">label</span>

// Correct: 次要路径用 meta + 完整已测 foreground；状态/section label 至少 text-xs
<span className="font-mono text-ui-meta text-muted-foreground">{path}</span>
<span className="text-xs font-medium text-muted-foreground">Source</span>
```
