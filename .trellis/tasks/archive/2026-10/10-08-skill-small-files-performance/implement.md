# 执行计划：先测量，再锁定优化范围

状态：用户已于 2026-10-08 明确批准「按照规划开始实施」。执行范围为 B 的隔离基线和 C 中测量支持的 D1–D4 优化、回归检查及正式门槛。先保存基线并锁定性能/事件预算，再修改生产行为；用户实际 Skill 数据不进入测试。增量写入、持久化索引、后台清理及跨 Skill batching 仍按原范围单独决策。

## A. 当前分析交付

- [x] 确认工作树、源码起点、Trellis phase 与相关规范。
- [x] 梳理 import/update/delete/leftover/recovery/cleanup，落盘文件操作与进度证据。
- [x] 创建最终 PRD、设计、执行计划与性能候选排序。
- [x] 清理示例 JSONL，配置真实 spec/research context，校验任务与路径锚点。
- [x] 向用户交付任务链接、分析结论、后续测量/风险边界；用户随后批准开始实施。

## B. 隔离基线（已授权）

执行记录：已完成修正版 HEAD 隔离构建、T stage/fingerprint/finalize 7 轮、S/L 服务全生命周期 7 轮、M/H 阶段单轮边界和 T 真实离线 pinned import source 计数。范围、原始日志和未完成矩阵见 [research/baseline-results.md](research/baseline-results.md)；优化范围与数值门槛已锁定于 [research/implementation-budget.md](research/implementation-budget.md)。截图二进制身份、native WebView、真实 acquisition/SSH/WSL、峰值资源及 M/T/H 服务全矩阵未由这些测量证明。

1. 核对截图使用的应用版本、构建配置和当前源码，分别记录已安装 runtime 与测试构建身份。
2. 在任务 research 下保存 fixture 规范、测量入口和原始结果；数据使用独立临时根及临时 DB，所有生成/清理路径提前解析并检查范围。
3. 测量入口调用生产 service，以非侵入 probe/测试专用计数收集 design 第 2 节指标。若需要产品插桩，先明确该文件和 diff 的授权范围。
4. 执行 S/M/T/H/L/X，分别测 backend release 与 native WebView；T 为 13,016 总文件。网络获取使用 mock HTTP/本地固定 archive，与真实网络分开。没有真实仓库样本仍可先完成确定性文件 IO 对照。
5. 完成 import/overwrite、check/no-op/small delta/full update、0/1/多 copy、symlink、delete/leftover、finalize 与 retry。记录至少 7 轮配对样本以及实际 files/dirs/bytes。
6. 将全部原始样本保留为 ms、bytes、counts；记录磁盘/缓存/系统扫描状态，不关闭安全软件、不清 OS cache、不修改系统设置。
7. 按实际耗时占比选择 D1/D2/D3/D4 子集，将数值目标、事件预算、允许回归和文件范围写入计划。当前批准覆盖该测量驱动选择；只有改变既定语义或扩大候选范围时才重新请求批准。技术未知不得以臆测性能数字补齐。

## C. 产品实施顺序（保存基线并锁定预算后）

- [x] C1 backend 实施（AC-I2）：真实写入计数、低频聚合、phase/error/final flush 和 retained-channel settle 回归已实现并通过 focused checks。source 数量目标通过；native UI/store 队列验收未执行，AC-I2 仍部分完成。
- [x] C2 实施（AC-I1/I3）：移动已有 bytes 所有权、operation-owned exact parent 去重、selected subtree SHA；旧摘要 framing 和路径检查保留。没有用低成本 clone 诊断声称端到端提速。
- [x] C3 选定实施（AC-I1/I4）：仅借用路径排序和去除冗余 update 排序；fresh mutation/recovery 校验保留。未增加并发、leftover 计划重写、持久化索引或后台删除；删除时间收益未获证明。
- [x] C4 回归（AC-I3/I4）：467 个相关 release 测试通过，涵盖 stage/swap/DB/copy/finalize、selected recovery、kill/reopen、symlink、失败尾计数及幂等。T corrected full-service 单轮及完整 CI 通过。
- [x] C5 复测与正式检查执行（AC-I1/I5）：交替 7 对和锁定预算已判定，T stage 目标 `NOT_MET`，source 数量目标 `PASS`。完整 `just ci` 15/15 阶段通过、退出 0，两次失败和最终重测记录均保留。删除 stage 与 L 控制 no-op 原因未查明；AC-I1/AC-I2 验收仍有缺口，按用户随后授权提交并归档。

默认用 Trellis `trellis-implement` / `trellis-check` 执行和独立检查；main 负责最终范围、研究/规范、审批及交付。dispatch 第一行必须为 `Active task: .trellis/tasks/10-08-skill-small-files-performance`；child 只写批准文件，不能改其他人的工作、提交或推送。若只有研究阶段，不派发产品实现。

增量更新、持久化索引、后台清理、跨 Skill batching 只在 design 第 4 节触发条件满足且新增范围被批准后执行。

## D. 重点文件与回退点

| 边界 | 当前定位 | 约束 |
|---|---|---|
| import progress | `src-tauri/src/services/github_import/import.rs`、`progress.rs` | 真实计数、snapshot 绑定、错误/终态 |
| content upsert/batch | `src-tauri/src/services/central_updates/core/content_upsert.rs`、`batch.rs` | 唯一 FS/DB apply、uid、selected recovery |
| write/hash/copy | `src-tauri/src/services/central_updates/fs.rs`、`fs/operation.rs`、`fs/operation/helpers.rs` | ownership、blocking、摘要格式、fresh validation |
| delete/finalize | `src-tauri/src/services/central_operation/fs.rs` | marker、指纹、operation paths、Windows symlink |
| leftover/installation | `src-tauri/src/services/central_updates/inventory/leftover_cleanup.rs`、`src-tauri/src/services/installation/native.rs` | shared-root、plugin/ownership、保留 copy |
| renderer | `src/stores/marketplaceStore.githubImportHelpers.ts`、GitHub wizard view-model | generation、bytes percent、进度/终态 |

该表为后续候选定位，未授权这些文件全部修改。每个实际 diff 必须追溯到已批准性能成本；不用调整相邻模块补偿。

回退代码时保留用户数据及未完成 journal；不 reset/stash/clean 工作树。涉及安全删除路径时保留 targeted regression、rollback 说明、失败/retest 记录。

## E. 验证命令

当前任务文档检查：

```powershell
python ./.trellis/scripts/task.py validate .trellis/tasks/10-08-skill-small-files-performance
python ./.trellis/scripts/task.py current
git status --short --branch -uall
git diff --check -- .trellis/tasks/10-08-skill-small-files-performance
```

后续 Rust 实施（从仓库 root）：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --locked services::github_import
cargo test --manifest-path src-tauri/Cargo.toml --locked services::central_updates
cargo test --manifest-path src-tauri/Cargo.toml --locked services::central_operation
cargo test --manifest-path src-tauri/Cargo.toml --locked services::installation
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
```

前端状态/进度变动时执行 `pnpm typecheck`、`pnpm lint`，并以 `pnpm test <相关测试路径>` 执行 import wizard utils、store 及实际修改组件的现有测试。最终 `just ci`。实际基准脚本、frozen binary 身份、source probe 与交替 7 对命令见 [research/benchmark-method.md](research/benchmark-method.md)；执行清单和原始日志见 [research/benchmark-results.md](research/benchmark-results.md)。

## F. 交付证据

analysis/baseline/after 表区分 `STATIC_ANALYSIS`、`PASS`、`NOT_RUN`、`UNVERIFIED`。Windows backend 不替代 native WebView；fake SSH/WSL 不替代真实 target；前端 build 不替代 installer。

实施交付需含各阶段结果、预算判定、失败与恢复、回退方式及未跑项；未经测量不声称性能改善。用户于 2026-10-08 明确授权本地提交全部改动及归档当前任务；没有推送或发布授权。
