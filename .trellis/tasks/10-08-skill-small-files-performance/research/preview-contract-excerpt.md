# 性能任务需要保留的 GitHub preview 合同

此摘要用于 task context 注入。原文件 `.trellis/spec/backend/github-import-preview-contract.md` 为 69,407 bytes，超过当前单文件注入 32,768 bytes；关键不可变 snapshot 合同位于文件后段，截断可能丢失。下列要求来源于原文件的 `Immutable Preview Snapshot Lifecycle` 场景（`:982`）；原文及当前源码优先。

## 导入权威与校验

- 每次 preview 只解析一次 commit；tree/raw/archive 均使用该 SHA，display branch 保留（原规范 `:1052` 起）。
- preview 保留实际内容；tree 下载全部 candidate 子树，根 candidate/超阈值使用 archive。
- `preview_id` 必填；顺序为 lease、binding、selection、digest verification，再 mutation。禁止重新解析 branch、重新下载 Local HTTP 或创建新 workspace fallback（原规范 `:1063` 起）。
- lease 单持有者；失败回 Ready 允许原 token 重试；成功原子 consume；lease 中的 discard 延迟处理（原规范 `:1069` 起）。
- repository/skill digest 保留 domain-separated `sha256-v1` 和原 framing/排序，不能与 update/delete fingerprint 混用（原规范 `:1034`、`:1060`）。

## 所有权与清理

- registry 有界：每 target 最多 4 个 Ready preview、Local retained bytes 最多 256 MiB、全局最多 64 项。Importing 占用预算且不能被 evict（原规范 `:1072` 起）。
- Local 过期项按 target 同步回收；Remote 到 CleanupPending，lookup/import fail closed，直到 owning target 删除并 ack generation ticket（原规范 `:1077` 起）。
- remote admission reservation 必须在创建 workspace 前取得；取消/失败后的已拥有 workspace 保留在同一 CleanupPending 槽位。
- sweep/cleanup 只能处理 owning target；remote 删除在 registry mutex 外；失败保留 pending ticket。renderer reset/替换 preview/切 target/关闭须 discard（原规范 `:1090` 起）。

## 持久化与保密

- per-skill commit/digest provenance 与 skill upsert/repository assignment 同事务写入 `skill_repository_members`；无 provenance 的后续写者不能清掉已知值（原规范 `:1096` 起）。
- repository snapshot digest 只与自身比较，transport retained 内容范围可能不同；不能用 Local repository digest 与 Remote repository digest 做直接等价判断。
- 错误使用既有稳定生命周期 code；禁止记录/展示 token、workspace path、digest 或文件内容（原规范 `:1107` 起）。

改变 acquisition、lease、snapshot bytes 或清理协议时，实施者须按需读取原规范对应完整场景，不能只凭摘要变更合同。
