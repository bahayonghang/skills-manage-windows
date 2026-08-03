# 修复 Marketplace 安装的 Central 一致性与路径边界

## Goal

删除 registry-backed Marketplace 的直接 URL -> display name path -> 单文件写入旁路，让 Marketplace 与 skills.sh/GitHub import 共用同一个受控 Central 安装 use case。修复任意路径写入，同时保证完整目录、数据库 skill row、repository provenance、锁、journal 和 Local/SSH/WSL 行为一致。

## Evidence

- `services/marketplace/mod.rs:475-476,525-551` 把远端 frontmatter `name` 直接拼进 Local/remote Central 路径。
- `services/github_import/source.rs:247-278` 证明该 name 来自远端 YAML，且没有单一路径组件约束。
- `services/marketplace/mod.rs:502-523` 信任缓存 `download_url` 并完整读取 body；`:555-559` 只更新 Marketplace 标记。
- `docs/architecture/marketplace-pipeline.md:38-53` 描述的完整目录、skill upsert、`ensure_centralized` 与当前实现不符。
- `services/marketplace/skills_sh.rs:243-309` 已提供可复用的 snapshot/candidate/import 形状。

## Requirements

1. `skill.name` 只用于展示；不得参与 Local `Path::join`、remote `remote_join`、shell 参数或目标目录命名。
2. install 以 `registry_id + marketplace skill id` 为稳定输入，从 registry 的受控 GitHub source 重新获取并固定一个 commit/snapshot；缓存 `download_url` 不再是 backend request authority。
3. 使用与 sync 相同的 candidate mapper，在新 snapshot 中匹配恰好一个 requested marketplace id。缺失、重复/歧义或 registry 已变化时，在 Central mutation 前 fail closed。
4. Local 安装复用 GitHub snapshot import；SSH/WSL 使用同一 candidate/selection 与 pinned repository identity，经现有 remote import/transport 执行。三种 target 都安装 candidate 的完整目录，不只写 `SKILL.md`。
5. final apply 必须复用 Central mutation lock、pending operation recovery、FS+DB journal、skill upsert 和 per-skill repository provenance；不得在 Marketplace service 重新实现这些步骤。经实施复核批准，现有 `central_update` journal 语义泛化为 Central 内容 upsert：首次导入使用 `UpdateManifest(had_target=false)`，不新增 schema/operation kind，并在同一 DB transaction 中提交 skill、repository provenance 与 `db_committed`。
6. 保持既有 overwrite 行为，但通过现有 duplicate resolution 和可恢复 swap 实现；不得直接覆盖目标文件。
7. `marketplace_skills.is_installed` 是派生 cache，不是安装事实 authority。只有完整 import 成功后才能更新/重算；任何 acquisition、validation 或 import 失败不得把它设为 true。若 cache marker 更新失败，不能把已经成功的 Central import伪装成未发生，后续 query/sync 必须能从 live Central 状态重建。
8. 保持现有 IPC command 名和用户可见成功流程；新增错误走 `MarketplaceError` -> stable `IpcError`，动态 name/path/URL/token 不进入公共错误或日志。
9. 更新 `docs/architecture/marketplace-pipeline.md` 为真实终态，并补结构测试防止 direct writer/URL downloader 回归。

## Acceptance Criteria

- [x] `central_skill_dir_for_name` 和 registry-backed direct `reqwest`/`std::fs::write`/remote `write_file` 安装路径从生产代码删除。
- [x] 表驱动测试覆盖 `../escape`、`/absolute`、Windows drive/UNC、slash/backslash、`.`/`..`、Unicode display name；所有恶意 name 都不能影响最终目录或写出 Central root。
- [x] 正常多文件 candidate 的 `SKILL.md`、references/scripts/assets peers 在 Local、Fake SSH、Fake WSL 结果一致。
- [x] 安装成功后存在 Central skill row、canonical path、repository/source path 和 commit/digest provenance；Marketplace query 显示 installed。
- [x] acquisition failure、candidate mismatch、duplicate ambiguity、lock busy、FS stage/swap failure和 DB upsert failure均不提前写 installed marker；journal/retry 语义符合现有 contract。Marketplace 定向测试覆盖 acquisition/identity/DB/marker，复用的 `central_mutation`、`central_updates::fs` 和 `batch_tests` 覆盖 lease contention、stage/swap/rollback/recovery。
- [x] 缓存 marker 故障注入不会回滚或误报已经提交的 Central 安装；下一次 query/sync 可从 live state 修复派生值。
- [x] structural test 证明 registry-backed install 只能进入共享 import use case，`download_url` 仅为 DTO/cache 字段。
- [x] `cargo test marketplace --locked`、`cargo test github_import --locked`、Rust fmt、all-targets locked Clippy、locked Rust tests和 `just ci` 通过。实测 Marketplace 22/22、GitHub import 137/137、完整 Rust 1056 passed/6 ignored、Node 22.23.2 下 `just ci` 通过。

## Non-Goals

- 不在本任务重做 Marketplace registry sync transaction/stale-row 清理；归 `08-03-transactional-metadata-mutations`。
- 不在本任务建立全局 HTTP gateway；被删除的 direct downloader 不需要“先加固再删除”。
- 不改变 frontend Marketplace 浏览、search/filter 或 skills.sh 产品语义。

## Dependency

本任务是父任务的第一实施项。`08-03-bounded-external-text-ingestion` 对 Marketplace 的最终范围必须以本任务完成后的代码为准。
