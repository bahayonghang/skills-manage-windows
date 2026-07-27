# 实施计划：Release 构建验证与 draft 原子公开

## 1. 激活与规范加载

- [x] `python ./.trellis/scripts/task.py start 07-24-release-pipeline-gate`，确认唯一 current task 为本子任务。
- [x] 加载 `trellis-before-dev`，阅读 quality CI gate、Rust entrypoint、release docs 与测试规范。
- [x] 记录并保护现有 Trellis runtime/tooling、审计报告和其他子任务规划改动。

## 2. Frozen Release Context

- [x] 新增可测试的 release-context 脚本：校验显式 `v<semver>` tag、tag peel SHA、`origin/main` ancestry、package/tauri/Cargo/Cargo.lock 版本一致性。
- [x] 为 tag push 和 `workflow_dispatch(tag)` 产出统一 `tag/version/sha/release_name` outputs；删除 release asset 逻辑对 `GITHUB_REF_NAME` 的依赖。
- [x] 增加 wrong tag、JSON/Cargo mismatch、unlocked Cargo metadata 的定向测试。

## 3. Signature、Metadata 与 Artifact Validators

- [x] 在现有 Rust package 中新增 release-only updater signature verifier，直接锁定 runtime 已使用的 `base64 0.22.1` 与 `minisign-verify 0.2.5`，按 Tauri base64 包装格式验证 NSIS 字节，并保持 desktop/skillport-cli/default-run 契约。
- [x] 扩展 `release-preflight.mjs` CLI，使结构验证通过后调用密码学 verifier；保留纯函数入口供 Vitest fixture 使用。
- [x] 增加严格 artifact inventory/checksum helper：必需 Windows/macOS/Linux x64 集合、optional arm64 all-or-none、拒绝重复/非预期文件、确定性 `SHA256SUMS` 生成与回验。
- [x] 测试有效签名，以及 installer/signature/public-key tamper；测试 malformed `latest.json`、wrong version/URL/platform/signature 与缺失/重复/partial optional artifact。

## 4. Reusable CI Contract

- [x] 为 `.github/workflows/ci.yml` 增加 `workflow_call(checkout_ref)`，让 `just-ci` checkout frozen ref，同时保持 PR/push/手动 dispatch 行为。
- [x] 删除 `release.published` trigger；手动 smoke package 继续只在 CI 的 `workflow_dispatch` 运行。
- [x] 更新 `ciWorkflowContract.test.ts`，保持 `just-ci` check name 不变，并覆盖 reusable input/ref 与 package guards。

## 5. Release Workflow State Machine

- [x] 将 `.github/workflows/release-desktop.yml` trigger 改为 `v*` tag push 与必填 manual tag。
- [x] 增加 release-context 和 reusable CI jobs；Windows/macOS/Linux build 全部 checkout frozen SHA 并依赖二者。
- [x] 保留 Windows signed NSIS/MSI/ZIP、macOS universal CLI、Linux x64/optional arm64 的现有 build contract；不顺手修改 Actions pinning。
- [x] 聚合 workflow artifacts，执行签名、metadata、完整清单和 checksum 验证后才创建/复用 draft。
- [x] 重置 draft assets、上传、API inventory 校验、fresh download checksum 回验；把 `draft=false` 保持为最后且唯一公开动作。
- [x] 增加 release workflow contract tests，对 required predecessor failure、post-upload failure、public same-tag rejection 和唯一 publish transition 做静态 DAG/状态机模拟。

## 6. 文档与 Spec

- [x] 同步 `docs/reference/release-process.md` 与 `docs/zh/reference/release-process.md` 的 tag/manual、draft failure recovery 和最终验证步骤。
- [x] 必要时同步 README/README_CN 的简短入口描述。
- [x] 在 Phase 3 更新 `.trellis/spec/quality/ci-quality-gate.md`：`workflow_call` 硬依赖、手动 smoke contract、release workflow ownership。

## 7. 验证梯度

- [x] 运行 release context/preflight/artifact/workflow contract 定向 Vitest。
- [x] `cargo test --manifest-path src-tauri/Cargo.toml --bin release-signature-verifier --locked`
- [x] `pnpm typecheck`
- [x] `pnpm lint`
- [x] `cd src-tauri; cargo fmt --all -- --check`
- [x] `cd src-tauri; cargo clippy --all-targets --locked -- -D warnings`
- [x] `cd src-tauri; cargo test --locked`
- [x] `just ci`
- [x] Windows 上运行 `pnpm tauri build`，确认 NSIS/MSI bundle 与 `skillport-cli.exe` 实际生成。

## 8. Diff、回滚与收尾

- [x] 检查 diff 只包含本子任务 workflow、scripts/tests、必要 Rust verifier、双语文档和 quality spec；不纳入其他未提交改动。
- [x] 若 draft upload/verify 失败，保留 draft 供排查并重跑；禁止手工提前 publish。若 workflow 必须回滚，回到 tag/manual + draft 状态机的上一可用提交，不恢复 `release.published` 先公开顺序。
- [x] 运行 `trellis-check`，修复 context checkout、GitHub `target_commitish` 语义、tag 竞态与最小权限问题。
- [ ] 提交工作改动后归档本子任务，并在父任务中登记完成与 `07-24-ci-supply-chain` 的后续合并顺序。
- [x] 不 push、不创建 tag、不执行真实 GitHub 发布；如需远端演练，先列出 tag、SHA、draft side effects 并取得明确授权。

## 9. 验证证据（2026-07-27）

- 独立 `trellis-check`：只读 GitHub API 证明现有 `v0.10.14` release 的
  `target_commitish` 为 `main`，而 tag peel SHA 为
  `61c544a49c57c0baca43f3c84e1f6d3b1f772225`；实现改为验证远端 tag
  本身，并在唯一 publish 调用中再次 re-peel。
- 聚焦测试：6 个 release/CI Vitest 文件共 27 项通过；release-only Rust
  verifier 2/2 通过；`pnpm typecheck` 与 `pnpm lint` 通过。
- `just ci`：前端 134 files，1470 passed / 1 skipped；Rust library 967
  passed / 6 ignored，verifier 2/2，integration 3/3、4/4、5/5；entrypoint、
  fmt、locked all-target Clippy、capability、size 与 production build 全通过。
- Windows bundle：`pnpm tauri build` 通过；随后
  `pnpm tauri build --bundles nsis,msi` 通过。新产物为 NSIS
  12,049,679 bytes、MSI 16,887,808 bytes、`skillport.exe` 42,738,176
  bytes、`skillport-cli.exe` 14,219,264 bytes，时间为 14:11-14:12。
- `python ./.trellis/scripts/task.py validate 07-24-release-pipeline-gate`
  通过，implement/check context 各 4 条。`actionlint` 本机未安装；workflow
  使用 YAML 1.2 contract tests 和完整本地 CI 验证，未把缺失工具记为通过。
