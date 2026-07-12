# 增加中英文 fallback 并优化设置界面

## Goal

保留 SkillPort 现有默认字体与安装包体积，为 Display 和 Body 字体角色增加独立、可配置的 Chinese Fallback Font；同时参考 Codex 设置界面的信息架构，降低共享 Settings shell 与 Appearance 页的视觉噪声。

## Background

- 当前 display/body 默认值分别是 Geist 与 JetBrains Mono，并通过 `src/lib/displayFont.ts:68-84` 独立持久化；custom family 目前只能保存一个字体名。
- 当前 custom font 会展开为 `Primary Font -> 固定系统 fallback`（`src/lib/displayFont.ts:102-141`），没有用户可配置的中文补字层。
- 全局正文经 `html -> font-sans -> --font-body` 生效（`src/index.css:889-897`），标题经 `.font-display` 生效（`src/index.css:1315-1320`）。
- 当前 Settings shell 存在重复标题层级和卡片化导航；Appearance 内含实验室 hero、装饰网格、模拟窗口、状态 chip、metrics 和多层 ControlGroup，形成 card-in-card。
- 用户确认采用标准 CSS 缺字 fallback、Display/Body 独立配置、preset + Custom 控件，以及“shared Settings shell + Appearance 深改”的范围。
- `Source Han Serif SC VF` 只作为本机字体预设，不复制进仓库或安装包；未安装时继续使用系统 fallback。默认 Chinese Fallback Font 为 System，因此升级后默认视觉和安装包体积不变。

## Requirements

### Font behavior

- 保持现有 Primary Font 默认值、预设顺序、custom family 和持久化 key 的语义不变。
- Display 与 Body 各自拥有独立 Chinese Fallback Font，字体链分别为 `Primary -> Chinese Fallback -> role-specific system fallbacks`。
- 使用标准 CSS 缺字 fallback：只有 Primary Font 缺少某个 glyph 时才尝试 Chinese Fallback Font；不实现 Unicode-range 强制脚本路由。
- 每个角色保存 fallback preset key 与 custom family。预设固定为 System、Source Han Serif SC VF、Custom。
- System 不追加项目指定字体；Source Han 通过系统字体 family name 查找；Custom 只接受一个 family name并安全序列化到 CSS。
- 只有选择 Custom 时才显示并应用 custom family；切回预设后保留但不应用输入值。
- 新 fallback key 缺失、无效或读取失败时默认 System，不覆盖或迁移旧 Primary Font 设置。
- Source Han 或 Custom 字体不存在、缺字或加载失败时继续使用系统 fallback，不阻塞应用启动。
- 不向仓库、Vite 产物或 Tauri 安装包加入任何字体二进制。

### Settings UI

- 参考 Codex 的产品型设置布局：扁平导航、单一页面标题层级、紧凑设置组、稳定 label/value 行和克制分隔。
- 重构共享 Settings shell、侧栏和 section 表面；其他设置页只继承共享外观，不改变业务字段、操作顺序、数据流或折叠语义。
- Appearance 按 Theme、Language、Typography、Density 分组，保留 SkillPort 的六套 flavor、accent、语言、Display/Body 与字号能力，不复制 Codex 的 Light/Dark 双配置模型。
- 移除 Appearance 的装饰性 hero、grid、模拟窗口、status chips、metrics 和嵌套卡片，改为紧凑中英文混排 specimen。
- Display 与 Body 各自显示 Primary Font、Chinese Fallback Font、按需展开的 Custom 输入和实际完整字体链 specimen。
- 所有控件完整支持键盘、focus-visible、hover、active、disabled 与可访问名称；交互 hit area 至少 40px，transition 只声明实际变化属性。
- 所有新增用户可见文案同步更新中英文 i18n。

### Compatibility

- 现有 settings 路由、active page、collapse localStorage 与非 Appearance 页面行为保持兼容。
- 最小支持窗口 900x600 下不出现文本溢出、控件重叠或不可达内容；1200x800 与宽屏保持紧凑扫描路径。
- 不增加新的运行时依赖或字体转换工具。

## Acceptance Criteria

- [x] 无新设置时，Primary Font 与 Chinese Fallback Font 分别保持现有默认值和 System 默认值，页面计算字体与当前版本一致。
- [x] 已有 display/body 预设和 custom family 在升级后保持不变。
- [x] Display/Body fallback 的 preset/custom 值可独立编辑、保存、加载、清空和预览，互不覆盖。
- [x] System、Source Han Serif SC VF、Custom 三个选项均能构造正确字体链；空或无效 Custom 回落 System。
- [x] Primary Font 缺字时浏览器尝试 Chinese Fallback Font；Primary 已含字形时允许继续使用 Primary。
- [x] 未安装 Source Han 或 Custom 字体时应用仍可用，并继续落到系统字体。
- [x] 仓库、`dist/` 与安装配置没有新增字体二进制或字体资源声明。
- [x] Settings 只保留一个页面主标题；侧栏为扁平导航，不再使用导航卡片或内层图标卡。
- [x] 共享 section 无主题装饰渐变、发光圆点或装饰短横线，并保留折叠、ARIA 与 localStorage 行为。
- [x] Appearance 不存在 card-in-card、装饰网格或重复摘要面板；Theme、Language、Typography、Density 均可快速扫描。
- [x] 非 Appearance 设置页的业务控件与自动化行为测试保持不变。
- [x] 中英文 i18n、字体偏好测试、字体契约测试和 SettingsView 交互测试覆盖新增行为。
- [ ] 900x600、1200x800、1440x900 截图验证无溢出、重叠、裁切或失焦，且结构方向与 Codex 参考一致但不做像素复刻。
- [x] `pnpm build` 与 `just ci` 通过。

## Out of Scope

- 改变 Geist、JetBrains Mono、Inter、Instrument Serif 或 System 的现有 Primary Font 默认排序。
- 强制按 Unicode 脚本切换字体或提供可任意排序的 font stack 编辑器。
- 捆绑、转换、下载或安装 Source Han、SF Mono 或其他字体文件。
- 重排 Connections、Platforms、Integrations、Skill Sources、About 的业务表单或工作流。
- 复制 Codex 的 Light/Dark 双主题配置、颜色模型或应用级导航。
- 调整主题颜色、字号比例或布局密度选项本身。

## Planning State

- 访谈问题已解决，PRD 已完成 convergence pass；用户已批准实施，任务状态为 `in_progress`。
- 2026-07-12：自动化视觉工具无法确认本地 URL allowlist，900x600、1200x800、1440x900 截图项保留为未验证；用户在知悉该限制后批准提交与归档。
