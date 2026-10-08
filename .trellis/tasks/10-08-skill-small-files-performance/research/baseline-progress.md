# 当前基线进度

状态：正式before选中矩阵已完成，见 `baseline-results.md`。本文保留历史能力smoke的失败、中断及诊断；以下旧smoke数字不用于正式性能收益判定。

## 原始证据

| 记录 | 状态 | 说明 |
| --- | --- | --- |
| `baseline-smoke-first.log` | FAIL | release test 编译 E0597；保留首个错误 |
| `baseline-smoke-retest-20261008-064545.log` | INTERRUPTED_UNVERIFIED | 编译 2m09s 成功，仅两条 component 输出；session 与进程消失，服务未返回 |
| `baseline-smoke-interruption-diagnosis.log` | PASS | 同 release exe 的 128-file 完整服务 smoke，1 test passed，6.46s |
| `baseline-smoke-t-recovered-20261008-065011.log` | HISTORICAL_DIAGNOSTIC | 1 test passed，597.91s；旧harness digest domain限制见下文 |

E0597 修复仅为测试 Layer 生命周期绑定；两条 unused imports 警告在 harness 中消除。失败与 retest 不覆盖。

## T：13,016 files / 206 directories / 13,327,429 bytes

| 操作 | 总 ms | 关键 wrapper await ms |
| --- | ---: | --- |
| snapshot collect/clone component | 11.4774 | 没有 IO phase |
| snapshot hash component | 13.4279 | 没有 IO phase |
| first content upsert | 38,365.7432 | stage 27,993.8298；swap 3,645.8089；finalize 6,629.6168 |
| overwrite content upsert | 55,172.7538 | stage 20,092.1649；swap 5,904.9336；finalize 21,662.5799；显式 hash 7,426.1122 |
| cached unchanged check | 3,631.1948 | 显式 hash 3,547.1914 |
| cached no-op apply | 4,131.8783 | 显式 hash 4,054.1244；没有 stage/swap/finalize |
| one payload file changed, no copy | 55,827.6671 | stage 20,856.8749；swap 7,606.6436；finalize 19,441.0418；显式 hash 7,774.8983 |
| 1% payload files changed, no copy | 61,264.0347 | stage 21,044.4704；swap 8,042.6351；finalize 23,055.6397 |
| all payload files changed, no copy | 48,664.1694 | stage 16,967.8095；swap 4,618.8197；finalize 16,432.1611 |
| all payload files changed, 1 copy | 70,889.3465 | copy 26,565.5745；stage 15,259.9233；finalize 16,581.6506 |
| all payload files changed, 3 copy | 144,219.3247 | copy 93,039.3646；stage 18,731.6574；finalize 20,553.2893 |
| delete Central, retain 3 copy | 17,709.9535 | fingerprint 3,625.5241；rename staging 20.9024；finalize 14,039.9759 |

总 ms 为真实 service 调用范围。component 计时没有取代 service。`db_persist` span 包含 manifest preparation 与显式 hash，不能解释为纯 SQLite transaction 时间。阶段值嵌套不能相加。stage 包含写入与 fresh hash；finalize 包含 fresh hash 与物理删除；worker 内部分项尚未测量。

单样本观察：纯内存 clone/摘要约 10–14ms，对这里的 38–56s 服务耗时占比小。首次 stage 与覆盖的 stage/finalize 是主要已测边界。磁盘 hash 在 check/no-op 中占主导。后续优化不可跳过 safety/fresh state validation；没有证据支持用复制优化单独宣称端到端明显改善。

## 测量身份

冻结 before binary 位于 `.git/skill-small-files-tools/benchmark-before.exe`，SHA-256 与 source/harness 身份见 `baseline-build-identity.json`。后续产品编辑后仍执行该 binary 保持 before 结果身份。环境与证据缺口见 `baseline-environment.json`、`benchmark-method.md`。

实际 screenshot runtime/source 身份 `UNVERIFIED`；没有运行中的 SkillPort 进程，不能由截图推断当前工作树构建。网络/IPC/native WebView/真实 SSH/WSL 均 `NOT_RUN`。

独立check指出历史 smoke `content_digest` 输入domain不正确，已修harness；以上旧binary的服务计时保留为诊断，不能作为provenance完整正确性PASS。正式before已从隔离HEAD source加修正版testseams构建，并同after使用修正版harness。

T历史smoke已结束：`test result: ok. 1 passed; 0 failed`，597.91s。leftover3copy清理20,224.8036ms；重复leftover11.1029ms；重复recovery1.0661ms；重复delete finalize0.1805ms。该test结果证明smoke自身断言通过；domain限制仍保留。

修正版before binary已在隔离HEAD tree构建，3m47s，无compile警告，128 targeted smoke PASS（0.52s）。新构建receipt为 `baseline-targeted-build-identity.json`，SHA-256 `46A19EC66B300DF79D7345724DB5CA2440160AC29A23202655965A41088AD36F`。T targeted7、service S/L7、M/H stage单轮及source emitter probe均已PASS；原始样本及未跑项见 `baseline-results.md`。

中断smoke遗留临时artifact `skillport-io-benchmark-pD341Y`（06:47:55创建，与中断log对应）保留为 `INTERRUPTED_UNVERIFIED`，没有删除或重写。成功样本的临时根按harness完成清理。
