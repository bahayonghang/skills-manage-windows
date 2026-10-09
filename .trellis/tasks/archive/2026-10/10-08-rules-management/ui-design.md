# Rules 界面设计

视觉主张：沿用 SkillPort 的紧凑工具界面，以文件列表和正文编辑区为主要内容，用现有主题层级区分区域。

内容结构：标题与导入动作 → 搜索与规则列表 → 正文/预览 → Claude Code 和 OMP 的链接状态及操作。

交互主张：搜索、规则切换和键盘保存没有入场动画；按钮沿用按压反馈；导入、接管和删除使用现有 Dialog。

## 1. 视觉主题

用户在桌面应用中维护日常指令。首屏显示名称、正文、保存状态和两个工具状态。正文是视觉中心，不增加介绍区域、统计大数或装饰背景。

## 2. 颜色角色

仅使用现有语义 token。下面是 src/index.css 中 Mocha 的当前值；组件继承主题切换，不硬编码色值。

| Token | Mocha 当前值 | 作用 |
| --- | --- | --- |
| background | #1e1e2e | 页面底面 |
| card / sidebar | #181825 | 列表、正文容器 |
| foreground | #cdd6f4 | 主要文字 |
| muted-foreground | #a6adc8 | 文件名和路径 |
| border | #313244 | 1px 分隔线 |
| primary / primary-text | #b4befe | 保存、选中和焦点 |
| destructive-text | 当前主题 ctp-red | 失败文字 |
| success / warning | 当前主题语义值 | 链接正常和冲突，配文字及图标 |

侧栏高亮复用既有 NavItem，不把侧条或渐变扩散到新页面。

## 3. 排版

标题继承 font-heading / --font-display，正文继承 font-sans / --font-body，编辑器用既有 font-mono。当前默认标题为 Geist，正文为 JetBrains Mono；尊重用户保存的 Primary Font 和 Chinese Fallback Font。

正文/操作用 text-sm，标签/状态用 text-xs，路径用 text-ui-meta，冗余数字用 text-ui-micro。无任意字号、局部缩小或新增字体包。正文保持原始换行，预览采用现有 Markdown 行高。

## 4. 组件与状态

- 导航复用 NavItem 和 Lucide FileText，Rules 与 Central Skills 同级。
- 规则是平面行列表：标题、次要文件名、两工具短状态；选中用背景和字重。
- 正文用现有 Textarea。顶部显示文件名、未保存状态、保存按钮；正文/预览切换在编辑区内。
- 源文件查看为只读完整内容，含 YAML 头部。正文预览不显示 SKILL.md 标签。
- Button 复用 default/ghost/outline/destructive、rounded-lg、disabled 和 focus-visible。手写控件只列必要 transition，指针按压 scale(0.96)，键盘不触发切换动画，遵循 reduced-motion。
- Dialog 只处理导入、接管、删除及切换规则的未保存确认；普通内容留在页面内。

## 5. 布局

```text
现有全局侧栏 | Rules                   导入现有规则  新建
             | 搜索          | 文件名  未保存        保存
             | 规则行        | 正文 / 预览
             | 规则行        | 可编辑的规则正文
             | ...           | --------------------------
             |               | Claude Code 已链接    停用
             |               | OMP         已链接    停用
```

全局侧栏沿用默认 208px、可调 168–360px、收起 56px。内列表默认 224px，窄桌面 176px；正文 min-width:0 占剩余空间。CSS 策略为 Tailwind v4 静态 utility 和既有 CSS token，不加入 CSS Modules 或 CSS-in-JS。

列表和正文分别滚动，操作栏保持可达。两工具行自然换行，不能覆盖正文或按钮。长文件名自然换行，完整路径可局部滚动和复制，不用省略号承载关键位置。

## 6. 层级与圆角

用 background/card 颜色差与 1px 分隔线，不使用阴影卡片网格。沿用 --radius=0.625rem，sm/md/lg/xl 分别为 0.6/0.8/1/1.4 倍，不建立新圆角系统。

## 7. 专项约束

- 状态同时有文字和图标，不能只靠颜色。
- 导入中央库后不自动接管工具；两个工具独立启停。
- 保存、工具启停和恢复三个动作有不同标签。
- 所有用户可见文本经 i18n，规则原文无需翻译。
- 热区沿用项目约定，可见键盘焦点与状态宣布完整。
- 不新增字体、图标、编辑器或动画依赖，不调整全局主题。

## 8. 尺寸与可访问性

原生支持最小 900×600、常用 1200×800；同时验 900×800 和 1200×600。中英文、最大字体 scale、导航展开/收起均检查。

空间不足时缩窄内列表并让工具行自然换行。浏览器低于原生最小宽度可用单列列表/详情加返回按钮，不承诺移动端产品。检验实际布局边界两侧。

焦点顺序为主导航、页面操作、搜索、列表、正文、工具操作。异步结果经可访问状态区宣布，规则切换确认含保存/放弃/取消。状态至少 text-xs，小字沿用现有 4.5:1 对比度契约。

## 9. 实施提示

- 规则列表：bg-card、text-foreground、1px border-border 分隔，224px/176px；标题 text-sm font-medium，文件名 text-ui-meta。继承字体，不写 hex。
- 编辑区：bg-background，标题 font-heading text-lg font-semibold；Textarea 为 font-mono text-sm，保存 Button 用既有 rounded-lg、bg-primary/text-primary-foreground。
- 工具行：工具名 text-sm font-medium，状态 text-xs，路径 text-ui-meta font-mono，失败 text-destructive-text；两个工具保持同一排列。

生产内容来自扫描及中央库，不内置个人规则正文。浏览器 fixture 标明验收数据，空库提供导入入口。

## 规划与实施证据

规划时核对源码 token、布局和组件，视觉与原生验收为 NOT_RUN。实施后复用现有 tokens 与 compact Markdown renderer，保持平面文件列表和正文编辑结构。

浏览器渲染已通过：中英文各 1200×800、900×800、1200×600、900×600；最大字体 preset 1.125、scale 1.5；719/721 和 1099/1101 布局边界；导航展开/收起；长文件名、空态、冲突、恢复、权限错误与确认。主会话查看全部 8 张基础截图，并检查预览、源文件和关键状态截图。截图在 `tmp/rules-ui`，探针和逐层证据见 [validation/results.md](validation/results.md)。

首轮 scale 1.5 工具区裁切已修复，详情内部滚动，操作可达性断言通过；原始截图保留。页面使用合成 fixtures。原生 Tauri WebView/IPC 为 UNVERIFIED，安装流程为 NOT_RUN。

采用九节 DESIGN.md 文档结构，实际文件名为 ui-design.md。Windows 文件名不区分大小写，不能与技术 design.md 同时使用 DESIGN.md。
