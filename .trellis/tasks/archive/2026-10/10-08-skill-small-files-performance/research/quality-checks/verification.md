# 正式质量检查

状态：`PASS`（正式质量门槛）。最终 canonical `just ci` 退出 0，common 12 阶段和 rust-platform 3 阶段全部完成。固定 pnpm 10.34.5；依赖、版本、门槛和 CI 配置均未修改。性能 `NOT_MET` 和运行时缺口保持独立状态。

## 首次失败与环境修复

`just ci` 首次退出 1。Clippy 的 Cargo 退出 101，无法创建 `src-tauri/target`：`Access is denied (os error 5)`。version 与 generated-doc checks 通过；typecheck 被并行 Rust 阶段失败取消，其他未执行阶段不能记为 PASS。

原始输出与 receipt：`just-ci-20261008-115051.log`、`just-ci-20261008-115051.json`。

`Get-Item` 确认 `src-tauri/target` 是现有 SymbolicLink，目标目录当前不存在。诊断见 `ci-target-environment-diagnosis.json`。重测仅为 CI 进程设置 `CARGO_TARGET_DIR=.git/skill-small-files-tools/quality-target`，保留原链接、ACL 和全局环境。构建路径已解析并确认位于仓库内。

重测仍执行原始 `just ci`，没有减少阶段或检查对象。构建路径替换只改变本地输出位置。冻结 release 性能 binary 不变。独立检查代理未修改生产源码。

## Clippy 首次代码失败与定点修复

环境修复后的 canonical `just ci` 仍退出 1。Clippy 发现新增 `github_import/progress/tests.rs` 两处 `RefCell` borrow 跨越后续 await；`await_holding_refcell_ref` 在 `-D warnings` 下失败。原始输出和 receipt 为 `just-ci-retest-20261008-115433.log`、同名 JSON。

独立检查代理只在 source probe 和 pinned write failure test 的观察断言/输出外增加词法作用域，让 borrow 在异步 pool/DB 操作前释放。断言、source publication 和生产行为均未改变，没有增加 lint allowance。`cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` 退出 0。

`clippy-refcell-fix.json` 保存修改后的测试源码 hash：`5951F0EB5C66CFCF58B16204488C1EB109F59AD6FF86EF455EF22C3A0CBEB252`。冻结 after 使用原测试 hash `F39A30DD187073DBDBF8FEFCE04A8BDE8E6105F2E9A530481FA3A63CA20DB27E`；历史 release binary 与最终 CI 测试源码身份明确分开。`performance_benchmark.rs` 和生产源码没有因该修复改变。

修复后重新执行 canonical `just ci`，15/15 阶段通过，进程退出 0。成功日志为 `just-ci-fixed-retest-20261008-115814.log`，SHA-256 `B3D91D3408BE0FE1BF5C5E3DAD0C890F80F4A63E27DF13B0AD8C0E94E7057AF4`，228,420 bytes；同名 JSON 保存退出码和 source/environment 身份。

| 正式执行范围 | 实际结果 |
| --- | --- |
| common: version/generated docs/typecheck/lint/capability/size/entrypoints/fmt/IPC | 全部通过 |
| Vitest | 178 files；2,078 passed / 1 skipped |
| frontend production build 与 documentation build | 通过 |
| Clippy all-targets locked，`-D warnings` | 通过 |
| Cargo locked tests | 1,574 passed / 0 failed / 9 ignored；7 result groups |
| Trellis Python | 54 total；50 passed / 4 POSIX-only skips |

`formal-ci-summary.json` 保存三次原始日志的 hash/bytes、全部阶段和忽略/跳过行。Vitest 默认 reporter 只有单项跳过总数，未记录名称；没有猜测该名称。成功后未重复全 gate。

## 冻结身份清单缺口

baseline 代理确认：冻结 after 时保存的 `after-source.diff` 只含 tracked diff，`after-build-identity.json` 的六个文件 hash 没有覆盖新增 production `central_updates/fs/progress.rs` 和移出的 cfg(test) `central_operation/fs/tests.rs`。没有额外构建时 receipt。冻结 binary 和 benchmark harness 的身份精确；完整 production 源码冻结清单为 `UNVERIFIED`。

`current-review-source-identity.json` 是正式检查期间补充的 21 个 changed/new Rust 文件 hash，不能作为历史构建时清单。原 after receipt 保留不变；性能比较和 `NOT_MET` 结论保留。该证据缺口不能通过事后改写历史 receipt 修复。

## 证据边界

Windows backend、frontend/jsdom 与 Python 的实际计数已从完整日志提取。native WebView、Windows bundle/安装器、真实 SSH/WSL、真实网络 acquisition、截图应用身份和峰值资源均为 `NOT_RUN` 或 `UNVERIFIED`。

T 13,016-file 全生命周期单轮 correctness-only 使用冻结 after binary，与 CI 可同时运行。该轮耗时受并行工作负载影响，不用于性能预算。T stage 的 7 对交替测量改善 6.56%，固定 10% 门槛为 `NOT_MET`；最初 7 轮和 controls 保留独立证据。删除 rename 回退原因未查明。
