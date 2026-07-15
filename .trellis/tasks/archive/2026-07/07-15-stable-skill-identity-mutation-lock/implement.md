# 实施计划

## 1. Schema And Identity

- [x] 在 `db/schema/core.rs` 增加 `skills.uid` migration、backfill、唯一索引和完整性校验。
- [x] 更新 `db::Skill`、row mapping、fixtures 与 insert/upsert repository；upsert 不覆盖既有 `uid`。
- [x] 更新 scanner/import/update/relocation/portable-state 路径，证明同实体 `uid` 保留。
- [x] 增加单一 `SkillRef` resolver 与 typed ambiguity/not-found error。
- [x] 增量更新 backend/frontend/portable-state DTO；旧 JSON/manifest 保持兼容。

验证：

```powershell
cd src-tauri; rtk cargo test db
cd src-tauri; rtk cargo test scanner
cd src-tauri; rtk cargo test portable_state
```

## 2. Mutation Lock Infrastructure

- [x] 在 `paths.rs` 定义 lock path，禁止散落 `.skillsmanage` 字面量。
- [x] 增加 typed `CentralMutationGuard`、有界 backoff/timeout 和 async blocking wrapper。
- [x] 增加独立 helper process integration tests：竞争、释放、timeout、crash release。
- [x] 通过 structured tracing 记录 operation 名称与等待时间，不记录 secret、用户名或完整敏感路径。

## 3. Mutation Entry Integration

- [x] 盘点 Local 中央写入口并形成测试锁定清单。
- [x] 接入 GitHub/skills.sh import final apply。
- [x] 接入 installation centralize/install。
- [x] 接入 central update/delete。
- [x] 接入 portable-state import 与 central-store relocation。
- [x] 将 network/preview/plan 保持在锁外，在锁内重读状态并拒绝 stale plan。
- [x] 消除 nested lock acquisition；锁只由顶层 final-apply use case 获取。

定向验证：

```powershell
cd src-tauri; rtk cargo test github_import
cd src-tauri; rtk cargo test marketplace
cd src-tauri; rtk cargo test installation
cd src-tauri; rtk cargo test central_updates
cd src-tauri; rtk cargo test central_skills
cd src-tauri; rtk cargo test central_store_location
```

## 4. Frontend Compatibility

- [x] TypeScript DTO 增加向后兼容的 optional `uid`，展示/路径继续使用现有 `id`/name。
- [x] URL、selection、fixtures 和 tests 不误用 `uid` 拼目录。
- [x] Busy/timeout typed error 经既有 IPC string boundary 与 async failure feedback 传播，不新增硬编码前端文案。

## 5. Full Gate And Rollback

```powershell
rtk pnpm typecheck
rtk pnpm lint
rtk pnpm test -- --run
rtk just ci
```

- [x] `rg` 确认新增路径字面量只在 `paths.rs`/测试白名单。
- [x] `git diff --check` 通过。
- [x] 回滚验证：前端 optional `uid` 保持旧载荷兼容；锁失败时 mutation 停止、查询不取锁。

## 6. Handoff To CLI Child

- [x] 固化并记录 `SkillRef`、CLI-facing DTO、mutation guard 的公开 façade。
- [x] 本任务验收后才允许 `07-15-shared-core-cli` 的 install/sync 命令进入完成状态。

## 7. Verification Evidence (2026-07-15)

- `rtk just ci`: 123 frontend files / 1346 tests passed; Rust clippy and 776 tests passed.
- Cross-process lock test passed on Windows, including raw lock-violation classification and crash release.
- `rtk cargo check --locked`, `rtk git diff --check`, and all affected Rust domain suites passed.
