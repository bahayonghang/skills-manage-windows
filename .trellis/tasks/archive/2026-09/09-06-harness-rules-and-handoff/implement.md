# 执行计划
依赖：session、doctor、bootstrap children已交付本地已验证结果；未完成外部证据可以继续明确记录UNVERIFIED，但不写为已验收。

1. 强模型冻结五工具来源、当前发现和project事实，保留父research中的历史证据日期。
2. 薄化CLAUDE→AGENTS；新增harness-guide（bootstrap、能力/权限/模型分工、交接模板）。
3. 修Kimi三个skill的错误声明；research采用explore→主会话持久化，不引入custom-agent兼容层。
4. 更新build-and-test、git-and-release、quality spec：readonly入口、Python lane、dev示例和REL状态。
5. 改developerExperienceContract的必要独立断言，不为文案镜像新增测试框架。
6. `pnpm exec vitest run src/test/contracts/developerExperienceContract.test.ts`；`pnpm docs:gen:check`；`pnpm docs:build`。CLI阻塞时不能宣称canonical门禁成功。
7. 执行父research/harness-checks.md列出的现有只读命令：五工具--version/--help、codex features list、grok inspect --json，以及明确路径的静态字段读取。按其中预期核对import、source/name、hook路径、工具/模型策略与Kimi文本；不用claude/codex agents冒充registry，不调用kimi --agent*或omp agents unpack。四套真实session发现、hook触发、最终模型/provider保留UNVERIFIED；Grok也不把inspect当执行通过。
8. 强模型独立复核五套规则无已知冲突、没有把提示词当sandbox、没有扩大REL/全局scope。
9. 父最终just ci、Trellis validate、git diff --check。完成后报告批准项落点与适用工具，未批准/未验证项单列。

父总验收前不提交/归档，不将规划写成已实施知识。
