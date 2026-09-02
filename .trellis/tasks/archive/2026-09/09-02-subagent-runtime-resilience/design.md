# Design

## Change List

| File / symbol | Minimal change | Covers |
| --- | --- | --- |
| `.trellis/scripts/common/config.py` / `get_context_injection_limits` | 给文件数与 JSONL 行数补内部固定常量并随现有限制返回；保留现有可配置字节键，不新增用户配置项 | R1、R5 |
| `.codex/hooks/inject-subagent-context.py` / `_Budget`、`_budgeted_block`、`read_jsonl_entries`、`_materialize_jsonl_entries`、`get_agent_context` | 所有片段先按 UTF-8 预算裁剪再追加；行/文件/总预算耗尽立即停止读取并只产生一个预算内摘要 | R1 |
| `.claude/hooks/inject-subagent-context.py` / 同名符号 | 与 Codex hook 保持同一预算语义和测试向量 | R1 |
| `.trellis/scripts/common/subprocess_supervision.py` / 新增 `run_bounded_process` | 唯一有界 subprocess helper：deadline、增量/有界捕获、timeout 状态、Windows process-group/tree cleanup、截断诊断 | R2、R5 |
| `.trellis/scripts/common/task_utils.py` / `run_task_hooks` | 用公共 helper 执行配置 hook，只消费结构化有界结果 | R2 |
| `.trellis/scripts/hooks/linear_sync.py` / `_linearis` | 用公共 helper 执行 `linearis`，区分 timeout、非零退出和无效 JSON | R2 |
| `.trellis/scripts/common/git.py` / `resolve_default_branch`、`.trellis/scripts/common/task_store.py` / `cmd_create` | 删除 `git remote show origin`；默认分支解析移到 `ensure_tasks_dir` / `mkdir` 之前，仅使用本地 refs/config 与明确 fallback | R3、R5 |
| `.agents/skills/trellis-spec-bootstarp/`、`.claude/skills/trellis-spec-bootstarp/` | 删除错拼镜像目录，不保留 alias | R4 |
| `src/test/contracts/developerExperienceContract.test.ts` / Trellis skill catalog contract | 解析两套 skill frontmatter，断言名称/入口唯一、镜像集合一致、无 `bootstarp` | R4 |
| `.trellis/scripts/tests/test_runtime_resilience.py` / 新增聚焦测试 | 超量 context、挂起进程树、有界输出、离线 task create 的临时 fixture | R1、R2、R3、R5 |

## Contract

- R1 / AC1-AC4：`_Budget` 是最终 payload 的唯一字节账本；任何字符串只能通过 budget-aware append 进入结果。文件数/JSONL 行数先受固定计数上限约束，再受相同总预算约束；停止条件不得通过 notice 再次超限。
- R2 / AC5-AC9：`run_bounded_process` 返回 `returncode`、有界 `stdout`/`stderr`、`timed_out`、`output_truncated`、`cleanup_failed`；调用方不得再次直接启动同一类子进程或回显原始环境。
- R3 / AC10-AC12：`resolve_default_branch` 只读取 local symbolic ref/config；`cmd_create` 在完成标题/meta/package/workflow/slug 与 base branch 决策前不创建 tasks 根、任务目录或 seed JSONL。
- R4 / AC13-AC14：正确 skill 名是唯一 canonical identity；删除错拼目录是有意破坏性清理，不提供兼容转发。
- R5 / AC15 限制实现面；AC16 是聚焦自动门禁，AC17 是人工与外部证据边界，AC18 是完整集成门禁。

## Compatibility

- 保留现有 `max_file_bytes`、`max_artifact_bytes`、`max_total_bytes` 键和合法小 payload 的顺序；新增内部固定计数上限只截断此前无界输入，不增加用户配置表面。
- 正常快速 hook/Linear 命令的退出码和输出语义保持；超过 deadline 或输出上限的行为从“可能挂起/无界”变为有界失败。
- task JSON schema 不变；有本地 `origin/HEAD`、显式 base branch 和当前分支 fallback 保持可用，不再通过网络修复缺失的 remote HEAD。
- `trellis-spec-bootstarp` 使用者必须改用 canonical 名称；不保留 alias 或迁移 shim。

## Verification Boundary

- 自动测试分别证明 AC1-AC16；不得以单一“命令返回”替代预算、停止读取、父子进程清理、输出截断、离线无网络、零半成品目录和 catalog 唯一性的独立断言。
- Windows process-tree 测试只在 Windows 证明本机后代清理；POSIX 需各自 runner 证据。平台未运行时标记 `missing evidence`。
- fixture 不证明真实 Linear 可用性、第三方 hook 的所有行为或数小时真实代理会话稳定性；这些保持 `UNVERIFIED`。

## Rollback

- Rollback point 1：预算与测试为独立单元；如合法小输入改变，成对回退两套 hook 预算修改，保留其他修复。
- Rollback point 2：`subprocess_supervision.py` + `run_task_hooks` + `_linearis` 为原子单元；若正常命令回归，整体回退，不留下某一调用方无监督。
- Rollback point 3：local-only default branch + create 前置重排为原子单元；回退不影响预算/监督。
- Rollback point 4：两套错拼 skill 删除 + catalog contract 为原子单元；回退必须同时恢复或删除两个镜像，不能形成平台漂移。
- 无数据库、task JSON schema 或外部服务数据迁移；回退不需要数据 backfill。

## Considered but Not Chosen

- 不为每种片段维护独立可调预算，避免多账本对账和配置膨胀；最终 payload 只认一个总预算。
- 不引入 daemon、任务队列或第三方 process 库；两个现有调用点复用一个小 helper 足够。
- 不保留远程 default-branch fallback、错拼 alias 或无限 timeout 开关，因为会重新打开已识别的故障方向。
- 不在本任务重复路径 containment；由兄弟任务提供并在集成时复用。
