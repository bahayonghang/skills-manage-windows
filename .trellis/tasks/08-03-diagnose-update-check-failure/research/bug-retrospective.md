# Bug Analysis: 检查全部只显示一个仓库且结果全空

## 1. Root Cause Category

- **Category**: B/D/E - 跨层合同、测试覆盖缺口与隐式假设叠加。
- **Archive transport**: 共享 GitHub client 的 no-redirect 安全合同传播到 archive 下载，但
  archive 仍隐式依赖 GitHub 的合法 `302 -> codeload.github.com`。缺少覆盖真实协议的一跳测试。
- **Inventory completeness**: backend 已把无 repository membership 的 skill 计算为
  `unsupported`，但 inventory 聚合、持久化、reload 和 UI 只建模 actionable bucket，隐式假设
  “没有可操作项”等于“没有需要展示的结果”。
- **Provenance loss**: scanner 把空 keep set 隐式当成权威空目录，没有记录 Central 根是否真的
  存在并成功扫描；根缺失时 stale parent 删除通过 FK cascade 清除了 UID、membership、baseline
  和 owned relations。

## 2. Why Fixes Failed

1. **重复更新 PAT**: 处理了用户最可见的认证变量，但同一凭据的 `/rate_limit` 200 与 archive
   302 证明认证成功后仍会失败。
2. **只修 archive redirect**: 恢复了唯一 queryable repository 的 snapshot，却暴露了此前被
   transport failure 遮蔽的 inventory completeness 缺陷；134 个 unsupported skills 仍被丢弃。
3. **只解释 `1 repository` 文案**: 能纠正进度维度，却不能让缺失的 134 个结果持久化或显示。
4. **从历史快照自动回填来源**: 111 个映射可精确对应，但剩余项缺少权威来源且跨快照存在冲突；
   自动恢复可能覆盖合法 detach/reassignment，因此不是安全修复。

## 3. Prevention Mechanisms

| Priority | Mechanism | Specific Action | Status |
| --- | --- | --- | --- |
| P0 | Architecture | archive 只允许严格验证的一跳 codeload，且第二跳不携带 Bearer | DONE |
| P0 | Test coverage | 用 production-shaped 302 fixture 锁定 redirect、SSRF、auth 与预算 | DONE |
| P0 | Cross-layer contract | `unsupported` 从分类、transaction、reload 到 UI/i18n 完整 round-trip | DONE |
| P0 | Data integrity | scanner 以 `central_root_scanned` 作为 Central stale 删除的显式权限 | DONE |
| P0 | Regression tests | 缺根/不可读保留，成功空根清理，entry trigger rollback，baseline 隔离 | DONE |
| P1 | Documentation | 更新 inventory、transaction、deletion、frontend 和 test ownership 规范 | DONE |
| P1 | Review discipline | 任何测试过滤结果为 0 都不得作为验收证据 | DONE |

## 4. Systematic Expansion

- **Similar Issues**: 其它 destructive reconciliation 也应区分“权威空快照”和“数据源未覆盖”；
  仅凭空集合执行删除是同类风险。
- **Design Improvement**: 进度单位、选择范围与结果分类必须是独立类型/字段，不能让 repository
  work units 代替 skill scope，也不能让 actionable buckets 代替完整分类。
- **Process Improvement**: 跨层新增状态必须验证 Source -> Transform -> Store -> Reload -> UI
  全链路；数据库回归同时比较 parent identity、owned relations 与非本任务 baseline。
- **Knowledge Gap**: repository membership 是更新来源的权威合同，不可从 skill 名称、`source`
  字符串或冲突历史快照推断。

## 5. Knowledge Capture

- [x] `.trellis/spec/backend/central-update-inventory-progress.md`
- [x] `.trellis/spec/backend/transactional-mutations.md`
- [x] `.trellis/spec/backend/skill-deletion-integrity.md`
- [x] `.trellis/spec/frontend/async-ui-test-stability.md`
- [x] `.trellis/spec/quality/test-suite-layout.md`
- [x] 任务 PRD/design/implement/research 保留不回填 provenance 与不修改真实数据库边界。

仓库不存在 `src/templates`，这些 project-local `.trellis/spec` 没有可同步的模板副本。规范变更
留在当前任务工作树中，按本任务提交审批门处理，不在本阶段自动创建 commit。

## Bug Analysis: Published Checksum Rejected A Healthy Database

### 1. Root Cause Category

- **Category**: C/D/E - Change propagation failure, test coverage gap and implicit assumption.
- **Specific Cause**: commit `a47c7cd9` correctly made future migration hashes line-ending stable, but replaced the already-published migration-1 Windows checksum without retaining it as a compatibility alias. Startup treated any schema initialization failure as rebuildable when the file existed, so a healthy metadata incompatibility was offered the same destructive recovery action as corruption.

### 2. Why Earlier Fixes Missed It

1. **Archive redirect fix**: restored network acquisition but only exposed the already-lost repository inventory.
2. **Unsupported bucket**: made 134 missing assignments visible but treated them as an inventory modeling problem, not historical data loss.
3. **Scanner coverage fix**: closed a real future cascade path whose data shape resembled the incident, but the retained `startup-recovery-*` directory and exact timestamp later discriminated the actual historical trigger.
4. **Checksum normalization test**: locked only the new canonical digest and cross-platform checkout, never reopened a database written by the immediately previous released checksum.

### 3. Prevention Mechanisms

| Priority | Mechanism | Specific Action | Status |
| --- | --- | --- | --- |
| P0 | Compatibility architecture | Store exact legacy checksum aliases on the owning migration descriptor; new writes remain canonical | DONE |
| P0 | Regression test | Open a real file DB with the published v1 alias and prove no rewrite, backup or data loss | DONE |
| P0 | Destructive-action guard | Only a positively corrupt integrity diagnostic may set `canRebuild=true` | DONE |
| P0 | Typed diagnosis | Classify SQLite `CORRUPT`/`NOTADB` by primary result code, never error text | DONE |
| P1 | Recovery workflow | Preview the immediate startup backup by stable skill ID and require a fresh backup plus explicit approval before merge | DONE (preview); APPLY GATED |

### 4. Systematic Expansion

- **Similar Issues**: every immutable migration checksum, fixture digest and persisted format version can create the same compatibility break if a published value is replaced rather than aliased or migrated.
- **Design Improvement**: integrity diagnosis and authorization for destructive recovery are separate contracts; file existence is not evidence that rebuild is appropriate.
- **Process Improvement**: checksum normalization changes require an N-1 production metadata fixture, not only a current-source hash test.
- **Knowledge Gap**: a startup backup proves recoverability of bytes, not semantic restoration into the new database; scanner can reconstruct files but not DB-only provenance, projects, tags, settings or operation history.

### 5. Knowledge Capture

- [x] Updated `database-migrations.md` with published alias and N-1 fixture rules.
- [x] Updated `startup-recovery.md` with positive corruption authorization.
- [x] Added `startup-rebuild-provenance-loss.md` and a read-only preview tool.
- [x] Kept real DB apply behind a separate user approval gate.
