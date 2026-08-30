# Codex 规划审阅处置（2026-08-24）

只读复核结论：5 Blocking、3 Should-fix、1 Note **全部成立**。下列机制已写回 `prd.md` / `design.md` / `implement.md`。

| ID | 原判定 | 复核 | 规划处置 |
| --- | --- | --- | --- |
| 1 并发互斥 | Blocking | 成立。exclusive job family 允许并行（`exclusive-job-lifecycle.md:5`）。install/uninstall 持有 `acquire_target_mutation_guard`（`install.rs:27,117`）。leftover apply 走 `central_update_jobs` 且 **不** 取 target mutation lock（`leftover_cleanup.rs:50-61`）。 | add/remove 必须取 Local target mutation lock；leftover 本地删除同样取该锁；exclusive job 只负责取消/进度。 |
| 2 Local-only | Blocking | 成立。`detect_agents` 按 ActiveTarget 分流（`agents/mod.rs:395-399`）。leftover 扫描只收 `active_db` + `agent_ids`（`skill_update_inventory.rs:761-768`）。 | 全部 skills_cli IPC 在非 Local 拒绝；leftover CLI 保护仅 Local；侧栏远程隐藏。 |
| 3 两层 -y | Blocking | 成立。[npx v11](https://docs.npmjs.com/cli/v11/commands/npx)：npx 旗标必须在位置参数前；之后的 `-y` 交给 `skills`。未安装包时 npx 会提示，需 `--yes`。npm `skills@1.5.23` 的 `gitHead` 为 `435076e`。 | argv：`npx --yes --package=skills@1.5.23 -- skills …`；skills 层另加 `-y`。预览 parser 锁该版本 fixture。 |
| 4 Windows npx.cmd | Blocking | 成立。Rust 1.97 对批处理/`cmd.exe` 有特殊处理。source 是用户 URL。 | Windows 用 `node.exe` + npx JS CLI，不用 `npx.cmd` 作为可执行文件。source 字符白名单。 |
| 5 AC 闭环 | Blocking | 成立。AC 未编号、未标 `[R#]`；R9/R10 无对应 AC；implement 缺取消测试。 | AC1–AC16 编号并标注 requirement；补并发/取消/超时/cap/Job Object/redaction/隔离测试。 |
| 6 Universal leftover | Should-fix | 成立。10 个 Universal Agents 共享 `~/.agents/skills`（`types.rs:70-81`）。 | 所有权以 lock 为准，禁止仅凭目录排除整个 canonical 根。 |
| 7 IPC 映射 | Should-fix | 成立。未知 Display → `internal.unexpected`（`ipc_error.rs:38-50`）；`legacy_code_message` 无 `skills_cli.*`。 | 每变体固定 code/message/retryable，登记 allowlist。 |
| 8 mapping 闭包 | Should-fix | 成立。seed 含 firebender/kimi-code-cli/warp/reasonix/openclaw/aider 等，设计表遗漏。 | 全 builtin id 表驱动：已映射或明确不支持。 |
| 9 base_branch | Note | 成立。`task.json` 为 `main`；任务 PR 目标为 `dev`。 | `set-base-branch dev`。 |

仍为 UNVERIFIED（实施后补证据）：实机 Windows Job Object 回收、junction 的 `link_type`、私有 git 凭据 redaction、native UI。
