# 本任务依赖的 immutable GitHub snapshot 契约

权威规范：`.trellis/spec/backend/github-import-preview-contract.md`。该文件超过 Trellis 单文件上下文注入上限；本文只提取本任务需要的既有约束，实施或检查发生冲突时必须回读权威规范全文。

## 已有不变量

1. Repository preview 先把 branch/tag/default branch 解析为完整 commit SHA，之后的 tree/raw/archive acquisition 只能使用该 SHA。
2. Repository snapshot digest 使用稳定、domain-separated、按 path 排序的文件 manifest；Local 与 remote inventory 对相同文件必须得到相同 digest。
3. 用户确认后的读取/导入只能使用已确认 snapshot；不得再次解析 branch 或下载另一份 branch-tip 内容。
4. Local snapshot 和 Remote workspace 都是 bounded storage；import 前验证 target/repository binding、selection membership 和 digest，失败发生在 Central/DB mutation 前。
5. Remote workspace-only importer 不会重新获取 repository；workspace 无论成功或失败都必须按 lifecycle 清理。
6. Import provenance 把 resolved commit SHA 和 candidate content digest 写入 per-skill repository membership；repository display branch 保持用户配置值。
7. Snapshot lifecycle 错误使用稳定 code；动态 manifest、token、URL、source path、本地/远端 workspace path 和 raw stderr 不得进入 IPC、日志或 DB provenance。
8. GitHub token 只发送到受信 GitHub direct endpoints，绝不发送给 public mirror/proxy fallback。

## Central Update 对该契约的应用

- Central repository sync/update 不使用 renderer preview token，也不得伪造 `previewId`；它通过自己的 verified inventory snapshot 建立确认边界。
- 本任务应复用 commit resolution、pinned ref、repository digest 和 snapshot/workspace-only import 的实现语义，但不把 Update Center pending rows 塞入 wizard registry。
- Cache 只优化同一进程内的 byte 复用；跨 TTL、驱逐、oversized 或重启的 authority 来自 pending row 持久化的 full commit SHA + repository digest。
- Cache miss 只能按 full SHA 重取并校验 digest；它不是“允许重新 preview branch”的信号。
- SSH/WSL 可重新创建 pinned remote workspace，但必须先核对同一 repository digest 再开始 mutation。

## 实施/检查提示

- 修改 GitHub snapshot、remote workspace、digest、token routing 或 provenance 代码前，直接打开权威规范对应章节核对；本文不替代其完整错误矩阵和资源预算。
- 若需要改变已有 digest framing、preview registry 或 provenance schema，必须回到 planning 说明原因；本任务默认不改变这些已发布契约。
