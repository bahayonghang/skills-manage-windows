# 隔离 Windows backend 基线

状态：选中阶段 T7、S/L 全生命周期7轮、M/H stage单轮边界与source event probe已通过。优化结果尚未计入。

## 可比构建

正式 before 使用修正版harness与 test-only seams，从 `git archive HEAD` 的独立源码树构建。source HEAD为 `063c8d3236197a06ff17083c28a9738487c07510`；frozen binary SHA-256为 `46A19EC66B300DF79D7345724DB5CA2440160AC29A23202655965A41088AD36F`。`baseline-targeted-build-identity.json`、source identity与diff receipts保存完整证据。

`benchmark-targeted-test-seams.json` 保存修正candidate digest之前的中间态harness hash，不是正式构建身份。正式身份以 `baseline-targeted-build-identity.json` 的6个文件hash、`baseline-targeted-source-identity.json` 和 `baseline-targeted-test-seams.diff` 为准；原始中间态文件保持不变。

after完整构建时源码清单 `UNVERIFIED`。已核验after冻结binary、同harness及tracked diff；after的6文件记录和tracked diff没有新增生产 `central_updates/fs/progress.rs` 及移出cfg(test) `central_operation/fs/tests.rs` 的构建时hash。独立check在 `quality-checks/current-review-source-identity.json` 事后补充当前hash和生产未改记录，该证据不能替代冻结时内容清单；原before/after receipts保持不变。

before/after须使用相同 `performance_benchmark.rs` SHA-256 `203949B25783EA044256268BF4D48CBD6967386A774D78A8515477A64970B923`、cfg(test) wrapper计时与fixture。阶段值为 wall await ms，包含blocking task调度，嵌套阶段不能相加。worker内CPU、syscall/mkdir/open计数、峰值内存均未测。

`db_persist`现有tracing span包含prepare/manifest construction与旧target hash，不等于纯SQLite事务时间；stage包含writer和fresh hash。没有按span label推断syscall次数或worker CPU耗时。

## T：13,016 files / 206 directories / 13,327,429 bytes

原始日志 `baseline-targeted-formal-20261008-070646.log`：1 test passed，331.66s。7轮统计来自同一日志；sample0与1–6在summary分别保留。

| 生产 helper阶段 | 中位数 ms | 最小–最大 ms |
| --- | ---: | ---: |
| durable stage（writer + fresh staging hash） | 21,371.1808 | 19,878.9027–23,294.1299 |
| delete manifest fingerprint | 3,404.1441 | 3,119.5943–6,093.3422 |
| delete staging rename | 21.8284 | 17.6402–25.5835 |
| delete finalize（fresh fingerprint + physical removal） | 18,588.7603 | 14,639.2708–20,292.4387 |
| repeated delete finalize | 0.1453 | 0.1213–0.2299 |

按既定10% stage目标，after中位数需不高于19,234.06272ms。delete排序不预先承诺时间收益；fingerprint与finalize的单样本波动较大，原因未查明。

## S/L：全生命周期，7轮

原始日志 `baseline-service-formal-20261008-071329.log`：1 test passed，97.18s。S包含128 files、4 directories、130,117 bytes；L包含32 files、3 directories、13,327,429 bytes，bytes精确匹配T。

| 操作 | S 中位数 ms | L 中位数 ms |
| --- | ---: | ---: |
| first content upsert | 363.5581 | 180.4219 |
| overwrite content upsert | 993.8461 | 279.3133 |
| cached unchanged check | 65.3939 | 36.2741 |
| cached no-op apply | 49.0069 | 32.6592 |
| all payload files changed, 0 copy | 857.8402 | 378.5608 |
| all payload files changed, 3 copy | 1,879.1741 | 607.9020 |
| delete Central, retain 3 copy | 225.1554 | 94.1945 |
| leftover cleanup 3 copy | 317.0900 | 95.3054 |

日志与summary另含单payload文件、1% payload、1copy、重复leftover/recovery/finalize及各phase counts。`baseline-service-summary.json`保留全部raw rows、sample index、中位数和范围。SKILL.md在payload变化操作中保留。

## M/H边界与source emitter

`baseline-boundary-20261008-071527.log`：1 test passed，96.76s；各fixture只有一次，不能用于稳定收益结论。

| Fixture | 实际 files / directories / bytes | stage ms | fingerprint ms | delete finalize ms |
| --- | --- | ---: | ---: | ---: |
| M | 4,096 / 66 / 4,193,349 | 7,667.1667 | 1,451.4400 | 5,565.4440 |
| H | 18,000 / 284 / 18,431,045 | 33,198.7321 | 6,381.3109 | 32,185.8614 |

`baseline-source-progress.log` 使用同一个新版before冻结binary，真实 `import_github_repo_skills_from_pinned_snapshot`，AppNone，1 test passed，51.49s。运行计数：source_calls13,018；writing13,017；preparing0；finalizing1；末尾13,016files/13,327,423bytes，与完整disk/DB断言一致。该sourcefixture的SKILL.md比FSfixture少6bytes；D1前后沿用该sourcefixture。

source emission attempts与IPC delivery不同。AppNone不能测IPC/WebView耗时；旧source的writing尝试在实际writer前累积，该probe证明调用数量与结果内容，不能把旧计数当真实写入完成时间。after须另验真实writer publication与失败末尾flush。

## 历史诊断与未跑项

旧T全生命周期smoke日志保留13,016文件真实服务耗时，但旧harness错误地把repository domain digest传入content_digest。该test断言通过不构成provenance正确性通过；不用该日志判定稳定端到端提速。旧首个E0597编译失败与外部中断log保留，见 `baseline-progress.md`。

M/T/H全生命周期7轮、X内容/边界扩展fixture、fault/crash/symlink矩阵、真实repo获取、IPC/WebView、真实SSH/WSL与安装器均没有由该基线证明。正式回归测试与其他验收面由主任务记录。未清OS cache，未修改安全扫描设置；扫描状态及成本 `UNVERIFIED`。
