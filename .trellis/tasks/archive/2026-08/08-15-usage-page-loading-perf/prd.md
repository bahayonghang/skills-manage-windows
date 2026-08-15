# Skill Usage 页加载性能优化

## Goal

消除 Skill Usage 页（尤其首次访问 / 缓存过期后）长时间卡在 "Scanning session logs across platforms…" 骨架屏的问题，让页面快速可用。

## Background / Confirmed Facts（代码调查结论）

- 现象：每次应用启动后首次进 `/usage` 都阻塞等待完整扫描（store 仅内存态，`overview===null` 即骨架屏；`commands/usage.rs:157` await 整个 `usage::refresh`）。前端 TTL 与后端 `CACHE_TTL_MS` 均为 5 分钟，过期后再访问也会后台全量重扫。
- 本机实测数据规模：codex 5.2 GB / 2554 文件（最大单文件 254 MB）、claude 406 MB、grok 314 MB、opencode 111 MB SQLite。每次非缓存刷新重读+重解析全部 ≈5.9 GB。
- **根因排序**：
  1. codex provider 无子串预过滤，`serde_json::from_str::<Value>` DOM 解析每一行（`providers/codex.rs:106-155`），且先把全部文件内容物化进一个 HashMap（`codex.rs:88-89`，峰值内存 ≈5.2 GB）→ 单核 20–60 s + 换页压力。grok updates.jsonl 同样无预过滤（`grok.rs:179`）。
  2. 8 个 provider 虽 `join_all`（`services/usage/mod.rs:262-279`），但 CPU 密集解析全在同一个 async worker 上串行执行，扫描期间拖累所有 IPC。
  3. 无增量扫描：`skill_call_scan_state` 只存 `last_full_scan_ms`（`db/schema/usage.rs:96-102`），每次 DELETE 全表再逐行 INSERT。
- 文件 IO 已合规走 `spawn_blocking`（`fs_util.rs:16-30`）；解析未走。
- 新增 `usage_get_unused_skills` 为纯 SQL（<50ms），非瓶颈。

## Requirements

- R1: codex/grok/droid provider 在 JSON 解析前加廉价子串预过滤（codex: `<skill>` / `session_meta`；grok updates: `<command-name>`），跳过不可能匹配的行。
- R2: codex 读取改为流式（逐文件/有界块），峰值内存从 ≈5.2 GB 降至最大单文件量级。
- R3: 扫描解析移入 `spawn_blocking`，不占用 async runtime worker（遵循 `spawn-blocking-io.md`）。
- R4: 感知延迟：存在旧缓存（即使过期）时先返回缓存页，后台重扫，完成后经事件通知前端刷新（复用 `usage://target-changed` 类 plumbing 或新增事件）；仅历史上首次扫描允许阻塞。
- R5: `replace_calls_for_target` 的逐行 INSERT 改为批量多行插入。
- R6: 不改变任何扫描结果的语义与口径（同样的调用事实、同样的 dedup 规则）。
- R7（已确认纳入）: 增量扫描——新增 `skill_call_file_cache` 表（migration 5），按 (provider, file_path, mtime_ms, size) 指纹跳过未变文件；按 provider 全量替换合并语义；稳态重扫不重复读盘。

## Acceptance Criteria

- [ ] AC1: 本机（5.9 GB 日志）全量重扫墙钟时间从基线（先测）降至个位数秒级；有基准前后对比记录。
- [ ] AC2: 扫描期间峰值内存降到 GB 级以下（不再物化全部文件内容到单 HashMap）。
- [ ] AC3: 应用启动后首次之外、缓存过期访问 `/usage`：页面立即展示缓存数据，后台扫描完成后自动更新，无长时间骨架屏。
- [ ] AC4: 全部现有 usage 测试保持绿；新增预过滤/流式解析/增量合并的行为等价测试（增量结果与全量结果 diff 为空）。
- [ ] AC5: `just ci` 通过（含 migration 5 的 checksum/fixture 锁定测试与 `docs:gen` 生成物）。

## Out of Scope（本期）

- opencode SQLite 表达式索引优化（0.5–3 s，非主要矛盾）。
- 前端 TTL 策略调整。

## Key Decisions（已确认）

- D1（用户已确认）: 本期范围 = R1–R5 **+ R7 增量扫描**（低成本组合 + 结构性方案一起做完）。
- D2: 增量扫描不碰 `skill_calls` 的事实纯度契约（表内不放文件路径）——文件级状态与缓存放独立新表 `skill_call_file_cache(target_id, provider, file_path, mtime_ms, size, calls_json)`，经 migration 5 引入（遵循 `database-migrations.md`：append-only、不可变源、checksum 锁定、fixture 同步 bump）。
- D3: 增量合并语义 = 按 provider 全量替换 `skill_calls`（DELETE 该 provider 行 + 批量 INSERT 合并后结果），未变文件的 calls 从缓存表读出，变化/新增文件才读盘解析；被删文件的缓存行清除。稳态重扫 ≈ 0 字节读取。

## Risks

- 预过滤子串选错会漏数据 → 每个 provider 的过滤词必须来自该格式的真实样本验证 + 等价性测试（全量重扫结果与旧实现 diff 为空）。
- 缓存优先返回改变 `usage_refresh` 命令语义（新增"后台扫描中"状态）→ 需保持 remote target 既有乐观路径不变。
- 增量缓存表引入首个含文件路径的 usage 侧表 → 仅作派生缓存，不进导出/日志（遵循 redaction-policy）。
