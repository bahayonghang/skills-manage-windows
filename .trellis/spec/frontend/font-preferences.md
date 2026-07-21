# 字体偏好与中文 fallback 契约

> 建立于 2026-07-12（任务 07-12-theme-default-fonts），由任务 07-12-typography-theme-density 扩展为亮色/暗色双 profile。字体偏好统一由 `src/lib/displayFont.ts` 管理；Source Han 只查找系统字体，不进入仓库或安装包。

## 1. Scope / Trigger

修改 Display/Body 字体预设、自定义 family、中文 fallback、主题字体模式、字号或对应设置项时适用。组件不得自行拼接 CSS font stack，也不得直接调用 Tauri `invoke()`。

## 2. Signatures

```ts
type FontThemeMode = "light" | "dark";
type ChineseFallbackKey = "system" | "sourceHanSerif" | "custom";

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

loadThemedFontPreferences(): Promise<ThemedFontPreferences>;
applyThemedFontPreferences(prefs, activeMode): void;
activateFontTheme(mode): void;
saveDisplayFont(mode, key, custom): Promise<void>;
saveDisplayChineseFallback(mode, key, custom): Promise<void>;
saveBodyFont(mode, key, custom): Promise<void>;
saveBodyChineseFallback(mode, key, custom): Promise<void>;
```

`fontThemeModeForFlavor()` 是 flavor 到 mode 的唯一分类边界：`latte`、`claude-light` 为 `light`，其余四个 flavor 为 `dark`。

## 3. Contracts

| 字段 | light setting key | dark setting key | 默认值 |
| --- | --- | --- | --- |
| Display Primary | `display_font_light_v2` | `display_font_dark_v2` | `geist` |
| Display Custom | `display_font_custom_light_v2` | `display_font_custom_dark_v2` | 空 |
| Display Chinese Fallback | `display_chinese_fallback_light_v2` | `display_chinese_fallback_dark_v2` | `system` |
| Display Fallback Custom | `display_chinese_fallback_custom_light_v2` | `display_chinese_fallback_custom_dark_v2` | 空 |
| Body Primary | `body_font_light_v2` | `body_font_dark_v2` | `jetbrains` |
| Body Custom | `body_font_custom_light_v2` | `body_font_custom_dark_v2` | 空 |
| Body Chinese Fallback | `body_chinese_fallback_light_v2` | `body_chinese_fallback_dark_v2` | `system` |
| Body Fallback Custom | `body_chinese_fallback_custom_light_v2` | `body_chinese_fallback_custom_dark_v2` | 空 |
| Scale | `font_scale_v1` | `font_scale_v1` | `1` |

- 同明暗类别 flavor 共享 profile；Scale 保持全局，不按 mode 拆分。
- 加载优先级固定为有效 v2 -> 旧全局 v1 -> 默认值。首次迁移把同一份旧 profile 同时写入 light/dark v2，旧 key 保留。
- 部分迁移按字段回退；加载期间发生的用户编辑必须通过 revision/合并保护，不得被迁移补写覆盖。
- 所有 `set_setting` 写入必须按 setting key 串行；迁移补写先入队，后续用户保存后入队并最终获胜。加载 Promise 在迁移写入结束后返回当时的最新缓存，不能返回写入前捕获的旧快照。
- active mode 保存后实时应用；inactive mode 只更新目标缓存与 setting，不能修改根节点字体 data attributes 或 CSS variables。
- 字体链顺序固定为 `Primary -> optional Chinese Fallback -> role-specific system tail`。
- fallback 使用标准 CSS 缺字机制，不通过 `unicode-range` 强制切换脚本。
- `system` 不追加指定 family；`sourceHanSerif` 追加 `"Source Han Serif SC VF", "思源宋体 VF"`；`custom` 作为单个加引号 family 安全序列化。
- 自定义 family 的反斜线、引号和控制字符必须被处理，不能把输入解释成任意 CSS stack。
- specimen 与 DOM apply 必须复用 `displayFont.ts` 的同一字体链解析函数。
- 禁止为 fallback 新增 `@font-face`、字体二进制、下载/安装逻辑或 Tauri resource。

## 4. Validation & Error Matrix

| 条件 | 行为 |
| --- | --- |
| v2 preset 缺失、读取失败或非法 | 回退同字段旧 v1；旧值也无效时回退默认值 |
| v2 Custom 为空字符串 | 视为明确值，不回退旧 Custom |
| Custom 仅空白 | 应用时按 `system` 处理，但保留原 setting |
| Custom 包含引号、反斜线或控制字符 | 序列化为一个有效 family，不允许 CSS 注入或整条声明失效 |
| Source Han / Custom 未安装或缺字 | 浏览器继续尝试 system tail，不阻塞启动 |
| scale 非数字 | 回落默认值；有效数字限制在 `0.75..1.5` |
| 迁移读取期间保存某字段 | 最终缓存和 v2 值保留用户新值，不被旧值/默认值覆盖 |
| 迁移补写期间再次保存同一 key | 新保存等待该 key 的迁移写入结束，最终持久化值与返回缓存都采用新值 |

## 5. Good / Base / Bad Cases

- Good：active=`dark`，编辑 light Display 只更新 light specimen；切到 Latte 后立即应用该 light profile。
- Base：新安装的 light/dark 均为 Display=`geist`、Body=`jetbrains`、两个 fallback=`system`，Scale=`1`。
- Migration：旧 v1 Display=`serif` 时，首次加载的 light/dark 都为 `serif`，后续可独立修改。
- Bad：在 Appearance 组件里拼 `font-family`、把用户输入当逗号 stack，或保存 inactive mode 时修改当前 DOM。

## 6. Tests Required

- `src/test/stores/themeStore.test.ts`：六个 flavor 到 light/dark 的完整映射。
- `src/test/lib/displayFont.test.ts`：默认值、三种 fallback、v1 双份迁移、部分/非法 v2、并发编辑保护、mode 隔离、安全字体链与全局 Scale。
- 并发编辑测试必须分别覆盖读取阶段和迁移写回阶段，并断言未编辑字段仍从持久化结果合并、同 key 的最新用户值最终落盘。
- `src/test/pages/SettingsView.test.tsx`：亮/暗 editor、当前标识、不切 Theme、Custom 渐进显示、mode-specific key 写入、specimen 与可访问名称。
- `src/test/contracts/fontContract.test.ts`：CSS token 生效且没有 fallback `@font-face` 声明。
- 改动后至少运行定向 Vitest、`pnpm build` 与 `just ci`。

## 7. Wrong vs Correct

```ts
// Wrong: 组件拼 stack，用户输入可改变 CSS 结构
style={{ fontFamily: `${custom}, sans-serif` }}

// Correct: 组件提交 mode + 结构化偏好，共享边界负责缓存、持久化和应用
await saveDisplayChineseFallback(editorMode, key, custom);
```
