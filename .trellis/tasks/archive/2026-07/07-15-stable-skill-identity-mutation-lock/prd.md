# 引入稳定技能身份与中央库并发锁

## Goal

为现有技能记录增加不可变 `uid`，使外部 API/CLI 引用不再绑定目录 slug；同时为同一台机器上的 SkillPort GUI 与 CLI 提供统一的 Local 中央库跨进程 mutation 锁，避免并发 rename/swap/delete 导致丢失更新。

## Background

- `src-tauri/src/services/scanner/mod.rs:47-63`、`:207-223` 证明当前 `skills.id` 由目录名规范化得到。
- `src-tauri/src/db/schema/core.rs:18-45` 证明现有 `skills.id` 同时是主键和安装关系键；直接改成 UUID 会扩大到扫描、安装、观察、集合、标签、来源和前端状态等多层。
- `src-tauri/src/services/central_updates/fs.rs:209-282` 已提供单进程原子目录替换与失败回滚，但当前没有 GUI/CLI 共用的跨进程总锁。
- 用户已排除 Git 备份与多设备合并，因此本任务采用 additive `uid`，保留现有 `skills.id` slug 兼容键，不进行全库主键替换。

## Requirements

- **I1 Schema**：为每个 `skills` row 提供唯一、非空、创建后不可变的 UUID `uid`；旧 DB 启动时幂等 backfill，新写入必须一次生成并持久化。
- **I2 Compatibility**：保留 `skills.id`、中央目录名和现有 relation key 语义；新增 `uid` 不得改变已安装路径、collections/tags、repository assignment、update state 或 frontend 展示。
- **I3 Preservation**：scan upsert、GitHub/skills.sh overwrite、central update、store relocation 和 portable-state import 若更新同一现有实体，必须保留原 `uid`；只有新实体生成新值。
- **I4 Resolver**：提供单一 skill resolver，确定性支持精确 `uid`、精确现有 `id`/slug 和唯一 name；多 name 匹配必须返回 typed ambiguity error。
- **I5 DTO**：中央技能的 backend/frontend/portable-state DTO 增量暴露 `uid`；旧 portable-state 输入没有 `uid` 时继续可导入。
- **I6 Lock**：在 app-data 固定路径提供跨进程 `CentralMutationGuard`；同机 GUI/CLI 的 Local central filesystem mutation 必须共享该锁。
- **I7 Lock Scope**：网络下载、archive inspection 和纯查询在锁外；锁内必须重新校验目标状态并完成原子 filesystem apply、对应 DB persist 和必要 operation log。
- **I8 Failure**：锁支持有界等待并返回 typed busy/timeout；崩溃由 OS 释放锁；任何错误都不得退化成无锁写入。
- **I9 Coverage**：盘点并接入至少 GitHub/skills.sh import、centralize/install、central update、central delete、portable-state import 和 central-store relocation 的 Local 最终写入边界。
- **I10 Platform**：Windows、macOS、Linux 行为一致；Windows 路径必须使用 `paths.rs`，锁测试必须包含独立进程竞争。

## Acceptance Criteria

- [x] Fresh DB 与至少一个旧 schema fixture 均得到唯一非空 `uid`，重复初始化不改变既有值。
- [x] 同一技能经过 scan、update、overwrite import 和 central relocation 后 `uid` 不变。
- [x] 新技能即使 slug/name 相同决策发生变化，也不会复用已删除实体的 `uid`。
- [x] resolver 的 uid/slug/name/ambiguity/not-found 行为有单元测试，并可被 CLI façade 复用。
- [x] 旧 portable-state manifest 可导入；新导出包含 `uid`，冲突时不会无条件覆盖已有实体身份。
- [x] 两个独立进程竞争同一 mutation lock 时仅一个进入临界区，另一个得到稳定 timeout 错误；进程终止后锁由 OS 释放。
- [x] 所有列出的 Local 中央写入入口均接入共享锁；纯查询、远端 mutation 和网络准备不持 Local 锁。
- [x] `db`、`scanner`、`central_skills`、`github_import`、`installation`、`central_updates`、`portable_state`、`central_store_location` 定向测试通过。
- [x] `just ci` 通过且未新增 Git backup/snapshot/merge 代码。

## Out Of Scope

- 把 `skills.id` 主键和所有 relation key 改成 UUID。
- 自动识别用户绕过 SkillPort 执行的任意外部目录 rename。
- 跨不同机器协调同一 SSH/WSL target 的分布式锁。
- Git repository、backup、snapshot、restore 或 multi-device merge。

## Dependency

本任务必须在 `07-15-shared-core-cli` 的 mutation 命令验收前完成。CLI 不得自建第二套 resolver 或 lock。
