# Design — 全项目依赖审计与分批升级

## 1. Boundary

本任务把依赖面分成四个相互关联但独立回滚的域：

```text
pnpm manifest + lock ─┐
Cargo manifest + lock ├─> just ci + just audit ─> Windows bundle smoke
toolchain pins        │
GitHub Action SHAs ───┘
```

只有 Git 跟踪的根依赖文件属于写入范围。`ref/` 等本地参考仓库即使含 manifest，也不进入 SkillPort 的 package manager、CI 或 release graph。

## 2. Risk Model

| 等级 | 判定 | 典型变更 |
| --- | --- | --- |
| Low | 同一 API 线的安全/patch 更新，调用面不变且可精确回滚 | vulnerable transitive refresh、Tauri patch、Rust unsound/yanked patch |
| Medium | 同 major 的行为/资源变化，或工具链/hosted Action major 但业务 API 不变 | UI/font minor、Rust 1.98、pnpm 10.x、Actions runtime major、VitePress 的 Vite 6 override |
| High | persistence、credential、network、archive 或 compiler/test-runtime Breaking Change | SQLx 0.9、keyring 4、reqwest 0.13、zip 8、TypeScript 7、jsdom 30 |

SemVer 只是初始分类，实际风险由项目调用面覆盖。例如 `zip 2 -> 8` 虽然编译迁移可能很小，但它处理不可信本地归档，因此按高风险验收。

## 3. Batch Transaction

每批遵循同一状态机：

```text
snapshot diff
  -> upgrade one dependency family
  -> inspect manifest/lock graph
  -> focused checks
  -> just ci
  -> just audit
  -> stable: record evidence and continue
  -> failure: diagnose + smallest same-batch fix + rerun focused/full gates
```

不使用 `git reset --hard`、`git checkout --` 或覆盖用户改动回滚。批次开始前记录精确文件集合；若需回退，只用 `apply_patch` 撤销本批已知行或包管理器的精确反向更新。

## 4. Compatibility Contracts

### npm / frontend

- React/DOM、types、Testing Library、Vitest/jsdom 必须保持 peer dependency 一致。
- Tauri JS API/plugin 与 Cargo plugin 作为一个兼容组审阅。
- `@lobehub/icons` 使用 10 个深路径 Mono component import；升级后至少运行类型检查、相关 icon/agent contract tests、生产 build，并观察 lock 中 `@lobehub/ui -> mermaid -> dompurify` 是否收敛。
- VitePress 1.6.4 是稳定最新版；不采用 v2 alpha。若 Vite 5 的 high advisory 无 5.x 修复，使用仅限 `vitepress>vite` 的 Vite 6 override 候选，并以 `pnpm docs:site:build` 和完整 CI 证明兼容，不能用全局 Vite downgrade/override。

### Cargo / backend

- Tauri family patch/minor 一起更新并保留 Windows-first bundle surface。
- `rkyv` / `rsa` advisory 先用 `cargo tree --invert ... --target all` 复核可达性；例外原因必须描述真实 lock-only 边界，不沿用过期说明。
- `sha2` 输出必须与现有 checksum fixture 字节一致。
- `zip` 必须保留 Stored/Deflated allowlist、路径规范化、大小/数量预算与 hostile archive 拒绝。
- `reqwest` 必须保留 no-proxy、redirect authority、Bearer 隔离、错误分类与请求上限。
- `keyring` 必须继续通过 `SecretStore` 边界，禁止明文回退扩大；真实 OS credential read-back 保持 `UNVERIFIED`，除非执行了对应手工验证。
- `sqlx` 必须保留 SQLite-only root feature、迁移 checksum、foreign-key validation、恢复日志与 pool 初始化。0.9 的 `SqlSafeStr` 会影响动态 query 字符串，必须逐个审阅而非机械包装所有 SQL。

### Toolchain / Actions

- Node 保持 26.x；仓库只声明 major，CI 会解析当前 patch。
- pnpm 先留在 10.x 最新线；11.x 延后为独立工具链迁移。
- Rust 1.97 -> 1.98 同时更新 `rust-toolchain.toml`、workflow rust-toolchain SHA/comment 与 developer-experience contract。
- 外部 Actions 保持 full SHA。checkout v7 的 fork 安全默认值、artifact v7/v8 与 Azure signing major 必须按工作流触发类型逐项审查；本地 contract test 不能替代 hosted execution。

## 5. Security Treatment

- `just audit` 的 blocking policy 保持：npm production high/critical + every Cargo vulnerability。
- 完整 `pnpm audit` 作为额外可见性门槛，目标是消除可稳定修复的开发 high；低/中 advisory 不通过扩大 production ignore 隐藏。
- 对没有稳定修复或仅存在于不可达 lock closure 的项目，记录 package path、upstream 状态、当前 reachability 与到期复核日期。
- 新例外属于用户风险决策，不由实现阶段自行添加。

## 6. Rollout And Rollback

- 每批只修改列出的 manifest/lock/config/source files；进入下一批前保留通过证据。
- 如果某个 Breaking Change 需要超出最小兼容修复的业务重构，回退该子批并把它拆成后续任务，不阻塞已稳定的低风险批次。
- 最终只生成本地 Windows bundle，不安装、不签名、不发布。GitHub hosted runner 和其他 OS 的行为等待后续授权后的 PR/CI 证据。
