# 应用内快捷键速查

## Goal

给键盘优先的重度用户一个应用内快捷键速查浮层（Nielsen #10 帮助文档 = 2 分的针对性补强）：随处可唤起、内容与实际行为一致、中英双语、随主题换肤。不做完整帮助中心。

## Confirmed Facts

- 快捷键基础设施已存在：`src/lib/keyboardShortcuts.ts`（组合键解析/匹配，含测试）+ `src/hooks/useHotkey.ts`（window keydown 绑定）。
- 现有散落快捷键（无集中注册表、无任何速查 UI）：
  - `mod+k`：全局搜索（`src/components/layout/GlobalSearchDialog.tsx:295`）与中央库 CommandPalette（`src/components/central/CommandPalette.tsx:59`）
  - Logs 页非编辑焦点下的本地键：`/` 聚焦搜索等，另有刷新/导出回调键位（`src/components/logs/useLogsKeyboard.ts:24-70`，完整键位实现时盘点）
  - `TargetQuickSwitcher.tsx:42` 自有 keydown 处理
- PRODUCT.md 明确「完整键盘可达」是可访问性基线；2026-07-06 评审中 persona Alex（键盘重度）与 Nielsen #7/#10 均指向缺少快捷键可发现性。
- 项目规定：所有用户可见文本走 i18n（zh/en）；弹层用 `ui/dialog` 原语；卡片签名 `rounded-xl + ring-1 ring-border + bg-card`。
- 产品决策（2026-07-06）：补快捷键速查入口，按推荐最小化实现（速查浮层，非帮助中心）。

## Requirements

- 新增快捷键速查浮层（基于 `ui/dialog`）：非编辑焦点按 `?`（Shift+/）或 `mod+/` 唤起，Esc 关闭；实现走 `useHotkey`/`keyboardShortcuts.ts`，不自造第二套监听。
- 集中式快捷键清单模块（如 `src/lib/shortcutRegistry.ts` 或静态数据文件）：按域分组（全局 / 中央库 / 日志 / 目标快切…），速查浮层从清单渲染；实现前先盘点现有全部快捷键收进清单，**表内每条与实际行为一致**（本地优先的诚实原则）。
- 键帽渲染用 `<kbd>` 样式（等宽字体 + `ring-1 ring-border` 小圆角），按平台显示 `Ctrl`/`⌘`（沿用 keyboardShortcuts.ts 的 mod 语义）。
- 可发现性：TopBar 或设置页至少一处可点击入口（图标按钮带 aria-label + title）。
- 全部文案走 i18n（zh/en 同步补齐）；不改变任何现有快捷键的行为。

## Acceptance Criteria

- [ ] 非编辑焦点按 `?` 或 `mod+/` 打开速查，Esc 关闭；输入框聚焦时按 `?` 不触发——组件测试覆盖。
- [ ] 速查列出的每条快捷键人工核对与实际行为一致；`mod+k`、Logs 本地键、目标快切均在列。
- [ ] zh/en 双语完整；4 套代表主题（Mocha/Latte/Claude Light/Claude Dark）目视无违和。
- [ ] 存在至少一处鼠标可发现入口（TopBar 或设置页）。
- [ ] `pnpm test`（新增组件与 registry 相关）、`pnpm typecheck && pnpm lint` 通过；收尾跑 `just ci`。

## Out Of Scope

- 新增/修改任何业务快捷键本身（只做盘点与展示）。
- 完整帮助中心、文档站、onboarding 引导。
- 快捷键自定义/重映射能力。
