# 优化技能详情页信息样式

## Goal

优化技能详情页右侧检查器的视觉层级和语义表达，让用户优先看清安装与更新状态，并能快速区分文件类型、来源元数据和归类信息，同时保持 SkillPort 高密度、键盘优先的“调度台”风格。

## Background

- `src/components/skill/SkillDetailSidebar.tsx:327-670` 依次渲染 Metadata、File tree、Update Status、Tags、Classification Management、Installation Status、Projects 和 Collections；核心安装状态位于次要信息之后。
- `src/components/skill/SkillDetailFileTree.tsx:24-67` 默认展开所有顶层目录，目录展开与路径打开被拆成两个按钮；普通文件、目录和符号链接目前均使用弱化中性色，没有扩展名级语义。
- `src/components/skill/SkillDetailViewShared.tsx:13-34` 的 section/field 标签和文件树标题使用 `text-[11px]` 及 `/70`、`/80` 透明度。Impeccable detector 在目标组件中报告 10 个 `design-system-font-size` advisory；独立视觉评估确认这些关键标签在部分主题/表面组合下低于 4.5:1。
- 平台安装按钮在 `src/components/skill/SkillDetailViewShared.tsx:112-131` 主要依赖 accent 与透明度区分 installed、uninstalled 和 locked，未暴露 `aria-pressed`。
- 产品已有 6 套主题、14 种 accent，以及 `primary-text`、`success`、`warning`、`info`、`destructive` 等跨主题 token；本任务应复用这些能力，不引入硬编码单主题色板。

## Requirements

### R1 Operational hierarchy

- 右侧栏首先呈现来源例外、安装状态和更新状态，再呈现归类/项目/集合、文件树和技术元数据。
- 保留所有现有信息、动作、权限判断和数据流；只调整展示顺序、分组与视觉权重。
- 核心状态使用克制的语义色和图标，次要元数据保持中性，避免所有区块都成为同权重卡片。

### R2 File tree semantics

- 为目录、符号链接、Markdown/文本、JSON/YAML/TOML、TypeScript/JavaScript、Python、Rust、图片、配置/环境文件、测试文件和未知文件提供稳定的图标与颜色/样式映射。
- 文件名保持主前景色，类别色主要落在图标或小型非文本线索上；类别同时通过图标形状表达，不能只靠颜色。
- 顶层目录默认折叠；展开按钮包含目录名和 `aria-expanded`，文件/目录打开动作名称唯一且有清晰 focus 状态。
- 保持缩进、长文件名换行、符号链接和路径打开行为；未知扩展名使用中性 fallback。

### R3 Metadata hierarchy

- 明确区分本地目录、GitHub 仓库、仓库内路径和折叠的技术详情，使用图标与间距建立层级，不使用装饰性大面积着色。
- 路径类值保持等宽字体、可换行，并保留复制、打开目录和打开仓库动作。
- 减少连续的重复边框卡片；仅为需要独立状态/动作的更新与归类控件保留 contained surface。

### R4 Status and management states

- Update Status 覆盖未检查、检查中、已最新、有更新、不支持、远端缺失和错误状态；每种状态同时提供文字、图标/形状和语义色。
- Repository/Tag 控件保持字段-选择-提交顺序，并覆盖 hover、focus、disabled、updating 和错误反馈。
- 平台安装切换提供 `aria-pressed` 或等价状态语义，并用 check/link/lock 等非颜色线索区分 installed、uninstalled 和 always-included。
- Projects 与 Collections 的 loading、empty、populated、read-only 状态使用一致的信息层级，现有链接类型和集合标签语义不变。

### R5 Theme and accessibility

- 用户可见文案继续走现有中英文 i18n；不得新增图标库、主题框架或运行时依赖。
- 适配 Catppuccin Mocha / Macchiato / Frappe / Latte 与 Claude light / dark，并兼容 14 种 accent。
- 意义明确的小标签使用设计系统 label 尺度（`0.72rem` 或等价值）和不低于 4.5:1 的前景色；焦点态沿用 `--ring`。
- 避免嵌套卡片、过度圆角、装饰性渐变、重阴影和仅为“增加颜色”而增加的色块。

## Acceptance Criteria

- [ ] AC1：打开技能详情后，无需越过文件树即可看到安装与更新状态；所有原有区块、权限判断和动作仍可用。
- [ ] AC2：R2 列出的文件家族可通过图标形状和主题化颜色/样式辨认，未知类型有中性 fallback，顶层目录默认折叠，长 Windows/Unix 路径不溢出。
- [ ] AC3：目录展开控件具有唯一可访问名称和 `aria-expanded`；平台切换具有可访问的当前状态及非颜色 installed/locked 线索。
- [ ] AC4：Metadata 中本地、GitHub、仓库路径和技术详情可一次扫视区分，重复卡片纹理减少，复制/打开行为不回归。
- [ ] AC5：更新状态的全部后端枚举及 checking 过渡态均有文字、图标与语义色；归类、项目和集合的 loading/empty/disabled/error 状态可辨认。
- [ ] AC6：目标组件不再触发 `design-system-font-size` advisory；核心标签在 6 套主题下达到 WCAG 2.1 AA，状态含非颜色线索。
- [ ] AC7：相关 RTL/Vitest 用例、`pnpm typecheck`、`pnpm lint` 和最终 `just ci` 通过。
- [ ] AC8：在 Tauri 桌面运行时检查默认深色、Latte 和 Claude light；宽/窄详情布局无文本、按钮或文件树重叠。

## Out of Scope

- 不改变 Tauri command、store 数据模型、技能安装/更新/归类业务逻辑。
- 不重做 Markdown / Raw Source / AI Explanation 主内容区。
- 不同步重构 GitHub Import File Tree；本任务只在确认出现第二个相同分类需求后再抽共享 helper。
- 不加入文件树搜索、全局 collapse-all 或完整 ARIA tree roving-focus/方向键模型；本轮先修复唯一名称、展开状态和可见焦点。
