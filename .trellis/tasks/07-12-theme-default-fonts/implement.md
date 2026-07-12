# Implementation Plan

## Approval Gate

- [x] 标准 CSS 缺字 fallback。
- [x] Display/Body 独立配置。
- [x] System、Source Han、Custom 预设模型，默认 System。
- [x] 不捆绑字体资产，不增加安装包体积。
- [x] Shared Settings shell + Appearance 深改，其他页面仅继承共享表面。
- [x] 用户审阅最终 PRD、设计与实施计划并明确批准实施。

## Steps

1. 扩展字体偏好模型。
   - 增加 fallback key/custom 类型、默认值、storage keys、coercion、load/save/apply API。
   - 集中实现安全 family 序列化和 Display/Body 字体链构造。
   - 保留现有 Primary key、custom 值和旧设置兼容。
2. 先更新字体单元契约。
   - 覆盖 System、Source Han、Custom、空/无效值、独立角色、旧设置读取和保存往返。
   - 更新 CSS token 断言，确保无字体 asset 或 `@font-face` 引用。
3. 重构共享 Settings shell。
   - 删除重复 Settings 标题条，收敛 page header和内容宽度。
   - 扁平化桌面/窄窗口 `SettingsSideNav`，保持路由与 `aria-current`。
   - 将共享 collapsible card 改为 unframed `SettingsSection`，保留折叠、ARIA 与 localStorage契约。
   - 机械更新其他设置页的容器引用，不改业务 children或事件。
4. 重做 Appearance。
   - 按 Theme、Language、Typography、Density 构建 label/value rows。
   - 为 Display/Body 增加独立 fallback selector、Custom 输入和混排 specimen。
   - 移除实验室 hero、装饰背景、模拟窗口、chips、metrics 与嵌套卡片 helper。
5. 更新中英文 i18n 与 SettingsView 测试。
   - 覆盖控件标签、可访问名称、Custom 渐进显隐、编辑竞态、独立保存、导航和 section 结构。
   - 保留非 Appearance 页面业务行为断言。
6. 运行代码与视觉验证。
   - 修复 typecheck/lint/test 发现的问题，不扩大到其他设置业务重排。
   - 启动本地前端，在 900x600、1200x800、1440x900 检查 Appearance 及至少一个多 section 页面。
   - 验证主题、语言、Primary/Chinese Fallback、Custom、Font Scale、collapse 与 settings page routing。

## Validation

- `pnpm exec vitest run src/test/displayFont.test.ts src/test/fontContract.test.ts src/test/SettingsView.test.tsx`
- `pnpm typecheck`
- `pnpm lint`
- `pnpm build`
- `just ci`
- 浏览器截图与控制台检查：900x600、1200x800、1440x900，无溢出、重叠、裁切、错误或资源加载失败。
- `rg` 检查不存在新增字体二进制、`@font-face` 或 Tauri font resource。

## Risk And Rollback Points

- 字体链风险集中在安全 quoting 与 custom 空值；通过纯函数测试锁定。
- Settings shell 影响所有设置路由；共享容器改动后先跑 SettingsView 全量测试，再做 Appearance 细节。
- 保留 collapse storage key 与非 Appearance children，避免视觉重构变成业务回归。
- 新 settings key 与现有 key 分离；回滚时无需迁移数据库。
- 无字体资产、Rust 或 Tauri bundle 变更。

## Validation Results

- 定向 Vitest：3 files、112 tests 通过。
- `pnpm build` 通过，产物只包含任务前已有的 Fontsource 资源。
- `just ci` 通过：前端 1336 tests 通过、1 skipped；Rust 767 unit tests 与 5 e2e 通过。
- `git diff --check`、旧 Settings 组件引用和新增字体二进制检查通过。
- 三档 viewport 截图未自动验证：Computer Use 无法确认本地 URL allowlist；该限制已在提交前告知用户。
