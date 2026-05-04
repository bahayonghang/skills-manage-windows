# 国际化与主题

SkillPort 提供两种语言和四种 Catppuccin 主题。切换入口都在顶部栏，可以随时切换且不丢状态。

## 语言

| 语言 | 源文件 |
|------|--------|
| English | `src/i18n/locales/en.json` |
| 简体中文 | `src/i18n/locales/zh.json` |

首次启动时由 `i18next-browser-languagedetector` 检测当前语言，之后持久化。所有面向用户的文案都走 `react-i18next`；源码里出现裸英文字符串会被视为缺陷。

## 主题

应用内置 Catppuccin 调色：

| 风格 | 类型 |
|------|------|
| Latte | 浅色 |
| Frappé | 中度暗 |
| Macchiato | 深色 |
| Mocha | 最深暗色 |

在顶部栏切换。选择会写入根 HTML 元素的 `data-theme` 属性，CSS 变量实时更新。

## 强调色

基于 Catppuccin 派生 14 种强调色。在外观面板中选定，结果保存为 `data-accent`。强调色只影响主操作、焦点环、少量高亮，不改变底色风格。

## 自定义

| 目标 | 在哪里改 |
|------|----------|
| 新增一条翻译键 | 同时改 `en.json` 与 `zh.json`；只改一边的 PR 会被拦下。 |
| 新增一个强调色 | 修改 `tokens.css` 与强调色选择器；有测试覆盖契约。 |
| 覆盖默认主题 | 在应用启动前设置根元素 `data-theme`（高级用法，一般不需要）。 |

## 文档站语言

本站点遵循相同约定。英文入口为 `/`，中文镜像在 `/zh/`。文档导航右上的语言切换沿用与桌面应用一致的 locale 路由。

---

Last reviewed: 2026-05-04
