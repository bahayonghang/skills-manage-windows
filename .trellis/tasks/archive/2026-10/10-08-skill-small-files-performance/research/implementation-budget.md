# 实施范围与预算锁定

锁定时间：2026-10-08，生产改动开始前。来源：`baseline-smoke-retest-20261008-064545.log` 的中断结果仅作 component 记录；完成中的 T smoke 服务样本由 baseline 代理保存独立日志。原始文件未覆盖。

## 已有运行证据

- Windows release、Local、13,016 files、206 directories、13,327,429 bytes，无 AppHandle。
- 首次 journaled import：38,365.743 ms；stage wrapper（实际写入与 fresh hash）27,993.830 ms；swap 3,645.809 ms；finalize 6,629.617 ms。
- collect/clone 11.477 ms，memory manifest hash 13.428 ms。字节复制和内存摘要未主导该样本，不以此声称端到端提速。
- S128 完整成功路径 PASS；T 生命周期 smoke 与正式样本仍在执行。单次样本不用于判定稳定收益。

独立检查随后发现旧 harness 的 upsert provenance 使用了 repository digest，已修为 candidate digest。上述时间只保留为历史诊断，不作为 provenance 正确性 PASS；正式 before/after 共用修正版 harness 和独立 HEAD 源码构建。数值预算保持不变。

## 选定范围

1. D1：writer 成功写入后累计，Local source bridge 每 256 files 或 100 ms 发布聚合值，成功/失败末尾精确 flush。async importer 最多每 100 ms 采样，保留开始、阶段切换及终态；AppHandle 不进入 blocking closure。Remote 单 Skill 计数在 service 成功后增加。
2. D3：operation-owned staging 内复用已成功创建的 exact parent，保持文件顺序、逐路径安全检查和原错误。
3. D2：移动已有所有权，消除 plan/write/stage/worker 重复 bytes clone；过滤选中子树后计算 candidate digest，保持旧 framing。保留首次从 retained snapshot 复制；不扩大 snapshot 缓存表示。
4. D4：delete fingerprint 使用借用路径排序，保留原全树收集/排序和全部 fresh reads。update hash 去掉冗余排序；流式 buffer 与有界 IO 仅在单独实验显示收益后采用。

## 数值判定

- Local writer 聚合 source publications ≤ `ceil(files / 256) + ceil(write_ms / 100) + 2`，不存在每文件 publication；source event / async observer tests 分开计数。T 文件主导的快速路径相对旧 source events 目标减少至少 90%。没有 AppHandle 时 IPC/WebView 不填 runtime 数字。
- 最后累计 files/bytes 精确，错误累计只包含已成功写入；preparing 为 0；writer 完成不表示 journal/finalize 完成。
- 相同 binary profile、fixture 和环境的 7 个样本比较。T stage 中位数目标至少降低 10%；端到端收益需超出前后样本波动。目标未达到须报告，不改变预算。
- S/L 对照及 update/delete 中位数允许最多 10% 回退；更大变化必须查明原因或回退对应优化。删除排序可按分配去除/摘要兼容验收；无稳定时间收益时不声称删除提速。
- 全矩阵以实际执行为准，首个失败、源码身份、raw ms、sample 0 和 sample 1–6 保留。若大文件数全生命周期 7 轮耗时过长，先完成选中 stage/fingerprint 的 7 轮及真实全生命周期 smoke，未完成完整矩阵显式标注，不能标为全部性能验收通过。

保留 uid、target lock、selected recovery、snapshot pin/digest、journal、copies_pending、同步 finalize、Windows symlink 目标保护和原始错误。无 schema/依赖/版本/用户数据变更。没有提交或发布授权。
