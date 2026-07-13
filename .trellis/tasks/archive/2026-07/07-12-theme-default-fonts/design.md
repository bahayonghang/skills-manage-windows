# Technical Design

## Decisions

- Primary Font 默认与现有持久化行为保持不变。
- Chinese Fallback Font 使用标准 CSS 缺字 fallback，不做严格脚本路由。
- Display 与 Body 各自拥有 preset key 和 custom family。
- preset 固定为 System、Source Han Serif SC VF、Custom；默认 System。
- Source Han 仅查找用户系统已安装字体，不进入仓库或安装包。
- 设置界面范围为 shared Settings shell + Appearance 深改；其他页面只继承共享表面。
- 本任务保持单任务实施，因为 shell、section 与 Appearance 需要一次跨页视觉验收，其他页面没有独立业务交付。

## Terminology

- **Primary Font**：现有 display/body 预设或 custom family，位于字体链首位。
- **Chinese Fallback Font**：Display 或 Body 角色独立配置、用于补齐 Primary Font 缺失中文字形的后续字体。
- **System fallback**：现有 role-specific CSS 尾链；当 fallback key 为 System 或指定字体不可用时承担最终渲染。

Primary Font 与 Chinese Fallback Font 已同步到根 `CONTEXT.md`。

## Preference Model

```ts
type ChineseFallbackKey = "system" | "sourceHanSerif" | "custom";

interface FontPreferences {
  display: DisplayFontKey;
  displayCustom: string;
  displayChineseFallback: ChineseFallbackKey;
  displayChineseFallbackCustom: string;
  body: BodyFontKey;
  bodyCustom: string;
  bodyChineseFallback: ChineseFallbackKey;
  bodyChineseFallbackCustom: string;
  scale: number;
}
```

`DEFAULT_FONT_PREFERENCES` 保持 display=`geist`、body=`jetbrains`，两个 fallback key 为 `system`，两个 fallback custom 为空。

为四个新字段使用独立 settings key。`loadFontPreferences()` 继续并发读取；fallback key 使用白名单 coercion，缺失或无效值回落 System。保存某个角色时同时写 key/custom，但不清除未激活的 custom 值。

## Font Chain Construction

`displayFont.ts` 成为字体链唯一构造边界。组件只传偏好，不拼接 CSS。

```text
Primary Font
  + optional Chinese Fallback segment
  + existing role-specific system tail
```

- System：optional segment 为空。
- Source Han：追加 `"Source Han Serif SC VF", "思源宋体 VF"`，依赖浏览器系统字体查找。
- Custom：trim 后为空则按 System；非空值作为单一 family name安全引用，转义反斜线与引号，不把输入解释为任意 CSS stack。
- 指定字体不存在时，CSS 原生继续尝试后续 family；不弹错误、不运行字体安装或网络下载。

预设通过 `data-display-font` / `data-body-font` 选择 Primary，完整链最终落到 `--font-display` / `--font-body`。为避免 inline custom Primary 与 preset CSS 分成两套 fallback 逻辑，应用函数统一计算并写入最终变量；数据属性保留用于预览/测试和现有选择器兼容。

## Settings Shell

### Page hierarchy

- 删除外层 Settings Center eyebrow + Settings H1 条带。
- 保留当前 page 的 H2/description 作为唯一页面标题，并降低 icon 装饰：图标直接作为语义图标或移除外层 icon card。
- 内容列从 `max-w-7xl` 收敛到适合 label/value 扫描的宽度；复杂页面可通过已有内容自然扩展，不引入 fluid typography。

### Navigation

- `SettingsSideNav` 改为紧凑平面列表，图标不再嵌套边框方块，item 静止态无卡片 border/shadow。
- active 使用单一 muted/primary-tinted 背景、文字和 `aria-current`；accent 不作非激活装饰。
- 桌面保持左侧栏；低于 `lg` 时使用可横向滚动的紧凑 page tabs，保证至少 40px hit area且不堆成六行卡片。

### Shared section

- 将 `SettingsCollapsibleCard` 收敛/重命名为语义化 `SettingsSection`，以 unframed full-width section + hairline divider 取代 Card。
- 保留 collapse toggle、`aria-controls`、`aria-expanded`、hidden content 与 `settings.sectionCollapsed.v1` 行为。
- 删除 per-section theme gradient、icon card、glowing dot、decorative bar 和 `settingsSectionTheme` 依赖；section icon仅在帮助扫描时保留。
- 非 Appearance 调用点只做容器替换，不改 children、props 或事件流。

## Appearance Information Architecture

按照 Theme、Language、Typography、Density 排列单列设置组：

- Theme：Flavor 选择与 Accent swatches，保留现有立即应用行为。
- Language：中文/English 控件，保持现有语言切换行为。
- Typography：Display 与 Body 各为一个紧凑 role group，包含 Primary selector、Primary Custom、Chinese Fallback selector、Fallback Custom 和 `Skill 技能 Aa 0123` specimen。
- Density：保留三档 Font Scale，使用紧凑 segmented control和实时字号 specimen。

移除实验室 hero、模拟窗口、summary chips、metrics、装饰 grid/gradient 和多层 ring container。分组之间使用 divider 与 spacing 建立层级，不把每组做成卡片。

## Interaction And Responsive Rules

- 菜单/选择器使用标准可访问 option control；Custom 输入渐进显示，不使用 modal。
- 所有交互提供 visible focus、disabled state、40px 最小 hit area与精确 150-250ms transition。
- press feedback 遵守现有双轨：共享 Button 下沉，手写 tile/swatch 使用 `active:scale-[0.96]`。
- headings 使用固定 rem 与 `text-balance`，短说明用 `text-pretty`；动态百分比使用 tabular numbers。
- 900x600 时 label/value 可换成上下布局，但控件不可溢出或被 sticky nav遮挡。

## Packaging And Compatibility

- 不创建 `@font-face`、`src/assets/fonts/` 或 Tauri resources，不改变 CSP。
- 不需要字体转换依赖，`package.json` 与安装包体积不因字体变化。
- 新 settings key 可被旧版本忽略；回滚代码后无数据库修复。
- Source Han preset 在未安装机器上属于 best-effort，系统 fallback 是设计内行为。

## Rejected Alternatives

- 严格 Unicode script routing：复杂度与用户所需 fallback 语义不匹配。
- Display/Body 共享 fallback：无法独立搭配标题与正文。
- 默认 Source Han：会改变升级用户默认中文视觉。
- 捆绑 Source Han：原始可变字体约 59.9 MB，安装包代价不符合用户选择。
- Appearance-only：无法解决共享导航和重复页面标题。
- 全量重做所有 Settings 业务页：范围过大，且与本任务没有共同业务验收。

## Rollback

- 回滚新字段、字体链构造、Settings shell/section 和 Appearance 结构即可；新 settings key 被忽略。
- 无字体资产、数据库迁移、Rust 或 Tauri bundle 变更需要回滚。
