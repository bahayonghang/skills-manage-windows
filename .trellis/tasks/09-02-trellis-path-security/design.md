# Design

## Change List

| File / symbol | Minimal change | Covers |
| --- | --- | --- |
| `.trellis/scripts/common/paths.py` / 新增单一 repo-containment helper | 接收不可信相对路径和允许根，返回已解析且确认受控的规范路径；使用 path component/`relative_to`，不使用字符串前缀 | R2、R3、R4 |
| `.trellis/scripts/common/task_store.py` / `cmd_create`、新增 `_validate_slug` | 在 `ensure_tasks_dir` 和 `task_dir.mkdir` 前验证封闭 slug；最终 task 落点必须是 tasks 根的直接子项，已有 reparse 落点 fail closed | R1、R5 |
| `.trellis/scripts/common/task_context.py` / `cmd_add_context`、`_resolve_context_entry_path` | 写 JSONL 前调用公共 containment，持久化规范 repo-relative 路径；不在校验失败时改写 seed/JSONL | R2、R4 |
| `.codex/hooks/inject-subagent-context.py` / `_read_file_bytes`、`read_jsonl_entries`、`_materialize_jsonl_entries` | 在任何 open/read 前重新验证 manifest 路径，返回有界且不泄露外部路径的拒绝结果 | R3、R4 |
| `.claude/hooks/inject-subagent-context.py` / 同名符号 | 应用与 Codex hook 相同的消费边界和测试向量 | R3、R4 |
| `.trellis/scripts/tests/test_path_security.py` / 新增聚焦测试 | 临时 repo、外部 sentinel、symlink/junction fixture 覆盖 CLI 与两套 hook | R1、R2、R3、R4、R5 |

## Contract

- R1 / AC1-AC2 的可信边界是 `cmd_create`：未经 `_validate_slug` 和“tasks 根直接子项”检查，不得调用 `ensure_tasks_dir`、`mkdir` 或写任务工件。
- R2 / AC3-AC4、AC7 的可信边界是 `cmd_add_context`：仅保存公共 helper 返回的规范 repo-relative 路径；词法安全但 realpath 逃逸同样拒绝。
- R3 / AC5-AC7 的可信边界是每套 hook 的 `_read_file_bytes` / materialize 路径：不能信任 CLI 曾校验过 JSONL，拒绝发生在读取目标内容之前。
- R4 / AC8-AC10 通过一份参数化路径向量分别驱动 CLI、Codex hook 和 Claude hook；允许/拒绝集合必须相同，平台专属构造只在对应平台运行。
- R5 / AC11 明确限制实施 diff；AC12-AC13 分别承担聚焦门禁与完整集成门禁。
- AC6 的错误仅包含稳定错误类别和经截断的 manifest 相对值；不得包含已解析的仓库外绝对路径、sentinel 内容或文件片段。

## Compatibility

- 合法的现有 ASCII slug、repo-relative context 文件和目录保持原行为。
- 非封闭显式 slug、仓库外 context、经 symlink/junction/reparse 逃逸的路径将立即失败，这是安全性破坏性收紧，不提供 alias、迁移或例外开关。
- `implement.jsonl` / `check.jsonl` 格式不变；无需旧数据回填。已存在的非法条目由 hook 在消费时 fail closed。

## Verification Boundary

- 自动测试分别证明 AC1-AC8、AC11-AC12；不得用一条“无异常”断言替代写入、读取、诊断与合法回归证据。
- Windows junction/reparse 与 POSIX symlink 必须在各自平台构造，分别对应 AC9、AC10；未运行的平台标记 `missing evidence`。
- 自动 fixture 不证明任意第三方文件系统或网络挂载的全部 reparse 语义；这些保持 `UNVERIFIED`，不扩展本任务到外部 allowlist。

## Rollback

- Rollback point 1：仅新增失败测试，不改变运行时，可独立撤回。
- Rollback point 2：公共 helper 与 `task_store` slug 边界为一个原子单元；若合法 create 回归，整体回退该单元，不放宽部分规则。
- Rollback point 3：`task_context` 与两套 hook 消费校验为一个原子单元；若镜像或合法注入回归，整体回退并保留 Rollback point 2。
- 没有 schema、任务数据或外部状态迁移；回退仅恢复脚本和测试文件。

## Considered but Not Chosen

- 不用字符串 `startswith` 做 containment，因为大小写、前缀同名目录和分隔符会产生歧义。
- 不只在 `add-context` 校验，因为 JSONL 可被手工编辑或由旧版本生成。
- 不建立仓库外 allowlist、路径 alias 表或旧 slug 兼容层，避免增加第二套策略与持久化迁移。
