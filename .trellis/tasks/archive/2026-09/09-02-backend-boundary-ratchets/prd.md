# 后端依赖方向与 repository 所有权

## Goal

移除 services 对 commands 层的反向依赖，并以三个现有高变更领域为最小迁移片，降低生产代码经宽 `crate::db` facade 调用 repository 函数的耦合。

## Confirmed Evidence

- `ARCH-001`（Medium / S）：`src-tauri/src/commands/mod.rs:31` 持有 `APP_USER_AGENT`；`src-tauri/src/services/ai_tagging/mod.rs`、`src-tauri/src/services/ai_provider/claude.rs`、`src-tauri/src/services/ai_provider/stream.rs`、`src-tauri/src/services/github_import/pat.rs`、`src-tauri/src/services/github_import/remote.rs` 共 6 个生产调用点反向依赖 commands。
- `ARCH-003`（Medium / L）：`src-tauri/src/db/mod.rs:7-58` 通过 `pub use repos::*` 暴露宽 facade；审计口径下约 87 个非测试生产文件直接使用 `crate::db`。该数字是规划基线，实施时须按 R3 的固定扫描口径重测，不能直接固化为门禁常量。

## Requirements

- R1：**依赖方向。** 将 `APP_USER_AGENT` 移至 commands/services 均可向下依赖的中性模块；`src-tauri/src/services/**` 的生产代码不得再引用 `crate::commands`。
- R2：**最小 repository 迁移片。** 仅迁移 `src-tauri/src/services/central_updates/**`、`src-tauri/src/services/skills_cli/**`、`src-tauri/src/services/installation/**` 中的 repository 函数调用，使其显式指向 `src-tauri/src/db/repos/*_repo.rs` 的 canonical owner；`DbPool` 和共享 row/domain type 可继续来自 `crate::db`，不在本任务重写类型边界。
- R3：**可复算 ratchet。** 静态契约必须分别记录“services→commands 生产引用数”和上述三个目录内“经 `crate::db::<function>` 调 repository 的调用数”；扫描排除 `#[cfg(test)]` 不足以可靠识别的内联测试时，使用明确的文件/目录 allowlist，并保存匹配明细而非仅保存总数。新代码不得增加基线，触及的三个领域必须使后者下降至 0。
- R4：**语义兼容。** 保持现有事务边界、target mutation lock、`ensure_centralized`、persisted `uid`、target-only skill、Local/SSH/WSL 行为和公开 IPC/CLI 结果不变。
- R5：**简单性边界。** 不建立 repository framework、DI 容器、泛型 repository、第二套 DB facade 或全库迁移；历史领域只由 ratchet 阻止新增，不在本任务清零。

## Acceptance Criteria

- [x] AC1（R1）：对 `src-tauri/src/services/**/*.rs` 的生产扫描中，`crate::commands`/`commands::APP_USER_AGENT` 命中为 0。
- [x] AC2（R1）：6 个既有 HTTP 调用仍发送相同的 `<package>/<version>` user-agent。代码仍为 `CARGO_PKG_NAME/CARGO_PKG_VERSION`；真实 GitHub 线上抓包 **UNVERIFIED**。
- [x] AC3（R2）：三个指定 service 目录的生产 repository 函数调用均通过 `crate::db::repos::<owner>_repo` 解析，且没有新增 SQL；`DbPool`/row type import 不计作失败。
- [x] AC4（R3）：`src/test/contracts/rustBoundaryContract.test.ts` 输出固定扫描根、排除项与逐文件命中，扫描口径可从干净 checkout 复算。
- [x] AC5（R3）：services→commands 基线和三个迁移片的宽函数调用基线均为 0；仓库其余历史命中只能持平或下降。
- [x] AC6（R4）：Central update 的 journal/transaction/lock/uid 与 target-only 定向 tests 通过。
- [x] AC7（R4）：Skills CLI 的 update/install/remove 定向 tests 通过。
- [x] AC8（R4）：Installation 的 Local/Fake SSH/Fake WSL 定向 tests 通过，公开 DTO、schema 与 IPC registry 无变化。真实 SSH/WSL **UNVERIFIED**。
- [x] AC9（R5）：diff 中没有新 framework、DI、schema、config surface 或第二套 DB facade；未迁移引用在 ratchet fixture 中明确列为 debt。
- [x] AC10（R1, R2, R3, R4）：`cargo fmt --all -- --check`、locked all-target Clippy/tests、相关 Vitest contract 和默认并发 `just ci` 通过。

## Out of Scope

- 一次性迁移全库约 87 个引用，或把共享 DB row/domain type 搬出 `crate::db`。
- 替换 SQLite、引入 ORM/依赖注入，修改 schema、公开 command/API 或 Central 业务语义。
