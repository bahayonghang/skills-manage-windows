# 前端

前端是单个 Vite 打包的 React 19 应用，运行在 Tauri webview 里。状态全部由 Zustand stores 持有，IPC 调用从 stores 出口收口。

## 路由

`src/App.tsx` 在唯一 `<AppShell />` 路由下挂载懒加载页面：

| 路径 | 页面 | 布局 |
| --- | --- | --- |
| `/dashboard` | `DashboardView` | 操作概览 |
| `/central` | `CentralSkillsView` | 双列技能卡片 |
| `/platform/:agentId` | `PlatformView` | 双列技能卡片 |
| `/skill/:skillId` | `SkillDetailPage` | Markdown + 侧栏 |
| `/collections` | `CollectionsListView` | 选中卡片 + 技能列表 |
| `/collection/:id` | `CollectionView` | 详情变体 |
| `/marketplace` | `MarketplaceView` | 三 Tab |
| `/discover` | `DiscoverView` | 项目列表 + 技能详情 |
| `/discover/:projectPath` | `DiscoverView` | 同上，按项目过滤 |
| `/obsidian` / `/obsidian/:vaultId` | `ObsidianVaultView` | Vault 列表 + 技能 |
| `/logs` | `OperationLogsView` | 可过滤的日志表 |
| `/settings` | `SettingsView` | 分区卡片 |

懒加载确保首屏成本可控，dashboard 路由不会拉入 marketplace 的 HTTP 代码。

## Stores

`src/stores/` 是唯一调用 `invoke()` 的位置。每个 store 对应一个业务域：

```text
┌──────────────────────────────┬─────────────────────────────────┐
│ Store                        │ 拥有                            │
├──────────────────────────────┼─────────────────────────────────┤
│ skillStore                   │ 各平台技能列表                  │
│ centralSkillsStore（拆分）   │ list / install / metadata /     │
│                              │ update slice                    │
│ skillDetailStore             │ Markdown + 文件树 + 状态        │
│ platformStore                │ 平台注册表 + 可见性             │
│ collectionStore              │ 集合 + 批量安装                 │
│ marketplaceStore（拆分）     │ 源 + GitHub 导入                │
│ discoverStore                │ 项目扫描根 + 结果               │
│ obsidianStore                │ Vault 列表 + Vault 技能         │
│ targetStore                  │ 活动目标 + SSH 目标             │
│ operationLogStore            │ 日志分页 + 过滤                 │
│ settingsStore                │ 键值设置 + 扫描目录             │
│ themeStore                   │ Catppuccin 风格 + accent        │
└──────────────────────────────┴─────────────────────────────────┘
```

拆分 store 按 slice 切，让每个文件保持在 sizecheck 800 行红线下。

## 组件

`src/components/` 按业务域分组：

```text
components/
├── layout/          AppShell、Sidebar、面包屑
├── skill/           UnifiedSkillCard、详情子件、文件树
├── central/         安装/管理抽屉、对话框
├── collections/     集合卡片与对话框
├── platform/        PlatformIcon（LobeHub + monogram fallback）
├── marketplace/     Tab、搜索、GitHub 导入抽屉
├── settings/        分区卡片
└── ui/              shadcn 基元，按设计系统封装
```

`UnifiedSkillCard` 是 central / platform / discover / marketplace / collection 五种场景共用的技能卡。新页面通过 props 复用，不要再造内联卡。

## i18n

`src/i18n/locales/en.json` 与 `zh.json` 是用户可见文本的唯一源。组件用 `useTranslation()` 读，测试断言 key，不断言渲染文本。

## IPC 边界规则

> 组件不直接调用 `invoke()`。

vitest 一次 mock `window.__TAURI_INTERNALS__.invoke`，所有 store 自动配合。

Last reviewed: 2026-05-04
