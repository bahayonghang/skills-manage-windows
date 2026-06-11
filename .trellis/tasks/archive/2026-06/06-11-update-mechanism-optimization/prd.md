# 优化更新机制重构

## Goal

重构 Central skill 更新机制，让“检查/刷新”和“应用变更”在产品语义、数据库状态、Tauri command、前端 Update Center、文档和测试中保持一致。

用户价值：

- 用户更新远端 skill 仓库后，手动刷新能真实检查远端最新内容，不被不可见缓存误导。
- 用户能明确看到为什么某个 skill 没有更新：远端 repo/branch/path、快照时间、cache 状态、本地基线 hash、远端 hash、version 元数据。
- “清空检查结果”只清空待处理 inventory，不影响已安装技能或成功应用后的基线状态。
- 没有 `version` 的 skill 仍能通过内容树 hash 检测更新；`version` 不再被误解为更新判断的唯一依据。
- 当普通检测链路仍然异常时，用户有一个明确、可确认的“强制更新/强制镜像同步”救援路径，可以绕过 inventory 判断，直接从远端 GitHub repo 拉取、覆盖已跟踪技能、导入远端新增，并删除远端已缺失的本地跟踪技能。

## Requirements

### Confirmed facts from repository evidence

- Update Center 文档声明 refresh 是 read-only，写入 inventory table，不改磁盘技能文件；apply 才执行实际 mutation。
- 现有计划 `plans/update-mechanism-overhaul-plan.md` 已明确要求 `refresh` 不写 `skill_update_states`，推荐新增 inventory 表。
- 当前 `refresh_skill_update_inventory_impl` 仍会对每个 checked skill 调用 `upsert_skill_update_state`，导致 refresh 写入旧状态表。
- 当前 `get_skill_update_inventory_impl_scoped` 从 `skill_update_states` 读取 `update_available` / `remote_missing`，而 remote additions 来自 `skill_repository_pending_additions`。
- 当前 `clear_skill_update_inventory_impl` 只清 `skill_repository_pending_additions`，不清 `skill_update_states`，因此清空 inventory 不能保证清掉 updatable / remote_missing。
- 当前 GitHub snapshot cache TTL 是 10 分钟；`prepare_snapshots_for_repo_refs` 优先使用 fresh cache，refresh scope 没有显式 bypass cache 参数。
- 当前 `update_central_skills` 已能从远端 snapshot 写回 Central skill，但当 `remote_hash == local_hash` 时会跳过；强制更新需要绕过这个 skip 分支。
- 当前 `update_one_skill` 使用 `write_skill_dir_atomic` 替换 Central skill 目录，并刷新 copy 类型平台安装；这可以作为强制覆盖的安全写入内核。
- 当前 GitHub import 支持 `DuplicateResolution::Overwrite`，并有 staging/backup/restore 逻辑；但它面向新增/导入流程，不应直接替代已跟踪 skill 的强制更新路径。
- 当前 repository sync apply 已经能组合 keep/delete/import 决策；强制镜像同步应复用删除和导入内核，但不要复用需要用户逐项勾选的旧预览决策模型。
- 当前删除 Central skill 会删除 Central 目录、DB skill、symlink/native 安装；copy 安装只会在 `remove_agent_ids` 被传入时删除，否则会保留为孤立 copy。
- 前端新 Update Center store 已使用 `refresh_skill_update_inventory` / `apply_skill_update_decisions` / `clear_skill_update_inventory` / `get_skill_update_inventory`。
- 前端旧 update slice 仍调用 deprecated `check_central_skill_updates` / `check_central_repository_sync` / `apply_central_repository_sync`。
- docs 中写明旧 commands 为兼容性保留，并计划后续 minor release 后移除。

### Functional requirements

- Manual Update Center refresh must be able to bypass the in-memory GitHub snapshot cache.
- User-triggered manual refresh must bypass snapshot cache by default.
- Refresh must not update installed/baseline update state in `skill_update_states`.
- Refresh must persist checked results into an inventory-owned persistence model, including existing-skill rows such as updatable and remote missing.
- Clear inventory must clear every persisted inventory bucket for the selected scope, including updatable, remote missing, remote added, and repository failures.
- Apply must be the only path that updates installed/baseline state after successful user decisions.
- Force update mode must be available as an explicit rescue action separate from normal refresh/apply.
- Force update mode must bypass snapshot cache and bypass the “already up to date” hash skip, then atomically overwrite selected tracked Central skill directories from their assigned GitHub source paths.
- Force update mode must have two explicit scopes:
  - skill force overwrite: overwrite selected tracked GitHub Central skills only.
  - repository force mirror sync: overwrite tracked skills, import remote-added skills, and delete local tracked skills whose source paths are missing from remote.
- Repository force mirror sync must be confirmable and must show counts for overwrite/import/delete before applying.
- Force overwrite must update baseline state after successful overwrite and refresh linked copy installations using the same safety rules as normal update.
- Force mirror sync must report per-skill/per-path results, including overwritten, imported, deleted, skipped unsupported, invalid remote candidate, and failed.
- Update detection must remain content-hash based. `version` may be displayed as metadata/diagnostic information but must not be the primary update detector.
- Inventory rows must preserve enough diagnostics to answer “why was no update detected?” for a GitHub-backed skill.
- Frontend actions shown to users should route through Update Center semantics; legacy flows may remain internally only for compatibility during the migration.
- User-visible copy changes must be reflected in both English and Chinese i18n resources.
- Documentation must be updated to match the implemented behavior.

### Non-functional requirements

- Preserve Windows-first behavior and existing Tauri desktop workflows.
- Keep changes surgical: reuse existing update/import/delete/duplicate-removal helpers where possible.
- Avoid hidden destructive behavior: refresh must not modify skill files, delete copies, import additions, or change source assignments.
- Keep migrations idempotent using the repository's existing SQLite schema style.
- Maintain backward compatibility for existing databases with `skill_update_states` rows.
- Keep legacy commands available unless explicitly removed by a separate release/migration decision.

## Acceptance Criteria

- [ ] Manual refresh can detect a just-pushed remote content change without waiting for the 10-minute snapshot TTL.
- [ ] A backend test proves `refresh_skill_update_inventory_impl` does not create or update `skill_update_states`.
- [ ] A backend test proves `clear_skill_update_inventory_impl` clears updatable and remote missing inventory rows as well as remote additions.
- [ ] A backend test proves `apply_skill_update_decisions` updates baseline state only for successfully applied update decisions.
- [ ] A backend test proves force update overwrites a GitHub-tracked Central skill even when remote hash equals local hash.
- [ ] A backend test proves force update bypasses the snapshot cache.
- [ ] A backend test proves repository-level force mirror overwrites tracked skills, imports remote-added skills, and deletes local tracked skills whose source paths are missing remotely.
- [ ] A backend test proves repository-level force mirror does not delete skills from other repositories or unknown/local-only sources.
- [ ] A backend test proves repository-level force mirror deletes all copy installations for remote-missing tracked Central skills.
- [ ] A backend test proves force update refreshes copy installations after successful overwrite.
- [ ] A backend test proves a skill without `version` is detected as updatable when the remote content hash changes.
- [ ] A backend or frontend test proves `version` changes are displayed as metadata but do not override content-hash equality.
- [ ] Frontend Update Center can show source repo/branch/path and refresh/cache/hash diagnostics for inventory rows.
- [ ] Frontend exposes skill force overwrite and repository force mirror as explicit confirmable rescue actions with warning copy, not as the default refresh/apply behavior.
- [ ] Old visible user entry points for checking updates route into the new Update Center flow or are explicitly hidden behind compatibility-only paths.
- [ ] `docs/guide/update-center.md` and `docs/zh/guide/update-center.md` describe the new cache, inventory, clear, and version/hash semantics.
- [ ] Targeted Rust tests for update inventory pass.
- [ ] Targeted frontend tests for Update Center store/components pass.
- [ ] Final implementation passes `just ci`.

## Notes

- This is a complex cross-layer task. It requires `design.md` and `implement.md` before `task.py start`.
- This task should not start implementation until the remaining product/risk decision below is answered or explicitly accepted as recommended.

## Out of Scope

- Making semantic `version` the primary update detector.
- Removing all deprecated backend commands in the same implementation unless approved separately.
- Changing GitHub repository import behavior unrelated to Update Center refresh/apply.
- Using `git clone` / `git pull` as a new transport in the first implementation. The first force mode should reuse the existing GitHub snapshot transport and PAT handling.
- Running force mirror automatically from startup, passive refresh, or normal apply.
- Reworking the entire Central Skills page layout.
- Adding a background auto-update scheduler.

## Open Questions

- Resolved: user-triggered manual refresh should bypass snapshot cache by default, even if that increases GitHub API usage.
- Resolved: repository-level force mode must include overwrite of tracked skills, import of remote-added skills, and deletion of local tracked skills missing from remote.
- Resolved: when force mirror deletes a remote-missing Central skill, it must also remove all copy installations for that skill. This keeps local Central/platform state aligned with the selected remote repository instead of leaving orphaned platform copies.
