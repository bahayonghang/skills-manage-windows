---
name: SkillPort
description: 跨平台 AI 技能管理桌面应用的视觉系统——调度台美学，本地优先，可换肤
colors:
  lavender-signal: "#b4befe"
  mocha-base: "#1e1e2e"
  mantle-panel: "#181825"
  crust-deep: "#11111b"
  catppuccin-text: "#cdd6f4"
  subtext-mute: "#a6adc8"
  surface-stroke: "#313244"
  surface-mute: "#45475a"
  surface-raise: "#585b70"
  catppuccin-red: "#f38ba8"
  claude-coral: "#cc785c"
typography:
  display:
    fontFamily: "Geist Variable, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.6rem"
    fontWeight: 650
    lineHeight: 1.15
    letterSpacing: "-0.03em"
  title:
    fontFamily: "Geist Variable, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1rem"
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: "normal"
  body:
    fontFamily: "JetBrains Mono Variable, ui-monospace, monospace"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.72
    letterSpacing: "normal"
  label:
    fontFamily: "JetBrains Mono Variable, ui-monospace, monospace"
    fontSize: "0.72rem"
    fontWeight: 650
    lineHeight: 1.4
    letterSpacing: "0.12em"
rounded:
  sm: "6px"
  md: "8px"
  lg: "10px"
  xl: "14px"
  2xl: "18px"
spacing:
  xs: "6px"
  sm: "8px"
  md: "10px"
  lg: "16px"
  xl: "24px"
components:
  button-primary:
    backgroundColor: "{colors.lavender-signal}"
    textColor: "{colors.mantle-panel}"
    typography: "{typography.label}"
    rounded: "{rounded.lg}"
    height: "32px"
    padding: "0 10px"
  button-outline:
    backgroundColor: "{colors.mocha-base}"
    textColor: "{colors.catppuccin-text}"
    rounded: "{rounded.lg}"
    height: "32px"
    padding: "0 10px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.catppuccin-text}"
    rounded: "{rounded.lg}"
    height: "32px"
    padding: "0 10px"
  button-destructive:
    backgroundColor: "{colors.catppuccin-red}"
    textColor: "{colors.catppuccin-red}"
    rounded: "{rounded.lg}"
    height: "32px"
    padding: "0 10px"
  card:
    backgroundColor: "{colors.mantle-panel}"
    textColor: "{colors.catppuccin-text}"
    rounded: "{rounded.xl}"
    padding: "16px"
  input-field:
    backgroundColor: "transparent"
    textColor: "{colors.catppuccin-text}"
    rounded: "{rounded.lg}"
    height: "32px"
    padding: "4px 10px"
  nav-item-active:
    backgroundColor: "{colors.lavender-signal}"
    textColor: "{colors.mantle-panel}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "6px 10px"
---

# Design System: SkillPort

## 1. Overview

**Creative North Star: "The Control Room / 调度台"**

SkillPort 是一个冷静、密集的指挥中心。重度用户每天住在这里，把中央目录当作技能的唯一真源，一眼看清每个技能装在哪些平台、再一次操作路由到位。整套视觉的目的不是被欣赏，而是让「技能装在哪、装没装、装失败了没」即时可读——工具消失在任务里。气质锚点是 Catppuccin 生态 / Ghostty / Obsidian / Zed 这类有个性但不牺牲密度与性能的开发者工具。

个性来自**主题系统本身，而非装饰**：Catppuccin 4 风味（Mocha / Macchiato / Frappé / Latte）+ Claude 亮/暗共 6 套主题、14 种可切换 accent。配色即身份。body 默认采用等宽字体（JetBrains Mono），强化终端原生的调度台手感；玻璃面板与径向光晕在 dashboard 营造「控制室」的纵深，而不是廉价的炫技。

这个系统明确**拒绝**：千篇一律的 SaaS 仪表盘（渐变大数字、hero-metric 模板、无差别卡片网格）；臃肿的企业级后台（工具栏塞满、层级混乱）；玩具感消费应用（过度圆角、气泡化）；以及一眼能看穿的 AI 生成通用感。可玩 ≠ 玩具感；鲜明 ≠ 喧闹。

**Key Characteristics:**
- 调度台密度：高信息量 + 清晰层级，键盘优先
- 主题即身份：6 主题 × 14 accent，配色承载个性
- 等宽 body：JetBrains Mono 默认正文，技术感冷静
- 分层玻璃：玻璃面板 + 径向光晕营造控制室纵深
- 本地优先的诚实：界面状态如实映射文件系统真相

## 2. Colors

调色板是 Catppuccin 的深色 surface 阶梯 + 一个可切换的高饱和 accent；克制、低噪、可换肤。下列十六进制为**默认主题 Mocha + 默认 accent Lavender** 的取值，是 frontmatter 的 canonical 来源；其余 5 套主题与 14 种 accent 通过 `[data-theme]` / `[data-accent]` 切换同名语义 token。

### Primary
- **Lavender Signal** (`#b4befe`)：默认 accent。仅用于主操作按钮、当前选中项、焦点环（`--ring`）与状态指示，**不做装饰**。可被 14 种 accent 之一覆盖（rosewater/mauve/peach/green/sky/blue…）。
- **Claude Coral** (`#cc785c`)：Claude 亮/暗主题的签名 accent，是第二条品牌身份线；仅在 claude-light / claude-dark 主题下作为 primary 出现。

### Neutral
- **Mocha Base** (`#1e1e2e`)：应用主背景（`--background`）。
- **Mantle Panel** (`#181825`)：卡片 / 弹层 / 侧栏的次级表面（`--card` / `--popover` / `--sidebar`），比正文背景略深，构成第二中性层。
- **Crust Deep** (`#11111b`)：最深底色，用于侧栏 primary 前景与极深分隔。
- **Catppuccin Text** (`#cdd6f4`)：正文与标题主文字（`--foreground`），对 Mocha Base 满足 ≥4.5:1。
- **Subtext Mute** (`#a6adc8`)：次要 / 占位文字（`--muted-foreground`）；占位符同样需达 4.5:1，不得再压暗。
- **Surface Stroke** (`#313244`)：描边 / 输入框 / 次级按钮底（`--border` / `--input` / `--secondary`）。
- **Surface Mute** (`#45475a`) 与 **Surface Raise** (`#585b70`)：hover 填充与 accent 表面（`--muted` / `--accent`），承载交互态而非静态装饰。

### Tertiary
- **Catppuccin Red** (`#f38ba8`)：唯一的 destructive 语义色（`--destructive`），以**低饱和着色**形式出现（红/10 底 + 红字），而非满涂实心红块。图表 5 色（chart-1..5）映射 green/blue/mauve/yellow/red，仅用于数据可视化。

### Named Rules
**The Theme-Is-Identity Rule.** 个性只允许通过主题与 accent 表达。任何「为了好玩」临时加的气泡化、玩具化装饰一律禁止。换肤是卖点；廉价点缀是噪音。

**The Accent-Is-Rare Rule.** Accent（primary/ring）只出现在主操作、当前选中、焦点与状态指示上，单屏占比保持克制。非激活状态禁止使用高饱和 accent 实色。

**The Color-Is-Never-Alone Rule.** 14 种 accent 与 chart 配色不得成为唯一信息载体。状态与图表除色相外，必须辅以图标、文字或形状，保证色弱可辨。

## 3. Typography

**Display Font:** Geist Variable（回退 ui-sans-serif / system-ui）
**Body Font:** JetBrains Mono Variable（回退 ui-monospace / monospace）
**Label/Mono Font:** JetBrains Mono Variable

**Character:** 几何无衬线（Geist）做标题，等宽（JetBrains Mono）做正文——这是一条真正的对比轴配对，而非两款相近的 sans。等宽正文给整套界面注入终端原生、可信赖的调度台语气。两者均为可变字体，用户可在设置里替换（display: geist/jetbrains/inter/serif/system；body: jetbrains/geist/inter/system），并有 0.875 / 1 / 1.125 三档 `--font-scale`。

### Hierarchy
- **Display** (Geist, 650, 1.6rem, line-height 1.15, letter-spacing -0.03em)：技能详情 / Markdown H1 等页面级主标题。固定 rem，不用 fluid clamp。
- **Title** (Geist, 500, 1rem / `text-base`, line-height 1.4)：卡片标题（`font-heading`），列表与面板的小标题。
- **Body** (JetBrains Mono, 400, 0.875rem / `text-sm`, line-height 1.72)：应用主力正文与数据。散文最大行宽 74ch；密集表格 / 代码可更密。
- **Label** (JetBrains Mono, 650, 0.72rem, letter-spacing 0.12em, UPPERCASE)：分组眉标、表头、徽标计数。仅限短标签，禁止整句全大写。

### Named Rules
**The Fixed-Scale Rule.** 产品 UI 用固定 rem 阶梯（步进 ~1.125–1.25），不用随视口缩放的 clamp 标题——侧栏里会缩水的 fluid h1 只会更丑。

**The Mono-Body Rule.** 正文默认等宽是刻意的身份选择，不是 bug。它服务调度台的技术冷静感；替换为 sans 是用户可选项，不是默认。

## 4. Elevation

分层玻璃为主，但克制。普通表面（卡片、列表项、输入）静止时近乎扁平——`ring-1` 细描边 + `shadow-sm`；阴影与发光主要作为 **hover / active / 选中 / 焦点** 的状态响应出现。真正的玻璃态（`.surface-glass` / `.surface-glass-strong` + 径向 `.bg-orbit`）保留给 dashboard 这类「控制室」营造点，靠 `backdrop-filter: blur(26–30px) saturate(155–160%)` + 大尺度软投影制造纵深，而非给每个卡片都套玻璃。

### Shadow Vocabulary
- **Rest card** (`box-shadow: 0 1px 2px 0 rgb(0 0 0 / 0.05)` ≈ `shadow-sm`)：卡片静止态，配合 `ring-1 ring-border`。
- **Hover lift** (`shadow-md`)：卡片悬停，唯一从 sm→md 的抬升，传达可交互。
- **Orbit glass** (`--shadow-orbit: 0 24px 70px color-mix(in oklch, var(--background) 86%, transparent)`)：dashboard 玻璃面板的大尺度软投影。
- **Inset hairline** (`inset 0 1px 0 color-mix(in oklch, var(--foreground) 6–8%, transparent)`)：玻璃 / 激活态顶部的 1px 高光，制造材质边。
- **Active-nav glow** (`0 10px 28px color-mix(in oklch, var(--sidebar-primary) 22%, transparent)`)：侧栏激活项的 accent 发光，仅限选中态。

### Named Rules
**The Flat-By-Default Rule.** 表面静止时扁平，深度靠中性分层（base → mantle → crust）传达。阴影/发光只作为状态响应出现，不作静态装饰。

**The Glass-Is-Earned Rule.** 玻璃态是营造点，不是默认皮肤。普通密集列表禁止铺满 `backdrop-filter`；玻璃只用在 dashboard 等少数控制室界面。

## 5. Components

### Buttons
- **Shape:** `rounded-lg`（10px），高 32px（`h-8`），内距 `0 10px`，`transition-colors` 150ms，`active:translate-y-px` 轻微下压。
- **Primary:** `bg-primary`（Lavender Signal）+ `text-primary-foreground`（Mantle）。
- **Outline / Secondary / Ghost:** outline = base 底 + border 描边 + hover muted；secondary = Surface Stroke 底；ghost = 透明 + hover muted。
- **Destructive（独特）:** 不是实心红块，而是**低饱和着色**——`bg-destructive/10` + `text-destructive`，hover 加深到 /20。
- **Focus:** `focus-visible:border-ring` + `ring-3 ring-ring/50`，焦点环始终可见。

### Cards / Containers
- **Corner Style:** `rounded-xl`（14px）。
- **Background:** `bg-card`（Mantle Panel）。
- **Shadow Strategy:** `shadow-sm` 静止 → `hover:shadow-md`（见 Elevation）。
- **Border:** **`ring-1 ring-border` 而非 border**——这是全应用卡片的统一签名样式。
- **Internal Padding:** 16px（`py-4` + `px-4`）；footer 用 `border-t bg-muted/50`。

### Inputs / Fields
- **Style:** `bg-transparent` + `border-input` 描边 + `rounded-lg`（10px），高 32px。
- **Focus:** `focus-visible:border-ring` + `ring-3 ring-ring/50`（聚光环）。
- **Error / Disabled:** `aria-invalid` → 红描边 + 红环；disabled → `bg-input/50` + 半透明 + 禁手势。

### Navigation
- **Style:** 侧栏（`bg-sidebar`），可折叠（208px ↔ 56px）且可拖拽调宽（168–360px）。导航项 `rounded-md`（8px）。
- **Default / Hover:** 文字 muted；hover → `bg-sidebar-accent` 填充。
- **Active:** sidebar-primary 渐变填充 + inset 高光 + accent 发光 + 左缘 3px 圆角指示条（active 态指示，非卡片侧边色条）。计数徽标用等宽 tabular-nums。
- **Focus:** `ring-2 ring-sidebar-ring ring-offset-1`。

### Signature: UnifiedSkillCard
全应用所有技能卡片的唯一实现，通过 props 自适应 5 种场景（central / platform / project / marketplace / collection）。统一样式：`rounded-xl` + `ring-1 ring-border` + `bg-card` + `shadow-sm`。平台图标分 LOBSTER / CODING 两行，点击即时切换安装/卸载。**禁止**在各页面重建内联卡片。

### Signature: Dashboard Readiness Panel
控制室的视觉中心：`.surface-glass` 面板 + `.bg-orbit` 径向背景 + `.readiness-score-plaque` 锥形渐变评分盘（`--score-angle` 驱动），辅以细网格 mask 与 inset 高光。这是「玻璃是营造点」的唯一合法重场景。

## 6. Do's and Don'ts

### Do:
- **Do** 用 `ring-1 ring-border` + `bg-card` + `rounded-xl` + `shadow-sm` 作为所有卡片的统一底；技能卡只用 `UnifiedSkillCard`。
- **Do** 让 accent（primary/ring）只承担主操作、当前选中、焦点与状态指示；保持单屏占比克制。
- **Do** 用中性分层（base → mantle → crust）传达深度，阴影/发光只作状态响应。
- **Do** 给每个交互组件补齐 default / hover / focus-visible / active / disabled / error 全套态。
- **Do** 让界面状态如实映射文件系统真相——安装/链接/失败/跳过/冲突都要诚实呈现。
- **Do** 让状态与图表除色相外，附带图标 / 文字 / 形状，保证色弱可辨（WCAG AA）。
- **Do** 把动效控制在 150–250ms，且只传达状态；尊重 `prefers-reduced-motion`，提供淡入或瞬切降级。

### Don't:
- **Don't** 做成千篇一律的 SaaS 仪表盘：禁止渐变大数字、hero-metric 模板、无差别的图标+标题+文字卡片网格。
- **Don't** 做成臃肿的企业级后台：禁止工具栏塞满、层级混乱、信息无优先级。
- **Don't** 做成玩具感消费应用：卡片圆角封顶 14px（`rounded-xl`），禁止 24/28/32px+ 的过度圆角与气泡化。
- **Don't** 露出 AI 生成的通用感：不套通用模板，个性只走主题/accent。
- **Don't** 用 `border-left/right > 1px` 的彩色侧边条作为卡片/列表/告警装饰（侧栏 active 的 3px 指示条是状态指示，例外且合法）。
- **Don't** 用渐变文字（`background-clip:text`）、把玻璃态当默认皮肤、或给密集列表铺满 `backdrop-filter`。
- **Don't** 用 fluid clamp 标题；产品 UI 用固定 rem 阶梯。
- **Don't** 用装饰性动效或不传达状态的 page-load 编排；调度台直接载入任务。
