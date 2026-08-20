# 执行计划：永久修复 Update Center 新增导入与 GitHub 失败分类

> 用户已确认实施，任务处于 `in_progress`。代码、spec 与完整 `just ci` 已通过；提交、归档和 journal 等待 Phase 3.4 的一次性提交确认。

实施结果与精确门禁证据见 `research/implementation-verification.md`。实际追加的是 migration 6；migration 5 已由仓库现有 usage cache 占用，migration 1 的 live schema source 保持不变以保护已发布 checksum。

## 阶段 0：固定基线与精确红灯

- [ ] 0.1 复核 active task、`dev...origin/dev` 与 working tree，只把本任务文件纳入实现范围。
- [ ] 0.2 在现有 Central inventory test harness 加入 fake GitHub request recorder：同仓库多个 additions、Refresh 成功、Apply 再次 acquisition 的最小复现。
- [ ] 0.3 增加 cache-cleared + branch-moved 对照，当前实现应证明按 branch重新获取；断言不得只写宽泛 `is_err()`。
- [ ] 0.4 加入 `GitHubAccessDenial.used_auth=true` 经 `from_denial` 后 code 变成 generic access-denied 的精确红灯。

定向验证（测试名以实现时最终命名为准）：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  apply_remote_additions_reuses_refresh_snapshot -- --nocapture
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  configured_github_denial_keeps_auth_context -- --nocapture
```

审查关口：失败证据必须分别指向二次 acquisition 和 `used_auth` 丢失，不能由 fixture、真实网络或本机 token 状态造成。

## 阶段 1：保留 typed auth context

- [ ] 1.1 调整 `GithubImportError` 的 access-denied 表示，让 `from_denial` 保留 `used_auth`；修正所有构造点和模式匹配。
- [ ] 1.2 由 typed variant直接映射 `rate_limited`、`access_denied`、`configured_token_failed`，不解析 Display 字符串。
- [ ] 1.3 让 Update Center item failure使用 fixed public message/code/category，移除动态 `error.to_string()` 的用户/日志边界。
- [ ] 1.4 增加 401/403/429 × auth absent/present、redaction、retryability/diagnostic tests；保留必要 legacy envelope 兼容。

验证：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked github_import::error
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked skill_update_apply_failure
```

## 阶段 2：Pinned snapshot acquisition 与 cache

- [ ] 2.1 将 Central update snapshot entry 扩展为 display repo + full commit SHA + repository digest + snapshot bytes。
- [ ] 2.2 Refresh miss 路径先解析 full SHA，再对 pinned ref 使用现有 bounded tree/archive acquisition；同 repo 并发去重、progress、retry和partial failure保持。
- [ ] 2.3 Cache key/lookup 同时校验 immutable identity；保持 TTL、LRU、byte budget 和 oversized current-use-only 行为。
- [ ] 2.4 更新 update/relocation/remote-added consumers 读取 snapshot payload，不复制 bytes、不暴露新 public DTO。
- [ ] 2.5 增加一次 refresh 一仓库一次 resolution/acquisition、cache hit、eviction、expiry、oversized 和 digest稳定测试。

验证：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked central_updates::snapshots
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked central_updates::inventory
```

## 阶段 3：Pending provenance migration

- [ ] 3.1 追加下一版 immutable migration，为 pending additions 增加 nullable commit/digest；更新连续 descriptor、locked checksum 与 future-version fixture。
- [ ] 3.2 同步 `SkillRepositoryPendingAddition`、CRUD、UPSERT、schema metadata和测试 fixtures；upsert 原子替换 path 对应 identity。
- [ ] 3.3 Refresh 持久化来自同一 pinned snapshot 的 commit/digest；reload/范围筛选不丢失字段。
- [ ] 3.4 覆盖 current -> new、new DB、旧 NULL、reopen、checksum/gap/future、migration failure rollback。
- [ ] 3.5 运行 `pnpm docs:gen` 并审查只包含预期 schema/architecture generated diff。

验证：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked db::migrations
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked pending_additions
rtk pnpm docs:gen:check
```

## 阶段 4：Repository-level Apply authority

- [ ] 4.1 先按 repository 归并 selections 并从 pending rows读取唯一 identity；legacy NULL、混合 identity、missing row 统一 fail closed。
- [ ] 4.2 Local 优先复用 exact cache entry；miss 时只按 full SHA 重取一次并校验 repository digest，随后走 snapshot-only importer。
- [ ] 4.3 SSH / WSL 新增窄 helper：按 full SHA 创建 remote workspace、校验 manifest digest、调用 workspace-only importer并可靠 cleanup。
- [ ] 4.4 在 repository mutation 前验证 selected candidates；成功后才删除 imported pending rows，失败 repository 保留。
- [ ] 4.5 保留 overwrite/rename/skip、Central mutation lock、per-skill commit/content provenance和不同 repository partial success。
- [ ] 4.6 加入 fresh-cache 零请求、cache miss pinned单请求、branch moved、digest mismatch、selection gone、legacy NULL、partial success、Local/SSH/WSL parity tests。

验证：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  apply_remote_additions -- --nocapture
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked \
  remote_workspace -- --nocapture
```

审查关口：用 request recorder/FakeRunner证明 Apply 不解析 branch；不能仅凭最终文件内容推断没有二次 acquisition。

## 阶段 5：前端文案与静态错误契约

- [ ] 5.1 为 refresh-required/snapshot-changed 和三种 GitHub denial code同步英文、中文 backend error 文案。
- [ ] 5.2 Update Center toast只按 stable code格式化；匿名拒绝不声称已经使用 token，configured-token failure 明确令牌已被使用。
- [ ] 5.3 补 backend-error/i18n tests，覆盖 item failure envelope 和 legacy compatibility；审计无 raw detail fallback。
- [ ] 5.4 运行 typecheck、lint 和受影响 Vitest；若 TypeScript IPC generated types实际变化，运行 codegen并审查产物，否则保持 generated map不变。

验证：

```bash
rtk pnpm test -- src/test/lib/backendError.test.ts
rtk pnpm typecheck
rtk pnpm lint
rtk pnpm ipc:codegen:check
```

## 阶段 6：Spec、全量门禁与安全审计

- [ ] 6.1 仅把已实现且有测试证明的不变量更新到 snapshot、inventory、redaction、domain error和migration specs。
- [ ] 6.2 审计 token/header、repo URL、source path、HTTP body、workspace/local path不会进入 IPC、日志、DB provenance或portable export。
- [ ] 6.3 运行 Rust fmt、全 targets Clippy、locked tests、generated docs/codegen checks和仓库 `just ci`；`just audit` 作为供应链回归。
- [ ] 6.4 检查最终 diff、迁移 immutable checksum、Windows-safe path/shell行为和用户已有改动。

验证：

```bash
rtk cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
rtk cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked
rtk pnpm docs:gen:check
rtk pnpm ipc:codegen:check
rtk just ci
rtk just audit
rtk git diff --check
```

## 阶段 7：完成与交付

- [ ] 7.1 记录精确验证结果和任何未验证的真实 Windows/GitHub/SSH/WSL 外部证据。
- [ ] 7.2 按 Phase 3 更新必要 spec，并用 `$git-commit` 流程组织原子提交；不创建额外 Trellis commit任务。
- [ ] 7.3 归档任务并写 workspace journal；不在未授权时 push、创建 PR 或发布 installer。
- [ ] 7.4 交付说明区分：已配置 token解决认证前提；代码修复解决二次获取、分支一致性和错误误导。

## 回滚点

- Typed auth error、pinned snapshot、migration、Apply authority和frontend文案分别保持可审查边界；任一阶段无法通过对应测试时停止在该阶段修正。
- Migration 一旦应用只向前保留 nullable 列，不做 destructive down migration。
- 回滚 Apply authority 时不得恢复 cache miss 后按 branch静默导入；临时降级只能 fail closed并要求 Refresh。
- 不清理用户 live inventory、Central 目录或真实凭据；所有测试 mutation局限于内存 DB/临时目录。
