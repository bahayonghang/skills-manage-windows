# 设计：审计整改任务树

## 任务边界

父任务是只读需求源与集成门，不承载产品实现。子任务按可独立验证的 trusted boundary 拆分：

```text
P0 github-import-fs-db-atomicity
P1 trellis-path-security
P1 windows-release-signing ──> P2 windows-installer-verification
P1 collection-search-correctness
P1 usage-refresh-failure-integrity
P2 subagent-runtime-resilience
P2 generated-schema-evidence
P2 dependency-audit-observability
P2 frontend-boundary-cleanup
P2 backend-boundary-ratchets
P2 typed-ipc-remainder
```

## Cross-Task Contracts

- GitHub import 与 usage 修复不得改变 target identity、SecretStore 或 Central mutation lock 的既有语义。
- Windows 签名顺序以 `compile exe → Authenticode exe → bundle → Authenticode installer → updater sign → metadata` 为唯一权威链；安装 smoke 只消费该链的最终资产。
- Trellis 安全任务共享 canonical containment 语义，但 path security 与 runtime resilience 分别验证“允许读写什么”和“允许消耗多少/运行多久”。
- 前后端边界整改不得制造新的兼容层；Typed IPC 继续以现有 Rust codegen 和 `@/lib/ipc` adapter 为权威。

## Requirement Mechanisms

| Requirement | Mechanism |
| --- | --- |
| R1 | finding ledger 与 child PRD 的 finding/R/AC 追溯表构成唯一责任映射；父集成时逐 ID 复核。 |
| R2 | 上述显式优先级 DAG；installer 最终验收只消费 release-signing 产出的最终资产。 |
| R3 | child 设计必须声明 trusted boundary、故障测试和 rollback point；恢复错误不得被降级为成功。 |
| R4 | 全部成员维持 `planning`，本父任务仅写规划和 review 证据。 |
| R5 | child 与父报告分别保留自动化、人工和外部证据边界，未执行项统一标为 `UNVERIFIED`。 |

## Integration And Rollback

- 每个 child 独立分支/提交/归档；父任务不合并未通过自身检查的 child。
- 若后续 child 发现 finding 前提失效，回到 planning 并在 ledger 中记录 `not reproducible` 及证据，不删除原 finding。
- Critical/High 回滚必须恢复到任务开始前的数据与构建边界，不能只保证编译通过。
