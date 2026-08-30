# 集合、元数据、项目与 Obsidian 操作日志覆盖

状态：**planning**。依赖：`08-26-observability-core-contracts` 完成并冻结 interface。

## Goal

补齐 catalog/workspace 相关的用户写操作和外部打开/导入行为，使 repository/tag/collection/view/group/agent/
project/vault 的变更可审计、失败可追踪，且不把名称、路径、query 或内容当作诊断载荷。

## Requirements

- C1：Central repository/tag assignment、AI tag suggestion/review/cancel 记录 stable action、逻辑 subject ID、
  safe counts/status；AI prompt/response、repository URL/ref/path 不记录。
- C2：Collections、saved views、tag groups 的 CRUD/reorder/assignment/import/export/batch install 可审计；
  saved query、collection payload 和 icon/custom labels 不进入 details/error。
- C3：custom agents 的 detect/add/update/remove/enable 依据实际持久化语义分类并记录；platform paths/list 为
  runtime-only。
- C4：Projects 的 add/rename/pin/rescan/remove/install/uninstall 记录；folder picker/list/detail 为 runtime-only；
  project filesystem path 不进入任何日志。
- C5：Obsidian imports 与显式 open-path 外部副作用记录；vault/skill listings为 runtime-only；vault/path/content
  不记录。
- C6：所有已知 failure 映射 stable code/category/phase/public message；batch/AI job保留 partial/cancel语义。

## Acceptance Criteria

- [x] 本 child 的所有 operation policy entries 都有 owning recorder，成功纯读取无 Operation row。
- [x] CRUD/reorder/assignment/batch 的 success/failure/partial/cancel matrix 有 focused tests且无重复行。
- [x] adversarial repo URL/ref, project/vault path, saved query, collection payload, AI prompt/response 不出现在日志/导出。
- [x] open Obsidian path记录动作结果但不记录路径；folder picker取消不制造长期 operation noise。
- [x] existing domain behavior、sorting/order、job events、project/agent target semantics不变。

## Out of Scope

- Central update/delete/install、settings、targets；由 Central child负责。
- Marketplace/GitHub import/local archive/Skills CLI；由 import child负责。
- 重构 catalog domain model、改变 collection/tag/project UX 或扫描算法。
