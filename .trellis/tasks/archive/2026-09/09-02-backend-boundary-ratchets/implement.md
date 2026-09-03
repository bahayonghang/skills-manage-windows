# Implementation Plan

> 当前仅规划；不得在本阶段修改产品代码或启动任务。

1. [ ] **基线与契约先行** [R3]：在 `src/test/contracts/rustBoundaryContract.test.ts` 先写当前可复算扫描与失败 fixture，记录扫描根/排除项/明细；确认 6 个反向引用与三个领域的宽函数调用。定向验证：`pnpm vitest run src/test/contracts/rustBoundaryContract.test.ts`。Rollback: RP5。
2. [ ] **移动 HTTP identity** [R1]：修改 `src-tauri/src/lib.rs`、`src-tauri/src/commands/mod.rs`，新建 `src-tauri/src/http_identity.rs`，迁移 `src-tauri/src/services/ai_tagging/mod.rs`、`src-tauri/src/services/ai_provider/claude.rs`、`src-tauri/src/services/ai_provider/stream.rs`、`src-tauri/src/services/github_import/pat.rs`、`src-tauri/src/services/github_import/remote.rs` 的 6 个调用点。定向验证：`cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked`，再运行 contract。Rollback: RP1。
3. [ ] **开放既有窄 repo 路径** [R2][R5]：仅把 `src-tauri/src/db/mod.rs::repos` 改为 crate 内可见；不改 `src-tauri/src/db/repos/mod.rs` owner 列表和函数签名。定向验证：`cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked`。Rollback: 与首个领域批次一起回退。
4. [ ] **Central updates 批次** [R2][R4]：迁移 `src-tauri/src/services/central_updates/**` 的 repository 函数 import，保持 `update_skills_batch`、journal transition 与 transaction 代码不动。定向验证：`cargo test --manifest-path src-tauri/Cargo.toml --locked services::central_updates` 与 contract。Rollback: RP2。
5. [ ] **Skills CLI 批次** [R2][R4]：迁移 `src-tauri/src/services/skills_cli/**` 的 repository 函数 import，不改变 runner、lock、target guard。定向验证：`cargo test --manifest-path src-tauri/Cargo.toml --locked services::skills_cli` 与 contract。Rollback: RP3。
6. [ ] **Installation 批次** [R2][R4]：迁移 `src-tauri/src/services/installation/**` 的 repository 函数 import，不改变 Local/SSH/WSL transport 或 `ensure_centralized`。定向验证：`cargo test --manifest-path src-tauri/Cargo.toml --locked services::installation` 与 contract。Rollback: RP4。
7. [ ] **总验证** [R1-R5]：运行 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`、`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`、`cargo test --manifest-path src-tauri/Cargo.toml --all-targets --locked`、`just ci`、`git diff --check` 和任务验证；确认无 schema/docs 生成漂移。
8. [ ] **证据边界与独立审查** [R4]：真实 SSH/WSL、GitHub provider 和 Windows 安装未执行时分别标记 `UNVERIFIED`；独立 review 核对未新增 wrapper/SQL/兼容层，并按 RP1-RP5 保持每批可单独回退。
