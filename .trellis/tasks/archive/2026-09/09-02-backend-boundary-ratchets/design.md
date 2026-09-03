# Design

## Change List / Symbols

1. `src-tauri/src/http_identity.rs`（新建）定义 `pub(crate) const APP_USER_AGENT`；`src-tauri/src/lib.rs` 注册该中性模块；删除 `src-tauri/src/commands/mod.rs::APP_USER_AGENT`，迁移审计确认的 6 个 service 调用点。[R1]
2. `src-tauri/src/db/mod.rs` 将既有 `repos` 仅提升为 `pub(crate)` 可见，保留现有兼容 re-export；不改 repository 函数签名。[R2][R5]
3. `src-tauri/src/services/central_updates/**` 显式使用 `fs_db_operations_repo`、`repositories_repo`、`installations_repo`、`skills_repo`、`update_inventory_repo`/`update_states_repo` 等既有 owner；重点符号包括 `update_skills_batch`、`persist_updated_skill_in_transaction` 和 recovery/inventory persistence 路径。[R2][R4]
4. `src-tauri/src/services/skills_cli/**` 显式使用 `agents_repo`、`skills_cli_updates_repo`、`skills_repo` 等既有 owner；重点符号包括 inventory、link/remove 和 `updates::{detect,apply,recover}`。[R2][R4]
5. `src-tauri/src/services/installation/**` 显式使用 `agents_repo`、`installations_repo`、`observations_repo`、`skills_repo` 等既有 owner；重点符号包括 install/centralize/native/remote/project/skip/transport。[R2][R4]
6. `src/test/contracts/rustBoundaryContract.test.ts` 固化可复算扫描、明细输出和 ratchet；不新增常驻检查服务或配置文件。[R3]

## Contract

依赖方向为 `commands → services → db::repos`。`http_identity` 是纯编译期 identity，不依赖 Tauri、commands、DB 或 service。repository SQL 的 canonical owner 仍是现有 `src-tauri/src/db/repos/*_repo.rs`；本任务只改变 import 路径，不复制函数、不移动 SQL。静态契约只把函数/模块级宽入口视为 debt，允许 `crate::db::{DbPool, Skill, ...}` 等共享类型，避免把类型重构偷渡进本任务。[R1][R2][R3][R5]

## Compatibility

- `APP_USER_AGENT` 的值仍由同一 crate 的 `CARGO_PKG_NAME/CARGO_PKG_VERSION` 拼接，网络请求字节语义不变。
- `db/mod.rs` 的 compatibility re-export 保留给未迁移领域；无 schema、migration、DTO、IPC 或 CLI 变化。
- 调用改为窄路径后仍调用同一函数，因此事务、锁、uid、target-only 与 error mapping 必须保持原样。[R4]

## Verification Boundary

自动化证明 import 方向、三个迁移片的调用归属和现有 Rust 行为回归；它不能证明真实 GitHub/SSH/WSL provider、Windows WebView 或生产数据库升级。未执行的真实远程与安装行为标记 `UNVERIFIED`，不以 fixture 代替。[R3][R4]

## Rollback

- RP1：`http_identity` 移动是独立单元，可整体恢复原常量与 6 个调用点。
- RP2/RP3/RP4：Central updates、Skills CLI、installation 按目录分批；任一批失败只回退该批 import 与 ratchet 基线，不影响已验证批次。
- RP5：静态契约最后启用；若扫描口径误报，回退契约变更并修正扫描，不恢复已通过的语义等价窄 import。

## Considered but Not Chosen

- 不新建 domain repository wrapper：现有 repo module 已是 canonical owner，再包一层只会复制边界。
- 不移除 `db/mod.rs` re-export：会把约 87 个历史引用拖入同一不可回滚 diff。
- 不引入 DI/ORM 或将 user-agent 做成配置：本问题不需要运行时选择。

