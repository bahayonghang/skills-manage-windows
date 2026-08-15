# Design — Skill Usage 页加载性能优化

## 变更点总览

1. **Provider 预过滤（R1）**：codex 解析前只保留含 `<skill>` 或 `session_meta` 的行；grok updates.jsonl 只保留含 `<command-name>` 的行；droid 同理评估。过滤词必须对照真实样本验证（本机数据可直接抽样 diff：旧解析结果 vs 新解析结果逐条相等）。
2. **流式读取（R2）**：`codex.rs:88-89` 不再 `read_many_to_strings` 物化全部文件到 HashMap；改为逐文件（walk 到一个处理一个）读取→过滤→解析→释放。峰值内存 ≈ 最大单文件（254 MB）。
3. **解析入 spawn_blocking（R3）**：每个 provider 的 collect 主体包进 `tauri::async_runtime::spawn_blocking`（IO 已在那里，把 CPU 解析也挪进去），`join_all` 的并发才真正跨线程。
4. **缓存优先返回（R4）**：`commands/usage.rs` 的 `usage_refresh`：本地 target 存在旧缓存（`last_scan_ms` 非空）但过期时，立即用现有缓存 `build_refresh_page` 返回（响应标记 `scanning: true` 之类），并 spawn 后台重扫；完成后 emit 事件（如 `usage://scan-completed`，带 target id）。前端 `usageStore` 订阅该事件→ `refresh(false)` 静默重取。首次扫描（无任何缓存）维持阻塞。remote 乐观路径不动。
5. **批量 INSERT（R5）**：`usage_repo.rs:142-155` 逐行 INSERT 改 `sqlx::QueryBuilder` 多行批插（注意 SQLite 变量上限，分块）。
6. **增量扫描（R7）**：
   - **migration 5**（`database-migrations.md`）：新表
     `skill_call_file_cache(target_id, provider, file_path, mtime_ms, size, calls_json, scanned_at_ms, PRIMARY KEY(target_id, provider, file_path))`。
     calls_json = 该文件解析出的 `SkillCall[]` 序列化。同步 bump future-version 预检 fixture；`pnpm docs:gen` 更新 data-model 文档。
   - 扫描流程：provider collect 前先取该 provider 已有指纹 map → walk 文件列表，mtime+size 未变 → 直接取缓存 calls_json 反序列化；变化/新增 → 读盘解析并 upsert 缓存；已消失文件 → 删缓存行。
   - 合并：`skill_calls` 仍按 provider DELETE+INSERT 全量替换（语义不变、行级 dedup 规则不变），数据来源 = 缓存 ∪ 新解析。
   - 契约红线：`skill_calls` 不放文件路径（`skill-usage-analytics.md:33`）；文件路径只在派生缓存表，不进日志/导出（`redaction-policy.md` 需读）。
   - 首次运行（缓存表为空）= 全量扫描并建立缓存。

## 数据流（改动后）

```
provider walk → 指纹 diff（skill_call_file_cache）
   ├─ 未变文件 → 缓存 calls_json 反序列化（零 IO）
   └─ 变化/新增 → spawn_blocking 流式读取 + 子串预过滤 + 解析 → upsert 缓存
        ▼
每 provider 合并 calls → DELETE+批量 INSERT skill_calls（事务不变）
        ▼
enrichment（不变）→ build_refresh_page
usage_refresh：有过期缓存 → 立即返回缓存页 + 后台重扫 → 事件 → 前端静默刷新
```

## 兼容性 / 回滚

- migration 5 纯增量新表；回滚 = 代码回退（表残留无害）。
- `usage_refresh` 响应新增字段为可选/默认 false，旧前端兼容。
- 行为等价验证：本机跑一次旧实现全量 vs 新实现增量，diff `skill_calls` 全表为空（测试内做临时表对比）。

## 测试

- 预过滤等价：构造含/不含关键词的行样本，断言解析输出与无过滤版一致（fixture 级）。
- 指纹：mtime 不变跳过读盘（FakeRunner/inject 文件读取计数或临时目录真实文件）；mtime/size 变化触发重解析；文件删除清除缓存行与对应 calls。
- 合并等价：增量结果 == 全量结果。
- migration 5：descriptor checksum 锁定、fixture bump、`docs:gen:check`。
- 缓存优先：有过期缓存时命令立即返回且 `scanning=true`；后台完成后事件发出。
