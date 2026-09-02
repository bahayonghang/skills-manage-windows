# Design

## Change List

| 文件 / 符号 | 计划变更 | 追溯 |
| --- | --- | --- |
| `src/lib/updateCheckMode.ts`（新）/ `UpdateCheckMode`、`CENTRAL_UPDATE_CHECK_MODE_SETTING_KEY`、`DEFAULT_UPDATE_CHECK_MODE`、`normalizeUpdateCheckMode` | 从 page 模块提取不依赖 page/store 的中性契约 | R1 |
| `src/pages/centralUpdateCheckMode.ts`、`src/lib/updateCenterRefreshScope.ts`、`src/stores/updateCenterStore.ts`、`src/stores/settingsStore.ts` 及相关 components | 改从中性模块消费；page 保留 view-specific scope builders | R1, R6 |
| `src/lib/ipc/invoke.ts`、`src/lib/ipc/index.ts` / `UnlistenFn` | 保留 `invoke.ts` 为唯一 Tauri event 入口，并从 public adapter surface re-export type | R2 |
| `src/lib/explanationStream.ts`、`src/stores/marketplaceStore.githubImportHelpers.ts`、`src/stores/projectsStore.ts`、`src/stores/skillDetailStore.ts` | 把 `UnlistenFn` import 改为 `@/lib/ipc` | R2, R6 |
| `src/pages/CollectionView.tsx`、`src/components/marketplace/SkillPreviewDialog.tsx`、`src/components/platform/DuplicatePlatformSkillsDialog.tsx`、`src/components/skill/SkillDetailPanelShell.tsx` | 经 reachability gate 后删除，不留 wrapper | R3 |
| `src/test/pages/CollectionView.test.tsx`、`src/test/components/marketplace/SkillPreviewDialog.test.tsx` | 随仅被测废弃模块删除 | R3 |
| `src/stores/settingsStore.aiSlice.ts`、`src/components/settings/AiSettingsSection.tsx`、`src/pages/MarketplaceView.tsx` | fixture 分支只传稳定状态，渲染边界用 `t(...)` 选择可见文本 | R4 |
| `src/i18n/locales/en.json`、`src/i18n/locales/zh.json` | 增加 AI/Marketplace browser fixture keys | R4 |
| `src/test/contracts/frontendArchitectureContract.test.ts`（新） | 固化 page 反向依赖、Tauri event 唯一入口、死文件和 root barrel no-growth 口径 | R1-R3, R5-R7 |

## Contract

依赖方向为：

```text
neutral domain types/functions -> IPC adapter/store -> page/component
```

- 中性 `updateCheckMode` 文件不得 import `src/pages/**` 或 `src/stores/**`；`centralUpdateCheckMode.ts` 只保留需要 page/view types 的构造函数。
- `src/lib/ipc/invoke.ts` 是生产代码直接 import `@tauri-apps/api/event` 的唯一允许文件。契约扫描范围是 `src/**/*.{ts,tsx}`，排除 `src/test/**`；断言匹配集合精确等于该一个 repo-relative path，而不是“全生产为 0”。
- `UnlistenFn` 由 `src/lib/ipc/index.ts` 公开，所有生产调用方经 `@/lib/ipc` 获取。
- dead-code gate 对 4 个完整 repo-relative path 检查 static import、lazy import、dynamic import、route 和 test-only import；删除后断言文件不存在并且没有生产 module specifier。
- root barrel 扫描只覆盖 `src/**/*.{ts,tsx}` 的生产文件，并排除 `src/test/**` 与 `src/types/**`；结果排序、去重后才计数。启动基线在实施时重测并记录，不把 193 写入 contract。

## Baseline Commands

Tauri event 唯一入口：

```powershell
rtk rg -l '@tauri-apps/api/event' src --glob '*.ts' --glob '*.tsx' --glob '!src/test/**' | Sort-Object
```

期望实现后唯一输出：`src/lib/ipc/invoke.ts`。

根 types barrel 的可复算启动基线：

```powershell
rtk rg -l "from\s+[`"']@/types[`"']" src --glob '*.ts' --glob '*.tsx' --glob '!src/test/**' --glob '!src/types/**' | Sort-Object -Unique
```

实施者必须保存这条命令的完整路径清单并以清单行数作为 R5 baseline；不复用审计中的 193。

4 个 dead module 的引用复核：

```powershell
rtk rg -n 'CollectionView|SkillPreviewDialog|DuplicatePlatformSkillsDialog|SkillDetailPanelShell' src --glob '*.ts' --glob '*.tsx'
```

人工解释搜索结果时，必须区分 production import、test import 与描述旧实现的注释；最终自动契约以文件存在性和 module specifier 为准。

## Compatibility

- Update Center 的 setting key、`UpdateCheckMode` 值域、默认值和 normalize 行为不变；只移动 canonical owner。
- `@/lib/ipc` 是现有 public adapter，新增 type re-export 不改变 runtime；4 个调用方只改 import source。
- 4 个模块已不可达，因此不提供 deprecated wrapper；若 reachability gate 找到真实生产入口，停止该文件删除并回到 planning，而不是保留半删除状态。
- i18n 只改变 browser fixture 的显示来源，不改变桌面 backend error payload。
- 不迁移未触及的 root barrel 消费者，不做旧 import alias。

## Verification Boundary

- 静态 contract 证明受扫描源码中的 import 方向、唯一 Tauri event allowlist、dead file 不存在和 root barrel no-growth。
- Vitest 证明 update mode 行为、browser fixture 文案和保留页面/抽屉入口；locale contract 证明 key parity。
- `pnpm typecheck`、`pnpm lint` 与 `just ci` 是总门禁。
- 静态 reachability 不能证明运行时插件式字符串加载，但当前 Vite routes/imports 均在扫描范围；Windows WebView2 中 Update Center、Settings AI、Marketplace 的可用性在人工 smoke 前保持 `UNVERIFIED`。

## Rollback

| 回滚单元 | 包含内容 | 回滚点 |
| --- | --- | --- |
| A | update mode canonical owner 与所有消费者 | update-center/settings 定向测试和 AC1-AC4 通过后 |
| B | IPC `UnlistenFn` re-export、4 个 import 迁移与唯一入口 contract | AC5-AC7 通过后；adapter 与调用方必须一起回滚 |
| C | 4 个 dead modules 与 2 个孤立测试删除 | AC8-AC11 通过后；按完整文件集合回滚 |
| D | browser fixture i18n 与 locale keys | AC12-AC15 通过后 |
| E | root barrel baseline/no-growth contract 与触及领域窄 import | AC16-AC20 通过后；不得用 193 替换实测值 |

任一单元失败只回滚该单元及对应测试；A/B 内部存在 import 原子性，不得只回滚 owner 或 public export 的一半。

## Considered but Not Chosen

- 不把 page-specific scope builder 全部移入 lib：它仍依赖 view state，强行下沉会扩大中性模块职责。
- 不要求生产 Tauri event import 总数为 0：adapter 必须直接连接 Tauri；正确 invariant 是 adapter 外为 0。
- 不保留 dead module wrapper：没有生产消费者，wrapper 只会制造新兼容面。
- 不全量拆分 `src/types/index.ts`：成本与本任务 finding 不相称；只做实测 no-growth 和触及域迁移。
- 不引入依赖图工具：现有 TypeScript/Vitest 与确定性 source scan 足以验证本范围。
