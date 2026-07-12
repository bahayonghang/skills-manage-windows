# Technical Design

## Decisions

- 亮色与暗色各保存一套完整的 Display/Body 字体 profile；同明暗类别的 flavor 共享 profile。
- `latte`、`claude-light` 映射到 `light`；`mocha`、`macchiato`、`frappe`、`claude-dark` 映射到 `dark`。
- Font Scale 保持全局 `font_scale_v1`，不进入主题 profile。
- 旧全局字体值同时迁移为亮色与暗色初始值，保证升级后视觉不突变。
- Typography 使用独立的亮/暗分段编辑器；它不切换应用 Theme。当前类别编辑实时应用，非当前类别只更新缓存、持久化和 specimen。
- 任务保持一个实施单元：偏好模型、全局主题切换和 Typography UI 共同组成一个不可分割的用户行为闭环。

## Terminology

- **Theme flavor**：现有六个具体界面主题，例如 `latte`、`mocha`。
- **Font theme mode**：字体偏好的两个共享桶，固定为 `light`、`dark`。
- **Active mode**：当前应用 Theme flavor 对应、实际应用到 DOM 的字体模式。
- **Editor mode**：Typography 当前正在编辑的字体模式，可与 active mode 不同。
- **Font profile**：Display、Body、两组 Chinese Fallback 及 Custom family，不含 Font Scale。

## Preference Model

```ts
type FontThemeMode = "light" | "dark";

interface FontProfile {
  display: DisplayFontKey;
  displayCustom: string;
  displayChineseFallback: ChineseFallbackKey;
  displayChineseFallbackCustom: string;
  body: BodyFontKey;
  bodyCustom: string;
  bodyChineseFallback: ChineseFallbackKey;
  bodyChineseFallbackCustom: string;
}

interface ThemedFontPreferences {
  light: FontProfile;
  dark: FontProfile;
  scale: number;
}
```

现有默认值继续作为两个 profile 的共同默认：Display=`geist`、Body=`jetbrains`、两组 Chinese Fallback=`system`、Custom 为空。字体链构造与安全 family quoting 仍由 `src/lib/displayFont.ts` 统一负责，组件不得拼接 CSS font stack。

`src/stores/themeStore.ts` 导出纯函数 `fontThemeModeForFlavor(flavor)`，成为 flavor 到 mode 的唯一分类边界。CSS、组件和测试不得各自维护另一份亮/暗列表。

## Storage And Migration

每个 mode 使用独立 v2 key，Font Scale 继续使用旧 key：

| Field | Light | Dark |
| --- | --- | --- |
| Display | `display_font_light_v2` | `display_font_dark_v2` |
| Display Custom | `display_font_custom_light_v2` | `display_font_custom_dark_v2` |
| Display Chinese Fallback | `display_chinese_fallback_light_v2` | `display_chinese_fallback_dark_v2` |
| Display Fallback Custom | `display_chinese_fallback_custom_light_v2` | `display_chinese_fallback_custom_dark_v2` |
| Body | `body_font_light_v2` | `body_font_dark_v2` |
| Body Custom | `body_font_custom_light_v2` | `body_font_custom_dark_v2` |
| Body Chinese Fallback | `body_chinese_fallback_light_v2` | `body_chinese_fallback_dark_v2` |
| Body Fallback Custom | `body_chinese_fallback_custom_light_v2` | `body_chinese_fallback_custom_dark_v2` |

加载每个字段时使用固定优先级：有效 mode-specific v2 值 -> 有效旧全局 v1 值 -> 现有默认值。Custom 字符串允许空值；preset key 继续经过白名单 coercion。

首次发现 v2 key 缺失或无效时，将解析后的完整 profile best-effort 写回该 mode 的 v2 keys。两套 mode 都以同一份旧 profile 为迁移来源，旧 v1 keys 保留不删，便于回滚和旧版本读取。单个写入失败不阻塞启动；下次加载仍按相同优先级恢复。

## Runtime Data Flow

`displayFont.ts` 维护进程内的两套 profile 缓存与 active mode，并对加载 Promise 做复用，避免 `main.tsx` 与 Settings 同时挂载时重复迁移。

```text
startup
  -> themeStore.init()
  -> derive active mode
  -> synchronously apply default profile + global scale
  -> load/migrate light + dark profiles
  -> refresh cache
  -> re-read current flavor and apply its profile

theme flavor changes anywhere
  -> main.tsx theme-store subscription
  -> derive target mode
  -> activate cached target profile
  -> same-mode flavor changes are a font no-op

Typography edit
  -> update editor-mode profile in component + shared cache
  -> persist only that mode's keys
  -> if editor mode == active mode, apply to DOM
  -> otherwise update only that mode's specimen
```

异步加载完成时必须重新读取当前 flavor，不能使用启动时捕获的旧 mode；快速切换主题时最终 DOM 必须与最后一个 flavor 一致。主题订阅位于应用启动集成层，保证用户在 Settings 之外切换主题时字体同样更新。

供 specimen 使用的 Display/Body 字体链解析函数从 `displayFont.ts` 导出，并与实际 DOM apply 复用同一逻辑。非当前 mode 的 specimen 使用显式 `fontFamily` 预览，但该值只能来自安全解析函数。

## Typography Layout

Appearance 继续保持 Theme、Language、Typography、Density 的扁平 section 顺序，不增加卡片层级。

- `SettingGroup` 的说明栏从比例列收敛为约 10-12rem 的固定轨道，内容列占剩余空间，间距使用现有 4pt/Tailwind 阶梯。
- Typography 内容顶部放置亮色/暗色分段编辑器；active mode 直接写入对应按钮文字，例如“亮色（当前）”，不能只靠 accent 或圆点表达。
- Display 与 Body role group 在内容宽度足够时并排，在窄内容区结构性堆叠；使用 gap 与 1px divider 分组，不包裹装饰卡片。
- 每个 role 内 Primary 与 Chinese Fallback 形成紧凑字段网格；Custom 输入只在对应 selector 选中 Custom 时出现，并留在所属字段下方。
- specimen 紧邻所属 role，使用当前 editor profile 的真实解析字体链；不增加重复说明、metrics 或装饰状态 chip。
- 900x600、440x900 与 1200x800 使用上下 role；1200px 的 role 内字段可并排。1440x900 在实际内容宽度允许时使用双 role 列，避免截图中的三段嵌套网格和中间空洞。

## Interaction And Accessibility

- 亮/暗编辑器复用现有 segmented button 语言，按钮使用 `aria-pressed`、可见 focus 和至少 40px hit area。
- 初次挂载时 editor mode 等于 active mode；用户手动切换后保持选择稳定。应用主题变化只更新“当前”文字和实际字体，不强制跳走正在编辑的 mode。
- 选择 editor mode 不调用 `onSetFlavor`；Theme flavor 只由 Theme 分组控制。
- 当前 mode 的每次选择或输入继续即时预览；非当前 mode 的更改不得改写根节点 `--font-display`、`--font-body` 或字体 data attributes。
- 所有新增状态文案走中英文 i18n；“当前”必须是可见文字，动态变更保留正确可访问名称。
- 不新增装饰动效；只保留 150-250ms 的现有状态 transition，并尊重 reduced motion 基线。

## Compatibility And Boundaries

- 不修改字体 preset、字体链顺序、Source Han 查找方式、Custom 安全序列化或 Font Scale 范围。
- 不新增字体资产、运行时依赖、Rust command、数据库 schema 或 Tauri resource。
- 现有 `get_setting` / `set_setting` IPC 继续承载字符串 key/value；仅增加 key 数量。
- 需要同步更新 `.trellis/spec/frontend/font-preferences.md` 与 `settings-structure.md`，明确新任务替代旧的“单套字体”边界。

## Rejected Alternatives

- 只显示当前 mode：用户必须切换整个应用主题才能编辑另一套，违背明确标注和高效设置目标。
- 同时展开两套完整表单：控件数量翻倍，重新制造纵向长度与扫描噪声。
- 六个 flavor 各自保存字体：超出用户要求，并增加不必要的配置重复。
- 把 Font Scale 放进 profile：Density 是独立全局语义，拆分会扩大迁移和认知负担。
- 在组件中监听 Theme 并应用字体：Settings 未挂载时失效，无法保证全局行为。
- specimen 自行拼接字体链：会复制安全敏感逻辑并产生预览与实际应用漂移。

## Rollback

- 回滚 themed model、启动订阅和 Typography segmented editor 后，旧 v1 keys 仍在，可恢复现有单套字体行为。
- 新 v2 keys 可被旧代码忽略，不需要数据库清理。
- 无字体资产、Rust、数据库 schema 或安装包资源需要回滚。
