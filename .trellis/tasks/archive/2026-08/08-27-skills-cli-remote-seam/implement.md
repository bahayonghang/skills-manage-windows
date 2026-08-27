# 执行计划 — Skills CLI 远端传输接缝与 spec 修订

依据 `prd.md` 与 `design.md`。按段执行，每段结束跑该段验证命令再进入下一段。

**前置**：`08-27-skills-cli-doctor-gate` 已合入 `dev`（远端 doctor 需对齐定型后的本机语义）。

## 段 1 — 接缝类型与路径解析（回滚单元 A）

- [ ] 1.1 新建 `src-tauri/src/services/skills_cli/transport.rs`：
      定义 `SkillsCliScope`、`SkillsCliTransport`、`SkillsCliPaths`、
      `SkillsCliCapability`、`SkillsCliCapabilities`（design §2.1、§2.2）。
- [ ] 1.2 同文件定义 `SkillsCliFs` trait，方法覆盖现有业务用到的 FS 原语：
      `inspect_path`（`symlink_metadata` 等价）、`read_file_bounded`、`atomic_write`、
      `list_dir`、`remove_tree`、`create_dir_all`、`exists`。
      签名以远端可实现为准（路径用 `&str`，返回值不含 `std::fs::Metadata`）。
- [ ] 1.3 实现 `LocalSkillsCliFs`：逐方法转调现有 `std::fs` 调用，行为与现状逐字等价。
      本段**不实现** `RemoteSkillsCliFs`（属段 3）。
- [ ] 1.4 `lock.rs` 新增 `remote_lock_path(xdg_state_home: Option<&str>, remote_home: &str) -> String`
      （design §2.3）。用 `crate::targets::remote_join`，引用 `UNIVERSAL_AGENTS_DIR_NAME` 常量，
      **禁止**出现 `.agents` 字面量，**禁止**用 `Path::join`。
- [ ] 1.5 `SkillsCliPaths::for_scope`：Local 走 `resolve_home_dir()` + `skills_cli_lock_path_from_env`；
      Remote 走 `remote_home()` + `remote_lock_path`，`XDG_STATE_HOME` 由调用方传入
      （段 3 的 doctor 探测提供），本段先接受 `Option<&str>` 参数。

验证：`cargo check -p skillport`

## 段 2 — 业务逻辑改走接缝（回滚单元 A 续）

- [ ] 2.1 逐个改造业务函数签名，加 `tx: &SkillsCliTransport` 首参：
      `list_global`、`doctor`、`add_global`、`preview_source`、`link_platform`、
      `unlink_platform`、`preview_remove_global`、`remove_global`、`read_skill_md`、
      `export_inventory`、`install_targets`，以及 `updates/` 下的对应入口。
- [ ] 2.2 把 design §1.2 列出的 **9 处** `resolve_home_dir()` 调用
      （`mod.rs:361`、`lock.rs:242`、`link.rs:41,62`、`files.rs:84,148`、
      `remove.rs:103,137`、`updates/apply.rs:124`）全部改为读 `tx.paths()`。
      改完 `rg "resolve_home_dir" src-tauri/src/services/skills_cli/` 应为空。
- [ ] 2.3 FS 调用改走 `tx.fs()`。`placement.rs` 的 `canonical_is_owned_directory`
      与 `directory_link.rs` 的 `observe_directory_slot` **本段不动**——
      它们是 inventory 的作业面，本段只保证 Local 路径不变。
- [ ] 2.4 命令层（`commands/skills_cli.rs`）在 `resolve_target_context()` 之后
      构造 `SkillsCliTransport::for_target(context.target())`，传给业务函数。
      本段**保留** `ensure_local_target` 调用不动（段 4 才替换），
      确保段 1+2 是纯重构、行为零变化。
- [ ] 2.5 更新 `tests.rs`：为现有用例构造 Local `SkillsCliTransport`。
      不改任何断言内容（R7 / AC7）。

验证：`cargo test -p skillport skills_cli` — 既有用例应全绿且**无断言改动**。
`git diff` 复核确认没有被删除的 `assert!`。

## 段 3 — 远端实现：Fs、Runner、doctor（回滚单元 C）

- [ ] 3.1 实现 `RemoteSkillsCliFs`：逐方法转调 `ConnectedRemoteTarget` 的
      `inspect_path` / `read_file_bounded` / `write_file` / `list_dir` /
      `remove_tree` / `mkdir_p` / `exists`（design §1.4 的 API 表）。
- [ ] 3.2 实现 `RemoteNodeRunner: SkillsCliRunner`：把 `RunnerRequest` 的
      program + args 用 `shell_quote`（`exec.rs:703`）拼成远端命令。
      本段只需支持 doctor 的 `node --version`；add/preview 的 argv 属 install-update。
- [ ] 3.3 新增远端 doctor 探测脚本（design §2.4），一次 `run_script` 返回
      `XDG` / `HOME` / `NODEV` 三行。解析复用 `parse_node_version`（`argv.rs:251`）。
- [ ] 3.4 `doctor` 按 scope 分派：Local 走 `doctor-gate` 定型后的路径，
      Remote 走 3.3。两者都**不探测** `skills --help`，返回同形状的 `SkillsCliDoctorReport`。
- [ ] 3.5 把 3.3 拿到的 `XDG` 回填给 `SkillsCliPaths`（段 1.5 预留的参数）。
      `HOME` 与配置的 `remote_home()` 不一致时 `tracing::warn!` 但以配置为准；
      warn 只记「不一致」这一事实，**不记两个路径值**（R8）。

验证：`cargo test -p skillport skills_cli`

## 段 4 — 能力矩阵替换闸门（回滚单元 B）

- [ ] 4.1 按 design §2.2 的开闸表初始化 `SkillsCliCapabilities`：
      本任务只开 `Doctor`，其余全部 `UnsupportedOnRemote`；
      `RevealFolder` 标为**永久**不支持并加注释说明理由。
- [ ] 4.2 逐一替换 `commands/skills_cli.rs` 的 **18 处** `domain::ensure_local_target(...)`
      为 `tx.ensure_capability(SkillsCliCapability::X)`。
      行号见 design §1.1；替换后 `rg "ensure_local_target" src-tauri/src/` 只应剩
      `central_store_location/mod.rs:74` 的同名但不同域的函数。
- [ ] 4.3 删除 `skills_cli/mod.rs:247-257` 的 `ensure_local_target` 与 `is_local_target`
      （若 `updates/mod.rs:181-183` 的 `ensure_local()` 仍有调用方，一并改为能力查询）。
- [ ] 4.4 `tests.rs:489-496` 的三条 `ensure_local_target` 用例改写为能力矩阵用例：
      远端 target + 未开闸能力 → `LocalTargetOnly`；远端 target + `Doctor` → `Ok`；
      Local target + 任意能力 → `Ok`。
- [ ] 4.5 为每个 `UnsupportedOnRemote` 能力加一条零写断言（AC5）：
      调用后远端 fake FS 的写方法调用计数为 0。

验证：`cargo test -p skillport`

## 段 5 — 测试补齐

- [ ] 5.1 AC1：静态断言 `services/skills_cli/` 中除 `transport.rs` 外不出现
      对 `ActiveTarget` 变体的 `match`。用一条读源码的测试或 `rg` 门禁实现，
      并把允许清单写死为 `transport.rs`。
- [ ] 5.2 AC2：注入与本机不同的 `remote_home`，断言远端 lock path 与 canonical root
      随之改变；同一测试断言结果**不包含**本机 home 的任何片段。
- [ ] 5.3 AC3：参数化测试覆盖「远端有 `XDG_STATE_HOME`」与「远端无」两分支；
      断言结果字符串不含 `.agents` 硬编码（用常量比对而非字面量）。
      同测试断言 `remote_lock_path` 与 `skills_cli_lock_path_from_env` 分支等价。
- [ ] 5.4 AC4：fake 远端 runner 断言 doctor 的远端命令调用次数在
      「1 个平台」与「6 个平台」输入下**相同**（应为 1）；
      Node 缺失与版本过旧分别返回 `skills_cli.node_missing`。
- [ ] 5.5 AC8：远端探测脚本的 stderr 植入哨兵 token，
      断言其不出现在 `IpcError.message` 与操作日志。

验证：`cargo test -p skillport skills_cli`

## 段 6 — spec 修订（R6，与段 4 同批提交）

- [ ] 6.1 `.trellis/spec/backend/skills-cli-global.md` §1：删除「MVP is Local target only」。
- [ ] 6.2 §3：Local gate 契约替换为能力矩阵契约，说明查询入口是
      `SkillsCliTransport::ensure_capability`。
- [ ] 6.3 §4 错误矩阵：`skills_cli.local_target_only` 行的适用范围改为
      「能力矩阵中标记为远端未支持的能力」，而非「所有非 Local 请求」。
- [ ] 6.4 §5 Base/Bad case：移除以 Local-only 为前提的表述。
- [ ] 6.5 §6 Tests Required：删除「Non-Local IPC reject」，
      「remote scan ignores local lock」保留（它在 `remote-mutate` 才落地，标注归属子任务）。
- [ ] 6.6 新增 Local/Remote 能力矩阵表，逐条覆盖
      doctor / list / link / unlink / remove / install / update / export / leftover（AC6）。
      表中每行标注「已支持 / 由哪个子任务开闸 / 永久不支持」。

## 段 7 — 收尾

- [ ] 7.1 `rg "ensure_local_target" src-tauri/src/services/skills_cli/` 应为空。
- [ ] 7.2 `rg "resolve_home_dir" src-tauri/src/services/skills_cli/` 应为空。
- [ ] 7.3 确认 IPC 命令签名未变 → 跑 `pnpm docs:gen:check` 应通过且无 diff（AC9）。
      若意外产生 diff，说明签名被改动，需回头补 `pnpm docs:gen` 与 `ipc_registry` 策略。
- [ ] 7.4 全量：`just ci`

## 风险文件与回滚点

回滚单元见 `design.md` §6。

| 文件 | 风险 | 回滚单元 |
| --- | --- | --- |
| `services/skills_cli/transport.rs`（新） | 接缝形状定错会波及全部后续子任务 | A |
| `services/skills_cli/lock.rs` | 远端 lock 路径算错会让远端读到空 lock 却不报错 | A |
| `services/skills_cli/{mod,link,files,remove}.rs` | 9 处 home 解析改造，漏一处就是本机/远端串路径 | A |
| `commands/skills_cli.rs` | 18 处闸门替换，漏一处就是未开闸能力被放行 | B |
| `.trellis/spec/backend/skills-cli-global.md` | 与实现必须同批提交，禁止只改一边 | 与 B、C 同批 |

## 前置检查

- [ ] `08-27-skills-cli-doctor-gate` 已合入 `dev`。
- [ ] 确认 `08-26-observability-governance-integration` 未在同一工作树改动
      `ipc_registry.rs`，避免冲突。
- [ ] 工作树干净（`git status --porcelain` 只有 `src-tauri/target/`）。
