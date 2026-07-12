# 字体偏好与中文 fallback 契约

> 建立于 2026-07-12（任务 07-12-theme-default-fonts）。字体偏好统一由 `src/lib/displayFont.ts` 管理；Source Han 只查找系统字体，不进入仓库或安装包。

## 1. Scope / Trigger

修改 Display/Body 字体预设、自定义 family、中文 fallback、字号或对应设置项时适用。组件不得自行拼接 CSS font stack，也不得直接调用 Tauri `invoke()`。

## 2. Signatures

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

loadFontPreferences(): Promise<FontPreferences>;
applyFontPreferences(prefs: FontPreferences): void;
saveDisplayChineseFallback(key, custom, display, displayCustom): Promise<void>;
saveBodyChineseFallback(key, custom, body, bodyCustom): Promise<void>;
```

## 3. Contracts

| 字段 | setting key | 默认值 |
| --- | --- | --- |
| Display Primary | `display_font_v1` / `display_font_custom_v1` | `geist` / 空 |
| Display Chinese Fallback | `display_chinese_fallback_v1` / `display_chinese_fallback_custom_v1` | `system` / 空 |
| Body Primary | `body_font_v1` / `body_font_custom_v1` | `jetbrains` / 空 |
| Body Chinese Fallback | `body_chinese_fallback_v1` / `body_chinese_fallback_custom_v1` | `system` / 空 |
| Scale | `font_scale_v1` | `1` |

- 字体链顺序固定为 `Primary -> optional Chinese Fallback -> role-specific system tail`。
- fallback 使用标准 CSS 缺字机制，不通过 `unicode-range` 强制切换脚本。
- `system` 不追加指定 family；`sourceHanSerif` 追加 `"Source Han Serif SC VF", "思源宋体 VF"`；`custom` 作为单个加引号 family 安全序列化。
- 自定义 family 的反斜线、引号和控制字符必须被处理，不能把输入解释成任意 CSS stack。
- 切回预设时保留 custom setting，但不应用；旧设置缺少 fallback key 时保持 System。
- 禁止为 fallback 新增 `@font-face`、字体二进制、下载/安装逻辑或 Tauri resource。

## 4. Validation & Error Matrix

| 条件 | 行为 |
| --- | --- |
| setting 缺失、读取失败或 fallback key 非法 | fallback 回落 `system`，不修改旧 Primary |
| Custom 为空或仅空白 | 按 `system` 处理 |
| Custom 包含引号、反斜线或控制字符 | 序列化为一个有效 family，不允许 CSS 注入或整条声明失效 |
| Source Han / Custom 未安装或缺字 | 浏览器继续尝试 system tail，不阻塞启动 |
| scale 非数字 | 回落默认值；有效数字限制在 `0.75..1.5` |

## 5. Good / Base / Bad Cases

- Good：Display=`geist` + Source Han，得到 `"Geist Variable", "Source Han Serif SC VF", ...system tail`。
- Base：新安装保持 Display=`geist`、Body=`jetbrains`、两个 fallback=`system`，计算字体与旧版本一致。
- Bad：在 Appearance 组件里用字符串拼 `font-family`，或把用户输入当成逗号分隔 stack。

## 6. Tests Required

- `src/test/displayFont.test.ts`：默认值、三种 fallback、Display/Body 独立、空值、控制字符、CSS 链顺序。
- `src/test/SettingsView.test.tsx`：Custom 渐进显示、独立 setting key 写入、混排 specimen 与可访问名称。
- `src/test/fontContract.test.ts`：CSS token 生效且没有 fallback `@font-face` 声明。
- 改动后至少运行定向 Vitest、`pnpm build` 与 `just ci`。

## 7. Wrong vs Correct

```ts
// Wrong: 组件拼 stack，用户输入可改变 CSS 结构
style={{ fontFamily: `${custom}, sans-serif` }}

// Correct: 组件只提交结构化偏好，统一边界负责构造和应用
await saveDisplayChineseFallback(key, custom, display, displayCustom);
```
