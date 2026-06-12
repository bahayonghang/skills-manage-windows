# PRD：CLAUDE.md 架构章节重写（子任务 B）

> 父任务：`06-11-analysis-driven-fixes` ｜ 执行顺序：第 6 位（最后）
> 依赖：必须在 A、C1–C3 全部完成后执行，以记录改造后的真实现状，避免文档二次返工。
> 来源：分析报告条目 #3

## Goal

消除 CLAUDE.md 与代码现状的漂移，使其重新成为 AI 协作的可靠事实依据。

## Requirements

1. 重写「架构概述」：按 commands（IPC 壳层）→ services（12 个业务域）→ db/repos（18 个数据访问模块）三层结构描述后端。
2. 更新「IPC 命令模块」表：覆盖实际全部 24+ 命令文件（含 central_updates、targets、usage、tag_groups、saved_views、portable_state、bootstrap、github_import、logs 等），命令总数按当时实际统计（当前为 171）。
3. 修正主题系统描述：6 套主题（mocha/macchiato/frappe/latte/claude-light/claude-dark）+ accent 体系，并补充 `@custom-variant dark` 机制与 `statusTone.ts` 的禁用 `dark:` 二元适配约定。
4. **新增错误处理约定章节**：记录 C1–C3 落地后的域错误枚举模式（thiserror、`#[from]`、IPC 边界字符串转换），替换隐含的「字符串错误」旧约定。
5. 补充 spawn_blocking 约定：重 IO 必须经阻塞包装（指向共享 fs_util 工具）。
6. 校对其余既有条目（路由表、页面布局、代码约定）与现状的一致性，逐条核实后保留或修正。

## Acceptance Criteria

- [ ] CLAUDE.md 中不存在分析报告条目 #3 列出的任何一条漂移描述。
- [ ] 新增的错误处理与 spawn_blocking 约定与实际代码模式一致（抽查 3 个域核对）。
- [ ] 文档遵循中文技术文档排版规范（中英文空格、全角标点）。
- [ ] 用户 review 通过。
