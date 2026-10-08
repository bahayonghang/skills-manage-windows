# 大量小文件 Skill 导入、更新与删除清理性能优化

## Goal

减少大量小文件 Skill 在导入、更新、删除及清理中的等待时间，保持内容完整性、操作恢复和删除范围约束。用户已于 2026-10-08 批准按照规划实施。D1–D4 的选定改动已实施；完整 `just ci` 15 个阶段通过，退出 0。真实用户数据保持隔离。

进度发布数量目标已通过；T staging 的交替 7 对测量下降 6.56%，未达到原定 10% 目标。删除 stage 和 L 控制组 no-op 的耗时回退原因未查明。用户于 2026-10-08 明确要求「提交所有改动和归档任务」，按该授权办理本地提交与归档；性能验收未全部通过。完整结果见 [research/benchmark-results.md](research/benchmark-results.md)。

## Background

- 用户报告：GitHub 仓库 `hugohe3/ppt-master` 的 Skill 含大量小文件，导入速度很慢。
- 截图记录：1 个选中 Skill，`5054 / 13016 files`、`51%`，阶段为 `Writing files`。当前组件优先按字节计算百分比（`src/components/marketplace/githubImportWizardViewModel.ts:262`），因此百分比可以与文件数比例不同。截图没有持续时间、文件字节或磁盘参数。
- 用户要求同步考虑后续更新、删除清理的性能。
- 已有删除正确性任务 `10-08-central-skill-delete-blocked` 独立保留。
- 分析起点 HEAD `063c8d3236197a06ff17083c28a9738487c07510` 在实际写入前逐文件发送 `Writing` 事件。实施后 Local 写入成功才累计计数，由 async importer 聚合发布；前端仍使用现有 store。截图二进制与源码身份尚未核对。
- 分析起点的调用链、成本模型和源码锚点见 [research/performance-analysis.md](research/performance-analysis.md)。运行结果单独保存，不将历史源码锚点用作当前代码定位。截图场景的实际瓶颈原因未查明。

## Requirements

### R1. 全生命周期

覆盖 GitHub 导入预览与确认、更新检查与应用、Central 删除、Platform leftover 清理、备份/暂存目录收尾、失败回滚及恢复。比较首次/覆盖导入、无变化/少量变化更新，识别重复遍历、读取、哈希、复制及进度事件。

### R2. 测量与证据

Windows Local 为主要性能目标。使用独立临时根目录、临时数据库和固定内容的合成 Skill，覆盖 13,016 个文件及总字节相近的少量大文件案例。记录源码/二进制版本、构建配置、文件/目录数、字节数、磁盘/文件系统、缓存条件和系统扫描状态。

分别计量获取/解包、快照校验、锁等待、计划生成、staging 写入、swap、DB、copy 刷新、指纹校验、物理清理及结果返回。记录文件操作数、进度事件数、吞吐、工作集/线程/句柄峰值和 native UI 响应。事实、假设和 `NOT_RUN` 结果分开记录。

### R3. 优化决策

按收益证据、改动范围和风险排序。优先研究真实进度与事件合并、减少整批字节重复复制、目录创建去重及操作内清单复用。文件并发、增量更新、持久化索引、后台清理作为实验/后续候选，测量后再决定实现范围。

已有无变化更新跳过写入、快照缓存、远程批处理及删除 rename 暂存作为基线，不重复添加已有机制。

### R4. 一致性与删除边界

- 保留预览固定 commit/内容、持久化 `uid`、repository/source/provenance、GUI/CLI 服务边界和 target 隔离。
- 保留 target mutation lock、可恢复 FS/DB operation journal、失败证据及 `copies_pending`。恢复必须重新检查实际状态；未完成日志/artifacts 无 TTL 清理。
- 删除限定在已验证的 operation-owned/Platform 路径。目录 symlink 只移除链接；保留未选中 copy、只读插件及非管理文件。
- 保留资源预算、路径检查、错误分类和取消后的 settle/recovery。不得通过丢文件、跳过必需校验或关闭安全软件达到性能目标。

### R5. 兼容性与真实进度

Local 优先；SSH/WSL、copy/symlink 和批量语义保持兼容。递归同步 IO 使用现有 blocking-FS wrapper，不按文件创建 blocking task，closure 不捕获 `AppHandle`。

进度反映已完成工作，准备阶段不增加写入完成计数。事件可低频合并，阶段转换、错误及终态必须送达，最后计数精确。canonical/DB 成功和物理清理完成沿用现有终态契约。

## Acceptance Criteria

### 当前分析交付

- [x] AC-A1（R1）：生命周期调用链、源码锚点和成本模型落盘，导入、更新、删除与清理均有独立分析。
- [x] AC-A2（R3）：候选优化按证据与风险排序，注明已有机制及会改变行为的后续候选。
- [x] AC-A3（R2、R4、R5）：基线指标、合成 fixture、正确性矩阵及运行时/跨平台证据边界落盘。
- [x] AC-A4（R1–R5）：分析阶段的 `prd.md`、`design.md`、`implement.md` 与真实 context 通过任务校验；当时改动仅在新任务目录内，状态为 `planning`。用户随后批准实施。

### 实施验收

- [ ] AC-I1（R2、R3）：保存当前基线后锁定优化范围、事件预算及性能预算；数字未确定前不标记实施就绪。同 fixture/构建/环境复测，保存原始样本、中位数、范围与阶段差值，不预先声称提升倍数。
- [ ] AC-I2（R5）：计数单调且无提前完成，事件符合预定预算，native UI 无事件队列持续积压，最后计数与磁盘/DB 结果一致。
- [x] AC-I3（R1、R4）：导入内容/路径与快照一致；覆盖/更新保留 uid；无变化更新不写 canonical 文件；变化更新保留本地冲突规则和 copy/symlink 结果。Local 与 fake transport 回归通过；真实 SSH/WSL 证据另列。
- [x] AC-I4（R1、R4）：删除、leftover、rollback、retry/finalize 幂等；symlink 目标和未选中 copy 保留；失败留下可恢复状态与稳定错误码。Local 与 fake transport 回归通过；真实 SSH/WSL 证据另列。
- [x] AC-I5（R2、R5）：受影响测试和完整 `just ci` 通过；分别报告 Windows backend、native WebView、bundle/安装器、SSH/WSL fake 与实测，未跑项标 `NOT_RUN`/`UNVERIFIED`。

当前验收证据：

| 条件 | 当前判断 | 证据与缺口 |
| --- | --- | --- |
| AC-I1 | `PARTIAL / NOT_MET` | 基线、原始样本和锁定预算已保存；T stage 19,810.1381→18,510.3364 ms，原 10% 目标未达到。删除 stage +12.77%，L 控制 no-op +30.72%，原因未查明；M/T/H 服务完整 7 轮矩阵未执行。 |
| AC-I2 | `PARTIAL` | 离线真实 pinned import source calls 13,018→98，下降 99.2472%；成功/错误尾计数有回归覆盖。AppNone 不验证 IPC/native WebView 队列。 |
| AC-I3 | `PASS: LOCAL / FAKE TRANSPORT` | 467 个相关 release 测试通过；修正版 T 完整服务单轮通过，验证内容、uid、no-op mtime、copy 和终态 journal。完整 CI 通过。 |
| AC-I4 | `PASS: LOCAL / FAKE TRANSPORT` | 相关 journal/rollback/selected recovery/kill-reopen、symlink 和重复清理回归通过；完整 CI 通过。真实 SSH/WSL 未执行。 |
| AC-I5 | `PASS: GATE AND DISCLOSURE` | `just ci` 15/15 阶段通过、退出 0：Rust 1,574 PASS/9 ignored，Vitest 178 files/2,078 PASS/1 skipped，Python 50 PASS/4 skipped。两次失败、修复、最终重测原始记录保存于 `research/quality-checks/`。native、安装器、真实 acquisition/SSH/WSL 保留未验证状态。 |

正式检查汇总见 [research/quality-checks/verification.md](research/quality-checks/verification.md)，独立评审见 [research/independent-check.md](research/independent-check.md)。源文件格式、大小、任务 context 与最终 diff 检查通过。用户随后授权本地提交和归档；没有推送或修改用户 Skill 数据。

归档关闭记录：AC-I1、AC-I2 的未完成条件、性能 `NOT_MET`、未解耗时回退、构建时源码清单 `UNVERIFIED` 和运行时未跑项继续保留。Trellis 归档的 `completed` 表示该任务按用户要求关闭，不表示上述验收条件通过。未创建或启动新的实施任务。任务目录内 `.gitattributes` 保留 `research/` 原始字节，避免换行转换使已记录 SHA 失配。

## In Scope

源码分析、规范核对、隔离测量及 D1–D4 中由测量支持的产品优化、相关回归和正式门槛。三类生命周期共用内容及事务边界，先在一个任务内统一实施；仅当后续方案可独立实施/验收时再拆子任务。

## Out of Scope

- 用户实际 Skill 更新/删除、真实仓库联网下载及系统安全扫描设置修改。
- 提交、推送、发布、修改依赖或版本。
- 全应用遥测平台、通用文件系统框架、持久化缓存和异步删除终态改造不进入默认方案。

## Related Work

- `.trellis/tasks/10-08-central-skill-delete-blocked/`：Windows 目录 symlink 删除正确性，独立保留。
- `.trellis/tasks/archive/2026-07/07-18-github-import-manifest-fast-path/`：获取路径的历史背景，当前快照协议以现行源码/spec 为准。

## Technical Unknowns

隔离 fixture 的文件数、目录数、字节数、阶段计时、source 发布次数和 selected D1–D4 范围已有记录。冻结 before/after binary 与共同 harness 的 SHA 已核验；after 构建时源码清单未覆盖新增 `central_updates/fs/progress.rs` 与拆分的 `central_operation/fs/tests.rs`。两文件的事后 hash 不能替代构建时记录，完整 after 源码冻结身份保留 `UNVERIFIED`。最终 CI 的测试源码另记，不改写原基准身份。

截图应用/源码身份、真实文件尺寸及 copy 数量、磁盘/扫描的因果影响、UI 队列和峰值资源仍未验证。T stage 未达到原性能目标，删除 stage 与 L 控制 no-op 的回退原因未查明；不能声明全部性能预算通过。
