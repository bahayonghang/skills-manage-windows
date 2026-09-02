# Trellis 路径边界与上下文隔离

## Goal

封闭任务目录写入和 context manifest 读取边界，阻止仓库外文件被写入或注入子代理上下文。

## Findings

- `SEC-002`（High / S）：`.trellis/scripts/common/task_store.py:296-300,306-347,419-437` 接受显式 slug 后直接拼接任务目录，`..`、绝对路径、UNC 或驱动器路径可逃逸 `.trellis/tasks/`。
- `SEC-001`（High / M）：`.trellis/scripts/common/task_context.py:77-85` 与 `.codex/hooks/inject-subagent-context.py:242-248,428-443,605-622`、`.claude/hooks/inject-subagent-context.py:242-248,428-443,605-622` 在消费 context 路径时没有做最终 realpath containment，可能读取仓库外文本并注入模型。

## Requirements

- R1：任务 slug 只接受非空 ASCII 小写字母、数字和单连字符组成的单个名称；在创建目录或其他任务文件前拒绝绝对路径、驱动器路径、UNC、任意分隔符、`.`、`..`、空值及最终落点不是 `.trellis/tasks/` 直接子项的路径。
- R2：`add-context` 在改写 `implement.jsonl` / `check.jsonl` 前同时执行词法检查和最终解析后的仓库根 containment，拒绝经 symlink、junction/reparse point 或大小写差异逃逸的文件和目录。
- R3：`.codex` 与 `.claude` 注入 hook 把 JSONL 视为不可信输入，在打开每个 context 路径前独立重复 R2 的最终 containment；拒绝时不得读取、输出或在诊断中回显仓库外文件内容。
- R4：CLI 与两套镜像 hook 对同一合法/非法路径向量给出一致的允许或拒绝结果，并保留合法仓库内文件和目录的现有注入行为。
- R5：不增加旧 slug 迁移、兼容 alias、仓库外 allowlist 或额外路径配置；路径判断只在任务写入边界和 context 消费边界各执行一次。

## Acceptance Criteria

- [ ] AC1（R1）：显式 slug 的空值、`.`、`..`、分隔符、绝对路径、驱动器路径、UNC 和非封闭语法矩阵均在首次任务目录写入前返回非零结果。
- [ ] AC2（R1）：预先存在的 symlink/junction/reparse task 落点不能导致 `.trellis/tasks/` 外的 sentinel 发生字节变化。
- [ ] AC3（R2）：`add-context` 对词法逃逸和最终解析逃逸矩阵均返回非零结果。
- [ ] AC4（R2）：AC3 的每个失败用例执行前后，目标 JSONL 字节完全相同。
- [ ] AC5（R3）：两套注入 hook 对手工植入 JSONL 的逃逸矩阵均在读取目标内容前拒绝。
- [ ] AC6（R3）：AC5 的每个拒绝结果均不包含 sentinel 内容或仓库外绝对路径。
- [ ] AC7（R2、R3）：合法仓库内文件和目录可由 `add-context` 保存并由 hook 注入。
- [ ] AC8（R4）：`.codex` 与 `.claude` hook 对同一测试向量产生等价结果，镜像差异检查无未说明漂移。
- [ ] AC9（R4）：Windows runner 覆盖驱动器、UNC、大小写和 junction/reparse 向量；未运行时记录 `missing evidence`。
- [ ] AC10（R4）：POSIX runner 覆盖绝对路径、`..` 和 symlink 向量；未运行时记录 `missing evidence`。
- [ ] AC11（R5）：实施 diff 不包含旧 slug 迁移、兼容 alias、仓库外 allowlist 或新增路径配置。
- [ ] AC12（R1、R2、R3、R4）：聚焦 Python 测试、相关脚本编译检查和任务结构校验分别通过。
- [ ] AC13（R1、R2、R3、R4、R5）：集成阶段完整 `just ci` 通过。

## Out of Scope

- 重写 Trellis 任务存储格式。
- 给任意外部目录增加 allowlist。
- 清理与路径边界无关的 hook 运行时问题；由 `subagent-runtime-resilience` 负责。
