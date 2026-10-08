# Windows Local 性能对照

状态：source进度调用数量目标 `PASS`；T stage原10%目标在交替7对中 `NOT_MET`。S/L首对照28个service项目有27个满足预算，控制测量保留未解耗时变化。不能标为全部性能验收通过，不能声称稳定删除提速。

## 构建与隔离

正式before从HEAD `063c8d3236197a06ff17083c28a9738487c07510` 的隔离archive源码构建。before binary SHA-256为 `46A19EC66B300DF79D7345724DB5CA2440160AC29A23202655965A41088AD36F`；after为 `41293FD91D399D680488B43AB1CCE547204776864FDDE309C32F1C07D76A3411`。两者均为release，使用相同harness SHA-256 `203949B25783EA044256268BF4D48CBD6967386A774D78A8515477A64970B923` 与cfg(test) wrapper计时。

已保存的构建证据为 `baseline-targeted-build-identity.json`、`baseline-targeted-source-identity.json`、`baseline-targeted-test-seams.diff`、`after-build-identity.json` 和 `after-source.diff`。`benchmark-targeted-test-seams.json` 为修正digest之前的中间态，保留原文件，不作为正式身份。

after冻结binary、harness和tracked diff已核验；构建时完整after源码清单为 `UNVERIFIED`。`after-build-identity.json` 的6文件清单没有新增生产文件 `src-tauri/src/services/central_updates/fs/progress.rs` 及移出的cfg(test)文件 `src-tauri/src/services/central_operation/fs/tests.rs`，`after-source.diff` 也未包含这两个untracked文件。没有找到两文件在12:31:16Z构建冻结时另存的hash/source receipt。

独立check在17:02:46Z保存 `quality-checks/current-review-source-identity.json`，补充两文件的当前hash并记录review仅修改GitHub进度测试词法作用域、未改生产代码。该清单用于当前review和最终CI源码核对，属于事后证据，不能证明两文件在冻结binary构建时的内容。原冻结receipts保持不变；不能称完整after源码清单已在构建时冻结。

临时DB、agent根、canonical/copy根、journal artifacts和实际mutation lock均在独立fixture根内。Local既有resolver仍可对用户优先/legacy数据库路径做有界metadata检查；没有读取用户DB内容、遍历实际Skill或写用户目录。没有改变home、OS cache或安全扫描设置。

## T stage：首7轮与交替7对

T包含13,016 files、206 directories、13,327,429 bytes。stage计时包含真实writer及fresh staging hash，wrapper await包含调度；没有分开worker CPU、open/mkdir/syscall。`db_persist` span包含prepare/manifest旧target hash，不能当纯SQLite事务时间；嵌套span不能相加。

| 记录 | before中位数 ms | after中位数 ms | 中位数差 | 判断 |
| --- | ---: | ---: | ---: | --- |
| 首7轮，before一组后after一组 | 21,371.1808 | 17,281.4260 | -19.14% | 数值达到10%，后续控制使稳定性未获证明 |
| 单轮paired control | 18,015.2822 | 22,860.9095 | +26.90% | 方向反转，不能替代7轮 |
| 交替7对，AB/BA按pair切换 | 19,810.1381 | 18,510.3364 | -6.56% | 原10%目标 `NOT_MET` |

交替pair索引0/2/4/6为before→after，1/3/5为after→before。14个run的raw log、stderr、sidecar及聚合清单 `alternating-t-runs-20261008-074725.json` 均保留。所有run为1 test passed、exit0、stderr为空，fixture/profile/mode/两种binary及同harness身份核对通过。`benchmark-paired-comparison.json` 保存各pair的5个操作、完整phase counts和CPU端点；没有合并首7轮或single controls。

| Pair | 顺序 | before stage ms | after stage ms | 配对差 |
| --- | --- | ---: | ---: | ---: |
| 0 | AB | 17,935.3411 | 16,353.7633 | -8.82% |
| 1 | BA | 19,735.7281 | 18,510.3364 | -6.21% |
| 2 | AB | 19,810.1381 | 16,425.5937 | -17.08% |
| 3 | BA | 20,600.3185 | 18,840.2772 | -8.54% |
| 4 | AB | 21,113.5114 | 18,954.9141 | -10.22% |
| 5 | BA | 19,212.2400 | 21,253.6528 | +10.63% |
| 6 | AB | 22,101.4482 | 17,972.2480 | -18.68% |

配对差中位数为-1,760.0413ms/-8.82%；6/7对较快，3/7对达到10%下降。before范围17,935.3411–22,101.4482ms，after范围16,353.7633–21,253.6528ms，范围重叠。保留首对照结果，但不以首7轮-19.14%宣称稳定达标。

每run的benchmark进程CPU端点可读，before中位数39.953125s，after38.296875s；数值包含fixture setup、validation和cleanup，不能解释为stage worker CPU。MsMpEng端点均 `UNMEASURED`，原因是没有单个可读进程值；没有据此确认或排除安全扫描原因。

## 删除与重复清理

| 交替7对阶段 | before中位数 ms | after中位数 ms | 中位数差 | 配对差中位数 |
| --- | ---: | ---: | ---: | ---: |
| manifest fingerprint | 3,304.0904 | 3,325.4234 | +0.65% | +15.26% |
| delete stage metadata/marker/rename | 24.4181 | 27.5360 | +3.1179ms/+12.77% | +3.2410ms/+14.29% |
| finalize fresh fingerprint + physical removal | 16,118.1083 | 16,643.2017 | +3.26% | -3.12% |
| repeated finalize | 0.1696 | 0.1801 | +0.0105ms/+6.19% | +0.0334ms/+21.29% |

delete stage首7轮为21.8284→29.5263ms，绝对+7.6979ms/+35.27%；范围17.6402–25.5835与23.6574–214.2064ms。单轮控制为27.5856→32.2467ms，绝对+4.6611ms/+16.90%。交替7对范围21.0831–30.2859与24.1703–35.9895ms，仍保留 `REGRESSION_REQUIRES_REVIEW`。原因未查明。

`benchmark-delete-diff-review.json` 核对HEAD与after的 `stage_delete_local`、`stage_delete_local_blocking`、`finalize_delete_local`、`finalize_delete_local_blocking` 函数体相同；fingerprint只改借用路径比较排序。stage成功路径不调用fingerprint。重复finalize的backup已不存在，不进入fingerprint或physical removal。该源码证据不能解释runtime差值，不能据此声明系统噪声已确认；删除时间收益保持 `UNVERIFIED`。

## S/L服务7轮与控制

S为128 files/4 directories/130,117 bytes；L为32 files/3 directories/13,327,429 bytes，bytes精确匹配T。首对照由 `baseline-service-formal-20261008-071329.log` 与 `after-service-formal-20261008-073713.log` 保存，分别1 test passed/97.18s与1 test passed/61.05s。

| 操作 | S before→after 中位数 ms | L before→after 中位数 ms |
| --- | ---: | ---: |
| first import | 363.5581→213.6697 | 180.4219→115.4433 |
| overwrite | 993.8461→449.5848 | 279.3133→210.7067 |
| cached unchanged check | 65.3939→24.5299 | 36.2741→30.1091 |
| no-op apply | 49.0069→30.7074 | 32.6592→28.8187 |
| all payload changed,0 copy | 857.8402→454.6042 | 378.5608→192.2332 |
| all payload changed,3 copy | 1,879.1741→1,434.3766 | 607.9020→478.0944 |
| delete Central,retain3 copy | 225.1554→147.2404 | 94.1945→82.7746 |
| leftover3 copy cleanup | 317.0900→228.1069 | 95.3054→80.6935 |

单文件、1% payload、1copy及重复操作全部raw rows/sample0/1–6/phase counts见 `benchmark-comparison.json`。该首对照的28个service预算项目为27个PASS、1个 `REGRESSION_REQUIRES_REVIEW`。pure snapshot component/摘要诊断没有用于service预算。

L repeated finalize首比0.1043→0.1171ms，绝对+0.0128ms/+12.27%；范围0.0861–0.1918与0.0931–0.1612ms。随后仅L fixture各7轮控制为0.1232→0.1201ms，绝对-0.0031ms/-2.52%，该增量未重现。首比和控制分别保留，不合并或删除首比。

L控制另见no-op30.5831→39.9770ms，绝对+9.3939ms/+30.72%；范围26.7932–37.2852与27.3695–61.2548ms。该项同样保留 `REGRESSION_REQUIRES_REVIEW`，原因未查明，不能标记全生命周期无回退。见 `benchmark-control-comparison.json`；控制包括T各1轮及L各7轮，未替代首正式7轮。

## Source进度与边界

真实离线 `import_github_repo_skills_from_pinned_snapshot`，AppNone，fixture为13,016 files/13,327,423 bytes。source fixture的SKILL.md比FS fixture少6bytes，前后内容相同。

before source calls13,018，writing13,017/finalizing1；after source calls98，writing97/finalizing1；preparing均为0。减少99.2472%，满足原至少90%目标，after上限为1,301条。末尾13,016files/13,327,423bytes、计数单调及完整disk/DB断言PASS。原始 `baseline-source-progress.log` / `after-source-progress.log` 与对应identity保存同source probe身份。

该计数为App分支前source emission attempts；AppNone没有执行Tauri IPC delivery。IPC、store更新次数、WebView render/队列和native UI响应均 `NOT_RUN`。真实writer/末尾错误flush的独立回归由production测试记录，source成功probe不替代错误矩阵。

M/H stage各1轮边界均PASS：M stage7,667.1667→3,762.8254ms；H33,198.7321→18,569.2399ms。单轮没有稳定收益结论，详见 `baseline-boundary-summary.json` / `after-boundary-summary.json`。没有把边界单轮混入T7。

## 正确性smoke、回退及未跑项

冻结after的corrected T fullservice1 `PASS`，日志 `after-service-t-correctness-20261008-115038.log` 为1 passed/0 failed/308.79s。first/overwrite、cached check/no-op、单payload/1%/全payload更新、0/1/3copy、保留copy删除、已记录leftover清理及重复recovery/finalize的完整assertions均通过。sidecar最终核对binary/harness SHA与raw log SHA，summary单独保留，不混入正式性能对照。

canonical `just ci`已获并行窗口；该smoke只判断完整assertions及test终态。运行期间CI load允许并行，且没有正式正确before T全生命周期7轮，因此任何单轮耗时都不用于gain/no-regression预算。独立check随后只调整 `github_import/progress/tests.rs` 两处RefCell borrow的词法作用域以通过Clippy；该测试源码修复不改变生产行为或冻结binary。冻结after/source probe身份继续以原 `after-build-identity.json` 为准，最终gate的修正版测试身份由check另记，不将当前源码重新编译混入既有测量。

早先T fullservice单轮为给交替测量释放窗口而停止own frozen benchmark进程，日志 `after-service-t-smoke-20261008-074400.log` 为 `INTERRUPTED_UNVERIFIED`；temp artifact `skillport-io-benchmark-FnTsnc` 保留。历史中断artifact `skillport-io-benchmark-pD341Y` 同样保留。没有删除或重写失败证据，没有清理用户Skill。

若因未达预算决定回退产品优化，按 `after-source.diff` 逐项审核批准的生产hunks，保留其他代理和用户编辑、test-only隔离/计时、raw receipts、持久化数据及未完成journal；不使用reset/stash/clean，不改manifest/schema/digest framing。回退对应优化后须使用新after身份复测受影响scope；当前测量没有执行产品回退。预算保持原值。

M/T/H全生命周期7轮、X扩展fixture、OS/worker IO分项、峰值内存/线程/句柄、真实GitHub acquisition、IPC/native WebView、installer/bundle、真实SSH/WSL均 `NOT_RUN` 或 `UNVERIFIED`。截图installed runtime身份仍 `UNVERIFIED`。success benchmark没有覆盖fault/crash/symlink完整矩阵；生产回归与canonical gate由独立check另行记录。CI通过不能将 `NOT_MET` 或未测项目改为PASS。
