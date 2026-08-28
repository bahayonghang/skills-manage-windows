# Skills CLI 全局页现状证据

日期：2026-08-27
父任务：`.trellis/tasks/08-27-skills-cli-availability-remote`

本文件记录规划阶段通过阅读仓库确认的事实。**这些是已确认事实，不是假设。**

---

## 1. `cli_unavailable` 警告链路

### 公开句与 i18n

| 位置 | 内容 |
| --- | --- |
| `src-tauri/src/ipc_error.rs:456-458` | `"skills_cli.cli_unavailable" => "The Skills CLI package could not be executed."` |
| `src/i18n/locales/en.json:78` | `"cli_unavailable": "The Skills CLI package could not be executed."` |
| `src/i18n/locales/zh.json:78` | `"cli_unavailable": "无法执行 Skills CLI 软件包。"` |

### 产生条件

`SkillsCliError::CliUnavailable`（`src-tauri/src/services/skills_cli/error.rs:27-30`）目前有两个产生点：

1. **launcher 解析失败** — `argv.rs:239-246`：在 node 可执行文件旁找不到 `npx-cli.js`。
2. **doctor probe 非零退出** — `mod.rs:334-349`：

```rust
// src-tauri/src/services/skills_cli/mod.rs:334-349
let probe = run_cli(runner, launcher, build_probe_argv(launcher), standard_policy(), None).await?;
if !probe.status_success {
    tracing::warn!(status_success = probe.status_success, "Skills CLI doctor probe failed");
    return Err(SkillsCliError::CliUnavailable);
}
```

probe argv（`argv.rs:286-290`）= `skills --help`，完整命令形如：

```
<node> <npx-cli.js> --yes --package=skills@1.5.23 -- skills --help
```

### 关键事实：应用已经在用 npx，不依赖 `npm install`

用户提出「本地没有通过 npm install 安装 skills，所以检测不到是正常的」。仓库证据显示应用**从未**检测本地已安装的 `skills` 包：

- `SKILLS_CLI_NPM_SPEC = "skills@1.5.23"`（spec `skills-cli-global.md:12`）
- argv 前缀固定为 `--yes --package=skills@1.5.23 -- skills`（spec `skills-cli-global.md:77-80`）

因此 probe 失败的真实原因**不是**「没有 npm install」，而是下列之一：网络/代理不可达、npx 缓存缺失且无法下载、`npx-cli.js` 未在 node 旁被解析到、子进程超时。

历史研究文件 `.trellis/tasks/archive/2026-08/08-24-skills-cli-inventory-ux/research/inventory-page-root-cause.md:69-80` 已记录过同一现象，并指出控制台闪烁证明 node + npx-cli.js **已被成功 spawn**，故当时的 `cli_unavailable` 是子进程退出码非零，而非找不到 npx。

### 关键事实：`runtimeBlocked` 过度封锁（既有缺陷）

`src/pages/SkillsCliView.tsx:151`：

```tsx
const runtimeBlocked = runtimeError !== null;
```

`runtimeBlocked` 被传播到：

| 消费点 | 文件:行 | 后果 |
| --- | --- | --- |
| 卡片 `isLoading={isMutating \|\| runtimeBlocked}` | `SkillsCliView.tsx:524` | 卡片上的「管理链接」「卸载」图标按钮 disabled |
| 批量栏 `runtimeBlocked` → `mutationsLocked` | `SkillsCliBatchBar.tsx:47` | 批量 link / unlink / uninstall 全部 disabled |
| 卸载对话框 `runtimeBlocked` | `SkillsCliView.tsx:587,590-592` | 对话框拒绝打开 |
| 详情抽屉 `runtimeBlocked` | `SkillsCliView.tsx:614` | 详情内 link/unlink 禁用 |
| Header 安装按钮 | `SkillsCliHeader` | Install 按钮 disabled |

**但按 spec `skills-cli-global.md` 契约，这些操作里只有 install（`add`）和 preview 会 spawn CLI：**

- `skills-cli-global.md:81-83`：「`skills_cli_list_global` 读 lock v3 + 文件系统…**It must not spawn the CLI**」
- `skills-cli-global.md:99-100`：「`skills_cli_remove_global` **does not spawn** `skills remove`」
- `skills-cli-global.md:184`：「Base: doctor/preview are Local reads that may spawn; list is a Local lock+FS read: **no CLI spawn**」
- link/unlink 走 `services/skills_cli/link.rs`，是本机 junction/symlink 操作，不 spawn

结论：**probe 失败时禁用 uninstall / link / unlink / 批量操作是过度封锁**。这些路径根本不需要 npx 包可执行。这是一个可独立论证的既有缺陷，不只是「文案噪音」。

### 现有测试约束（改动时必须一并更新）

| 测试 | 断言 |
| --- | --- |
| `src/test/pages/SkillsCliView.test.tsx:300-308` | doctor 报 `cli_unavailable` 时库存仍渲染 |
| `src/test/pages/SkillsCliView.test.tsx:993-1000` | 同上另一场景 |
| `src/test/components/skillsCli/SkillsCliHeader.test.tsx:41-92` | 显示安全 runtime error、**禁用 Install**、保留计数 |
| `src/test/stores/skillsCliStore.test.ts:106-122` | doctor 拒绝时不清空 inventory，`runtimeError` 不含 `npm ERR!` |
| `src-tauri` `tests.rs:351-385` | probe 失败写 tracing warn，且 stderr 不进 IPC message |
| 归档 PRD `08-25-skills-cli-inventory-frontend/prd.md:54` | R5：`cli_unavailable` 出现时**安装/卸载按钮禁用** |

归档 PRD 的 R5 是「卸载也禁用」的来源。本次若放宽，需要在 spec 与该行为上做显式变更说明。

---

## 2. `Unavailable` 徽章语义

### 徽章渲染位置

`src/components/skill/SkillCardDenseRow.tsx:188-225`。只有在 `managed.length === 0`（该技能在**所有**平台上都没有 managed_link）时才计算聚合状态 `denseRowStatus`。

`denseRowStatus`（`SkillCardDenseRow.tsx:30-60`）的优先级：

```
conflict  > direct_copy(copy) > missing > unavailable
```

`missing` 会覆盖 `unavailable`（第 41-45 行）。因此**卡片显示 `Unavailable` 当且仅当：该技能所有 placement 都是 `unavailable`**（没有任何 managed_link / direct_copy / conflict / missing）。

### 后端产生 `Unavailable` 的四种原因

`src-tauri/src/services/skills_cli/placement.rs:73-110` `classify_absent()`，按顺序：

| 顺序 | reason_code | 条件 | 语义 |
| --- | --- | --- | --- |
| 1 | `canonical_missing` | `!canonical_is_owned_directory(canonical)` | **canonical 目录不存在**（lock 里有名字，`~/.agents/skills/<name>` 已消失）→ 真正的失效/幽灵条目 |
| 2 | `platform_unsupported` | `!platform.supports_local_placement` | 平台不支持本地放置 |
| 3 | `platform_not_detected` | `!platform.is_detected` | 本机未检测到该平台 |
| 4 | `platform_disabled` | `!platform.is_enabled` | 平台被用户禁用 |

**关键区分**：`canonical_missing` 在 `classify_absent` 中**先于**其他三项判断，且与平台无关 —— canonical 缺失时**每个**平台都会返回 `Unavailable/canonical_missing`。

而 2/3/4 是**平台侧**原因：技能本体完全健康，只是没有可放置的平台。若本机所有 mapped∩detected 平台都被禁用/未检测，一个健康技能也会满屏 `Unavailable`。

### 结论：不能按徽章直接批量删除

「删除所有 Unavailable」若按徽章字面执行，会在「平台全未检测/全禁用」的机器上**删除完全健康的技能**。安全的批量清理必须以 `reasonCode === "canonical_missing"` 为判据，而不是 placement state。

`reasonCode` 已经暴露给前端：`src/lib/ipc/generatedCommandMap.ts:1048-1055`。

---

## 3. 多选与批量操作现状（已存在，勿重复造）

归档任务 `.trellis/tasks/archive/2026-08/08-26-batch-actions/` 已交付完整多选框架。

### 已有能力

| 能力 | 实现位置 |
| --- | --- |
| 选择模式开关 | `SkillsCliToolbar` `Select` 按钮 → `SkillsCliView.tsx:96,318-319` |
| 卡片复选框 | `SkillCardDenseRow.tsx:103-116` |
| 组头 `Select all` | `SkillsCliGroupHeader` → `handleSelectAll`（`skillsCliPageHandlers.ts:121-130`） |
| 批量栏 | `src/components/skillsCli/SkillsCliBatchBar.tsx` |
| 批量 Link to platform | `linkPlatformBatch`，按 agent 选择，只对 `missing` 发 IPC |
| 批量 Unlink | `unlinkManagedBatch`，只对 `managed_link` 发 IPC |
| 批量 Export selected | `exportSkillsCliInventory` |
| 批量 Uninstall | `removeGlobalBatch` + `SkillsCliUninstallDialog` |
| 选择集与库存对账 | `reconcileSelectedNames`（`skillsCliBatchModel.ts:92-104`） |
| partial failure 语义 | `PlacementMutationOutcome` succeeded/failed/skipped |

### 后端的批量能力分布（决定前端批量操作的规模上限与反馈形态）

| 操作 | 后端原生批量 | 形态 |
| --- | --- | --- |
| `skills_cli_add_global` | 是 | 一次 CLI 调用，`build_add_global_argv`（`argv.rs:304-324`）重复 `-s` / `-a` |
| `skills_cli_apply_updates` | 是（单仓库内多技能） | 一次 mutation guard 覆盖整个 apply |
| `skills_cli_verify_update_baseline` | 是 | 一个 job lease 下循环 |
| `skills_cli_remove_global` | **否**，签名是单个 `skill_name` | 前端 `removeGlobalBatch` 逐项循环 |
| `skills_cli_link_platform` / `unlink_platform` | **否** | 前端 `runPlacementBatch` 逐项循环 |

逐项路径每一项都**独立**申请并释放一次 exclusive job lease（`AppState.skills_cli_jobs`，
`lib.rs:63-66`）与一次 `acquire_target_mutation_guard`（默认 10s 超时）。
清理 N 个技能 = N 轮加解锁，期间其他写操作会间歇性撞上 `skills_cli.busy`。

**进度事件只存在于 update 子系统**：`UPDATE_PROGRESS_EVENT = "skills-cli://update-progress"`
（`services/skills_cli/updates/mod.rs:32`），由 `updates/detect.rs` 与 `updates/apply.rs` 发出。
install / link / unlink / remove **没有**进度通道，取消依赖 `AtomicBool` 轮询而非进度负载。

### 错误码折叠（影响 doctor-gate）

`add_global` 在 CLI 非零退出时返回 `SkillsCliError::CliFailed`（`mod.rs:573`），
其文档注释为「The CLI ran but exited with a failure status for the request」（`error.rs:64-66`）。
它与 `OutputLimitExceeded`、`ListUnparsed` 一起被折叠成 `internal.unexpected`：

```169:171:src-tauri/src/services/skills_cli/error.rs
            Self::OutputLimitExceeded { .. } | Self::CliFailed | Self::ListUnparsed => {
                "internal.unexpected"
            }
```

`CliFailed`（CLI 跑起来了但这次请求失败）与 `CliUnavailable`（包根本无法执行）语义不同，
不可互相替代。

### 缺口

1. **批量栏没有 Update 按钮**。更新入口只有两个：
   - 组头 `onUpdateAll`（`SkillsCliView.tsx:474-483`），按**仓库分组**
   - 详情抽屉单技能 `onUpdate`（`SkillsCliView.tsx:635-646`）
2. **没有「清理失效条目」入口**（既无批量栏按钮，也无工具栏按钮）。
3. `unlinkManagedBatch` 是「解链所有平台」，不能选择只解某一个平台（link 侧有 agent 菜单，unlink 侧没有对称能力）。

### 批量更新的硬约束

`skills_cli_apply_updates` 的请求体每次只接受**一个** `repositoryKey`：

```ts
// src/lib/ipc/generatedCommandMap.ts:989-993
export type SkillsCliApplyUpdateRequest = {
	jobId: string,
	repositoryKey: string,
	selections: SkillsCliApplySelection[],
};
```

`openUpdateSurface`（`skillsCliPageHandlers.ts:211-230`）在 `repositoryKey` 为空时直接 toast `skillsCli.updates.checkFirst` 并返回。

因此任意跨仓库选择集的批量更新，必须按 repositoryKey 分组后**串行发多次 apply**（每次独立 jobId），或限制为单仓库选择。这是产品决策，不是实现细节。

---

## 4. 远端 SSH 现状

### Skills CLI 目前是显式 Local-only

| 闸门 | 位置 |
| --- | --- |
| 后端 | `src-tauri/src/services/skills_cli/mod.rs:247-252` `ensure_local_target()` → `SkillsCliError::LocalTargetOnly` |
| spec | `.trellis/spec/backend/skills-cli-global.md:12`「MVP is Local target only」、`:70-72` Local gate 契约 |
| 页面 | `src/pages/SkillsCliView.tsx:210-217`，非 Local 直接渲染 `skillsCli.localOnly` 占位 |
| 侧边栏 | `Sidebar.tsx:113-114` 非 Local 隐藏入口 |
| 加载 | `SkillsCliView.tsx:111-116`，`!isLocal` 时不调用 `loadAll()` |

### 可复用的远端基础设施

| 抽象 | 位置 | 用途 |
| --- | --- | --- |
| `TargetContext` | `targets/model.rs:294-324`，spec `target-context.md` | 请求内冻结 target + DbPool |
| `ConnectedRemoteTarget` | `targets/remote.rs:19-229` | 统一 SSH/WSL 门面；`run_command`、`read_file`、`list_dir`、`exists` 等 |
| `connect_remote_target` | `targets/remote.rs:9-17` | 入口 |
| `InstallTransport` | `services/installation/transport.rs:27-73`，spec `transport-seam.md` | install/uninstall 的 Local/Remote seam 范式 |
| `Scope` + `FsBackend` | `services/usage/mod.rs:74-167`，`fs_backend.rs:37-76` | usage 分析的 Local/Remote seam 范式 |
| `acquire_target_mutation_guard` | `central_mutation/mod.rs:63-78` | 已是 target-scoped |
| `ProcessRunner` / `ProcessPolicy` | `targets/runner.rs:40-77` | 超时/输出上限/取消 |

### 传输实现要点

- **不是 russh/ssh2**：外壳调用系统 `ssh.exe` / `wsl.exe`（`targets/exec.rs:131-145,201-259`）
- **无持久 SSH 会话池**：每次 `run_command` 重新握手；连接超时 10s，keepalive 15s×3
- 密码走 `SSH_ASKPASS`，凭据在 keyring（service `"SkillPort SSH Targets"`）+ Windows DPAPI，**不在**主 `SecretStore`

### 远端化必须解决的具体问题

1. `ensure_local_target` 移除后，lock 路径需从远端 `remote_home` 推导，而非本机 `resolve_home_dir()`（spec `:74-76`）
2. 远端 Node ≥ 22.20 检测（现 doctor 只探测本机）
3. 远端 junction/symlink：Windows junction 走 reparse API，**远端 Linux/macOS 只有 symlink**；`create_skills_cli_directory_link` 是本机 API 调用，远端需要改成远端命令
4. 平台探测（detected/enabled）在远端主机上的语义
5. leftover 保护：spec `:131-132` 明确「Remote leftover must not use this machine's lock」
6. 每次 SSH 往返有握手开销，inventory 列举需批量化，否则逐文件 stat 会极慢
7. `.trellis/spec/backend/skills-cli-global.md` 的 Local-only 契约必须先修订

### 与在飞任务的冲突面

- `08-26-ssh-update-observability-dialog`：尽管名字含 SSH，PRD 第 21 行明确「**不再调查 SSH transport**」，实为可观测性/日志治理，冲突低
- `08-26-observability-governance-integration`：CI/registry 一致性闸门；新增 `skills_cli_*` 命令或改签名时需同步日志策略与 registry
