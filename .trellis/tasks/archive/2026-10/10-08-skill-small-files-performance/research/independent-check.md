# 独立检查记录

状态：`QUALITY_PASS / PERFORMANCE_NOT_MET / RUNTIME_PARTIAL`。最终 canonical `just ci` 退出 0，15 个阶段全部通过；已自修新增测试中两处 `RefCell` borrow 跨 await。T stage 原 10% 目标未达到，删除 stage 与 L control no-op 回退原因未查明。任务保持 `in_progress`；native WebView、安装器、真实 SSH/WSL 和完整大规模性能矩阵未由质量检查证明。

最终报告见下方「正式检查结果」与当前验收表。阶段 1 和初版 diff 的状态仅为历史记录；当时为保护测量窗口未运行 Cargo 或大型 IO。原失败、中断及修复后的 receipts 均保留。

## 阶段 1：已核对

- 读取完整保存的 Trellis hook、`check.jsonl` 的规范、PRD、design、implement 与 `implementation-budget.md`。工作树为 `dev`，领先 `origin/dev` 3 个提交；并行代理改动保留。
- ignored service benchmark 调用生产 content-upsert、check/apply、delete、leftover 与 recovery 边界。`stage` 模式调用生产 operation hooks，但没有 DB journal；该模式的结果只能作为 component 数据。
- fixture 总文件数包含 `SKILL.md`，目录条目和 bytes 均受现有预算约束。32-file fixture 的总 bytes 与 13,016-file fixture 相同。所有已命名文件 mutation 路径来自新 temporary root 或 temporary file-backed DB；全部 DB agent root 在 mutation 前重定向。
- test lock override 为 scoped thread-local，guard 使用 `PhantomData<Rc<()>>` 限制跨线程移动；Drop 恢复 previous 值。入口为 current-thread Tokio test。锁路径在 blocking await 前替换；production resolver 未改变。`fs_util` span 只标明 coherent blocking wrapper await 的 wall time，不能表示 worker CPU 或 syscall 分项。
- 原 Local path resolver 在 override 前仍会对优先/legacy database 路径执行 bounded `exists` metadata lookup。该 resolver 不打开用户 DB、不读取内容、不遍历 Skill、不写真实 lock；实际 lock IO 使用 isolated path。隔离结果不能表述为完全没有真实 filesystem metadata lookup。
- `Get-FileHash .git/skill-small-files-tools/benchmark-before.exe -Algorithm SHA256` 得到 `F94616DE25677272FFEA8928C37021EA6B0C961EB656119A72FBA593C08CCFA8`，与原 `baseline-build-identity.json` 一致。该 binary 在下述 fixture 修复前冻结，仅保留历史诊断价值。
- D2 ownership transfer 后的 commit/state/provenance 路径只使用 plan metadata、remote hash/digest 与 manifest；未发现 commit/recovery 必须重读 `plan.remote.files` 的 production caller。single-stage Remote API、ordinary/force/mirror 和 archive builders 仍需在最终 diff 中复核。
- D4 借用 `Path` 比较与旧 `PathBuf` key 比较使用相同 `Ord`；全树收集、错误顺序、framing、fresh 文件读取与 marker 验证保留。删除指纹原本已使用 64 KiB 流式读取，不能声称移除了整文件内存分配。

## 已发现并交由 owning agent 修复

1. `performance_benchmark.rs::upsert_input` 将 repository snapshot digest 用作 per-skill `content_digest`。两者具有不同 domain，单 candidate 仓库也不等价。baseline 代理已改为 `candidate_content_digest_from_snapshot(snapshot, SOURCE_PATH)`；旧 smoke 保留为历史诊断，正式 before 必须从 HEAD archive 加修正版 test seams 构建，after 使用同一修正版 harness。最终正确性通过不能沿用旧 provenance fixture。
2. 初始 `many_files_publish_bounded_monotone_aggregates_and_an_exact_tail` 只按 256-file 门槛限制 publication，但 production 另有 100 ms 门槛。已通知实施代理按锁定预算计入 elapsed interval，避免慢机或调度暂停产生合法 publication 后误判。
3. 初始 async settlement 测试依赖 250 ms sleep，runtime 暂停后 service 与 interval 可能同时 ready。实施代理已使用 oneshot：observer 看见 full count 后，service 才可 settle。生产 observer 无需修改。
4. 新规范初稿将 closed channel 行写为停止 observation；实现保留 100 ms periodic sampling 直到 service settle。main 已将规范改为禁止 busy polling、保留最新计数并继续等待 service。

## 初版生产 diff 静态复核（历史）

- ordinary update、force update、force mirror 将 remote content 移入 plan。batch 将 files 依次移入 write/stage；Local worker 和 Remote archive builder 接收拥有的数据。Remote chunk 使用 owning iterator，并在 archive 返回时交还 chunk；剩余 `chunk.to_vec()` 属于未修改的 cfg(test) legacy writer。commit、state、provenance 与 recovery 使用 metadata/hash/manifest，未发现必须重读已移走 files 的 production caller。
- source validation 循环保留路径检查，计数移到成功 write。Remote skill count 移到成功 service 返回后。新 FakeRunner 测试在首个 Remote service IO 失败时断言 `(0, 0)` 与一次 runner 调用；测试尚待执行。
- 新 pinned 三文件 collision 测试断言只有前两个成功文件进入尾计数，typed service failure 传播，journal 为 `rolled_back`，Central 目标、operation siblings 与 DB skill 均不存在；未发送 Finalizing。测试尚待执行。
- D3 新 scoped cfg(test) mkdir probe 默认关闭，记录实际 API 返回的 path/success。正确性 tests 包含共享 parent、unsafe path 和 parent/file collision；未将该局部 probe 解释为全服务 syscall 计数。
- 独立文本检查证明原 HEAD inline delete tests 与新 `fs/tests.rs` 的 Rust tokens、字符串、注释、7 个函数名、test 属性和 1 个 ignore 属性一致。首个仅 dedent 的逐字比较失败；diff 只有三处 rustfmt 换行，修正为保留 literal 的 token 比较后 PASS。`central_operation/fs.rs` 为 621 lines，production 行为改动仅借用排序，另有 test module wiring。
- `git diff --check` 在初版 diff 上通过。后续正式 gate 仍需对最终 diff 执行。

## 当前验收表

| 要求 | 当前状态 | 需要的证据 |
| --- | --- | --- |
| before/after binary 与 harness 身份 | `PASS` | 独立 HEAD before archive、同一修正版 harness、两冻结 binary hashes、14 个交替 logs/sidecars |
| after 完整 production 源码冻结清单 | `UNVERIFIED` | 冻结时清单未包含新增 `central_updates/fs/progress.rs` 和移出的 cfg(test) `central_operation/fs/tests.rs`；当前 hash 仅为事后 review snapshot |
| D1 真实计数与 publication budget | `PASS`（backend/source） | 成功写后计数、Preparing 为 0、部分失败尾值、闭合 channel/service settle、Remote 失败 0 count 均有通过的回归；source calls 13,018→98 |
| D1 native UI 队列无持续积压 | `NOT_RUN` | Windows native WebView 运行证据；AppNone source probe 只证明 source emission attempts |
| D2 摘要兼容 | `PASS`（Windows tests） | 旧 inventory 与 selected-subtree 算法对照，root/nested、unrelated binary、empty 与 exact manifest 错误；正式 suite 通过 |
| D3 parent mkdir 去重 | `PASS`（targeted probe） | 实际 API probe：8 个共享 parent 文件只有 root/shared parent 两次成功创建；unsafe/collision 保留失败及 byte-exact 断言；不宣称全服务 syscall 计数 |
| D4 directory fingerprint 兼容 | `PASS`（Windows tests） | 旧排序/新排序摘要对照含 prefix、Unicode、empty directory、symlink；fresh validation 与 collision 回归通过 |
| journal 与 copy 恢复 | `PASS`（Windows tests） | stage/swap/DB/copy/finalize、commit-unknown、kill/reopen、selected-row 隔离及 repeated retry；不代表全部大规模 fault/crash 性能矩阵 |
| Local/SSH/WSL caller 同形 sweep | `PASS`（static/fake） | ordinary/force/mirror、owning archive chunk、existing batching 与 fake failure assertions；真实 SSH/WSL 仍未执行 |
| 性能预算 | `PARTIAL / NOT_MET` | T 交替 7 对 stage 中位数 -6.56%，固定 10% 目标未达到；delete stage +12.77%、L control no-op +30.72% 原因未查明；首正式与 control 样本分别保留 |
| lint/typecheck/tests/just ci | `PASS` | canonical `just ci` exit 0；common 12 阶段与 rust-platform 3 阶段全部完成，原两次 FAIL 保留 |
| installer、真实 SSH/WSL、网络 acquisition | `NOT_RUN` | 单独 runtime/distribution 证据，不由 backend 或 fake transport 替代 |

## 正式检查结果

### 已修复的问题

- `github_import/progress/tests.rs`：source probe 的观察 borrow 跨 `pool.close().await`，pinned write failure 的观察 borrow 跨 DB await；Clippy `await_holding_refcell_ref` 失败。只增加两段词法作用域，使 borrow 在 await 前释放。保留全部断言、输出和生产行为，不增加 lint allowance。新测试 hash 为 `5951F0EB5C66CFCF58B16204488C1EB109F59AD6FF86EF455EF22C3A0CBEB252`。
- CI 环境：现有 `src-tauri/target` 为指向缺失 mbx 目录的 SymbolicLink，Cargo 创建目录报 Access denied。只为 CI 进程设置仓库内 `CARGO_TARGET_DIR=.git/skill-small-files-tools/quality-target`，保留链接、ACL、全局配置和原 canonical gate。

### 命令与实际结果

| 命令 | 结果 |
| --- | --- |
| `just ci`，首次原环境 | exit 1；Clippy Cargo exit 101，无法创建失效 target link；common 2 阶段完成，其余被取消或未执行 |
| `just ci`，修复构建目录后 | exit 1；Clippy 两处新测试 borrow 跨 await；common 8 阶段完成，IPC 被取消，其他后续阶段未执行 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`，作用域修复后 | exit 0 |
| `just ci`，作用域修复后 | exit 0；15/15 阶段完成。pnpm 10.34.5，Node/Rust 使用项目固定工具链 |
| `git diff --check -- .trellis/spec/backend/central-update-batching.md .trellis/spec/backend/spawn-blocking-io.md src-tauri/src` | exit 0 |
| `python .trellis/scripts/task.py validate .trellis/tasks/10-08-skill-small-files-performance` | exit 0；implement 9 entries、check 11 entries |

正式 gate 的 Rust 为 1,574 passed / 0 failed / 9 ignored：lib 1,559；两个有测试的 binary 各 3；两个 integration suites 分别 4 和 5；另两个 result groups 为 0。Vitest 为 178 files、2,078 passed、1 skipped。Python 为 54 total、50 passed、4 POSIX-only skips。各忽略/跳过行和所有阶段计数见 `quality-checks/formal-ci-summary.json`；Vitest 默认 reporter 只记录跳过总数，没有记录单项名称，未补写猜测名称。

完整 raw logs/receipts：`quality-checks/just-ci-20261008-115051.*`、`just-ci-retest-20261008-115433.*`、`just-ci-fixed-retest-20261008-115814.*`。成功日志 SHA-256 为 `B3D91D3408BE0FE1BF5C5E3DAD0C890F80F4A63E27DF13B0AD8C0E94E7057AF4`，228,420 bytes。三次 logs 与 receipts 的 hash/bytes 均核对通过。未用 focused tests 替代正式 gate，成功后未重复全 gate。

最终复核已读取当前 PRD/design/implement/task notes 和两个已修改 backend specs。单 Skill reporter、closed-channel retained counters、fresh disk validation、ordinary/force/mirror ownership 与实际实现一致。原 inline delete tests 移动仅改变 wrapper/缩进/rustfmt 换行，tokens、literals、注释、7 函数和 1 ignore 属性一致。新增 parent probe 默认关闭并 scoped thread-local，锁隔离与 journal 安全边界保留。除两处 test borrow scope，独立检查未修改生产代码。

### 未修复的验收缺口

T stage 的固定 10% 目标为 `NOT_MET`，删除 stage 和 L control no-op 超过 10% 回退，原因未查明。没有证据可把回退归因于扫描或缓存；相关 source body equality 不能解释运行时差值。保留首对照、controls 和 7 对交替结果，不扩大优化范围、不调整预算。

baseline 代理确认冻结时没有保存两新增文件的额外 hash。after binary/harness 精确身份保持有效；完整 production 源码冻结清单为 `UNVERIFIED`。`quality-checks/current-review-source-identity.json` 记录检查时 21 个 changed/new Rust 文件 hash，不能改写成历史构建时 receipt。最终 gate 使用修正版测试源码，冻结 release/source probe 使用原测试源码身份；测试 scope 修复未改变生产行为。

T corrected fullservice 单轮 1 test PASS 验证 first/overwrite、uid、no-op mtime、0/1/3 copies、retain/delete/leftover 和 repeated recovery/finalize，允许与 CI 并行，只作为正确性证据。M/T/H 服务完整 7 轮、X 扩展性能 fixture、完整大规模 fault/crash/symlink 矩阵、worker IO/峰值资源、native IPC/WebView、Windows bundle/安装器、真实 GitHub acquisition、真实 SSH/WSL 与 Linux/macOS hosted checks 均未执行。截图 installed runtime 身份保持 `UNVERIFIED`。

回退仅审核批准的生产 hunks，并保留数据、pending journal、marker、backup/staging、隔离测试和原始 receipts；不执行 reset/stash/clean。若后续改变生产行为，需建立新的 after binary 身份并复测受影响范围。当前没有产品回退、依赖/版本变更、提交、归档或外部写入。
