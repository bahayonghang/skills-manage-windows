# 实施计划

## 成功标准

- 统计按系统本地自然日归档，固定时间口径可见。
- 技能 identity 只在唯一中央匹配时可跳转，未匹配/歧义仍有统计详情。
- Skill.md 静态估算通过 target-aware FsBackend 构建并与 calls 原子缓存。
- 平台/target 快速切换不产生陈旧提交，缓存失败状态诚实可见。
- 新界面在深浅主题、中文/英文和宽/窄桌面下可读、可键盘操作。
- 定向测试与 `just ci` 全部通过。

## 0. 实施前基线

- [ ] 重新读取 `trellis-before-dev`、frontend/backend spec indexes、`ipc-adapter.md`、`spawn-blocking-io.md` 与本任务三份规划文档。
- [ ] 记录当前脏树；`package.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、`src-tauri/tauri.conf.json` 是用户既有改动，不得覆盖或顺手格式化。
- [ ] 运行当前 usage 定向测试，记录任何基线失败：

```powershell
rtk pnpm exec vitest run src/test/SkillUsage.components.test.tsx src/test/usageStore.test.ts src/test/useSkillCallCounts.test.ts
rtk cargo test --manifest-path src-tauri/Cargo.toml services::usage
```

## 1. 先写后端失败用例

- [ ] 在 usage schema/repo 测试加入 `skill_usage_metadata` 建表与 target 隔离断言。
- [ ] 为 resolver 写 exact id、唯一 name、重复 name、无匹配、大小写与空白用例。
- [ ] 为 metadata 原子替换写回滚/旧 target 保留用例。
- [ ] 为静态估算写 ASCII、CJK、混合、空文件与超预算用例。
- [ ] 为本地日界写 Asia/Shanghai 跨 UTC 日界用例，并用可切换 offset 的 fake resolver 验证每事件动态 offset。
- [ ] 为详情 source 过滤和项目 `sessions` 去重写查询用例。

验证：

```powershell
rtk cargo test --manifest-path src-tauri/Cargo.toml usage
```

预期：新增测试在实现前失败，完成步骤 2-4 后通过。

## 2. 增加 metadata schema 与 repo 契约

- [ ] 在 `src-tauri/src/db/schema/usage.rs` additive 创建 `skill_usage_metadata`、CHECK、主键和必要索引。
- [ ] 在 `src-tauri/src/db/repos/usage_repo.rs` 增加 `NewSkillUsageMetadata` / row 类型、中央候选批量查询、metadata 查询与时间戳查询。
- [ ] 扩展 `replace_calls_for_target` 参数和事务，按 target 同时替换 calls/providers/metadata/scan state。
- [ ] 保持旧 DB 无迁移脚本即可启动；验证重复 init 幂等。

回滚点：本步骤只增加派生表和 repo API；若失败可移除新表调用而不改 `skill_calls` 数据。

## 3. 实现身份与静态指标 enrichment

- [ ] 新建 `src-tauri/src/services/usage/enrichment.rs`，定义 `UsageSkillMatchStatus`、candidate resolver、静态估算和 metadata builder。
- [ ] 在 `services/usage/mod.rs::refresh_with_providers` 中对 distinct skill names 批量解析中央候选。
- [ ] 仅对 matched 候选调用当前 `Scope::fs_backend().read_many_to_strings`；读取结果执行 ResourceBudget 检查。
- [ ] 读取失败/超预算只生成 NULL 静态指标并记录 debug/warn，不让 provider calls 丢失。
- [ ] 把 metadata 随 calls/provider outcomes 交给同一事务。
- [ ] 修改 `usage_resolve_skill_id` 复用唯一匹配结果，删除“排序后 LIMIT 1”猜测。

验证：

```powershell
rtk cargo test --manifest-path src-tauri/Cargo.toml usage
rtk cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## 4. 修正 overview、recent、detail 与本地日期

- [ ] 用语义类型替代复用/伪装：`SkillUsageSummary`、`RecentSkillCall`、`SkillProjectCount`。
- [ ] overview join metadata，并返回全部技能 summary；默认仍按 count/last/name 排序。
- [ ] recent 批量附加 match status / resolved id，不在前端逐行 invoke。
- [ ] `usage_get_skill_detail` 接受可选 `source`，返回 metadata 与正确的 by-project session 数。
- [ ] repo 改为读 raw `timestamp_ms`；聚合层以系统 Local 逐事件分日并生成连续 112 日网格。
- [ ] cutoff 多覆盖一个自然日但最终严格截断到 16 周；非法 timestamp 不得生成 1970 噪声格。

验证：

```powershell
rtk cargo test --manifest-path src-tauri/Cargo.toml usage
```

## 5. 更新 typed IPC、fixtures 与 TS 类型

- [ ] 更新 `src/types/usage.ts` 的 summary/recent/detail/match status 契约。
- [ ] 同步 `src/lib/ipc/commandMap.ts`，为 detail 增加 `source` 参数；保留 resolve 命令兼容类型。
- [ ] 扩充 `src/fixtures/usage.ts`：matched、ambiguous、unmatched、静态不可用、缓存失败。
- [ ] 更新 `src/test/ipcCommandCoverage.test.ts` / `browserFixtures.test.ts` 相关断言，禁止组件直接 invoke。

验证：

```powershell
rtk pnpm exec vitest run src/test/ipcCommandCoverage.test.ts src/test/browserFixtures.test.ts src/test/ipc.test.ts
rtk pnpm typecheck
```

## 6. 收敛 usageStore 的一致性

- [ ] 增加 `selectedSkill`、`usedCachedData`、`refreshError` 和 detail loading 状态。
- [ ] 为 source/page/detail 请求增加 sequence + target + source 守卫。
- [ ] source 切换时 Promise.all 获取 overview/recent 后单次 set；失败保留旧数据并给出可恢复错误。
- [ ] refresh 在有 source 筛选时不先提交未过滤 overview；保留旧筛选数据到重拉完成。
- [ ] target/source 切换清空 detail 并使旧详情请求失效。
- [ ] 删除 SkillBarChart/RecentCallsFeed 的点击时 resolve invoke 依赖，改用返回的 identity 字段。
- [ ] 在 `usageStore.test.ts` 增加快速 A->B、target change、filtered refresh、cached failure 和 stale detail 回归。

验证：

```powershell
rtk pnpm exec vitest run src/test/usageStore.test.ts src/test/useSkillCallCounts.test.ts
```

## 7. 重构页面信息层级

- [ ] 用单一紧凑 `UsageMetricStrip` 替代四张渐变 KPI 卡，保留 tabular nums 和明确“全部已记录”标签。
- [ ] 新建 `SkillUsageTable` 替代 `SkillBarChart`：显式 segmented sort、稳定列、匹配状态、静态估算、row selection、独立打开技能 icon action。
- [ ] 新建非 modal 的 `SkillUsageDetailPanel`，展示摘要、项目分布、单技能热力图与打开技能动作，并处理焦点返回。
- [ ] 更新 `ActivityHeatmap`：本地日期、月份、分位数色阶、图例、hover/focus tooltip、aria 与 reduced motion。
- [ ] 更新 `RecentCallsFeed` 使用统一详情选择/打开技能动作，项目只显示 basename。
- [ ] 把 `ProviderHealthList` 放入底部 disclosure summary，保留筛选联动与状态语义。
- [ ] 重排 `SkillUsageView`：`xl` 主从两列，1024px/窄桌面单列；不嵌套卡片，不添加玻璃/渐变装饰。
- [ ] 首屏 skeleton、无数据、无缓存错误、缓存过期和远程不可达状态都保持稳定布局。

风险检查：按钮不得嵌套；长技能名、中英文状态、远程 target label 不得撑破行；热力图格子/tooltip 不得改变面板尺寸。

## 8. i18n 与前端回归测试

- [ ] 同步 `src/i18n/locales/zh.json` 与 `en.json`：固定范围、匹配状态、静态估算、详情、缓存、空态、provider disclosure、可访问标签。
- [ ] 把误放在 marketplace namespace 的缓存提示补到 `skillUsage` 下，并修正远程缓存文案。
- [ ] 重写/扩展 `SkillUsage.components.test.tsx`，覆盖三种 identity、排序、详情选择、打开技能、heatmap 分位数、键盘、disclosure、空态与长文本。
- [ ] 保留 Sidebar、Central/Platform 30 天徽标和 browser fixture 回归。

验证：

```powershell
rtk pnpm exec vitest run src/test/SkillUsage.components.test.tsx src/test/usageStore.test.ts src/test/useSkillCallCounts.test.ts src/test/browserFixtures.test.ts
rtk pnpm typecheck
rtk pnpm lint
```

## 9. 视觉与交互验证

- [ ] 启动浏览器 fixture dev server：`rtk pnpm dev`，记录实际 URL 和停止方法。
- [ ] 使用 in-app Browser 检查 `/usage`，不依赖 Tauri-only 假设。
- [ ] 截图并检查至少：1440x900 深色、1280x720 深色、1024x768 浅色、窄桌面宽度；每个视口检查中文与英文最长文案。
- [ ] 操作平台筛选、快速连续切换、排序、行选择、详情关闭/焦点返回、打开技能、刷新与 provider disclosure。
- [ ] 检查 matched/ambiguous/unmatched、静态不可用、无调用、有缓存失败、无缓存失败。
- [ ] 确认无重叠、无横向页面溢出、无布局跳动、对比度可读，且 16 周热力图非空、tooltip 与键盘焦点有效。
- [ ] 视觉验证后停止仅为本任务启动的 dev server。

## 10. 最终质量门

按由窄到宽顺序执行，失败先定向复现再修改：

```powershell
rtk pnpm exec vitest run src/test/SkillUsage.components.test.tsx src/test/usageStore.test.ts src/test/useSkillCallCounts.test.ts src/test/browserFixtures.test.ts src/test/ipcCommandCoverage.test.ts
rtk cargo test --manifest-path src-tauri/Cargo.toml usage
rtk pnpm typecheck
rtk pnpm lint
rtk cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
rtk just ci
rtk git diff --check
rtk python ./.trellis/scripts/task.py validate .trellis/tasks/07-15-skill-usage-analytics-ux
```

- [ ] 检查 diff 只包含本任务文件；用户已有四个配置/打包改动保持原语义。
- [ ] 复读 PRD 验收项逐条映射测试/截图证据。
- [ ] 未经用户批准不运行 `task.py start`、不提交、不推送。

## 主要风险文件与回滚点

| 文件/区域 | 风险 | 回滚点 |
| --- | --- | --- |
| `db/schema/usage.rs`、`usage_repo.rs` | 缓存原子性/旧 DB | metadata additive；calls 表不迁移 |
| `services/usage/mod.rs`、`enrichment.rs` | remote I/O 与扫描时延 | enrichment 可降级为空 metadata |
| `commands/usage.rs`、TS IPC types | Rust/TS 契约漂移 | commandMap + fixture + IPC tests 同步 |
| `usageStore.ts` | 竞态与旧 target 串台 | sequence guard 用定向延迟测试锁定 |
| `SkillUsageView` / usage components | 高密度响应式与可访问性 | 保留旧组件直到新组件测试和截图通过后再删除 |
