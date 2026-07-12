# Implementation Plan

## Approval Gate

- [x] 亮色与暗色分别保存完整的 Display/Body 字体 profile。
- [x] 同明暗类别 flavor 共享 profile；Font Scale 保持全局。
- [x] 旧全局字体偏好同时迁移为两套初始值。
- [x] Typography 使用独立亮/暗分段编辑器，不切换应用 Theme。
- [x] 用户审阅最终 PRD、设计与实施计划并明确批准实施。

## Steps

1. 先锁定主题分类与迁移契约。
   - 在 `themeStore.test.ts` 覆盖六个 flavor 到 light/dark 的唯一映射。
   - 在 `displayFont.test.ts` 增加两套默认值、v2 key、v1 双份迁移、部分/无效 v2 回退和全局 scale 测试。
   - 验证 active/inactive mode 隔离：只允许 active mode 修改 DOM，inactive mode 仅更新缓存与持久化。
2. 扩展字体偏好边界。
   - 在 `themeStore.ts` 增加 `FontThemeMode` 与纯分类 helper。
   - 在 `displayFont.ts` 拆分 `FontProfile` / `ThemedFontPreferences`，增加 mode-specific key map、加载复用、best-effort 迁移和两套内存缓存。
   - 导出供 DOM 与 specimen 共同使用的安全字体链解析函数；保留现有 coercion、Custom quoting 和 fallback 语义。
   - 让 mode-aware save API 只写目标 mode，并仅在目标为 active mode 时应用到 DOM。
3. 接入全局 Theme 切换。
   - 调整 `main.tsx` 启动顺序：同步应用默认 profile，再异步加载/迁移两套偏好并应用完成时的当前 mode。
   - 订阅 `themeStore` flavor 变化，跨 mode 时激活缓存 profile，同 mode flavor 切换保持字体不变。
   - 防止异步加载与快速主题切换造成旧 mode 覆盖最终 DOM。
4. 重排 Appearance Typography。
   - 将 `SettingGroup` 收敛为固定说明轨道 + 弹性内容列，减少所有 Appearance 分组的宽屏无效留白。
   - 增加亮/暗 segmented editor、可见“当前”文字和独立 editor mode 状态；不得调用 `onSetFlavor`。
   - 将 Display/Body 改为内容宽度驱动的双列/单列 role 布局，移除内部 0.3fr/0.7fr 嵌套 label 网格。
   - 将 Primary、Chinese Fallback、按需 Custom 与 specimen 紧密归组；inactive mode specimen 使用共享安全解析函数。
   - 保持控件至少 40px、键盘/focus/ARIA 和现有按压反馈。
5. 更新 i18n 与集成测试。
   - 增加中英文 Light、Dark、Current 和主题字体编辑语义文案。
   - 在 `SettingsView.test.tsx` 覆盖默认 editor mode、当前标识、独立切换、active/inactive 编辑、同/跨 mode flavor 行为、Custom 渐进显示及现有 Font Scale。
   - 保留 Theme、Language、Density 与非 Appearance 设置行为断言。
6. 更新持久规范。
   - 修改 `font-preferences.md`：记录两套 profile、v2 keys、v1 迁移、active/inactive apply 和全局 scale。
   - 修改 `settings-structure.md`：记录 segmented editor、紧凑 role grid 与响应式结构，替代旧的单套字体边界。
7. 完成代码与视觉验证。
   - 先运行定向测试并修复，再运行 typecheck、lint、build 与 `just ci`。
   - 启动本地前端，在 900x600、1200x800、1440x900、440x900 检查亮/暗编辑、Custom 展开和 Theme 切换。
   - 用截图对比原始问题：标签与控件邻近、无大面积中间空洞、页面纵向长度降低、无溢出或重叠。

## Validation

- `pnpm exec vitest run src/test/displayFont.test.ts src/test/themeStore.test.ts src/test/fontContract.test.ts src/test/SettingsView.test.tsx`
- `pnpm typecheck`
- `pnpm lint`
- `pnpm build`
- `just ci`
- `git diff --check`
- 浏览器截图与控制台检查：900x600、1200x800、1440x900、440x900。
- `rg` 检查没有新增字体二进制、`@font-face`、Tauri font resource 或新的运行时依赖。

## Risk And Rollback Points

- 最高风险是异步加载与 Theme 快速切换竞态；通过“加载完成时重新读取当前 flavor”和 active-mode 单点 apply 测试锁定。
- v2 key 数量增加且可能部分写入；逐字段读取优先级与 best-effort 补写必须覆盖部分迁移场景。
- `SettingGroup` 影响 Appearance 的四个分组；视觉验证同时检查 Theme、Language、Typography、Density，避免只修 Typography 后破坏其他布局。
- inactive specimen 必须复用安全字体链解析，禁止在组件中重新拼接用户输入。
- `main.tsx` 的长期订阅影响全应用；回滚点是启动订阅与 themed cache，旧 v1 keys 保证可恢复。
- 不涉及 Rust、数据库 schema、字体资产或打包配置；默认不要求 `pnpm tauri build`。

## Validation Results

- 定向 Vitest：4 个文件、146 项测试通过。
- `pnpm typecheck`、`pnpm lint`、`pnpm build`、`git diff --check` 通过。
- `just ci` 通过：前端 123 个测试文件（1346 通过、1 跳过），Rust 767 个单元测试与 5 个 E2E 通过，clippy 与 sizecheck 通过。
- 浏览器检查通过：1024x800 与 440x900 下 document / Appearance 水平溢出均为 0，Light/Dark 按钮实测高度均为 40px；1024px 下字体角色上下堆叠。
- 浏览器交互通过：编辑 inactive Light profile 不改写 Mocha DOM；切换 Latte 后应用该 profile，editor mode 保持稳定，控制台无 warning/error。
- 早期视觉检查覆盖 900x600、1200x800、1440x900 与 440x900；1024px 发现的 role overflow 已通过将双列断点收敛到 `xl` 修复并复验。
