# Central、目标、设置与安全操作日志覆盖

状态：**planning**。依赖：`08-26-observability-core-contracts` 完成并冻结 interface。

## Goal

把 Central、target、settings/security、scan/sync、startup/recovery 与日志管理命令接入统一审计 interface，
保留既有日志价值并消除 raw error、漏记和 action/details 漂移。

## Requirements

- S1：逐项核对本 child 所有 command policy；成功的 get/list/preview 不写 Operation Log，失败由 Runtime child
  统一处理。
- S2：targets 的 create/update/password/delete/switch/test、scan、local/remote sync 使用 stable action 和
  reviewed failure；不记录 host、username、credential、distribution/path 或 stdout/stderr。
- S3：Central install/uninstall/delete/reset/update/force/mirror/repository sync/store relocation 与 recovery
  保留 batch/partial/journal semantics，operation ID 与 batch ID 分离。
- S4：settings、scan-directory、AI API key set/clear 与 target credential changes 只记录类别、数量、是否存储，
  不记录 key/value/path/secret。
- S5：Operation/Runtime log clear/export、startup retry/rebuild/exit 与 recovery admin action 本身可审计；
  clear Operation Logs 后留下新的 clear 事实。
- S6：既有 `OperationLogEvent.error(error)`/Display 路径迁移为 reviewed code/category/phase/public message；
  Operation Log 写失败不改变业务结果。

## Acceptance Criteria

- [ ] 本 child policy 中所有 `operation` command 都有 success/failure，适用时有 partial/cancel/started tests。
- [ ] 现有 Central/update/recovery 日志的 counts、batch 和 safe failure items 不退化，不产生重复 operation rows。
- [ ] target/secret/settings/log-admin 对抗种子不出现在 IPC、Operation、Runtime read/export 或 test DOM fixture。
- [ ] clear-all 后仅保留新的 `logs.operation.clear` 事实，count/filter 安全且导出不递归包含自身事件。
- [ ] deprecated runtime commands 仍被明确委托、审计或受控排除，不能因为“旧入口”静默漏记。
- [ ] focused Rust tests、existing Central/target/settings regressions、format/clippy 通过。

## Out of Scope

- Central metadata/tags/collections/projects；由 catalog child 负责。
- GitHub PAT、Marketplace/import/portable state/Skills CLI；由 import child 负责。
- 修改 SSH transport 或任何远端配置；真实远端证据保持 `UNVERIFIED`。
