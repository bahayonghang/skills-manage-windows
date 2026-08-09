# 永久修复 Update Center Skills 范围重试冲突

## Goal

永久修复 Skills / Platform 范围 Update Center 清单在按仓库重试时产生重复条目、持久化失败并误报“资源冲突”的回归，同时保证升级前已持久化的清单无需人工清库即可恢复。

## Problem Statement

- Skills / Platform 范围生成的仓库型 `updatable` / `remote_missing` 条目可能缺少仓库归属。
- 仓库分片重试只按 `repository_id` 替换基线条目；旧条目归属为空时会被保留，再与新分片中的同一技能叠加，最终触发 inventory 唯一键失败。
- SQLite 唯一键文本经遗留 IPC 规则被归类为 `resource.conflict`，让内部清单不变量缺陷看起来像用户数据冲突。
- 已持久化的旧清单会在应用升级后继续存在，所以只修复新清单的生成逻辑不足以恢复首次重试。

## Requirements

1. 任意检查范围生成的仓库型可操作条目，都必须携带其技能分配记录中的权威仓库归属；Skills / Platform 不得因范围没有显式 `repository_ids` 而丢失归属。
2. 仓库分片重试必须完整替换目标仓库在基线中的 `updatable`、`remote_missing`、`remote_added` 和 `failed_repositories` 结果，不得产生同桶同技能重复项。
3. 对升级前 `repository_id = null` 的基线可操作条目，首次重试必须依据当前持久化的仓库成员关系识别目标条目；不得要求用户清空 inventory、编辑 SQLite 或重建 Central。
4. 当目标技能在重试时已经变为最新状态、因而不再出现在分片结果中时，旧的可操作条目必须被移除。
5. 无法用明确仓库归属或当前成员关系证明属于目标仓库的旧空归属条目必须保留，禁止按技能名称、URL 文本或路径猜测并删除。
6. Inventory 持久化必须继续执行严格唯一键约束；发现同一清单中重复的 `(bucket, entity_key)` 时应在写库前安全失败，不得使用覆盖插入或静默去重掩盖缺陷。
7. 该内部不变量失败必须通过稳定、脱敏、不可重试的 Update Center 错误码报告，不得再显示为 `resource.conflict`，不得泄漏技能 ID、仓库 URL、本地路径、数据库结构或凭据。
8. 现有 `repository_id: Option<String>` IPC / 持久化载荷保持向后兼容；本任务不引入数据库 schema 迁移，也不要求前端清理旧状态。
9. 常规模式对上游真实删除且无法唯一归位的技能，仍保持 `decision_required` 语义并交由用户选择保留或删除；本修复不得把真实删除误判为更新成功。
10. All / Repositories 范围、远端新增、自动归位、平台冗余扫描、清单整体替换和基线 mode / inventory id 语义不得回归。

## Constraints

- 不修改 Central 技能目录或用户现有数据库内容；测试只使用内存数据库和临时目录。
- 不通过 `INSERT OR REPLACE`、后写覆盖、按数组顺序取胜或任意 dedup 修复。
- 不扩大为全局 IPC 遗留错误映射重构；只收紧 Update Center inventory 的已知内部失败路径。
- 不新增生产依赖，不改变 Tauri command 签名，不改变发布或打包配置。
- 修复应位于 Rust service / repository 层，桌面端与未来共享入口均使用同一行为。

## Acceptance Criteria

- [x] Skills + Regular 基线中同一仓库包含一个可更新技能和一个远端缺失技能时，以 Sync 模式重试该仓库成功；结果无重复、可持久化并可从原 Skills inventory id 完整回读。
- [x] Platform + Regular 的等价场景成功，且保留该平台清单原有的平台扫描桶。
- [x] 新生成的 Skills / Platform `updatable` 与 `remote_missing` 仓库型条目均带正确 `repository_id`。
- [x] 模拟升级前 `repository_id = null` 的基线后，目标仓库重试会替换这些旧条目；不相关的空归属条目保持不变。
- [x] 旧空归属条目在目标技能已变为最新状态时被移除，不会残留为可更新项。
- [x] 合并结果满足每个 bucket 内 `entity_key` 唯一；持久化前置校验对人为构造的重复项返回稳定的 inventory invariant 错误，并保留此前已持久化的清单。
- [x] IPC 错误精确为约定的 `code / message / retryable=false`，Operation Log 只记录固定分类；响应和日志中不含原始唯一键文本或动态敏感内容。
- [x] 既有 repository retry、relocation、All / Repositories、scope filtering 与 inventory reload 测试保持通过。
- [x] Rust 格式、全 targets Clippy、锁文件测试和仓库 `just ci` 全部通过。

## Out of Scope

- 自动替用户决定 `archive-planning` 或其它上游已删除技能应保留还是删除。
- 修改全局 `legacy_plain_message` 对其它业务域历史冲突错误的兼容规则。
- 数据库 schema 迁移、清单批量清理命令、Windows 安装包或 updater 发布链路变更。

## Evidence

- 根因与可复现命令：`../archive/2026-08/08-08-diagnose-update-source-conflict/research/diagnosis.md`
