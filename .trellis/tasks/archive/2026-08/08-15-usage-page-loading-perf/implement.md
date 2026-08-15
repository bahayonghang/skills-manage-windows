# Implement — Skill Usage 页加载性能优化

## Checklist（按序）

1. **基线测量**：本机临时计时（现有实现全量重扫墙钟 + 峰值内存粗测），写入任务 notes 供 AC1 对比。
2. **预过滤 + 流式（R1/R2/R3）**：
   - [ ] codex.rs：逐文件流式处理 + 行级子串预过滤（`<skill>` / `session_meta`），解析移入 blocking 闭包
   - [ ] grok.rs updates 路径：`<command-name>` 预过滤；droid.rs 评估同类
   - [ ] 用本机真实数据抽样验证过滤词命中率（漏报 = 0）
3. **批量 INSERT（R5）**：`replace_calls_for_target` 改 QueryBuilder 分块批插。
4. **migration 5（R7）**：新表 `skill_call_file_cache`；descriptor + checksum + fixture bump；`pnpm docs:gen`。
5. **增量扫描编排**：`services/usage/mod.rs` refresh 流程接入指纹 diff；缓存 upsert/清理；provider 接口按需扩展（collect 返回 calls 的同时输出文件指纹，或拆成 walk + parse 两段）。
6. **缓存优先返回（R4）**：命令层 + 事件 + `usageStore` 订阅静默刷新；首次扫描仍阻塞；remote 路径不动。
7. **测试**：等价性（增量 vs 全量 diff 为空）、指纹跳过/重解析/删除、迁移锁定、缓存优先命令行为。
8. **收尾**：`just ci`；AC1 前后对比数据记录到任务 notes。

## Validation

- `cargo test --locked usage` + `cargo test db::migrations --locked` + 前端 `pnpm test`（store 事件订阅）。
- `just ci`。

## Risky files / rollback points

- `providers/codex.rs`（过滤词漏报风险最高）——每个过滤词必须样本验证。
- `db/migrations`（checksum 锁定流程，严格按 database-migrations.md 第 6 节测试清单）。
- `commands/usage.rs` 命令语义变化——保持 remote 乐观路径与 `usedCachedData` 既有契约。
- 回滚：全部增量，无破坏性变更。

## Follow-up before task.py start

- 无。范围与方案已确认（低成本组合 + 增量扫描）。
