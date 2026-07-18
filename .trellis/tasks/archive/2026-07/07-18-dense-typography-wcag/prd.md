# 高密度排版 Token 与 WCAG 治理

## Goal

让 SkillPort 的高密度排版从散落的固定像素值收敛为可缩放、可测试的语义层级，在不降低默认信息密度、不重做现有页面结构的前提下，提高小字标签、元数据、状态和计数在六套主题中的可读性，并防止任意字号继续增长。

## Background

- 2026-07-18 planning 快照中，生产 `src/**/*.ts(x)` 有 173 处数值型 `text-[...]` arbitrary 字号、分布于 64 个文件：133 处 px（`10px` 23、`11px` 107、`12px` 2、`13px` 1）和 40 处 rem；其中 10–12px 合计 132 处，另有 1 处 13px。热点包括 GitHub 导入预览、Marketplace 详情、Central shell/sidebar、`UnifiedSkillCard` 及其 meta/badge、Settings 远程目标、Dashboard、Logs 与 Usage。
- 其中 22 处 px 小字同时使用 alpha 前景：21 处 `text-muted-foreground/*` / `text-foreground/*`，以及 1 处 `text-primary/85`。只匹配名称包含 `foreground` 的命令会得到 21；本任务按所有文字前景透明度风险计为 22。它们包含导航分组、导入/来源信息、远程路径和状态标签，不能把“弱化”直接当成满足 AA 的证据。
- `src/index.css:894-896` 通过根级 `font-size: calc(16px * var(--font-scale))` 实现 0.875 / 1 / 1.125 三档字号；固定 px utility 不随该根字号缩放，造成同一语义角色的缩放行为不一致。
- `src/test/themeContrast.test.ts` 已对六套主题、14 个 accent、核心前景/背景 token 建立静态对比度基础；`src/test/fontContract.test.ts` 已锁定根级 font scale。新治理应扩展这些测试，不引入第二套颜色或字体模型。
- Central 在列表超过 60、网格超过 40 时使用固定高度虚拟化（`CentralSkillListContent.tsx:21-35`、`:188-210`）。小字改为 `rem` 后，1.125 档会真实增高，必须验证绝对定位行高与卡片实高仍一致，避免以可读性修复换来重叠或截断。
- `.trellis/spec/frontend/font-preferences.md` 已规定字体 profile、中文 fallback、安全 family 序列化和全局 Scale。本任务只治理字号/前景角色，不改字体资产、preset、存储 key 或 fallback 算法。
- 2026-07-16 的 `07-16-skill-detail-visual-hierarchy` 已完成技能详情信息层级、标签与状态可访问性修复。本任务不得重排技能详情；若共享排版 token 触及该区域，只允许默认尺寸等价的 class 替换和回归验证。

## Requirements

### R1. 证据化排版清单

- 实施前在任务 `research/` 固化基线：任意 px 命中、语义角色、所在 surface、是否交互/决策相关、前景 token、缩放风险和迁移目标。
- 任务启动时必须重新运行 inventory，将 planning 快照与 task-start 快照并列记录；若前序 UI 子任务改变数量，以 task-start inventory 作为实施分母，但保留 2026-07-18 的 173/133/40/22 证据用于解释漂移。
- 每个现有命中归入以下角色之一：正文/操作、section label、secondary metadata、code/path/id、status/badge、numeric micro/axis、纯装饰 glyph。
- detector 与正则只负责发现漂移；最终判断必须结合文本是否承载用户决策、上下文是否已有等价可访问名称、真实背景和主题 token。

### R2. 语义字号与缩放契约

- 在现有 Tailwind 4 `@theme inline` / utility 边界定义最小必要的 `rem` 语义小字号 token；常规标签和正文优先复用现有 `text-xs` / `text-sm`，不为每个组件创建专用字号。
- 只新增 `ui-micro`（紧凑计数、坐标轴、冗余 glyph）和 `ui-meta`（次要 metadata、路径、ID、keycap）两个低于 `text-xs` 的角色；有意义的 section/status/action label 至少使用 `text-xs`。
- 生产 TS/TSX 中所有数值型 arbitrary 字号（planning 快照 173 处：133 px + 40 rem）全部迁移到语义 token、标准排版阶梯或有明确所有者的命名 component utility；不保留按文件/行号或按数值的 allowlist。确需保留 10/11px 默认密度或 Dashboard display 几何的场景也必须使用命名语义，不得继续写 `text-[0.65rem]`、`text-[2.35rem]` 等 arbitrary class。
- no-growth 守卫禁止生产 TS/TSX 中整个 `text-[...]` class 家族；当前没有需要保留的任意 text color class，未来颜色也必须通过 theme token/named utility 表达，不能借 arbitrary syntax 绕过字号契约。
- token 使用 `rem`，自然继承 0.875 / 1 / 1.125 根级 Scale；不得增加 viewport 驱动字号、运行时 DOM 扫描或组件内字号计算。
- 默认 Scale=1 时应保持 `ui-micro` / `ui-meta` 对应的现有几何尺寸；只有从“微型文本”提升为真正 label/body 的角色才允许有依据地增大。

### R3. 对比度与非颜色语义

- 所有承载标签、状态、路径、错误、来源、操作或用户决策的小字，在实际使用的 `background`、`card`、`popover`、`sidebar` 和状态 surface 上达到 WCAG 2.1 AA 普通文本 4.5:1。
- 对 22 处“小字号 + 透明前景色”逐项处理：有意义文本改用完整、已测的语义前景 token；只有确属冗余装饰且已有可访问等价信息时才可保留弱化，并记录理由。
- `primary` 继续用于填充/强调，普通 accent 文本使用已存在的 `primary-text`；success/warning/info/destructive 使用现有 semantic foreground，不新增硬编码组件色。
- 不能只靠颜色区分状态或交互；迁移不得移除现有文字、图标、`aria-*`、focus-visible、键盘行为或 40px 交互 hit area。

### R4. 分区迁移与兼容边界

- 按风险迁移：先 token/测试，再 Central + `UnifiedSkillCard`，GitHub import，app shell/remote settings，Marketplace/Projects/Usage，最后处理低风险例外。
- 共享技能卡继续只改 `UnifiedSkillCard` 及现有 meta/badge/footer 组成，不复制卡片实现；不因排版治理改变 store、IPC、路由、导入、安装、更新或虚拟化阈值语义。
- 技能详情只接受默认尺寸等价的共享 token 替换，不重排信息架构、不改文件分类和状态展示。
- 用户可见文案预计不变；若为可访问名称或错误说明新增文案，必须同步中英文 i18n。

### R5. 密度、性能与响应式

- 默认 Scale=1 的高密度视图不得因机械放大造成卡片高度、工具栏层级或单位屏信息量显著下降；必要提升仅限决策相关 label/status。
- 在 0.875 / 1 / 1.125 三档下验证 Central 非虚拟和虚拟列表/网格、GitHub import preview、Marketplace、Settings、Usage 的长中英文文本、长 Windows 路径、badge 和按钮，无重叠、遮挡、不可达内容或异常布局抖动。
- 对 >60 列表和 >40 网格 fixture 比较虚拟 item 固定高度与实际卡片边界；若 1.125 档产生截断/重叠，实施范围允许做最小的 scale-aware 高度修正，但不借机重写通用虚拟化组件。
- 新 token 和守卫只产生 CSS/测试期成本；不得增加生产依赖、运行时 observer、全局 resize listener 或扫描逻辑。虚拟化渲染数量与 overscan 保持现状。

## Acceptance Criteria

- [ ] 任务 research 同时记录 planning 快照（173 个数值型 arbitrary 字号、64 文件、133 px + 40 rem）和 task-start 快照/delta，并明确 22 处 alpha 前景风险（21 foreground + 1 primary）的处理结论；命令与数量可复现。
- [ ] `src/index.css` 提供最小语义小字号 token，使用 `rem` 并继续由根级 `--font-scale` 控制；现有字体 profile/fallback 契约不变。
- [ ] 生产 TS/TSX 中 `text-[...]` 命中降为 0；focused contract test 同时阻止 px、rem、em、calc/clamp 或 arbitrary color 等绕过，不依赖脆弱的行号/数值 allowlist。
- [ ] section/status/action label 不落入 `ui-micro`；micro 仅用于紧凑计数、图表轴或已有等价上下文的次要信息，code/path/id 使用清晰的 meta/code 角色。
- [ ] 22 处透明小字均完成审计；承载意义的文本使用在对应 surface 实测 >=4.5:1 的完整 foreground token。
- [ ] 扩展后的 contrast tests 覆盖六套主题、14 accent 和本任务实际使用的 background/card/popover/sidebar/semantic-state 组合；未测组合不写成通过。
- [ ] 三档 Scale 下 Central 的 >60 列表与 >40 网格没有虚拟行重叠、截断、空洞或异常渲染增长，非虚拟与虚拟布局保持一致语义。
- [ ] 900x600、1200x800、1440x900 下的 Central、GitHub import、Marketplace、Settings、Projects/Usage 无文本或控件重叠；至少覆盖长英文、中文、长路径和两类 body font profile。
- [ ] 默认 Scale=1 的代表性前后截图保持 SkillPort 调度台密度、6 主题和 14 accent 身份；不重做 2026-07-16 技能详情视觉层级。
- [ ] 定向 Vitest、`pnpm typecheck`、`pnpm lint`、`pnpm build`、`git diff --check` 和最终 `just ci` 通过。

## Out of Scope

- 全站视觉重设计、移动端产品化、修改导航/卡片/详情信息架构或复刻 SkillKit 视觉。
- 修改 Display/Body 字体 preset、字体文件、明暗 profile、fallback、Scale 选项或持久化 key。
- 新增 axe/Playwright/runtime typography 依赖；需要浏览器截图时复用现有开发与测试工具。
- 把所有 `text-xs` / `text-sm` 机械替换成新 utility，或抽象新的 Badge/Card/Text 组件只为减少 class 字符数。
- 通用动态高度虚拟器重写、虚拟化阈值调整或全应用性能遥测。
