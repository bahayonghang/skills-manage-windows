# Implementation Plan

## Ordered Steps

1. [x] 在 `.trellis/scripts/tests/test_path_security.py` 建立 `TemporaryDirectory` repo、仓库外 sentinel、合法文件/目录，以及平台可用的 symlink/junction fixture；先固化 AC1-AC10 的允许/拒绝矩阵和独立的零写入、零读取、诊断不泄漏、合法回归断言。
   - 验证命令：`python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_path_security.py"`
   - 预期：新增安全用例在实现前失败，既有合法基线通过；测试不得在真实 `.trellis/tasks/` 创建任务。
   - Rollback point：仅测试文件，可独立撤回。
2. [x] 在 `.trellis/scripts/common/paths.py` 增加单一 containment helper，并在 `.trellis/scripts/common/task_store.py::_validate_slug` / `cmd_create` 接入 R1 / AC1-AC2；把 slug 与最终直接子项检查移动到 `ensure_tasks_dir` 及任何 `mkdir` 之前。
   - 验证命令：`python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_path_security.py" -k task_slug`
   - 验证命令：`python -X utf8 -m py_compile .trellis/scripts/common/paths.py .trellis/scripts/common/task_store.py`
   - Rollback point：helper + task create 边界作为一个原子单元回退。
3. [x] 在 `.trellis/scripts/common/task_context.py::cmd_add_context`、`_resolve_context_entry_path` 接入同一 helper，分别满足 AC3、AC4、AC7：拒绝逃逸、失败零改写、合法条目只保存规范 repo-relative 路径。
   - 验证命令：`python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_path_security.py" -k add_context`
   - 验证命令：`python -X utf8 -m py_compile .trellis/scripts/common/task_context.py`
   - Rollback point：`task_context` 接线可在不撤回 task create 修复的情况下回退。
4. [x] 在 `.codex/hooks/inject-subagent-context.py` 与 `.claude/hooks/inject-subagent-context.py` 的 `_read_file_bytes`、`read_jsonl_entries`、`_materialize_jsonl_entries` 接入消费时校验；分别满足 AC5-AC8：读取前拒绝、诊断不泄漏、合法注入、镜像等价。
   - 验证命令：`python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_path_security.py" -k hook`
   - 验证命令：`python -X utf8 -m py_compile .codex/hooks/inject-subagent-context.py .claude/hooks/inject-subagent-context.py`
   - 验证命令：`powershell -NoProfile -Command "(Get-FileHash .codex/hooks/inject-subagent-context.py).Hash -eq (Get-FileHash .claude/hooks/inject-subagent-context.py).Hash"`
   - Rollback point：两套 hook 必须成对回退；不得留下单平台放宽。
5. [x] 用原始攻击方向复测 AC1-AC10，并静态核对 AC11；Windows 与 POSIX 平台证据分别记录，不互相替代。
   - 验证命令：`python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_path_security.py"`
   - 验证命令：`python -X utf8 .trellis/scripts/task.py validate .trellis/tasks/09-02-trellis-path-security`

## Integrated Verification

```powershell
python -X utf8 -m py_compile .trellis/scripts/common/paths.py .trellis/scripts/common/task_store.py .trellis/scripts/common/task_context.py .codex/hooks/inject-subagent-context.py .claude/hooks/inject-subagent-context.py
python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_path_security.py"
python -X utf8 .trellis/scripts/task.py validate .trellis/tasks/09-02-trellis-path-security
just ci
```

通过条件：AC1-AC13 各有对应证据；`git diff -- .trellis/tasks` 之外的实施 diff 只包含 Change List 文件；不存在仓库外写入/读取；完整门禁无失败。

## Manual / External Evidence

- 在 Windows runner 记录 junction/reparse 与大小写用例；在 Linux/macOS runner 记录 POSIX symlink 用例。任何未执行平台均报告 `missing evidence`。
- 网络共享、第三方文件系统和特殊 reparse provider 的语义不由本地 fixture 证明，保持 `UNVERIFIED`。
- 本规划阶段不执行修改、不启动任务，也不把结构校验当成运行时安全证明。
