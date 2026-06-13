# 更新中心

更新中心（Update Center）是刷新远端状态、应用升级、清理平台冗余副本和处理孤立本地副本的统一入口。它把原来的三个独立 dialog —— **检查更新**、**远端新增 / 删除**、**仓库同步** —— 合到一个屏幕里，按五类分桶展示全部待处理变更。

## 什么时候用

- 导入一个 GitHub 仓库之后，确认上游新增或删除了哪些 skills。
- 怀疑某个 skill 上游已经改过，但本地卡片还没出现更新 badge。
- 手工改过中央库或平台目录之后，确认两边是否还对得上。
- 一次性 review 大量决策，再统一提交。

## 打开方式

在 `/central` 顶栏点 **更新中心**。过渡期内旧的 **检查更新** 按钮仍然可用，两条路径写入的是同一个中央库，但新 dialog 在一个屏幕里覆盖了所有流程。

PlatformView 的 **扫描重复** 按钮也会跳到这里，并直接定位到 **平台冗余** Tab，不需要再单独维护那一个 dialog。

## 刷新与应用强分离

刷新只读。手动刷新会绕过内存里的 GitHub snapshot cache，按 scope 拉每个 GitHub 仓库的最新远端树，和本地中央库、平台安装的内容 hash 做 diff，把结果写到 inventory 表里。**刷新过程不会改动磁盘上的任何文件**，可以随便点。

应用才真正改中央库和平台 symlink / copy。五个 Tab 的勾选会合并成一次提交，再按固定顺序执行，保证同一个 skill id 的新增 / 删除 / 更新 / 冗余清理不会互相覆盖。

这一拆是有意为之。浏览 inventory 永远不该顺手写磁盘，写磁盘也不应该藏在看似只是“刷新”的按钮后面。`skill_update_states` 只作为成功应用 / 更新后的已安装基线；刷新结果写入独立的 update inventory。

更新检测以内容 hash 为准。`SKILL.md` 里的 `version` 字段只作为可展示的诊断元数据；没有版本号也能检测更新，版本号变化也不会覆盖“内容 hash 相同”的判断。

## scope

刷新按钮旁边的下拉决定本次刷新覆盖哪些范围。

| Scope | 含义 |
|-------|------|
| 全部 | 所有中央 skills 加所有已登记的 GitHub 仓库。 |
| 当前仓库 | 仅当前 Central 视图过滤所涉及的仓库。 |
| 当前结果 | 仅当前搜索 / facet 过滤后看到的 skills。 |

scope 在 session 内保持。刷新按钮旁会显示当前 scope，避免“窄范围刷新”被误读成 inventory 是空的。

## 五个 Tab

每个 Tab 是一类待处理变更。Tab 标题上的计数反映 inventory 长度，而不是当前勾选数。

### 可更新

上游内容相对上一次成功同步发生变化的 skills。每行展示来源仓库和新的更新时间。勾选要更新的行，apply 阶段会拉取新的 SKILL.md 树，并刷新所有跟随中央库的平台副本。

### 远端新增

上游仓库新增、本地中央库还没有的 skills。每行支持三种逐条决策：

- **覆盖** —— 仅在 id 与现有中央 skill 冲突时可选，会替换原有目录。
- **重命名** —— 用另一个 id 保存新树。
- **跳过** —— 不动中央库。

行的 id 与现有中央 skill 冲突时，默认决策是 **跳过**，避免误 apply 抹掉本地修改。

### 远端删除

上游仓库已经删除、但本地中央库还保留的 skills。每行支持两种决策：

- **保留** —— 解除远端来源关联，保留本地文件。用于上游放弃维护但本地仍要继续使用的场景。
- **删除** —— 从中央库与所有已链接平台移除。

### 平台冗余

同一个 skill id 在同一平台上既有插件只读副本（例如 `~/.claude/plugins/marketplaces/...`），又有手工安装的可写副本。这一 Tab 用于挑出要删除的可写路径。只读插件副本会列出来提供上下文，但不能从这里删除。

这一 Tab 与 PlatformView 的 **扫描重复** 共用同一份数据。

### 失链孤儿

预留给指向已删除中央目录的悬空 symlink。检测能力暂未实现，先把 Tab 占位放在这里，保证布局和决策控件在后续版本之间不漂移。

## 持久化 inventory

刷新结果会写到本地 SkillPort 数据库里，关 dialog 再开数据还在，应用重启也不丢。inventory 按 scope 区分存储，做 **当前结果** 的刷新不会覆盖之前 **全部** 刷新留下的其他桶数据。

页脚的 **清空 inventory** 按钮会丢弃持久化的刷新结果。大规模重组之后用一下，避免遗留条目影响判断。**清空只是重置 checklist，不会删除任何 skill 或平台副本。**

## 强制恢复动作

当普通检测链路疑似出错时，更新中心还提供显式恢复动作。

- **强制更新** 会从已跟踪的 GitHub 远端路径覆盖选中的 Central skills。它会绕过 snapshot cache，即使本地和远端内容 hash 相同也会覆盖，并刷新关联的 copy 安装。
- **强制镜像仓库** 只在仓库 scope 下可用。它会绕过 snapshot cache，覆盖已跟踪 skills，导入远端新增，删除远端缺失的本地已跟踪 skills，并删除这些被删 skills 的 copy 安装。

强制镜像是破坏性操作，不会在启动、被动刷新或普通应用流程中自动执行。界面会在执行前要求确认。

## 幕后变化

- Tauri 命令：`refresh_skill_update_inventory`、`apply_skill_update_decisions`、`clear_skill_update_inventory`、`get_skill_update_inventory`、`force_update_central_skills`、`force_mirror_central_repositories`、`scan_platform_duplicate_skills`。
- DB 表：`skill_update_inventory_runs`、`skill_update_inventory_entries`，以及兼容保留的 `skill_repository_pending_additions`。新增字段：`skill_repositories.last_synced_at`。
- 后端 `SkillUpdateStatus` enum 取代原来的字符串常量。
- 旧的 `check_central_skill_updates`、`check_central_repository_sync`、`apply_central_repository_sync` 命令仍保留兼容性，会在下一个 minor release 之后删除。

## 从旧版“检查更新”迁移

新旧入口都可用。如果现有工作流依赖旧按钮就继续用；更新中心覆盖了同样的检查，并把原本散落在三个 dialog 里的 **远端新增**、**远端删除**、**平台冗余** 一起收进来。等更新中心的反馈稳定后，旧按钮会被下线。

## 下一步

- 中央库背景与搜索语法：[中央技能库](./central-skills)。
- 平台可写副本所在位置：[平台](./platforms)。
- GitHub 仓库如何进入中央库：[GitHub 导入](./github-import)。

---

Last reviewed: 2026-06-11
