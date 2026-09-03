# Design: 风险导向测试覆盖补强（第二轮）

## Architecture And Boundaries

本任务沿生产责任边界补测，不建立独立 coverage framework：

1. Rust service/database tests 验证 Central 迁根、`ensure_centralized`、SSH/WSL create/update 和 target ID 的文件系统、SQLite、SecretStore 与路径边界结果。
2. React/Zustand store tests 通过现有 IPC mock 验证元数据/review、PAT、Central install/delete 和 Update Center apply 的失败、loading、rethrow、secret 隔离与 reload-required。
3. Release 脚本测试在 `src/test/scripts/` 用 owned temp directory 直接 `import()` 生产者模块，使空签名和重复 NSIS 在生成时失败，而不是只靠下游 preflight。

## Contracts Reused

- 后端：`.trellis/spec/backend/transactional-mutations.md` 的 validation-before-write 与单一顶层事务；失败后 FS+DB 收敛到调用前或完整新状态。
- 凭据：`.trellis/spec/backend/settings-domain-boundary.md` 对删除已要求 settings+credential+pool 一起回滚；创建/更新采用同一结果契约。
- 路径：target ID 字符类必须与 `sanitize_target_id` / `remote_cache_db_path` 对齐，非法 ID 不得在 cache 根外建目录。
- 前端：沿用第一轮 `targetStore.requiresTargetReload`。Central store 增加 `requiresCentralReload`；Update Center 增加 `requiresInventoryReload`。mutation 成功、refresh 失败时置位该标志、清 loading、写 error、rethrow。
- 凭据 renderer：`.trellis/spec/frontend/renderer-authority-boundary.md` — PAT 只写不读；递归 sentinel 检查。
- 发布：空 `.sig` 与重复 NSIS 与 `release-preflight.mjs` 一样 fail-closed。

## Test Seams

### Central relocate

- 使用 owned temp 源根/目标根 + `mem_pool`/`file_pool` 种子。
- SQLite trigger 分别拒绝 `UPDATE agents SET global_skills_dir` 和后续 `skills` 路径更新。
- 路径改写改为“等于旧根或旧根 + 分隔符”的前缀，而不是无边界 SQL `REPLACE`。
- 四个 path-bearing `UPDATE` 放进同一个顶层事务。
- DB 失败后删除本次新建的目标技能目录；不尝试还原覆盖前的同名目标内容。
- 断言完整路径列快照、源字节、target-only 字节；去掉 trigger 后重试成功。

### ensure_centralized

- 种子：`is_central=false`，`file_path` 在 Central 外，canonical 目标尚不存在。
- Trigger 拒绝 `upsert_skill`（或等价 skills 写）。
- 成功复制后的 upsert 失败必须补偿删除刚创建的 canonical 目录。
- 存在 `SKILL.md` 但 `is_central=false` 时不得 early-return；retry 必须修复行。
- 再经 Local install 公共入口证明不能成功返回非 Central 行。

### SSH/WSL create/update

- 从 `create_*_impl` / `update_*_impl` 抽出 probe 成功之后的 persist+credential+cache helper（`pub(crate)`），测试直接调用 helper，不走 live `probe_ssh_target`。
- `MemoryCredentialBackend`；settings trigger 拒绝 `ssh_targets_v1` / `wsl_targets_v1`。
- `remote_db` 失败用可注入的 registry/cache 打开失败，或测试 helper 接受可选 cache-init 闭包；失败后回滚列表与凭据。
- 密码只出现在 SecretStore；settings JSON 与错误文本不含 sentinel。
- 若 `commands.rs` 将超过 800 行，把 helper 放到 `targets/` 下的 sibling 文件。

### Target ID parity

- 表测非法与合法 ID。
- 非法 ID 走 quarantine 或 load-time reject，断言无 cache 目录落在 `app_data/targets/` 之外。
- 同一输入矩阵比较 `validate_target_ids` 与 `sanitize_target_id`。

### Zustand metadata / install / PAT / Update Center

- 复用 `src/test/support` 的命令路由 IPC mock 与 deferred promise。
- 空输入断言零 `invoke`。
- 首命令 reject：loading 清、error 写、rethrow、列表/reviews 保持。
- mutation resolve + refresh reject：置 reload-required，不清空已提交语义。
- PAT：递归遍历 store JSON 与 `String(error)` 确认 sentinel 缺席。

### Release producers

- 与 `releaseArtifacts.test.ts` 相同的 ESM `import()`。
- 自有 temp `--asset-dir`；不写仓库内 `release-assets/`。
- `findAsset` 在 0 或 >1 个匹配时 throw；`readSignature` 在缺失或 trim 后为空时 throw。
- `prepare-release-body` 用临时 notes 目录覆盖精确/系列/fallback。

## Compatibility And Trade-offs

- 不新增依赖和 coverage 配置。
- 测试放在现有拥有者模块；release 新文件为 `src/test/scripts/releaseMetadataGeneration.test.ts`。
- 不为 connection-test / sync apply 的 `Ok` 载荷写 redaction 快照。
- 不为短 mutation 引入 jobId/generation 机制。
- 迁根覆盖恢复留给 journal 任务，避免在本轮引入完整 FS saga。

## Rollback

- 无 schema migration、无新依赖、无公共 IPC 命令签名变更（reload-required 只增加 store 字段）。
- 回滚以模块级测试/最小修复为单位。
- 若某不变量需要跨模块架构重写（例如迁根 backup/journal），停止该项并记为独立缺陷，不以弱化测试完成。

## Deferred Evidence

- 没有数值 line/branch coverage 结论。
- 没有真实 SSH、系统 keyring、provider、原生 GUI、Windows symlink 特权或发布环境验证。
