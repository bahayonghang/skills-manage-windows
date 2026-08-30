# Skills CLI 全局页远端 SSH/WSL 支持（中层父任务）

父任务：`08-27-skills-cli-availability-remote`
源需求：U5
类型：中层父任务，拥有远端需求集、子任务映射与跨子任务验收；本身不作为实现目标

## Goal

把 `/skills-cli`（Skills CLI global）从 Local-only 扩展到远端 SSH / WSL 目标，
支持在远端主机上完整地查看、链接、卸载、安装与更新 Skills CLI 全局技能。

## Decisions

- **D1（Q3 = 方案 C）**：交付**完整远端读写**，含 install / remove / update。
  该范围超出单个子任务的可控体量，因此本任务降为中层父任务，拆为四个递进子任务。
- **D2**：拆分按「风险与依赖递进」而非按代码分层，保证每个子任务都能独立验收并单独合入。

## Confirmed Facts

### 当前是刻意的 Local-only 设计，不是遗漏

| 闸门 | 位置 |
| --- | --- |
| 后端 | `services/skills_cli/mod.rs:247-252` `ensure_local_target()` → `SkillsCliError::LocalTargetOnly` |
| spec | `.trellis/spec/backend/skills-cli-global.md:12`「MVP is Local target only」、`:70-72` |
| 页面 | `src/pages/SkillsCliView.tsx:210-217` 非 Local 渲染 `skillsCli.localOnly`；`:111-116` 不 `loadAll()` |
| 侧边栏 | `Sidebar.tsx:113-114` 非 Local 隐藏入口 |

### 可复用的远端基座

| 抽象 | 位置 | 用途 |
| --- | --- | --- |
| `TargetContext` | `targets/model.rs:294-324`，spec `target-context.md` | 请求入口冻结 target + DbPool |
| `ConnectedRemoteTarget` | `targets/remote.rs:19-229` | SSH/WSL 统一门面：`run_command`、`run_script`、`exists`、`read_file`、`list_dir`、`remove_tree` |
| `connect_remote_target` | `targets/remote.rs:9-17` | 由 `ActiveTarget` 建立连接 |
| `InstallTransport` | `services/installation/transport.rs:27-73`，spec `transport-seam.md` | Local/Remote seam 范式 |
| `Scope` + `FsBackend` | `services/usage/mod.rs:74-167`，`fs_backend.rs:37-76` | 另一个 Local/Remote seam 范式 |
| `acquire_target_mutation_guard` | `central_mutation/mod.rs:63-78` | 已是 target-scoped |
| `ProcessRunner` / `ProcessPolicy` | `targets/runner.rs:40-77` | Probe 30s、Standard 120s、BulkTransfer 15min |

### 传输物理特性（约束所有子任务）

- **不是 russh / ssh2**：外壳调用系统 `ssh.exe` / `wsl.exe`（`targets/exec.rs:131-145,201-259`）。
- **无持久 SSH 会话池**：每次 `run_command` 一次全新握手，连接超时 10s，keepalive 15s×3。
  → 任何「逐技能 / 逐平台」的远端 stat 都会把握手成本放大 N×M 倍，不可接受。
- 凭据在 keyring（service `"SkillPort SSH Targets"`）+ Windows DPAPI，密码走 `SSH_ASKPASS`；
  **不在**主 `SecretStore`。

### 必须逐项解决的问题（分配到子任务）

| # | 问题 | 归属 |
| --- | --- | --- |
| 1 | lock 路径需由 `remote_home` 推导，不得复用本机 `resolve_home_dir()`（spec `:74-76`） | seam |
| 2 | 远端 Node ≥ 22.20 与 npx 可执行性探测 | seam |
| 3 | Local/Remote 单一接缝，禁止业务逻辑散落 `match ActiveTarget` | seam |
| 4 | spec `skills-cli-global.md` Local-only 契约修订 | seam |
| 5 | 远端 inventory 常数级往返 | inventory |
| 6 | 远端平台探测（detected / enabled） | inventory |
| 7 | 远端目录链接：Linux/macOS symlink vs 远端 Windows junction；`create_skills_cli_directory_link` 是本机系统调用 | mutate |
| 8 | 远端 mutation guard 与 lease → guard → recheck 顺序（spec `:112-116`） | mutate |
| 9 | 远端 remove recovery journal 与 lock fingerprint CAS（spec `:99-102`） | mutate |
| 10 | 远端 leftover 不得使用本机 lock（spec `:131-132`） | mutate |
| 11 | 远端 npx spawn 与远端主机访问 npm registry 的能力 | install-update |
| 12 | 远端 update 的 GitHub 快照下发路径与 journal（spec `:117-127`） | install-update |
| 13 | `install_origin` 标注在远端的语义（spec `:134-135`） | install-update |

### 与在飞任务的关系

- `08-26-ssh-update-observability-dialog`：名字含 SSH，但 `prd.md:21` 明确「不再调查 SSH transport」，
  实为可观测性/日志治理。冲突低。
- `08-26-observability-governance-integration`：registry 一致性与日志策略 CI 闸门。
  新增或改签名 `skills_cli_*` 命令时需同步 `ipc_registry` 日志策略并运行 `pnpm docs:gen`。

## Task Map

| 子任务 | 交付物 | 独立验收 |
| --- | --- | --- |
| `08-27-skills-cli-remote-seam` | Local/Remote 传输接缝、远端路径与 doctor、spec 修订 | 接缝存在且 Local 行为零回归；远端 doctor 可返回结果 |
| `08-27-skills-cli-remote-inventory` | 远端只读列举、平台探测、前端读路径解闸 | 切到 SSH target 后页面可列出远端技能与 placement |
| `08-27-skills-cli-remote-mutate` | 远端 link / unlink / 安全卸载 / leftover | 远端可链接与卸载，conflict 零写，recovery 可用 |
| `08-27-skills-cli-remote-install-update` | 远端 install 与 update | 远端可安装新技能并应用更新 |

## Ordering Constraints

严格递进，写入各子任务工件：

1. `remote-seam` 是其余三个的前置，必须先合入 `dev`。
2. `remote-inventory` 依赖 seam 的路径解析与传输抽象。
3. `remote-mutate` 依赖 inventory 的 placement 分类结果。
4. `remote-install-update` 依赖 mutate 的 guard 与 journal 顺序。

此外整棵树依赖 `08-27-skills-cli-doctor-gate` 先定型本机 doctor 语义，
否则远端 doctor 会二次返工。

## Cross-Child Acceptance Criteria

- [ ] RAC1：`.trellis/spec/backend/skills-cli-global.md` 存在 Local/Remote 能力矩阵，
      仓库中不存在与之矛盾的 `ensure_local_target` 残留调用或 spec 中间态。
- [ ] RAC2：所有远端路径解析可追溯到 `ActiveTarget` 的 `remote_home`；
      不存在远端流程读取本机 `resolve_home_dir()` 或本机 lock 的路径。
- [ ] RAC3：业务逻辑中不存在散落的 `match ActiveTarget`；Local/Remote 差异集中在单一接缝。
- [ ] RAC4：四个子任务全部合入后，Local 目标的既有行为零回归
      （既有 Skills CLI 测试全绿，无断言被删除）。
- [ ] RAC5：远端 stdout / stderr / 路径不出现在 `IpcError.message` 或未脱敏操作日志。
- [ ] RAC6：`pnpm docs:gen:check` 通过，`ipc_registry` 日志策略无缺口。
- [ ] RAC7：真实 SSH 主机上的端到端行为在用户提供可用远端环境前，一律标记 `UNVERIFIED`，
      不得以单元测试通过声称远端已验证。

## Out of Scope

- 迁移到 russh 或引入持久 SSH 会话池（`plans/ssh-perf` 独立议题）。
- 远端 Node 的自动安装或版本升级。
- 改变 `SKILLS_CLI_NPM_SPEC` PIN 版本。
- SkillPort Central 与 `skillport-cli` 的远端行为（已各自有实现）。
