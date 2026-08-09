# 全技能检查结果缺失诊断

## 用户可见症状

- 模式弹窗显示 `Current scope: Check all (141)`，但进度显示 `Checked 0 / 1 repositories`，且只列出 `jakubkrehel/skills`。
- refresh 完成后的 Update Center 所有现有 tab 都为 0，没有解释其余技能的状态。

## 当前数据库只读证据

对 `C:\Users\lyh\.skillsmanage\db.sqlite` 的只读查询结果：

| 项目 | 数量 |
| --- | ---: |
| Central skills | 141 |
| 有 repository membership | 7 |
| 无 repository membership | 134 |
| Marketplace skills | 0 |
| skill_update_states | 0 |
| inventory runs | 1 |
| inventory entries | 0 |
| 可同步且去重的 GitHub repositories | 1 |

当前 141 个 skill row 的 `source` 均为 `native`；7 个已分配技能共同属于 `jakubkrehel/skills`。

## 调用链结论

1. `inventory/mod.rs` 先加载 scope 内全部 141 个技能。
2. `prepare_skill_updates` 对无 membership 的技能生成 `local-unknown` assignment。
3. `unsupported_state_from_assignment` 正确把这些技能分类为 `unsupported`。
4. repository progress 只对可查询 GitHub repository 去重，所以分母为 1；这是网络工作进度，不是 scope 过滤结果。
5. refresh 聚合只保留 actionable/error/platform buckets，丢弃 `up_to_date` 与 `unsupported`。
6. `persist_refresh_inventory` 只写现有可行动 bucket，因此产生一个零 entry 的 run；reload 和前端都无法展示 134 个 unsupported 技能。

`inventory/mod.rs` 的步骤注释曾声称 refresh 会写每个 state，但模块头、数据库 repository 注释、产品文档、归档任务合同和 `refresh_clears_stale_update_inventory_without_touching_baseline` 测试共同规定相反语义：`skill_update_states` 是成功 apply/update 后的安装 baseline，refresh 结果只能写独立 inventory。当前 state 表为 0 不是缺陷，修复不得向它写 unsupported/up-to-date/error。

根因是结果建模和持久化缺口，不是新的筛选条件，也不是 GitHub token。archive 302 修复只恢复了唯一可查询 repository 的快照获取，因此把此前被 transport failure 遮蔽的缺口暴露出来。

## Provenance 证据与限制

- 一个只读的迁移前备份包含 111 个当前无绑定技能的 membership，均能按稳定 skill ID、唯一 name 与路径后缀精确对应；UID 已变化。
- 在全部可读 snapshot 中，134 个当前无绑定技能有 112 个曾出现 membership：82 个只有一个 repo identity，30 个跨 snapshot 冲突，22 个从未出现。
- scanner 的普通 upsert 会覆盖 `skills.source`，但不会删除 membership；membership 可通过显式 detach/reassignment 或 stale-skill 删除的 FK cascade 合法消失。
- 因此历史记录不能作为当前 assignment 的自动权威来源。本任务不写真实数据库、不回填 membership、不推断 repo。

## 修复结论

最小完整修复是：

1. 保留 repository progress 的去重语义，但明确它与技能 scope 是两个维度。
2. 为 inventory 增加只读 `unsupported` bucket，使每个 scope skill 都有可见分类。
3. 继续在同一事务写 inventory run 与 entries，并保持 `skill_update_states` 完全不变。
4. reload 与前端展示 unsupported，并在它是唯一非空 bucket 时默认选中。
5. 不改变现有 update/apply 决策，不自动恢复 provenance。

## Scanner 来源关系丢失风险

- local scanner 会过滤不存在的 agent root，但仍提交全局 stale reconciliation；`scan_keep_skills` 因此缺少原 Central skill。
- 持久化事务随后删除所有不在 keep set 的 parent skill，SQLite FK cascade 同步删除 update baseline、repository membership 与其它 owned relations，并 prune 变空的 repository。
- 后续根目录恢复时 scanner 会重新插入同 ID skill 并生成新 UID，这与备份中 111 条“ID/路径稳定、UID 全变且 membership 消失”的记录形状一致。
- 这条链路是可独立复现的未来来源关系丢失风险，修复必须区分“权威 Central 根成功扫描为空”和“Central 根未扫描”，后者禁止 destructive reconciliation。
- 本任务只增加未来保护和回归测试；不读取这些历史证据执行写入，不修改真实数据库或 Central 文件。

## 7 月 29 日实际历史触发点

后续证据推翻了“scanner 是本次历史丢失直接触发点”的早期判断：

- `startup-recovery-20260729T035522.330Z-*` 保存的原数据库仍有 134 skills、23 GitHub repositories 与 111 memberships，`quick_check=ok`。
- 当前数据库在同一秒创建，首次 scan 随后把磁盘技能登记为 `native`；当前 141 skills 中只有 8 月 3 日新导入的 7 个技能保留 1 个 repository。
- 恢复库的 migration 1 checksum 是旧 Windows 值 `aabde4...`，当前代码锁定值是 `173296...`；migration 2-4 完全一致。
- 7 月 29 日 runtime log 在重建前记录 `startup.schema_initialization_failed`、`diagnostic=Healthy`。提交 `a47c7cd9` 在归一化换行时替换了已发布 v1 checksum，未保留 compatibility alias。

因此本次历史数据丢失链路是：migration checksum 兼容回归 -> 健康数据库被拒绝 -> 用户从启动恢复页执行 rebuild -> 原数据库进入保留目录 -> 空库扫描只能恢复技能文件，不能恢复 DB-only repository provenance 与其它 metadata。Scanner 权威覆盖缺陷仍需保留已完成的防护，但不能再作为这次事件的主根因。
