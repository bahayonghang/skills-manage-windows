# 全链路日志系统实施计划

状态：**planning ready for final review**。父任务只负责需求、依赖和集成验收；实现由子任务完成。

## Preconditions

- [ ] 用户在本次最终规划总结之后明确批准实施。
- [ ] 保留当前 working tree 中其它 08-26 任务与 Skills CLI 改动，不覆盖、不顺手提交。
- [ ] 每个 child 在 `task.py start` 前单独 validate，并加载自己的 implement/check manifests。
- [ ] 不连接远端、不上传日志、不新增依赖；原生/异常退出验证只在明确实施范围内进行。

## Execution Order

### Stage 1 — Core contracts

1. `08-26-observability-core-contracts`
   - authoritative command policy；
   - observability deep module；
   - Operation ID/lifecycle/repository updates；
   - optional `IpcError.correlationId` 与兼容 adapter；
   - registry parity、redaction、lifecycle tests。

Gate：core focused Rust/frontend contract tests、IPC codegen check、DB old-row compatibility 全部通过。未通过前不启动
任何 coverage/UI child。

### Stage 2 — Parallel domain/runtime coverage

Core gate 通过后，可并行实施：

2. `08-26-audit-central-target-settings`
3. `08-26-audit-catalog-project-obsidian`
4. `08-26-audit-marketplace-import-cli`
5. `08-26-runtime-diagnostics-correlation`

每个 coverage child 必须：

- 对照 policy inventory，仅修改自己的 command/domain ownership；
- 把成功/失败/partial/cancelled 与适用的 started/interrupted 接入统一 interface；
- 把 raw `Display`/ad-hoc details 迁移为 reviewed diagnostic；
- 用内存 DB、fake runner/provider 和 adversarial seeds 验证，不依赖真实远端/凭据。

Stage 2 gate：四个 child 各自 focused tests、typecheck/format/lint 与跨域无重复 action 检查通过。

### Stage 3 — Console

6. `08-26-observability-console-dialog`
   - 依赖 core 与 Runtime child 的稳定 DTO/parser；
   - Operation/Runtime correlation filter/跳转；
   - centered compact detail Dialog；
   - started/interrupted/coded diagnostic/legacy fallback；
   - keyboard/focus/responsive/i18n tests。

Gate：jsdom/DOM contract 通过；Windows 原生视觉与 focus 仍单列证据。

### Stage 4 — Governance and integration

7. `08-26-observability-governance-integration`
   - 比较 policy inventory 与全部 runtime commands；
   - 逐 action 覆盖矩阵、重复/遗漏/敏感字段审计；
   - 更新 runtime observability、Trellis specs、generated IPC/data model docs；
   - 删除 core compatibility adapter 和过时的散点 logger；
   - 全量 gates、Windows smoke、异常退出/stale started 复核。

## Parent Acceptance Trace

| Parent AC | Owning child/gate |
| --- | --- |
| AC1 policy completeness | core + governance |
| AC2 operation coverage/lifecycle | core + three domain coverage children |
| AC3 cross-layer diagnostic parity | three domain children + runtime |
| AC4 privacy/raw Display absence | core + all coverage + governance |
| AC5 operation ID correlation | core + runtime + console |
| AC6 backend Runtime evidence | runtime |
| AC7 best-effort/admin audit | core + central/settings child |
| AC8 centered actionable detail | console |
| AC9 coverage/migration/UI contracts | governance |
| AC10 full CI/native evidence | governance + parent integration review |

## Validation Ladder

### Focused while iterating

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --locked observability
cargo test --manifest-path src-tauri/Cargo.toml --locked operation_log
pnpm exec vitest run src/test/runtime src/test/components/logs src/test/stores/operationLogStore.test.ts
pnpm ipc:codegen:check
```

Each child adds exact domain test filters; any command yielding 0 tests is invalid evidence.

### Cross-layer gates after Stage 2/3

```powershell
cargo fmt --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked
pnpm typecheck
pnpm lint
pnpm test
pnpm sizecheck
pnpm docs:gen:check
pnpm docs:build
git diff --check
```

### Final gate

```powershell
just ci
```

## Manual / Native Evidence

- [ ] Windows Tauri/WebView2：Operation/Runtime 两层按同一 ID 检索与跳转；
- [ ] 100%/125%/150% 缩放及窄窗：详情 Dialog 居中、不溢出、Escape/overlay/Close/focus restore；
- [ ] controlled failure：Toast/inline 的 diagnostic reference 能定位 backend + frontend Runtime evidence；
- [ ] controlled process termination：长任务遗留 started，重启后变 interrupted，不篡改 recovery journal；
- [ ] runtime file rollover/14-day cleanup 与 clear/export admin audit；
- [ ] 未执行的原生/异常退出证据标 `UNVERIFIED`，自动测试不得替代。

## Rollback and Integration Safety

- 子任务使用独占文件/领域 ownership；并行 child 不修改 core interface，变更需求回到 core child复核；
- 若 core interface 失败，先保留 compatibility adapter，停止 coverage 扩展；
- Operation Log 写入始终 best-effort，任何日志故障不得改变业务 return、DB transaction 或 FS/remote mutation；
- 不回填历史日志，不删除用户现有 Operation/Runtime data；
- 最终提交/归档/合并按用户后续授权执行，不自动 push。

## Final Planning Gate

- [ ] Parent PRD 无 blocking question，AC1–AC10 均有 owner。
- [ ] 七个 child 的依赖、PRD、design、implement 与 manifests 完整。
- [ ] Parent/children `task.py validate`、JSON/JSONL、manifest path 与 PRD convergence 通过。
- [ ] 最新规划总结交用户复核；只有后续明确批准才可 start core child。
