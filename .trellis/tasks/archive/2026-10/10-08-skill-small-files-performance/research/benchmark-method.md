# 隔离服务基准

状态：正式 before 的 T stage 7轮、S/L服务7轮、M/H stage单轮及source probe已完成，见 `baseline-results.md`。after沿用修正版harness及同fixture；首个编译失败保留在 `baseline-smoke-first.log`。

从仓库根执行：

```powershell
pwsh -NoProfile -File .trellis/tasks/10-08-skill-small-files-performance/research/benchmark-run.ps1 -Label baseline-smoke -Files 13016 -Samples 1
pwsh -NoProfile -File .trellis/tasks/10-08-skill-small-files-performance/research/benchmark-run.ps1 -Label baseline -Samples 7
pwsh -NoProfile -File .trellis/tasks/10-08-skill-small-files-performance/research/benchmark-run.ps1 -Label baseline-service -Files '128,32' -Samples 7 -Binary .git/skill-small-files-tools/benchmark-before-targeted.exe
pwsh -NoProfile -File .trellis/tasks/10-08-skill-small-files-performance/research/benchmark-run.ps1 -Label baseline-targeted -Files 13016 -Samples 7 -Mode stage -Binary .git/skill-small-files-tools/benchmark-before-targeted.exe
pwsh -NoProfile -File .trellis/tasks/10-08-skill-small-files-performance/research/benchmark-run.ps1 -Label after-targeted -Files 13016 -Samples 7 -Mode stage -Binary .git/skill-small-files-tools/benchmark-after.exe
pwsh -NoProfile -File .trellis/tasks/10-08-skill-small-files-performance/research/benchmark-run.ps1 -Label after-service -Files '128,32' -Samples 7 -Binary .git/skill-small-files-tools/benchmark-after.exe
pwsh -NoProfile -File .trellis/tasks/10-08-skill-small-files-performance/research/benchmark-run.ps1 -Label after-boundary -Files '4096,18000' -Samples 1 -Mode stage -Binary .git/skill-small-files-tools/benchmark-after.exe
python .trellis/tasks/10-08-skill-small-files-performance/research/benchmark-summarize.py <raw-log> --output .trellis/tasks/10-08-skill-small-files-performance/research/baseline-summary.json
pwsh -NoProfile -File .trellis/tasks/10-08-skill-small-files-performance/research/benchmark-alternating.ps1 -Pairs 7
python .trellis/tasks/10-08-skill-small-files-performance/research/benchmark-paired-summary.py --runs .trellis/tasks/10-08-skill-small-files-performance/research/alternating-t-runs-20261008-074725.json --output .trellis/tasks/10-08-skill-small-files-performance/research/benchmark-paired-comparison.json
```

实现位于 `central_updates/core/performance_benchmark.rs`，单个 ignored test。普通 CI 不生成大型 fixture。仅使用现有 Cargo 依赖、PowerShell 和 Python 标准库。

## 测量入口

- `journaled_central_content_upsert_with_fs`：首次导入、覆盖，含真实 lock/recovery/journal/stage/swap/DB/finalize。
- `check_central_skill_updates_impl`、`update_central_skills_impl`：已填充不可变 snapshot cache，测 check、no-op、单文件、1% 及全文件变化；变化更新包含 0/1/3 copy。第一轮 copy 初始化在计时外，后续更新为覆盖现有 copy。
- `delete_central_skills_impl`：Central 删除、保留 3 copy；`apply_remove_deleted_platform_copies_step`：清理 3 个已记录 copy 及重复调用。
- `recover_pending_operations`、`finalize_delete_local`：收敛后重复恢复/清理。
- `collect_remote_skill_files`、`hash_remote_files`、`repository_snapshot_digest_from_local`：单独的现有生产 helper 诊断计时，不能替代服务总耗时。

`-Mode stage` 使用相同 fixture、target lease 和生产 `build_operation_update_manifest` / `stage_operation_updates`，计时范围从已构建的 owned stage 输入开始，包含 stage worker 文件写入与 fresh staging hash。随后通过 `build_local_delete_manifest` 测 fingerprint，执行真实 delete stage/finalize/repeat；update marker 由 production rollback 消费。该入口没有业务 DB apply、copy 或 canonical swap，标为 component，对照完整服务 smoke使用。

数据库通过 `test_support::file_pool` 创建，所有 DB agent root 重定向到 fixture。文件、operation artifacts 和 mutation lock 都在新临时目录内。`central_mutation` scoped test guard 在 current-thread runtime 设置路径，并在 Drop 恢复；guard 不可跨线程移动。不修改用户 home、环境中的数据根或实际 Skill 目录。

Local guard 的既有路径resolver仍可对优先/legacy `db.sqlite` 路径做有界 `exists`/metadata检查，再由test seam替换实际lock路径。该检查没有读取数据库内容、遍历Skill或写用户目录；全部mutation IO在临时根内。未对全局home/env resolver做改动。

Central 删除会 FK cascade installation rows。leftover 阶段按现有 scanner/test fixture 重建非 Central skill 与 copy installation rows；重建在计时外，cleanup 计时包含真实 remove/install DB 边界。

## Fixture

总文件数包含有效 `SKILL.md`。其他文件固定为 1,024 bytes，每 64 个文件共享一个 parent，树深度固定。32-file fixture 使用相同 manifest，31 个较大文件的总 bytes 精确匹配 13,016-file fixture。每条样本记录实际 files/directories/bytes。文件加目录少于 20,000 copy-entry 预算；展开总 bytes 少于 256 MiB；单文件少于 32 MiB。

选中正式矩阵为T stage 7轮、S/L服务7轮；M/H stage各1轮作为边界，T after服务单轮为正确性smoke。未完成原始全矩阵的项目明确标为 `NOT_RUN`，见 `baseline-results.md` 与 `implementation-budget.md`。sample 0 与后续 6 次分别保留；顺序与 before/after 一致。每轮均创建新的磁盘路径。文件内容、目录数、uid、no-op manifest mtime、copy 内容、terminal journal 和 artifacts 完成核验。fixture 验证在总操作计时外。

## 证据边界

构建为 `cargo test --release --locked --lib`。before/after 使用完全相同的 ignored harness 和 cfg(test) instrumentation。`fs_util` test span 包住一个 coherent blocking task 的创建至 await 返回；span 记录 wrapper 总等待 wall ms，包含 task 排队。现有 phase spans 有嵌套；阶段值不能直接相加。

历史 smoke binary 位于 `.git/skill-small-files-tools/benchmark-before.exe`，身份保存于 `baseline-build-identity.json`。修正版正式before binary位于 `.git/skill-small-files-tools/benchmark-before-targeted.exe`，身份保存于 `baseline-targeted-build-identity.json`。after binary位于 `.git/skill-small-files-tools/benchmark-after.exe`，身份保存于 `after-build-identity.json`。产品代码开始编辑后，before样本只执行冻结binary，不重新cargo build后混入优化版本。

after记录已核验冻结binary、同harness及tracked diff，完整构建时源码清单 `UNVERIFIED`。6文件hash清单与tracked diff漏记新增生产 `central_updates/fs/progress.rs` 和移出cfg(test) `central_operation/fs/tests.rs`，没有另存两文件的构建时hash。独立check事后保存 `quality-checks/current-review-source-identity.json`，包含当前hash与生产未改记录；该snapshot仅用于当前review/CI，不作为冻结构建时内容证明。原receipts不覆盖，不以事后补hash宣称构建时已保存完整after源码清单。

独立检查发现历史 smoke harness 的 `content_digest` 输入误用了 repository domain，已修为 production candidate digest。旧 binary 的耗时保留为诊断历史，不能作为 provenance正确性验收或稳定端到端收益对照。正式before已由 `git archive HEAD` 的隔离源码树加修正后的测试seam构建，targeted/service正式样本均执行新冻结binary；after使用同修正版harness。构建身份另行记录。

`benchmark-targeted-test-seams.json` 为修正candidate digest之前的中间态清单，保留原证据。该文件中的harness hash `E3107194...` 不用于正式比较。正式before以 `baseline-targeted-build-identity.json` 的6文件hash及 `baseline-targeted-source-identity.json` / `baseline-targeted-test-seams.diff` 为准；后两项单列core.rs wiring，确认其余370个tracked Rust文件未改。正式before与after的harness hash均为 `203949B25783EA044256268BF4D48CBD6967386A774D78A8515477A64970B923`。

summary按日志分组并记录日志SHA-256、profile/mode、sample index、phase calls与原始rows。sample0、sample1–6分别保留，不把smoke与正式7轮合并。没有把7次样本称为可靠p95。

因首7轮与单control出现方向变化，另执行7对T stage交替测量。`benchmark-alternating.ps1`固定上述两种binary/harness SHA和fixture/profile，逐run hidden process，不启动Cargo；pair0/2/4/6为before→after，pair1/3/5为after→before。每run独立Samples1/raw/stderr/identity，聚合文件保留pair index及进程CPU端点。`benchmark-paired-summary.py`核验14个完整test PASS、日志hash、sidecar、顺序及CPU端点，另存pair差值和median；不合并首7轮。benchmark CPU包括setup/validation/cleanup，scanner端点不可读记 `UNMEASURED`，不归因安全扫描。

原10% T stage目标在交替7对中未达到；完整数字、首对照与控制变化见 `benchmark-results.md`。该结果不因correctness/CI通过而改为PASS。

source进度probe执行真实pinned import：

```powershell
$priorProbeFiles = $env:SKILLPORT_PROGRESS_PROBE_FILES
try {
    $env:SKILLPORT_PROGRESS_PROBE_FILES = '13016'
    & .git/skill-small-files-tools/benchmark-after.exe pinned_snapshot_source_progress_probe --ignored --nocapture --test-threads=1
}
finally {
    $env:SKILLPORT_PROGRESS_PROBE_FILES = $priorProbeFiles
}
```

将binary替换为 `benchmark-before-targeted.exe` 可重跑before source probe。source fixture的SKILL.md少6bytes；该fixture前后保持相同，不能与FS fixture计时或字节混用。source observer在App分支之前计数；AppNone证明source emission attempts与最后计数，不能证明IPC delivery。

没有 AppHandle，Tauri IPC delivery、store和WebView没有进入测试，未测native UI；网络、archive获取和真实SSH/WSL未执行。没有worker内CPU、峰值内存、syscall/open/mkdir数或安全扫描耗时，不能由wrapper数量推断这些数值。未清OS cache、未关闭安全软件、未修改系统设置。构建耗时与fixture/setup/validation/cleanup分开，不计入服务操作ms。

错误路径、崩溃恢复、symlink target 保留及 snapshot acquisition 预算的正式验证由相关回归测试负责；当前成功路径基准不代替这些验收。
