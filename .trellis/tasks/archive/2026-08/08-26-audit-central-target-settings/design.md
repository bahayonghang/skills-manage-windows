# Central/Target/Settings 覆盖设计

## Ownership

本 child 只消费 core `run_operation` interface，不修改 policy/type/lifecycle interface。command ownership：

- `targets.rs`, `scanner.rs`, `local_remote_sync.rs`;
- `settings.rs`（含 AI secret），`central_store_location.rs`;
- `skills.rs`, `linker.rs`, `central_updates.rs`, `skill_update_inventory.rs`;
- `logs.rs` admin/recovery commands, `startup.rs` user actions。

## Recording Rules

- 一个用户 command 最多一个 owning operation row；若内部委托已有 logger，迁移到外层 owner并关闭内层重复；
- batch command 一个 row + safe counts/失败项；journal operation ID作为 safe related id，不替代 audit operation ID；
- test/probe 是用户可观察外部副作用，记 Operation；preview/get/list 是 RuntimeOnly；
- secret/setting details 使用 typed safe struct，不允许 caller keys/values；
- started lifecycle 用于 update/apply/sync/scan/recovery/store relocation/rebuild，短 settings/target metadata 写用 terminal-only。

## Error Mapping

复用 domain `IpcError` mapper。缺少 reviewed code 的 operation variant在拥有 domain 中补固定 code/message；
observability fallback 只保底，不以 `internal.unexpected` 代替已知失败族。所有 raw targets/DB/FS Display
从 Operation builder 移除。

## Log Administration

clear/export 先完成目标动作，再记录 admin operation。clear-all 的新 row 不应被刚执行的 DELETE 清除；export
payload 在记录 export event 前生成，因此不包含自身事件。Runtime file clear/export 仍遵守 filename whitelist。
