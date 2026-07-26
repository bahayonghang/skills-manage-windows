# 实施计划：Skill 删除事务与 orphan 修复

## 1. 激活与规范

- [ ] `python ./.trellis/scripts/task.py start 07-24-db-stale-cleanup-fix`
- [ ] 加载 `trellis-before-dev`，阅读 backend index、test-support、best-effort writes 与 DB repo conventions。
- [ ] 保护现有 Trellis runtime/tooling 和其他任务规划改动。

## 2. 集中关系清单与事务 helper

- [ ] 定义 7 张拥有型 relation spec 常量，附 ownership 注释与 observation/历史/项目表排除说明。
- [ ] 实现 transaction-scoped 单 ID、keep IDs、scanner keep-table 删除 helpers。
- [ ] 提取 transaction-scoped repository prune，保留现有 pool API 兼容调用者。
- [ ] 将 `delete_skill` 与空/非空 `delete_skills_not_in_scope` 收敛到 begin/commit 单事务。
- [ ] 将 scanner stale relation cleanup 改为复用同一 relation spec/helper。

## 3. Startup orphan repair

- [ ] 定义可序列化、稳定排序、无路径/内容的 `OrphanRepairReport`。
- [ ] 增加 transaction-scoped operation log insert；同一 transaction 中执行 inventory、JSON encode、audit insert、delete、commit，失败整体回滚。
- [ ] 在 schema init 后、seed 前调用；非空报告持久化 category=`database`/action=`orphan_repair`，桌面端可额外 tracing 摘要，零报告不制造噪声。
- [ ] 保证 local、CLI 与 lazy remote-cache DB 都经过同一 init repair。

## 4. 回归与 fault injection

- [ ] 单删测试覆盖 7 张拥有型关系和 repository prune。
- [ ] `delete_skills_not_in_scope` 空 keep-set 与非空 keep-set测试覆盖全部关系。
- [ ] scanner full scan stale cleanup 测试覆盖原遗漏的 collection/review/explanation，并证明 observation 只按 touched-agent keep-set 清理、不进入全局 skill cascade。
- [ ] trigger abort 测试分别证明 audit insert/中间 relation 失败时 log/parent/relation/repository 全部回滚。
- [ ] startup repair fixture插入 7 表 orphan，断言 report JSON、operation log、stable ordering、清理、幂等第二次运行。
- [ ] ID reuse测试证明不继承旧 metadata，同时 observation/project/usage历史不被误删。

## 5. 验证梯度

- [ ] `cd src-tauri; cargo test db:: --locked`
- [ ] `cd src-tauri; cargo test scanner --locked`
- [ ] `cd src-tauri; cargo fmt --all -- --check`
- [ ] `cd src-tauri; cargo clippy --all-targets --locked -- -D warnings`
- [ ] `cd src-tauri; cargo test --locked`
- [ ] `just ci`

## 6. Spec、审计与收尾

- [ ] 新增 `skill-deletion-integrity.md`，记录 ownership list、transaction 与 orphan repair contract。
- [ ] 核对 `db-schema-versioning-fk` 的 whole-DB backup 在最终启动顺序中先于 repair/migration；本任务不提前实现该备份。
- [ ] `rg` 对照 schema 中所有 `skill_id`/`resolved_skill_id`，确认每个表已被拥有型 cascade 或有明确排除理由。
- [ ] 运行 `trellis-check`，检查 diff 未包含 FK/schema versioning 或其他子任务产品改动。
- [ ] 提交工作、归档本子任务，在父任务登记完成并解锁 `db-schema-versioning-fk`。
