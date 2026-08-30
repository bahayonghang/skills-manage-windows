# Central 技能库重置以便重新导入

## Goal

给当前活动 target（本机 Local 与远程 SSH/WSL）提供一个显式、可确认的重置入口：只清掉 **没有受支持远端来源**（Update Center `unknown_source`）的 Central 技能，然后用户用现有 GitHub / Add Skill 重新导入，使这些技能带上 repository membership，能够检查更新。

用户价值：不必手工多选几十个无来源技能；重新导入后 Update Center 不再把它们堆在 Unsupported。

## Background

`unknown_source` 来自 `npx skills add` 一类只落到 Central 目录、没有写入 `skill_repository_members` 的安装。Update Center 按设计把它们标成 Unsupported。`Clear inventory` 只清检查结果，技能文件和 `skills` 行仍在，刷新后还会回来。

用户截图是 **远程 SSH** 上的 Update Center（Current results，Unsupported 54）。只读核对 `~/.skillsmanage/targets/ssh-9829f9ed-.../db.sqlite`：54 个 Central 技能、0 条 membership、54 条 `unsupported` inventory；`file_path` 均在远端 `~/.skillsmanage/skills/<id>/SKILL.md`。另一 SSH target 与 WSL、以及本机已修复的 GitHub 绑定库不在这次用户操作对象里，但功能必须在 Local 与 SSH 上都能用。

本机 `db.sqlite` 仍有 23 个无 membership 技能（同样是 `unknown_source`）。实现与测试**不得**拿这份已修复的本机仓库当活数据改写；产品功能本身不禁止用户以后在 Local 上执行重置。

## Confirmed Facts

- 工作仓库是 SkillPort（`skills-manage-windows`），不是 CCR。
- 删除范围已定为：仅无 repository membership 的 Central 技能（与 `UnsupportedSkillReasonCode::UnknownSource` 同一判定），不是清空整个 Central 库。
- 功能必须覆盖当前活动 target：Local 走现有本地删除，SSH/WSL 走现有 `delete_central_skills_remote_impl`。一次重置只动该 target 的 cache DB 与该 target 的 Central 根，不得串写其它 target。
- `Clear inventory` 只删 `skill_update_inventory_*` 与 `pending_additions`。
- 现有 `preview_delete_central_skills` / `delete_central_skills` 已按 `skill-deletion-integrity.md` 与 `fs-db-operation-journal.md` 处理 Central 文件、FK cascade、链接安装自动卸除、copy 安装可选删除，以及 Local/SSH/WSL 分流。
- `rebuild_startup_database` 只允许损坏库；健康库不得当重置用。
- GitHub 导入对已存在同名 Central 目录走 conflict；不先删目录就无法干净重建来源。
- 原生 `npx` 安装若只记为 `native` 安装行，现有删除不会把 Central 根以外的 agent 目录当 copy 删掉；Central 路径本身会进删除清单。SSH 上这 54 个文件就在远端 Central 根下。

## Requirements

### R1. 独立重置入口

- 保留 `Clear inventory` 原语义。
- 新增中英文动作，说明只删除无远端来源的 Central 技能，随后需重新导入。
- 入口放在 Update Center 的 Unsupported 场景（用户当前所在位置），Local 与 SSH 活动 target 下都可发现；不得做成启动页 rebuild。

### R2. 预览与确认

- 预览数据以 **当前 target 数据库** 为准：`is_central = 1` 且没有 `skill_repository_members` 的技能，而不是可能过期的 inventory 计数。
- 预览展示：将删除的技能数、自动卸除的链接安装数、可选 copy 安装数。
- 二次确认使用现有 Dialog，不用系统 `confirm()`。
- 取消后该 target 的磁盘与数据库不变。
- 失败用 `formatBackendError`；toast + 对话框内联错误；不含路径、token、SQL。

### R3. 删除走现有 Central delete

- 复用现有 batch-delete、FS+DB journal、target mutation lock。禁止 `DELETE FROM skills`、drop 数据库、`rebuild_startup_database`。
- 通过 `resolve_target_context()` 冻结 target 与 DB：Local → `delete_central_skills_impl`；SSH/WSL → `delete_central_skills_remote_impl`。
- 父行删除后 FK cascade 七张 owned 关系；空仓库 prune 沿用现有逻辑。
- 链接安装自动移除；copy 安装默认保留，预览里可勾选删除（与 BatchDelete 一致）。
- 已有 GitHub membership 的 Central 技能不得进入候选集。

### R4. 重置后视图一致

- 成功后清空**当前 target** 的 update inventory 与 pending additions。
- 刷新 Central 列表；被删技能消失。
- 随后 GitHub 导入这些 skill id 必须能写入 membership；Refresh 不得再因 `unknown_source` 列出它们。

### R5. 不得触碰的状态

- 整个 `db.sqlite` / target cache 文件、WAL、启动恢复备份
- GitHub PAT 与其它密钥
- Settings、agent 注册、remote target 配置、Marketplace 缓存
- 集合 / 标签 / saved view 的定义行
- 用量、operation logs、runtime logs
- **其它 target** 的 Central 文件与 cache DB（Local 重置不得改 SSH cache，反之亦然）

### R6. 可观察性

- Operation Log：动作名、当前 target kind（`local|ssh|wsl`）、删除技能数、成功/失败计数、失败时的稳定 error code。
- 不含绝对路径、仓库 URL、token、技能内容。

### R7. 测试隔离

- 自动化测试只用 `test_support` 的 `mem_pool` / `file_pool` / `FakeRunner`，或一次性 TempDir。
- 禁止打开或改写开发者本机 `~/.skillsmanage/db.sqlite`、`~/.skillsmanage/skills/`，以及已有 SSH/WSL target cache。本机仓库已修复，测试不得拿它当夹具。

## Out of Scope

- 启动页 rebuild、checksum 兼容、provenance 回填
- 按名字猜测 GitHub 仓库并自动绑定 membership
- 修改 GitHub archive / Update Center 检查协议
- 修复 Current results 过期 inventory 显示问题（重置后以 DB 候选集为准即可）
- 修复 `claude-md-improver` 源路径缺失的 Failed 仓库
- 清空整个 Central 库或卸载全部平台 copy
- CCR 仓库改动

## Acceptance Criteria

- [ ] AC1: `Clear inventory` 行为不变。
- [ ] AC2: 预览列出的技能 = 当前 target 上无 membership 的 Central 技能；有 GitHub membership 的技能不出现。取消后无 FS/DB 变化。
- [ ] AC3: Local 活动 target 确认后，只删除该 Local Central 根下的候选技能及 owned relations；SSH cache DB 与远端 `~/.skillsmanage/skills` 不变。
- [ ] AC4: SSH 活动 target 确认后，只删除该 SSH Central 根下的候选技能及该 target cache 行；本机 Local `db.sqlite` 与本机 Central 目录不变。
- [ ] AC5: 链接安装自动卸除；未勾选的 copy 安装保留。
- [ ] AC6: 设置、PAT、agent、Marketplace 缓存、db 文件本身仍在。
- [ ] AC7: 成功后当前 target 的 Unsupported 不再包含已删技能；再 Refresh 不会因旧 inventory 把它们变回来。
- [ ] AC8: 对已删 skill id 做 GitHub 导入后有 membership，不再被标成 `unknown_source`。
- [ ] AC9: 失败只显示稳定 i18n code；部分失败可区分已成功与失败项。
- [ ] AC10: 定向 Rust（Local fixture + Fake SSH）与 Vitest 覆盖预览/确认/拒绝取消/跨 target 隔离；测试不触碰开发者本机仓库。相关门禁通过。

## Notes

- 证据：SSH `ssh-9829f9ed-11c5-4dac-9b87-845190e90189` 与截图 54 条 Unsupported 一致；本机 23 条 `unknown_source` 是同类 npx 残留，功能可用，但测试与本次人工验证不要改那份已修复本机库。
- 关键现有能力：`preview_delete_central_skills_*`、`delete_central_skills_*`、`clear_skill_update_inventory_impl`、`BatchDeleteCentralSkillsDialog`、`UnsupportedTabPanel`。
- 规范：`skill-deletion-integrity.md`、`fs-db-operation-journal.md`、`target-context.md`、`central-mutation-lock.md`、`startup-recovery.md`、`test-support.md`、`async-error-feedback.md`、`redaction-policy.md`。
