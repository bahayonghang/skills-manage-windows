# Implementation Plan

## Ordered Steps

1. 在 `.trellis/scripts/tests/test_runtime_resilience.py` 建立超长 reason/notice、超量 JSONL/文件、挂起父子进程、超大 stdout/stderr 和离线临时 Git repo fixture；在 `src/test/contracts/developerExperienceContract.test.ts` 先加入 AC13-AC14 的 canonical skill catalog 断言。
   - 验证命令：`python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_runtime_resilience.py"`
   - 验证命令：`pnpm vitest run src/test/contracts/developerExperienceContract.test.ts`
   - 预期：新增 R1-R4 失败方向先失败，既有合同保持通过。
   - Rollback point：仅测试/contract 断言，可独立撤回。
2. 在 `.trellis/scripts/common/config.py::get_context_injection_limits` 以内部常量补固定计数上限，不新增 `.trellis/config.yaml` 键；修改两套 `inject-subagent-context.py` 的 `_Budget`、`_budgeted_block`、`read_jsonl_entries`、`_materialize_jsonl_entries`、`get_agent_context`，分别满足 AC1-AC4 的硬预算、停止读取、单摘要与小输入兼容。
   - 验证命令：`python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_runtime_resilience.py" -k context_budget`
   - 验证命令：`python -X utf8 -m py_compile .trellis/scripts/common/config.py .codex/hooks/inject-subagent-context.py .claude/hooks/inject-subagent-context.py`
   - 验证命令：`powershell -NoProfile -Command "(Get-FileHash .codex/hooks/inject-subagent-context.py).Hash -eq (Get-FileHash .claude/hooks/inject-subagent-context.py).Hash"`
   - Rollback point：配置解析与两套 hook 成对回退；不保留单平台差异。
3. 新增 `.trellis/scripts/common/subprocess_supervision.py::run_bounded_process`，并替换 `.trellis/scripts/common/task_utils.py::run_task_hooks` 与 `.trellis/scripts/hooks/linear_sync.py::_linearis` 的直接调用；用独立测试分别满足 AC5-AC9 的 timeout、后代 cleanup、诊断上限、结果分类和有界捕获。
   - 验证命令：`python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_runtime_resilience.py" -k process`
   - 验证命令：`python -X utf8 -m py_compile .trellis/scripts/common/subprocess_supervision.py .trellis/scripts/common/task_utils.py .trellis/scripts/hooks/linear_sync.py`
   - Rollback point：helper 与两个调用方作为一个原子单元回退。
4. 修改 `.trellis/scripts/common/git.py::resolve_default_branch` 为 local-only；在 `.trellis/scripts/common/task_store.py::cmd_create` 中把分支解析与已有输入校验移到 `ensure_tasks_dir` 和 `task_dir.mkdir` 前，分别满足 AC10-AC12 的零网络、零半成品和三类 base branch 结果。
   - 验证命令：`python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_runtime_resilience.py" -k offline_task_create`
   - 验证命令：`python -X utf8 -m py_compile .trellis/scripts/common/git.py .trellis/scripts/common/task_store.py`
   - Rollback point：local-only resolver 与 create 重排成对回退，不影响 R1/R2。
5. 删除 `.agents/skills/trellis-spec-bootstarp/` 与 `.claude/skills/trellis-spec-bootstarp/`，保留两套 `trellis-spec-bootstrap`；完成 AC13-AC14 的存在性和 catalog 唯一性/镜像合同，并核对 AC15 的范围限制。
   - 验证命令：`pnpm vitest run src/test/contracts/developerExperienceContract.test.ts`
   - 验证命令：`powershell -NoProfile -Command "-not (Test-Path .agents/skills/trellis-spec-bootstarp) -and -not (Test-Path .claude/skills/trellis-spec-bootstarp)"`
   - Rollback point：两个镜像删除与 catalog contract 同步回退。
6. 按 AC1-AC16 原始参数整体验证：确认超限 payload 硬封顶、读取停止、timeout 后无后代进程、离线 create 零网络/零半成品、skill catalog 唯一；按 AC17 单列外部证据状态，并以 `just ci` 闭环 AC18。
   - 验证命令：`python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_runtime_resilience.py"`
   - 验证命令：`python -X utf8 .trellis/scripts/task.py validate .trellis/tasks/09-02-subagent-runtime-resilience`

## Integrated Verification

```powershell
python -X utf8 -m py_compile .trellis/scripts/common/config.py .trellis/scripts/common/subprocess_supervision.py .trellis/scripts/common/task_utils.py .trellis/scripts/common/git.py .trellis/scripts/common/task_store.py .trellis/scripts/hooks/linear_sync.py .codex/hooks/inject-subagent-context.py .claude/hooks/inject-subagent-context.py
python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_runtime_resilience.py"
pnpm vitest run src/test/contracts/developerExperienceContract.test.ts
python -X utf8 .trellis/scripts/task.py validate .trellis/tasks/09-02-subagent-runtime-resilience
just ci
```

通过条件：AC1-AC18 分别有测试、静态检查、完整门禁或明确证据边界；无超预算 payload、遗留 fixture 子进程、网络 default-branch 调用或半创建任务；两套 skill/hook 镜像一致；完整门禁无失败。

## Manual / External Evidence

- Windows runner 必须记录真实后代进程树 timeout/cleanup；POSIX runner 分别记录其 process-group cleanup。未运行的平台标记 `missing evidence`。
- 真实 Linear 服务调用、用户自定义第三方 hook 和长时间真实子代理会话稳定性保持 `UNVERIFIED`；fixture 成功不能替代这些证据。
- 不需要也不授权在线 Linear 调用、依赖安装或任务启动；本规划阶段只修改规划文件。
