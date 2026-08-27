# 执行计划 — Skills CLI doctor 门禁降级与警告去噪

依据 `prd.md` 与 `design.md`。按段执行，每段结束跑该段的验证命令再进入下一段。

## 段 1 — 后端：拆分 launcher 解析

> 段 1 与段 2 构成原子回滚单元 A（design §6 / TPR-06）：必须同批提交，
> 中间状态不可用（只做段 1 则 doctor 未改，只做段 2 则编译失败）。

- [x] 1.1 `src-tauri/src/services/skills_cli/argv.rs`：新增 `resolve_node_program_from_dirs(&[PathBuf])`，
      只做 `find_node_in_paths` + `canonicalize`，失败返回 `NodeMissing`。
- [x] 1.2 同文件：`resolve_node_launcher_from_dirs` 改为复用 1.1，再补 `npx_js` 查找；
      保持既有签名、错误语义与 `tracing::warn!` 不变。
- [x] 1.3 `mod.rs`：新增 `resolve_node_program()`（读 `PATH`），与既有 `resolve_launcher()` 并存。
- [x] 1.4 导出调整：`mod.rs` 的 `pub use argv::{…}` 增加新函数。

验证：`cargo check -p skillport`

## 段 2 — 后端：doctor 去探测

- [x] 2.1 `mod.rs`：`doctor()` 改为调用 `resolve_node_program()`；
      `doctor_with_launcher` 重命名/改造为 `doctor_with_program`，
      删除 `build_probe_argv` 调用与 `if !probe.status_success { … CliUnavailable }` 块。
- [x] 2.2 保留 `SkillsCliDoctorReport { node_version, npm_spec }` 的构造与字段值。
- [x] 2.2b **（TPR-05）** `doctor_with_program` 把 runner 返回的
      `SkillsCliError::CliUnavailable` 就地重映射为 `SkillsCliError::NodeMissing`。
      理由见 design §2.4.1：段 3 之后 spawn 失败会产生 `CliUnavailable`，
      若不重映射，「node 文件在但起不来」会让 header 重新显示
      「无法执行 Skills CLI 软件包」——正是本任务要消灭的那句话；
      且 `domain-error-enums.md` 记录 `cli_unavailable` 是 write-path only。
      本步在段 3 之前落地也无害（此时该分支尚不会被触发）。
- [x] 2.3 `build_probe_argv` 保留在 `argv.rs` 但不再被 doctor 引用；
      若产生 dead_code 警告，暂以 `pub` 导出保持可用（远端子树可能复用），不加 `#[allow]` 掩盖。
- [x] 2.4 更新 `src-tauri/src/services/skills_cli/tests.rs`：
      - 改写 `ac10_doctor_probe_failure_warns_without_stderr_and_keeps_public_message`
        → 迁移到 add 路径（见段 3）
      - `ac8_doctor_reports_missing_npx_js_without_path_mutation` 断言对象改为
        `resolve_node_launcher_from_dirs`（仍应返回 `CliUnavailable`），
        并新增一条断言 `resolve_node_program_from_dirs` 在同一输入下**成功**
      - 新增：fake runner 断言 `doctor()` 只发生一次子进程调用（AC1）
      - 新增：npx-cli.js 缺失但 node 正常时 `doctor()` 成功（AC2）

验证：`cargo test -p skillport skills_cli`

## 段 3 — 后端：错误映射修正 + spec

- [x] 3.1 `error.rs:169-171`：把 `CliFailed` 从 `internal.unexpected` 折叠中拆出，
      新增映射 `Self::CliFailed => "skills_cli.cli_failed"`。
      `OutputLimitExceeded` 与 `ListUnparsed` 保持 `internal.unexpected`。
      `mod.rs:573` 的 `CliFailed` 返回点不变（语义本来就对，只是码错了）。
- [x] 3.2 `ipc_error.rs`：为 `skills_cli.cli_failed` 增加公开句，
      措辞区别于 `cli_unavailable`（后者=环境装不起来，前者=命令没成功）。
      不得含 stderr、路径或 URL。
- [x] 3.3 i18n：`en.json` / `zh.json` 的 `backendErrors.skills_cli` 下成对新增 `cli_failed`。
- [x] 3.4 运行 `pnpm ipc:codegen` 刷新 `generatedCommandMap.ts` 的 reviewed codes 列表；
      该文件是生成物，不手改。
- [x] 3.5 **生产改动（TPR-03）** `runner.rs:56-65`：`map_runner_error` 的
      `RunnerError::Io { phase: RunnerPhase::Start, source }` 分支由
      `SkillsCliError::Io { context: "spawn Skills CLI process", source }`
      改为 `SkillsCliError::CliUnavailable`。
      其余 phase（supervise）保持 `SkillsCliError::Io` 不变。
      这一条不是「确认」——当前 `SkillsCliError::Io` 对外是 `internal.unexpected`
      （`error.rs:172`），不改则 AC6 的 spawn 分支必然失败。
- [x] 3.5b 同分支内加 `tracing::warn!`，字段只有 `phase = "start"` 与
      `io_kind = ?source.kind()`。**不得**记录 `source` 的 Display（可能含程序路径）。
      丢弃 `source` 后 `CliUnavailable` 是无载荷变体，编译上无残留。
- [x] 3.5c **生产改动（TPR-04）** `runner.rs:31-38`：`CliOutput` 新增
      `pub exit_code: Option<i32>`；唯一构造点 `CliOutput::from_std`（`:42-48`）
      填 `output.status.code()`。同步修 `tests.rs` 中 `FakeCliRunner` 的
      `push_ok` / `push_output` 构造 helper。
- [x] 3.5d **生产改动（TPR-04）** `mod.rs:572-574`：在 `add_global_locked` 的
      `!output.status_success` 分支 `return Err(CliFailed)` **之前**加结构化 warn，
      字段严格取 design §2.5 白名单：`operation`、`exit_code`、`stderr_bytes`、
      `stdout_bytes`、`skill_count`、`agent_count`、`source_kind`。
      `source_kind` 是由 `SkillSource` 变体映射的 `&'static str`，不得记原始 source 字符串。
- [x] 3.6 新增测试（AC7）：queue 一个 `status_success = false` 且 stderr 含
      `SECRET_STDERR_TOKEN` 的 `CliOutput`，同时断言
      （a）warn 存在且带 `operation` 与 `exit_code`；
      （b）token 既不在 tracing 输出也不在 `IpcError.message`。
      沿用原 `tests.rs:351-385` 的日志捕获手法，迁移到 add 路径。
- [x] 3.6b 新增测试（AC7b）：让 fake runner 返回
      `RunnerError::Io { phase: Start }`，断言 IPC code 为 `skills_cli.cli_unavailable`
      （覆盖 3.5），且 warn 只含 `phase` / `io_kind`。
- [x] 3.6c 把 `ac14_ipc_message_never_contains_stderr`（`tests.rs:313-321`）的变体列表
      补上 `CliFailed` 的新码。
- [x] 3.7 `.trellis/spec/backend/skills-cli-global.md`：
      - §2 signatures 不变
      - §3 Launcher 段补一句：doctor 只解析 node，npx-cli.js 解析属 spawn 路径
      - §4 错误矩阵：`cli_unavailable` 行收敛为「npx JS 无法解析 / 无法 spawn」；
        新增 `skills_cli.cli_failed` 行（add 非零退出）；
        `internal.unexpected` 行移除 add 非零退出，保留 lock/FS IO 与输出上限
      - §5 Base case 中「doctor/preview are Local reads that may spawn」改为「preview may spawn；
        doctor 只探测 node」
      - §6 Tests Required 中「Doctor: missing node / too old / missing npx JS」
        改为「missing node / too old；missing npx JS 只影响 spawn 路径」

验证：`cargo test -p skillport skills_cli`

## 段 4 — 前端：收敛 `runtimeBlocked`

- [x] 4.1 `src/pages/SkillsCliView.tsx`：删除 `runtimeBlocked` 变量（`:151`）。
- [x] 4.2 同文件：卡片 `isLoading={isMutating || runtimeBlocked}` → `isLoading={isMutating}`（`:524`）。
- [x] 4.3 同文件：`SkillsCliBatchBar` 移除 `runtimeBlocked` prop（`:555`）。
- [x] 4.4 同文件：`SkillsCliUninstallDialog` 移除 `runtimeBlocked` prop 与
      `onOpenChange` 中的 `if (open && runtimeBlocked) return;` 早退（`:587,590-592`）。
- [x] 4.5 同文件：`SkillsCliDetailDrawer` 移除 `runtimeBlocked` prop（`:614`）。
- [x] 4.6 三个组件删除各自的 `runtimeBlocked` prop 与内部引用：
      `SkillsCliBatchBar.tsx:14,47`、`SkillsCliUninstallDialog.tsx`、`SkillsCliDetailDrawer.tsx`。
- [x] 4.7 `SkillsCliHeader.tsx` **不改代码**，但依据修正（TPR-05）：
      `runtimeError` 不是只可能 `node_missing`，而是
      `node_missing` / `timeout` / `cancelled` / `internal.unexpected` 的集合
      （design §2.4.1）。保留 `installDisabled = !installAvailable || runtimeError !== null`
      的理由是 **fail-closed**——doctor 未成功即无法确认 Node ≥ 22.20。
      瞬时失败的重试复用 header 已有的 Refresh 按钮，不新增控件。
      本步只需确认现状符合该策略，并在段 5 补瞬时码用例。

验证：`pnpm typecheck && pnpm lint`

## 段 5 — 前端测试

- [x] 5.1 `src/test/components/skillsCli/SkillsCliHeader.test.tsx:41-92`：
      两条用例的 `runtimeError` 从 `cli_unavailable` 改为 `node_missing`，
      断言仍为「显示安全 runtime error、禁用 Install、保留计数」。
- [x] 5.2 `src/test/pages/SkillsCliView.test.tsx:300-308,993-1000`：
      改写为「doctor 成功但 add 失败」场景，断言库存渲染 + 卸载可交互（AC4）。
- [x] 5.3 `src/test/pages/SkillsCliView.test.tsx:422-429`：
      `skills_cli_remove_global` 抛 `cli_unavailable` 的用例——确认卸载对话框仍能打开并展示错误
      （原先 `runtimeBlocked` 会阻止打开）。
- [x] 5.4 `src/test/stores/skillsCliStore.test.ts:106-122`：
      `keeps the inventory when doctor rejects cli_unavailable` 改为 `node_missing`，
      断言 `runtimeError` 不含原始 stderr 的部分保留。
- [x] 5.5 新增用例：doctor 成功时，批量栏 Unlink / Uninstall 与卡片卸载按钮均可交互（AC4）。
- [x] 5.6 新增用例：`skills_cli_add_global` 失败返回 `cli_unavailable` 时，
      错误经 toast 呈现且 header 状态行仍显示 `doctorOk`（AC6）。
- [x] 5.7 `src/test/components/skillsCli/SkillsCliUninstallDialog.test.tsx`、
      `SkillsCliDetailDrawer.test.tsx`：移除 `runtimeBlocked` prop 传入。
- [x] 5.8 **（TPR-05）** 新增 `SkillsCliHeader.test.tsx` 用例：`runtimeError` 为
      `skills_cli.timeout` 与 `internal.unexpected` 两个瞬时/未知码时，
      断言状态行显示对应公开句、Install 禁用、Refresh 按钮仍可点击。
      这两条覆盖 fail-closed 策略，防止「只测 node_missing」漏掉真实错误态。
- [x] 5.9 **（TPR-05）** 新增 `skillsCliStore.test.ts` 用例：doctor 以
      `skills_cli.timeout` 拒绝时 `runtimeError` 被写入且库存不被清空
      （确认分轨对非 `node_missing` 码同样成立）。

验证：`pnpm vitest run src/test/pages/SkillsCliView.test.tsx src/test/components/skillsCli src/test/stores/skillsCliStore.test.ts`

## 段 6 — 收尾

- [x] 6.1 i18n：确认段 3 新增的 `backendErrors.skills_cli.cli_failed` en/zh 成对（AC10）。
- [x] 6.1b 确认 `generatedCommandMap.ts` 的改动全部来自 `pnpm ipc:codegen`，无手工编辑。
- [x] 6.2 `rg "runtimeBlocked" src/` 确认无残留。
- [x] 6.3 `rg "build_probe_argv" src-tauri/` 确认只剩定义与可能的远端预留，doctor 路径无引用。
- [ ] 6.4 全量：`just ci`（skipped: dispatch asked not to run the release gate）

## 风险文件与回滚点

回滚单元见 `design.md` §6。**段 1 与段 2 是同一个原子单元 A**，
不得分别回滚——半回滚要么编译失败，要么让 `cli_unavailable` 横幅回归（TPR-06）。

| 文件 | 风险 | 回滚单元 |
| --- | --- | --- |
| `services/skills_cli/argv.rs` | launcher 拆分若改错错误语义，会让 add 路径丢失 `cli_unavailable` | **A（段 1+2 原子）** |
| `services/skills_cli/mod.rs`（doctor 段） | doctor 与 add 两处同改，注意不要把 add 的 `NodeMissing` 也改掉 | **A（段 1+2 原子）** |
| `services/skills_cli/runner.rs` | spawn 分支改错会波及所有 skills_cli 子进程调用，不只 add | B（段 3） |
| `services/skills_cli/mod.rs`（add warn） | warn 字段若误加 stderr 派生值会破坏 AC7 | B（段 3） |
| `pages/SkillsCliView.tsx` | 与 `08-27-skills-cli-bulk-cleanup` 冲突面，必须先合入本任务 | C（段 4+5） |
| `.trellis/spec/backend/skills-cli-global.md` | spec 与实现必须同批次提交，禁止只改一边 | 与 B 同批 |

提交建议：A 一个 commit、B 一个 commit、C 一个 commit，便于按单元 revert。

## 前置检查

- [ ] 确认 `08-26-ssh-update-observability-dialog` 树未在同一工作树改动
      `services/skills_cli/` 或 `ipc_registry.rs`，避免冲突。
- [ ] 确认工作树干净（`git status --porcelain` 只有 `src-tauri/target/` 构建产物）。
