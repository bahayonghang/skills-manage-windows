# Trellis 子代理运行韧性

## Goal

给子代理 context 注入、线性同步和任务创建增加严格资源上限及可诊断失败，避免无限输出、挂起进程与隐式网络阻塞。

## Findings

- `TOOL-001`（Medium / S）：`.codex/hooks/inject-subagent-context.py:276-299,288-319,362-447` 与 `.claude/hooks/inject-subagent-context.py` 的 notice/reason/JSONL 汇总可绕过 `max_total_bytes`，大输入会产生无硬上限上下文。
- `TOOL-002`（Medium / M）：`.trellis/scripts/common/task_utils.py:242-280` 与 `.trellis/scripts/hooks/linear_sync.py:90-104` 的 hook/subprocess 没有 timeout、输出上限或进程树终止。
- `TOOL-003`（Medium / S）：`.trellis/scripts/common/git.py:41-55` 的 `resolve_default_branch` 可能调用无 timeout 的 `git remote show origin`，且 `.trellis/scripts/common/task_store.py:296-367` 在该探测前已创建任务目录。
- `TOOL-004`（Low / S）：`.agents/skills/trellis-spec-bootstarp/`、`.claude/skills/trellis-spec-bootstarp/` 与正确的 `trellis-spec-bootstrap` 并存，增加路由歧义。

## Requirements

- R1：context 注入对文件数、JSONL 行数、单文件字节、artifact 字节和最终总 payload 字节实施固定硬上限；正文、wrapper、标题、notice、reason、索引和截断摘要都从同一总预算扣减，预算或行/文件上限耗尽后停止继续读取。
- R2：`run_task_hooks` 与 Linear `linearis` 调用具有固定 deadline、有界 stdout/stderr、超时标志和 Windows 后代进程树清理；用户可见诊断经过长度限制且不输出环境、凭据或未截断的子进程内容。
- R3：`task create` 的默认分支只从本地 refs/config 推断，并在任何任务目录或 seed 文件写入前完成；普通创建不得隐式访问网络，无法推断时沿用明确 `--base-branch` 或既有本地 fallback。
- R4：删除两套平台中错误拼写的 `trellis-spec-bootstarp` skill，仅保留 `trellis-spec-bootstrap`；catalog 合同阻止重复 name、重复 entrypoint 和已知错拼再次出现，不保留 alias。
- R5：本任务不引入常驻队列、新运行时服务、依赖包或路径安全的第二套实现；路径逃逸只由 `trellis-path-security` 闭环。

## Acceptance Criteria

- [x] AC1（R1）：超长 reason/notice、超大 artifact、超量文件和超量 JSONL 行的每类 fixture，其最终 UTF-8 payload 均不超过 `max_total_bytes`。
- [x] AC2（R1）：达到文件数上限后，测试探针证明后续 context 目标未被打开。
- [x] AC3（R1）：达到 JSONL 行数上限后，测试探针证明后续行引用的目标未被打开。
- [x] AC4（R1）：预算耗尽只生成一个预算内截断摘要，合法小输入的字节内容与顺序保持不变。
- [x] AC5（R2）：超过 deadline 的 task hook 返回可识别的 timeout 结果。
- [x] AC6（R2）：AC5 的父进程和后代进程均在有界时间内终止，cleanup 失败被单独标记。
- [x] AC7（R2）：task hook 的 stdout/stderr 驻留和用户可见诊断均受固定字节上限约束，fixture secret 不出现在诊断中。
- [x] AC8（R2）：`linearis` 的正常、非零退出、timeout 和无效 JSON 结果可由调用方确定区分。
- [x] AC9（R2）：`linearis` 的超大 stdout/stderr 被有界捕获，fixture secret 不出现在诊断中。
- [x] AC10（R3）：无 `origin/HEAD` 的离线临时 repo 中，测试探针证明 `task create` 不调用网络命令。
- [x] AC11（R3）：默认分支无法解析或后续输入校验失败时，临时 repo 不留下半创建任务目录或 seed JSONL。
- [x] AC12（R3）：本地 `refs/remotes/origin/HEAD`、显式 `--base-branch` 和仅有当前分支三类 fixture 分别得到确定 `base_branch`，task JSON 形状不变。
- [x] AC13（R4）：`.agents/skills` 与 `.claude/skills` 中均不存在 `trellis-spec-bootstarp`，且 canonical `trellis-spec-bootstrap` 仍存在。
- [x] AC14（R4）：catalog 合同对重复 skill name、重复 entrypoint 或 `bootstarp` 字符串分别失败。
- [x] AC15（R5）：实施 diff 不包含常驻队列、新运行时服务、新依赖或第二套路径 containment。
- [x] AC16（R1、R2、R3、R4）：聚焦 Python/contract 测试、相关脚本编译检查和任务结构校验分别通过。
- [x] AC17（R1、R2、R3、R4、R5）：真实长会话、真实 Linear 和未运行平台的进程树证据均显式标记 `UNVERIFIED` 或 `missing evidence`。
- [x] AC18（R1、R2、R3、R4、R5）：集成阶段完整 `just ci` 通过。

## Out of Scope

- 引入常驻任务队列或新的运行时服务。
- 为历史错拼 skill 增加兼容层。
- 重复实现 `trellis-path-security` 的 repo-containment 规则。
