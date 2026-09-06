# 项目结构与会话隔离审查

日期：2026-09-06。基线：`dev` / `a81b7c2d`，起始工作区干净。本文件是只读审查与批准前规划材料。

## 项目结构与责任边界

| 路径 | 责任与已读证据 | 本轮审查重点 |
|---|---|---|
| `CONTEXT.md`、`AGENTS.md`、`code_map.md` | SkillPort 是 Windows-first React/Tauri 桌面应用；Central 与 Universal Agents 是不同目录 | 产品支持的平台不等于开发 harness 的接线能力 |
| `src/App.tsx`、`src/stores/`、`src/lib/ipc/index.ts` | 路由 → Zustand 域状态 → typed IPC/fixtures/error normalization | 前端测试与类型检查是逻辑证据，不能证明 WebView/安装器可用 |
| `src-tauri/src/lib.rs`、`src-tauri/src/services/github_import/remote.rs` | AppState 注入 DB、SecretStore、target/job registries；业务服务持有远程 target 边界 | Rust/服务边界、凭据、持久化风险由强模型审查 |
| `scripts/check/run-ci.mjs`、`justfile`、`package.json` | quick/common/rust-platform；本地 just 先同步版本，远程直接只读编排 | 原始门禁失败与替代入口测试结果分开记录 |
| `.github/workflows/`、`docs/agents/git-and-release.md` | PR 多平台 CI、桌面发布与 Pages 发布分别有触发与验收边界 | 远程 run 的具体 ref、失败步骤和日志，不把旧失败泛化到 HEAD |
| `.trellis/scripts/common/active_task.py`、`paths.py`、`session_context.py` | 当前任务解析、会话状态及给 Agent 的摘要 | 已复现跨会话借用与清理边界缺陷 |
| `.trellis/tasks/archive/2026-09/09-02-engineering-audit-remediation/` | 上轮改造已完成归档，REL 以显式合同 wontfix 关闭 ledger | 不将历史规划状态或旧 finding 当作当前缺陷；保留风险和不实施决定 |

## SES-001 · P1 · 已知新会话借用旧会话任务，清理时删除其他会话记录

**现象与实证。** 开场 `get_context.py` 在 active task 列表为空时返回旧 `08-26-runtime-diagnostics-correlation`，来源 `session-fallback`。隔离夹具 `research/probe-active-task.py` 直接调用当前源码，未改真实 `.trellis/.runtime/`：

```text
new-session-missing-task: task_path=.trellis/tasks/01-01-old, source=session-fallback, stale=true
new-session-borrows-live-task: context_key=codex_old, source=session-fallback, stale=false
clear-new-session: cleared_source=session-fallback:codex_old, other_session_survives=false
```

复现命令：`python -X utf8 .trellis/tasks/09-06-evergreen-five-harness-audit/research/probe-active-task.py`，exit 0（该脚本输出观测，不把 exit 0 当作正确性断言）。

**根因。** `.trellis/scripts/common/active_task.py:584` 在已解析到新 context_key、但该会话没有 current_task 时，仍向 `:591` 单会话 fallback 继续执行；`clear_active_task` 在 `:689` 使用相同解析，随后按返回的旧 `context_key` 于 `:693` 删除会话文件。不是模型“记性差”，是任务所有权在共享解析层丢失。`task.py:225` 的当前任务 workflow 写入同样依赖该解析路径，应在修复验收覆盖。

**最小改造方向。** 已知身份只读取自己的任务；无身份的子代理使用明确的 `Active task: <path>`/context 注入，不靠任意旧会话推断。清理只作用于本次确定的会话 key，不能经 fallback 清理他人记录。保留 stale 状态供诊断，但摘要不得把 stale 任务表述为可继续执行任务。共享边界一次确定身份，不添加新的 session alias 表、迁移层或清理守护进程。

**文件与验收。** 修改 `active_task.py`；按需调整 `session_context.py` 的 text/json/record 呈现及 `task.py` 的 current-session mutation 路径。在 `.trellis/scripts/tests/test_active_task_isolation.py` 新增行为回归：新会话+唯一旧任务、缺失旧任务、无身份、两个会话、同会话正常读取/清理、workflow 写入不跨会话；运行全 Python Trellis tests 与现有 context-injection tests，五工具真实 session smoke 单独列证据。仅临时 fixture 允许删除，绝不拿真实 session 验证删除路径。

**工具分工。** Codex/Claude Code 强模型负责共享会话语义及独立审查；Grok Build/Kimi Code 负责批准合同下的 pull-context smoke；OMP 负责 session extension smoke。便宜模型只可按已定断言补夹具/收集日志，不自主修改 ownership 或 fallback 语义。

## DOC-STATE-001 · P2 · 发布风险的执行状态说明落后于归档合同

`docs/agents/git-and-release.md:44` 附近称 REL-001/REL-002 “stay open”；归档父任务 `research/rel-001-002-wontfix-contract-2026-09-03.md:20` 已确定合同关闭、风险保留，`:37` 明确重开需要新授权。两者会使新 harness 把非执行项重复立项。批准后应将活动项目指南更新为“wontfix by recorded scope decision; residual risk remains”，链接既有合同，不改历史报告、不改签名工作流、不声称 fixed。

## TASK-DOC-001 · P2 · 项目任务默认 PR base 与实际创建入口不一致

`AGENTS.md:44` 要求 task PR → dev；`task.py create` 本轮实测种子 `base_branch=main`。`.trellis/scripts/common/task_store.py:271` 优先明确参数，否则从默认远程分支推导。通用 Trellis 行为本身成立，项目启动说明缺少 `--base-branch dev`。本轮已只更正新规划父任务 metadata 为 dev，后续 child 明确传入该参数；建议更新项目创建示例，不新增通用分支配置。

## 保留的边界

REL-001/REL-002 仍有真实残留风险，但既有不实施决定继续有效。测试/夹具不能证明包内 EXE 预签、真实证书、SSH/WSL/provider 或用户 Windows 安装体验。未获得该证据的部分均为 UNVERIFIED。
