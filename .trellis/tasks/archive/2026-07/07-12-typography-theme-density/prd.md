# 优化字体设置的主题标识与紧凑排版

## Goal

让用户在 Appearance 的字体设置区能够明确判断当前字体配置与亮色/暗色主题的关系，并在不牺牲可读性、可访问性和响应式行为的前提下压缩无效留白，提高 Display 与 Body 字体设置的扫描和操作效率。

## Background

- 用户提供的 2026-07-12 截图显示：Typography 区在宽屏下由 section 说明列、字体角色标签列和控件列组成三段横向层级，主要控件集中在右侧，左侧与中间形成大面积无效空白；Display 与 Body 两组纵向重复，页面高度较长。
- `AppearanceSettingsSection.tsx` 当前由 `SettingGroup` 统一建立 `0.34fr / 0.66fr` 两列 section 布局；`FontRoleControl` 又建立 `0.3fr / 0.7fr` 的 label/value 子网格，因此 Typography 在宽屏下形成嵌套列。
- 当前字体偏好由 `src/lib/displayFont.ts` 管理，Display、Body、两组 Chinese Fallback 与字号各只有一套全局持久化值；主题 flavor 由 `themeStore` 独立管理。现有实现没有亮色/暗色各自一套字体偏好。
- `src/stores/themeStore.ts` 定义六个 flavor；按现有 CSS 与系统主题行为，`latte`、`claude-light` 属于亮色，`mocha`、`macchiato`、`frappe`、`claude-dark` 属于暗色。
- 既有 `.trellis/spec/frontend/settings-structure.md` 要求 Appearance 保持 Theme、Language、Typography、Density 分组，不新增第二套视觉状态模型，选择控件最小 hit area 为 40px，并在 900x600、1200x800、1440x900 检查视觉结果。
- 上一任务 `07-12-theme-default-fonts` 曾明确排除 Light/Dark 双主题字体配置；用户本次选择为亮色与暗色分别保存并应用独立字体偏好，因此本任务明确替代该边界。

## Requirements

### Theme semantics

- 字体设置区必须以中英文可见文案明确表达当前配置与亮色/暗色主题的关系，不能只依赖颜色、图标或用户从页面背景自行推断。
- 主题标识必须具有可访问名称，并与当前主题状态保持一致；切换 Theme flavor 后，标识同步更新。
- 亮色与暗色分别保存并应用独立的 Display、Display Chinese Fallback、Body、Body Chinese Fallback 及对应 Custom family；同一明暗类别内的 flavor 共享该类别的字体偏好。
- `latte`、`claude-light` 使用亮色字体偏好；`mocha`、`macchiato`、`frappe`、`claude-dark` 使用暗色字体偏好。
- Theme flavor 跨明暗类别切换时立即加载并应用目标类别字体；同类别 flavor 切换不得重置或复制字体偏好。
- Font Scale 继续作为 Density 分组中的全局偏好，不按主题明暗拆分。
- Typography 提供独立的“亮色 / 暗色”分段切换，默认选中当前应用主题类别；切换字体编辑类别不得改变 Theme flavor 或整个应用主题。
- 分段切换必须以可见文字标出当前应用正在使用的主题类别。编辑当前类别时字体即时应用到界面；编辑非当前类别时只更新该类别配置与 specimen，不改变当前界面字体。
- Theme flavor 后续发生明暗类别切换时，应用立即使用目标类别已保存字体；Typography 的编辑类别保持稳定，避免用户正在编辑的表单上下文被强制跳走。

### Compact layout

- 优先消除 Typography 内部嵌套网格造成的横向空洞，使标题字体、正文、中文 fallback、自定义输入和 specimen 形成更短、更直接的扫描路径。
- Display 与 Body 必须仍是两个清晰可辨的字体角色；Primary、Chinese Fallback、按需 Custom 输入和 specimen 的字段语义与操作顺序保持完整。
- 在桌面宽度下减少 Typography 的纵向高度和无效横向空白；在窄窗口下允许标签与控件换行或堆叠，不得通过缩小点击区域换取密度。
- 不引入装饰卡片、嵌套卡片、渐变、阴影堆叠或仅用于填充空白的元素；继续使用现有 divider、控件和间距语言。

### Compatibility and quality

- 扩展字体偏好模型和 setting keys 以保存亮色/暗色两套字体；保持字体链构造、单个 family 安全序列化和 store/API 分层边界不变。
- 升级后首次读取时，将旧 `*_v1` 全局 Display/Body 与 Chinese Fallback 偏好作为亮色、暗色两套配置的共同初始值；迁移前后当前界面的计算字体保持一致。
- 迁移完成后亮色与暗色使用各自的新 setting keys，编辑一套不得覆盖另一套；旧 key 保留用于兼容读取，不要求删除或重写。
- 新主题专用 key 缺失、无效或读取失败时，优先回退旧全局偏好；旧偏好也不可用时才使用现有 `DEFAULT_FONT_PREFERENCES`。
- 所有新增用户可见文案同步更新 `src/i18n/locales/zh.json` 与 `src/i18n/locales/en.json`。
- 保持现有字体选择、Custom 渐进显示、Chinese Fallback 独立保存、specimen 和字号行为，不改变 Theme、Language、Density 或其他 Settings 页面业务逻辑。
- 控件保持键盘可操作、可见 focus、可访问名称及至少 40px hit area；文案不得在支持视口中溢出或遮挡。

## Acceptance Criteria

- [ ] Typography 区以可见的中英文标签明确说明当前正在编辑亮色或暗色字体偏好。
- [ ] 在当前支持的六个 Theme flavor 间切换时，主题明暗标识正确且无需刷新页面。
- [ ] “亮色 / 暗色”分段切换默认指向当前类别，切换编辑类别不调用 Theme flavor 切换；当前类别以文字而非仅颜色标识。
- [ ] Display 与 Body 两组仍可独立设置 Primary Font、Chinese Fallback 与按需 Custom 字体，并显示对应 specimen。
- [ ] 亮色与暗色各自保存完整的 Display/Body 字体偏好；同类别 flavor 共享，跨类别切换立即应用对应偏好且互不覆盖。
- [ ] 编辑当前类别时应用字体实时更新；编辑非当前类别时当前应用字体保持不变，但该类别 specimen 与持久化值同步更新。
- [ ] 仅存在旧全局字体 key 的用户升级后，亮色与暗色均继承旧值，首次跨主题切换不发生字体突变；后续可分别修改。
- [ ] Font Scale 在所有 flavor 间继续全局共享。
- [ ] 相比用户截图，Typography 在 1440x900 宽屏下不再出现由嵌套 label/value 网格造成的大面积中间空白，主要控件与其标签保持邻近。
- [ ] 900x600、1200x800、1440x900 和 440x900 下无水平不可达内容、文本溢出、控件重叠或标签与字段归属不清。
- [ ] 所有字体控件 hit area 不小于 40px，键盘、focus-visible 与可访问名称保持有效。
- [ ] 中英文 i18n 文案覆盖主题标识及必要的辅助说明。
- [ ] `SettingsView.test.tsx` 覆盖主题标识、跨明暗与同明暗主题切换、紧凑结构中的现有字体交互和窄屏可访问语义；`displayFont.test.ts` 覆盖两套配置读写、隔离、应用与旧 key 兼容。
- [ ] 定向 Vitest、`pnpm typecheck`、`pnpm lint` 与 `just ci` 通过；若修改打包链路则额外执行 `pnpm tauri build`，本任务默认不涉及打包链路。

## Out of Scope

- 修改字体预设列表、默认字体、字体链顺序或 Chinese Fallback 算法。
- 引入、下载或打包新字体文件。
- 重做 Theme、Language、Density 或其他 Settings 页面。
- 修改主题色、accent 色板或新增 Theme flavor。
- 为六个 flavor 分别保存字体，或把 Font Scale 拆成亮色/暗色两套。
- 为减少空白而降低可读性、点击区域或响应式支持范围。
