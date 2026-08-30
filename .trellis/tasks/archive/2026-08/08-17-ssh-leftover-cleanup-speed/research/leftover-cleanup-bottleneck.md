# SSH Platform leftover 清理速度分析

日期：2026-08-17  
现场：Update Center「Platform leftovers (309)」+「Cleaning leftovers…」；路径形如 `/home/lyh/.agents/skills/<skill>`（SSH remote target）。

## 1. 用户动作对应的代码路径

一键清理走前端 `UpdateCenterDialog.handleCleanAllDeletedPlatformCopies`：

1. 用当前 inventory 构造 **只含** `removeDeletedPlatformCopies` 的 `SkillUpdateDecisions`（`buildDeletedPlatformCopyCleanupDecisions`）。
2. 调用 `updateCenterStore.apply` → IPC `apply_skill_update_decisions`。
3. 后端 `apply_skill_update_decisions_impl` 顺序执行 7 步；leftover 是步骤 7：`apply_remove_deleted_platform_copies_step`。
4. apply 返回后前端再 `loadInventory`。`get_skill_update_inventory` 会 **重新扫描** leftover（`view.rs`），不是只读上次 persist 的 leftover 列表。

「Cleaning leftovers…」只是 `isCleaningLeftovers` 开关，没有已完成/总数，也没有取消。

## 2. 远端删除现状（主导瓶颈）

`apply_steps.rs` 对每条 leftover **路径**串行：

```text
for removal in removals:                  # 一个 (agent_id, skill_id)
  ensure_central_still_missing(skill_id)  # 每次 DB
  get_agent_by_id(agent_id)               # 每次 DB
  connect_remote_target(active_target)    # 每次新建 ConnectedSshTarget
  connection.remove_tree(path)            # 每次新 ssh.exe：rm -rf -- <path>
  delete_skill_installation(skill, agent) # 只删 installation，不删 observation
```

关键事实：

| 事实 | 位置 | 含义 |
| --- | --- | --- |
| `ConnectedSshTarget` 不是持久会话 | `targets/askpass.rs` `open_ssh_target`；`07-11-remote-update-performance` PRD | `connect_remote_target` 只组装命令模板。每次 `run_command` 都新启动 `ssh.exe` / `wsl.exe` 并完整握手。 |
| `remove_tree` = 一条远端命令 | `targets/exec.rs:444` | `rm -rf -- <quoted path>`，policy 为 `ProcessPolicy::standard()`（120 s）。 |
| leftover 步骤不复用 `InstallTransport` | `apply_steps.rs:334` | install/uninstall 家族已按 transport seam「打开一次、循环复用」。leftover 仍每条自连。 |
| leftover 步骤不读 apply 的 `cancel` | `apply_skill_update_decisions_impl` 有 `AtomicBool`，步骤 7 未传入 | 清理中无法取消。 |
| leftover 步骤无 progress 事件 | 对比 `update_central_skills_impl` | 309 条时 UI 只显示转圈。 |
| 远端 leftover **没有** FakeRunner 测试 | `inventory/tests.rs` 仅 Local | 远端进程数回归目前测不到。 |

握手量级（沿用 `plans/ssh-perf` 与 07-11 记录）：单次 `ssh.exe` 约 0.5–2 s。309 次串行即约 **3–10 分钟**，与「非常慢」一致。目录很大时 `rm -rf` 本身还会叠加。

## 3. 309 条的构成

leftover 分组键是 `(agent_id, skill_id)`，不是物理路径。

- 10 个 Universal Agents 共享同一目录 `~/.agents/skills/`：`amp`、`cline`、`codex`、`cursor`、`deep-agents`、`firebender`、`copilot`、`kimi-code-cli`、`opencode`、`warp`（`db/types.rs` `UNIVERSAL_AGENT_IDS`）。
- 卡片标题「ask-matt in amp」+ 路径 `/home/lyh/.agents/skills/ask-matt` 符合该共享根。
- 分组按 `agent_id` 再 `skill_id` 排序，`amp` 会排在最前。截图只看到 amp，不能排除后面还有 cursor/codex 等同路径组。
- `countDeletedPlatformCopyPaths` **不跨组去重**。10 个平台各有 1 条相同路径，计数为 10。
- 309 ≈ 10 平台 × 约 31 个已从 Central 消失的 skill。这是最吻合的库存形状；未在用户机器上点验。

因此一键清理会把 **同一物理目录 `rm -rf` 约 10 次**，每次一次完整 SSH 握手。

## 4. 扫描侧（不是本次转圈的主因）

`scan_deleted_platform_copies_with_pool` 只读 DB：`agent_skill_observations` + 无 Central 行的 `skill_installations`。它 **不** 列远端目录。

但扫描用本机路径语义：

- `is_candidate_entry_deletable_shape` 调 `std::fs::symlink_metadata(path)`。Windows 主机上的 POSIX 路径几乎总是 `NotFound`，而 `NotFound` 被当成可删，所以远端 leftover 一律入列。
- `is_candidate_path_within_agent_root` 用 `Path::canonicalize` + `starts_with`。这是 Local 语义；远端 apply 另有 `ensure_remote_child_path`。

apply 结束后 `loadInventory` 会再跑上述扫描。远端清理只删 `skill_installations`，**不删 observations**。Local 路径走 `uninstall_skill`，会删 installation + observation。

结果：

1. 远端文件已删。
2. observation 仍在。
3. 本机 `symlink_metadata` → `NotFound` → 仍算 leftover。
4. 弹成功 toast 后，列表可能马上回到接近 309。

这会让用户再次点清理，再付一次 309 次 SSH 成本。

## 5. 与已有优化的关系

| 已有工作 | 对 leftover 的覆盖 |
| --- | --- |
| `07-08-one-click-leftover-cleanup` | 只加了一键入口，复用原 apply 串行删除。 |
| `plans/ssh-perf` / ADR-001 russh | 扫描向方案；未落地。ControlMaster 已因 Windows OpenSSH 否决。 |
| `07-11-remote-update-performance` | Central **更新** 写入按 16 分块、copy refresh 按 32 分块并去重。leftover 删除未纳入。 |
| `.trellis/spec/backend/central-update-batching.md` | 禁止「每个 skill 一个 ssh.exe」。leftover 步骤违反该精神。 |
| `InstallTransport::for_target` | leftover 未使用。注释写明 Remote 对象可在整次循环复用，但复用对象 **不能** 省握手。 |

## 6. 优化选项

### A. 按物理路径去重后再串行 `remove_tree`（不够）

309 → 约 31 次 SSH。仍可能 15–60 s。不修 observation 时，成功后 leftover 仍会回来。

### B. 推荐：先校验，再一次远端脚本删全部唯一路径

1. 本地校验每条路径：`central` 仍缺失、`agent_id != central`、路径等于 `remote_join(global_skills_dir, skill_id)`、`ensure_remote_child_path` 通过、禁止删平台根。
2. 按规范化 POSIX 路径去重。
3. **一次** `CommandRunner` 调用：stdin 脚本逐个 `rm -rf --` 已校验路径，stdout 报告 `OK` / `MISSING` / `ERR`（`MISSING` 与现状一样算成功）。
4. 成功路径：删对应 `skill_installations`，并删同路径的 `agent_skill_observations`。共享根上一次 `rm` 清掉所有共享该路径的平台记账。
5. 失败路径：只让使用该路径的 removal 失败；其他路径继续。
6. Policy 用 `bulk_transfer`（15 min）。把 apply `cancel` 传入 `ProcessCancellation::Atomic`。
7. Local leftover 保持现有 `uninstall_skill`。

预期：leftover-only SSH apply 的远端进程数从 **N** 降到 **1**。约 31 个中等目录时，墙钟时间由分钟级降到数秒加 `rm` 本身。

不选「并发多个 ssh.exe」：`central-update-batching.md` 已定为错误形态（墙钟可能下降，认证压力和进程数仍线性）。

### C. russh 持久会话

可去掉每次握手，但属于已推迟的大改（`07-11-ssh-persistent-session`）。不作为 leftover 任务的前置。

### D. leftover 列表按唯一路径折叠

能把 309 张卡片收成约 31 张，并修正计数。不降低一键清理的 SSH 次数，除非同时做 B。作为后续 UX，不阻塞速度修复。

## 7. 安全约束（批量 `rm` 必须保留）

当前远端守卫已经很窄：只允许 `remote_join(agent.global_skills_dir, skill_id)`，禁止 `..` / NUL / 非绝对路径 / 等于平台根 / 越出平台根。批量脚本只能接收 **通过该守卫的路径**。路径用 `shell_quote` 或位置参数，禁止把未校验用户字符串拼进脚本源码。

共享根：删 `/home/lyh/.agents/skills/ask-matt` 会同时从 amp/cursor/codex 等平台拿走该 skill。这是目录事实。DB 必须一起清，避免幽灵 leftover。

## 8. 次要成本（非主导）

- leftover-only apply 仍会 `CentralFs::from_active_target`（多一个连接对象，不额外 spawn，除非后续 hash/write）。
- 每条路径两次 DB 读 + 一次 installation 删：相对 SSH 可忽略；实现时可按 `skill_id` / `agent_id` 缓存。
- 扫描里 `deleted_installation_skill_name` 按 installation 逐条查 skill 名：只影响 Refresh，不影响 Cleaning 转圈。

## 9. 建议验收锚点

- FakeRunner：同一共享路径的 10 个 leftover 组 → **1** 次 runner 调用；命令走 `CommandRunner`，stdin 含全部唯一路径。
- 混合 `OK` / `MISSING` / `ERR` 保持部分成功。
- 未通过守卫的路径：0 次 runner 调用。
- 成功清理后 `scan_deleted_platform_copies_with_pool` 不再返回这些路径（observation 已删）。
- Local 单测保持现有行为。
