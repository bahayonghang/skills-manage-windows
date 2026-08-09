# Bug Analysis: 删除被无关恢复阻断与仓库检查重复失败

## 1. Root Cause Category

- **Category B - Cross-Layer Contract**：Central mutation lock 只规定了 target 级串行化，但旧 delete single/batch 入口把 startup/显式 Retry 的全 target recovery 语义复用到普通删除。调用方选择一个技能，服务层却恢复整个 target，作用域契约不一致。
- **Category D - Test Coverage Gap**：既有删除测试覆盖单项成功、批量去重和 journal batch ID，没有构造「无关技能存在不可恢复 journal」的组合状态。Local/SSH/WSL 的 guard、connection、pending inventory 生命周期也没有在 shared batch seam 上统一断言。
- **Category B - Cross-Layer Contract**：`GithubImportError::Http` 同时代表 timeout、request、body 和普通 HTTP 状态；inventory、Operation Log 与 Runtime Log 只保留 `transport_failed`。行为层无法安全判断是否自动重试，诊断层也无法区分静态失败子类。
- **Category E - Implicit Assumption**：首轮并发 4 被视为足够可靠，默认瞬时 codeload/body 失败应由人工点击 Retry 处理。该假设没有写入契约，也没有用批后补偿测试验证。

## 2. Why Fixes Failed

1. 前一轮只修复 update apply 的 selected-skill recovery。delete-missing 复用另一套 `central_skills` single/batch 编排，因此同类全 target recovery 缺陷仍存在。
2. 前一轮补齐 apply item diagnostics，但 `FailedCentralSkillDelete` 仍只保留 `skill_id + error String`。delete recovery 的 typed code 在进入 inventory adapter 前已经丢失。
3. 手动 Retry 只能重新运行相同下载路径。首轮失败没有 typed retryability，也没有批后串行补偿，因此瞬时失败会稳定留在 Failed 页等待人工处理。
4. `transport_failed` 的固定 public code 满足脱敏要求，但缺少第二层静态 category。日志安全与日志可诊断性被错误地当成二选一。

## 3. Prevention Mechanisms

| Priority | Mechanism | Specific Action | Status |
| --- | --- | --- | --- |
| P0 | Architecture | 普通 Central mutation 在一个 top-level target guard 下只恢复 selected skill rows；startup、显式 Retry/Reconcile 保留 full-target 语义。 | DONE |
| P0 | Test Coverage | Local 回归固定无关 row 的 phase、updated_at 和 error evidence；Fake SSH 回归固定一次 connection、target identity 与命令数。 | DONE |
| P0 | Compile-time | delete failure DTO 保留 phase/code/category；snapshot retryability 与 diagnostic category 由同一个 typed classifier 提供。 | DONE |
| P0 | Runtime | snapshot 首轮全部 settled 后，仅对 typed transient family 串行补偿一次；terminal family 不重试。 | DONE |
| P1 | Observability | Operation Log 保存最多 50 个安全 repository failure items；Runtime Log 只保存 code/category 聚合与 retry counts。 | DONE |
| P1 | Documentation | 更新 mutation lock、journal、GitHub archive、inventory retry/progress 和 redaction specs。 | DONE |

## 4. Systematic Expansion

- **Similar Issues**：任何新 Central single/batch mutation 都可能误用 full-target recovery。代码审查必须检查 recovery selection、guard ownership 和 remote connection ownership，而不只检查是否“调用了 recovery”。
- **Design Improvement**：full-target 与 selected-skill recovery 必须使用不同命名和签名。普通 mutation 不应能在无显式 selected IDs 的情况下调用 recovery helper。
- **Process Improvement**：恢复类回归至少构造两个技能，其中一个携带不可恢复 journal；网络批处理回归至少区分首轮并发、补偿并发、最终结算和 terminal no-retry。
- **Knowledge Gap**：稳定 public code 不是完整诊断模型。需要同时保留固定 public message/code、静态内部 category 和严格禁止的动态 source detail。

## 5. Knowledge Capture

- [x] 更新 `.trellis/spec/backend/central-mutation-lock.md`。
- [x] 更新 `.trellis/spec/backend/fs-db-operation-journal.md`。
- [x] 更新 `.trellis/spec/backend/github-import-preview-contract.md`。
- [x] 更新 `.trellis/spec/backend/update-inventory-retry.md` 与 `central-update-inventory-progress.md`。
- [x] 更新 `.trellis/spec/backend/redaction-policy.md`。
- [x] 增加 Local、Fake SSH、snapshot retry、typed category 和 bounded log 回归。
- [ ] 现场重新执行 Delete/Refresh。该步骤会修改用户数据或触发真实网络，必须取得单独授权；当前保持 `UNVERIFIED`。

