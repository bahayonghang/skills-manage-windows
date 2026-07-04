# 为 Update Center 落 service 域：业务从 commands 壳层归位

## Goal

把更新中心 + 中央仓库位置迁移这一完整业务域（≈4000+ 行，不含测试）从 `commands/` 迁入新的 services 域，恢复「commands = 纯壳」的三层契约，让该域逻辑可以穿 service interface 测试并使用域错误枚举。

## 背景与证据（2026-07-04 架构评审，已人工复核）

`services/` 下 12 个域目录中**没有** central_updates / central_store_location 对应域，而 commands 层承载了：

- `commands/central_updates.rs` — 1212 行，37 个 fn / 仅 5 个 `#[tauri::command]`，14 处编排循环。
- `commands/central_updates/repository_sync.rs` — 683 行 + `repository_sync/` 子目录（tests.rs 21.2K）。
- `commands/central_updates_fs.rs` — 783 行，17 处 `std::fs` + 7 处 `spawn_blocking`；其注释自认是「typed service domains 的 commands/IPC 边界对应物」——service 形状的逻辑穿着壳层外衣。
- `commands/central_store_location.rs` — 691 行，10 处 `sqlx::query` + 14 处 `std::fs`，一个完整迁移域无任何 backing service。
- `commands/skill_update_inventory.rs` — 698 行 + `skill_update_inventory/` 9 个子模块（apply_steps/force/scan/persistence/relocation/scope/view/types/repositories，tests.rs 95.6K）。

对照组：`settings.rs`、`bootstrap.rs`、`linker.rs` 等均为干净纯壳（0 SQL / 0 fs）。

## Requirements

1. 新建 services 域承接上述业务逻辑；域的切分（单域还是 `central_updates` + `central_store_location` 两域、`skill_update_inventory` 归属）在 design 阶段裁决。
2. 域错误枚举遵循 `.trellis/spec/backend/domain-error-enums.md`：services 返回 thiserror 枚举，commands 是唯一 `.map_err(|e| e.to_string())` 层，禁止 `error.contains()` 字符串嗅探。
3. 重 IO 统一走 `fs_util.rs` 的 `run_blocking_fs_with`（`.trellis/spec/backend/spawn-blocking-io.md`）；`central_updates_fs.rs` 的自建 façade 并入该 seam 或退役。
4. 事件推送 / 进度发射留在 async 侧；`AppHandle` 不得按值进 blocking 闭包（Windows 测试二进制会崩）。
5. commands 层收回纯壳职责：参数翻译、操作日志、错误字符串化。
6. 既有测试（含 skill_update_inventory 95.6K tests.rs、central_updates 命令测试 15+7 条）随逻辑迁移，不允许净减少覆盖。

## Constraints

- IPC 命令名与参数/返回结构不变：前端 `invoke()` 契约零破坏，本任务不改前端。
- `#[error(...)]` 文案逐字保留（前端 toast 直接展示 Display 输出）。
- 更新中心的 GitHub 请求行为（鉴权、重试回退）不变。
- 工程量大：design 必须给出分阶段迁移方案与每阶段回滚点，禁止一把梭。

## Acceptance Criteria

- [ ] `services/` 下出现承接域，且 `commands/central_updates*.rs`、`commands/central_store_location.rs`、`commands/skill_update_inventory*` 中 grep 不到 `sqlx::query` 与 `std::fs` 直调。
- [ ] 该域全部 services 函数返回域错误枚举，无 `Result<T, String>`。
- [ ] `cd src-tauri && cargo test` 全过；`cargo clippy -- -D warnings` 通过。
- [ ] 前端零改动、`pnpm test` 不受影响（IPC 契约未变的旁证）。

## Notes

- 复杂度：complex（本专项工程量最大）→ 必须有 `design.md` + `implement.md`，design 含分阶段计划。
- **硬依赖**：本任务必须先于 `07-04-transport-seam` 完成（两者触碰同一批命令文件，先归位再收拢，避免返工）。
- 建议在 `07-04-rust-test-support` 之后做，迁移验证成本更低。
