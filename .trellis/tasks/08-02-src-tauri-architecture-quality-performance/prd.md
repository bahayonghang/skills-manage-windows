# src-tauri 架构、质量与性能优化总纲

## Goal

在 `dev@b242ed92` 上对 `src-tauri` 做增量深审计，识别 2026-07-24 系统整改之后仍存在的安全、正确性、资源生命周期、数据库一致性和查询性能缺口，并把每个可独立验收的交付项拆成 Trellis 子任务。本父任务只保存需求源、任务地图和跨子任务验收，不直接修改产品代码。

完整证据见 `research/src-tauri-deep-audit.md`。

## Requirements

1. 所有结论必须能回指当前代码或活动 spec；不得用文件大小、`*_impl` 数量或测试数量单独推导缺陷。
2. 复用已经成熟的 `github_import`、Central mutation lock、FS+DB operation journal、`TargetContext`、`SecretStore`、process supervision 和 `ResourceBudget`，禁止另建平行实现。
3. 将外部可控的路径、网络响应、文本和缓存视为不可信输入；在分配或写入之前验证，而不是事后检查。
4. 多语句 DB mutation 必须明确原子性边界；批量 API 要么全部提交，要么全部回滚，除非 API 明确建模为 partial result。
5. 性能优化必须先记录基线，并以资源上限、读取/富化行数、查询计划和行为等价性为主要证据；不以单次 wall-clock 数字作为唯一门禁。
6. 共享 Central、SQLite schema、CLI 和 Tauri command 契约保持兼容。涉及 schema 或 command 时同步生成架构文档并执行现有 contract gates。
7. 本轮只完成规划。父任务及全部子任务保持 `planning`，等待用户明确批准后再对具体 child 执行 `task.py start`。

## Task Map

| 子任务 | 优先级 | 交付边界 | 依赖 |
| --- | --- | --- | --- |
| `08-03-marketplace-install-central-contract` | P0 | 修复 Marketplace 安装的路径越界与 Central 安装契约绕过 | 首先实施 |
| `08-03-bounded-github-snapshot-lifecycle` | P1 | 约束 Central update cache、preview registry 和远端 workspace 生命周期 | 可独立实施 |
| `08-03-bounded-external-text-ingestion` | P1 | 为 AI、Git tree、SKILL.md 等输入补预算、期限和 UTF-8 安全截断 | Marketplace 部分后于 P0 子任务 |
| `08-03-sql-central-pagination` | P2 | 将 Central filter/sort/count/page 下推 SQLite，只富化当前页 | 可独立实施，先建基线 |
| `08-03-transactional-metadata-mutations` | P2 | 为 metadata/cache 多语句 mutation 建立事务和 stale-row 清理 | Marketplace install 路径不在其范围 |

## Cross-Child Acceptance Criteria

- [ ] P0 安装任务证明恶意 frontmatter name 不能写出 Central 根，Local/SSH/WSL 均通过同一受控 use case 安装完整技能目录，并且失败不写 installed 状态。
- [ ] 两类 GitHub snapshot 状态都有明确 entry/byte 上限；过期或被淘汰的远端 workspace 只由其 owning target 释放，活跃 import lease 不被回收。
- [ ] 所有纳入范围的 HTTP body、SSE、主 `SKILL.md` 和 scanner/AI 文件读取在完整分配前受限；多字节 UTF-8 截断不 panic。
- [ ] Central page 对筛选、排序、total、offset/limit 与特殊标签/安装状态的行为有等价性测试；当前页最多富化 `limit <= 500` 行，大 fixture 不再走全量 enrichment。
- [ ] metadata/cache 批量写入在混合有效/无效输入和故障注入下全回滚；成功 Marketplace sync 原子替换该 registry 的缓存并删除 stale rows。
- [ ] 所有子任务分别通过定向测试、Rust fmt、all-targets locked Clippy、locked Rust tests 和最终 `just ci`；schema/IPC 改动同时通过生成文档检查。
- [ ] 父任务最终复核任务间没有重复 helper、锁顺序冲突、资源预算漂移或文档与代码不一致。

## Non-Goals

- 不重写整个 `src-tauri`，不因 38 个 command-side `*_impl` 或 broad `db::*` re-export 单独发起全仓搬家。
- 不重复 2026-07-24 已完成的 SSRF、remote canonical path、process supervision、operation journal、target snapshot、typed IPC、startup recovery 等整改。
- 不改变产品功能、IPC 名称、用户数据格式或发布流程，除非某个子任务的设计明确证明这是完成其安全/正确性验收所必需。
- 不在规划轮运行网络写操作、生产 apply、PR、push 或 commit。
