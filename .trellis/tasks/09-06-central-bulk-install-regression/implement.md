# 执行计划
依赖用户批准，不依赖其他child代码；canonical全门禁依赖doctor/toolchain问题解决。

1. 阅读同域当前toolbar/selection/bulk fixtures，确认失效skip与等价用户链。
2. 用当前交互重写该用例，移除it.skip，保持反例数据和精确batch请求断言。
3. `pnpm exec vitest run src/test/pages/CentralSkillsView.repositories-and-installs.test.tsx`；如果pnpm仍阻塞，先直接 `node node_modules/vitest/vitest.mjs run src/test/pages/CentralSkillsView.repositories-and-installs.test.tsx`，标direct证据。
4. `pnpm typecheck`、`pnpm test`，全量frontend不再有该skip；不可通过删测试降低总数假装恢复。
5. 强模型独立审查未直接调用store绕过UI、未放宽batch参数、未修改生产代码。新产品bug需回主线程界定。
6. 父集成统一just ci，rules child可记录低风险执行范例与适用工具，不生成无关skill。
