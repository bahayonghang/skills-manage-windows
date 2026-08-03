# Journal - lyh (Part 2)

> Continuation from `journal-1.md` (archived at ~2000 lines)
> Started: 2026-07-20

---



## Session 51: 完成内置标签 taxonomy 子任务

**Date**: 2026-07-20
**Task**: 完成内置标签 taxonomy 子任务
**Branch**: `dev`

### Summary

扩充 12 项内置 taxonomy，保护同 id/同名自定义标签，放开 AI 候选并按 usage 控制普通筛选可见性；just ci 全绿并归档 child A。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `52a03268` | (see git log) |
| `9fcd428e` | (see git log) |
| `c6324407` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 52: 完成 AI 新标签 proposal/review 子任务

**Date**: 2026-07-20
**Task**: 完成 AI 新标签 proposal/review 子任务
**Branch**: `dev`

### Summary

实现 AI 优先复用并提议新标签、review-only 持久化、接受时原子创建与复用、跳过零残留、前端 proposal 标识；focused checks 与 just ci 全绿，已归档子任务。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `12d546f6` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 53: 完成标签 taxonomy 与 AI 提议父任务验收

**Date**: 2026-07-20
**Task**: 完成标签 taxonomy 与 AI 提议父任务验收
**Branch**: `dev`

### Summary

核对两个归档子任务与 central-skill-tags spec，完成 custom 同名升级、既有 tag 自动应用、新 tag review-only/接受创建及 UI 显隐的跨子项冒烟；最终 just ci 全绿并归档 parent。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `0196def5` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 54: Dashboard 首页优化：驾驶舱重组 + 三个聚合 IPC + 去玻璃模糊

**Date**: 2026-07-20
**Task**: Dashboard 首页优化：驾驶舱重组 + 三个聚合 IPC + 去玻璃模糊
**Branch**: `dev`

### Summary

按评审后的 Trellis 方案完成 Dashboard 首页优化：删除 Hero 营销块改紧凑状态头；工作队列去 tab 横排 4 项（0 值可见）；Readiness 瘦身；平台迷你条形；Activity 改为后端真实 14 天柱状图（get_daily_operation_counts，本地日+零填充）；新增 get_central_top_tags（is_central 限定）；暴露 get_dashboard_central_summary 并落实三个刷新触发点（挂载/scanGeneration/更新完成回调）；surface-glass 去 backdrop-filter；i18n 删 78 死键；新增 dashboard-data-contract spec。just ci 全绿（前端 1411 + Rust 897），12 张视觉矩阵截图存档任务 research/。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `c2f45ebe` | (see git log) |
| `f31908cf` | (see git log) |
| `b6f1028a` | (see git log) |
| `e813ee9e` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 55: Central Skills 刷新按钮与检查后自动刷新

**Date**: 2026-07-21
**Task**: Central Skills 刷新按钮与检查后自动刷新
**Branch**: `dev`

### Summary

Central Skills 工具栏新增手动刷新按钮（useCentralRefreshButton hook，列表/计数并行刷新互不阻断），更新检查 Start check 成功后自动重取技能列表再打开 Update Center。store 层落地 loadCentralSkills({throwOnError}) 可选 rethrow 契约、isRefreshingList 刷新态与 requestId latest-wins 防护；列表重取失败只报 central.refreshError，不影响 Update Center 打开。经 Codex 规划审阅 6 条意见校验后补齐 design/implement；sizecheck 冻结基线约束下 D6 改为 shell 内 hook 装配、CentralSkillsView 零改动。just ci 全绿，新增 12 个测试用例；spec async-error-feedback 补两条契约。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `f71efc2b` | (see git log) |
| `efc7096e` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 56: 整理测试目录并补齐 Rust 集成测试

**Date**: 2026-07-21
**Task**: 整理测试目录并补齐 Rust 集成测试
**Branch**: `dev`

### Summary

将 127 个前端测试按源码归属迁移到子目录，补充递归串行发现回归测试，并新增 CLI 公共 API 外部 crate 集成契约与共享 fixture；完整 just ci 通过。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `ca97eb07` | (see git log) |
| `2bd4de4` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 57: 完成 GitHub 网络边界与 SSRF 收敛

**Date**: 2026-07-26
**Task**: 完成 GitHub 网络边界与 SSRF 收敛
**Branch**: `dev`

### Summary

将 Markdown 预览改为结构化 repo/path IPC，固定 GitHub endpoint，禁用重定向并增加超时、流式预算与远端 workspace repo 绑定。

### Main Changes

- 移除 renderer 控制的 downloadUrl 权威输入，统一 raw/API 请求构造与 endpoint policy。
- 补齐 remote workspace repo mismatch、SSRF 矩阵、redirect、chunked cap+1 与 PAT mirror 回归。

### Git Commits

| Hash | Message |
|------|---------|
| `35e0c086` | (see git log) |

### Testing

- [OK] just ci

### Status

[OK] **Completed**

### Next Steps

- 实施 07-24-target-context-snapshot。


## Session 58: Close request-scoped TargetContext snapshot

**Date**: 2026-07-26
**Task**: Close request-scoped TargetContext snapshot
**Branch**: `dev`

### Summary

Bound target identity, cache DB, remote resources, events, and operation logs to one owned request context; migrated split-brain command paths and documented the invariant.

### Main Changes

- Added TargetContext plus single-read registry/AppState resolvers and explicit db_for_target selection.
- Migrated commands and GitHub remote workspace helpers away from paired ambient target/DB reads; preserved real SSH/WSL log IDs and labels.
- Added barrier-based Local/SSH-A/SSH-B/WSL regression coverage, architecture grep enforcement, backend spec, and architecture docs.

### Git Commits

| Hash | Message |
|------|---------|
| `0e1f70c4` | (see git log) |

### Testing

- [OK] cargo fmt --all -- --check
- [OK] cargo clippy --all-targets --locked -- -D warnings
- [OK] cargo test --locked (907 passed, 4 ignored; CLI/E2E passed)
- [OK] just ci (web 1425 passed, 1 skipped; Rust and production build passed)

### Status

[OK] **Completed**

### Next Steps

- Continue with 07-24-remote-process-supervisor, which can now consume explicit TargetContext snapshots.


## Session 59: 完成 SSH/WSL 异步进程监督

**Date**: 2026-07-26
**Task**: 完成 SSH/WSL 异步进程监督
**Branch**: `dev`

### Summary

将 SSH、WSL 与 discovery 执行统一迁移到异步 ProcessRunner，增加分级 deadline、50 ms 取消适配、bounded stdout/stderr、Windows Job Object 与 Unix process-group 清理，并迁移 Central batch/remote sync 调用链。补齐真实进程与 FakeRunner 回归，修复并发测试的墙钟阈值脆弱性；Windows just ci 全量通过，Unix cross-check 因 cross-compilation libdbus pkg-config/sysroot 缺失未 live 验证。

### Git Commits

| Hash | Message |
|------|---------|
| `47359025` | (see git log) |

### Status

[OK] **Completed**


## Session 60: 远端路径 canonical 边界

**Date**: 2026-07-26
**Task**: 远端路径 canonical 边界
**Branch**: `dev`

### Summary

为 Central Skills SSH/WSL 文件读取与目录入口增加 canonical containment，允许 root 内符号链接并拒绝 canonical escape；补齐 typed error、NUL 协议、SSH/WSL FakeRunner parity、路径策略规范及完整 CI 验证。

### Git Commits

| Hash | Message |
|------|---------|
| `83e1ba5f` | (see git log) |

### Status

[OK] **Completed**


## Session 61: Renderer 权限最小化与 capability drift check

**Date**: 2026-07-26
**Task**: Renderer 权限最小化与 capability drift check
**Branch**: `dev`

### Summary

移除主 WebView 文件系统权限与明文 secret reveal，将 portability/Marketplace 写盘迁移到 backend，并把 capability 漂移检查纳入 required CI；归档提交 59b773e3。

### Main Changes

- 移除 plugin-fs、HOME/user-dir scopes 和 shell:default，仅保留 shell:allow-open。
- 新增 portability 有界文件适配器、原子导出，以及 Marketplace backend 安装路由。
- 删除 PAT/AI key reveal IPC/UI，新增 capability inventory 的机器契约、确定性表格和 drift gate。

### Git Commits

| Hash | Message |
|------|---------|
| `12b9b248` | (see git log) |

### Testing

- [OK] just ci：frontend 1438 passed / 1 skipped；Rust 926 passed / 5 ignored，integration 3/4/5 passed；typecheck、lint、capabilitycheck、sizecheck、build、fmt、Clippy passed。
- [OK] pnpm tauri build：生成 SkillPort_0.10.14_x64-setup.exe，15137555 bytes，SHA-256 185762B273CF57ADB57E6565FCD9167060E29B0D1D63C84B0D1A17DD5AC00E5。
- [OK] task.py validate、git diff --check、capabilitycheck 和定向 capability Vitest 6/6 通过。

### Status

[OK] **Completed**

### Next Steps

- 继续 07-24-db-stale-cleanup-fix 的 planning review 与单一产品决策门禁。


## Session 62: 完成数据库 stale 清理事务化与 orphan 修复

**Date**: 2026-07-26
**Task**: 完成数据库 stale 清理事务化与 orphan 修复
**Branch**: `dev`

### Summary

集中七张 skill owned relations，事务化单删/批删/scanner 清理与 repository prune；在 schema 后 seed 前持久审计并原子修复 orphan；补齐 fault injection、ID 重用、独立历史保留与 FK preflight 回归，just ci 全绿。

### Git Commits

| Hash | Message |
|------|---------|
| `21eb82a9` | (see git log) |

### Status

[OK] **Completed**


## Session 63: 完成数据库版本化迁移与外键级联

**Date**: 2026-07-26
**Task**: 完成数据库版本化迁移与外键级联
**Branch**: `dev`

### Summary

统一 desktop、CLI 与远端缓存数据库打开边界，加入带 checksum 的两版迁移、升级前备份恢复、每连接外键强制与七张 owned relation 级联删除；五版历史 fixture、数据库定向测试和 just ci 均通过。

### Git Commits

| Hash | Message |
|------|---------|
| `6594b85e298d24f2ce7dd3e0b24856a5c5d016f3` | (see git log) |

### Status

[OK] **Completed**


## Session 64: 完成 FS+DB 可恢复操作协议

**Date**: 2026-07-27
**Task**: 完成 FS+DB 可恢复操作协议
**Branch**: `dev`

### Summary

完成 Central 删除与批量更新的 FS+DB Saga、持久化操作日志、按目标互斥锁、启动恢复与恢复 UI；补齐迁移 3、崩溃/远端协议/补偿/脱敏测试及后端契约，默认并发 just ci 通过。

### Git Commits

| Hash | Message |
|------|---------|
| `cf7cee37` | (see git log) |

### Status

[OK] **Completed**


## Session 65: 完成 Release 原子发布门禁

**Date**: 2026-07-27
**Task**: 完成 Release 原子发布门禁
**Branch**: `dev`

### Summary

完成 frozen tag/SHA、可复用 CI、必需平台构建、真实 updater 验签、严格产物与 checksum、draft 回验后唯一公开；独立检查修复 target_commitish 语义与 tag 竞态，just ci 和 Windows NSIS/MSI bundle 通过。

### Git Commits

| Hash | Message |
|------|---------|
| `3d3473b8` | (see git log) |

### Status

[OK] **Completed**


## Session 66: 完成 Settings API 域化与目标配置隔离

**Date**: 2026-07-27
**Task**: 完成 Settings API 域化与目标配置隔离
**Branch**: `dev`

### Summary

完成 generic settings 显式 allowlist 与 typed value validation；实现 SSH/WSL 独立 target 配置隔离、Local fallback、脱敏持久状态 IPC 和 Settings 告警；新增后端契约 spec，并通过 just ci。

### Git Commits

| Hash | Message |
|------|---------|
| `c11aa3f4` | (see git log) |

### Status

[OK] **Completed**


## Session 67: GitHub 不可变 preview snapshot

**Date**: 2026-07-27
**Task**: GitHub 不可变 preview snapshot
**Branch**: `dev`

### Summary

统一 Local/SSH/WSL 的 preview snapshot 注册表，import 与 markdown 读取改为必填 previewId，删除全部分支重拉 fallback；新增 digest v1 与 migration v4 per-skill provenance；六个生命周期错误走稳定 code 信封映射中英文重新预览提示。

### Main Changes

- preview 一次性钉住 commit SHA，tree/raw/tarball 与远程 tarball 全部使用该 SHA，展示数据仍用分支名
- digest v1：域分隔 + u64 大端长度分帧 + 逐文件 SHA-256，排序在 helper 内部完成
- snapshot registry 统一三种 target，单 import lease：失败释放可重试、成功原子消费、lease 期间 discard 延迟到 release
- 删除 resolve_remote_import_workspace 与 fetch_skill_markdown，import/markdown 只读已注册 snapshot
- migration v4 append-only 追加 nullable resolved_commit_sha/content_digest，写入用 COALESCE 防被无 provenance 写入方擦除
- spec 新增 Immutable Preview Snapshot Lifecycle 场景与 frontend snapshot token 契约，并修正已过期的 Markdown Fetch Boundary

### Git Commits

| Hash | Message |
|------|---------|
| `8394e8c7` | (see git log) |

### Testing

- [OK] just ci 全绿；cargo test --locked 1014 passed；受影响 Vitest 485 passed

### Status

[OK] **Completed**

### Next Steps

- connect_remote_target 缺测试缝，真实 SSH/WSL snapshot 读/导入未端到端覆盖，需单独 transport seam 重构
- 父任务 07-24-audit-remediation 剩余 5 个子任务（11/16）


## Session 68: 并发作业独占 lease 与迁移协调

**Date**: 2026-07-27
**Task**: 并发作业独占 lease 与迁移协调
**Branch**: `dev`

### Summary

用 renderer jobId、独占 RAII lease 和陈旧事件过滤消除 Central update/SkillPort portability 共享取消竞态；legacy Central migration 纳入 Local mutation lock 与 blocking I/O 边界。

### Main Changes

- 新增 fail-closed ExclusiveJobRegistry：同族单 active job、精确 ID 取消、有界 pending cancel、stale lease 隔离与稳定 coded errors
- 迁移 8 个 start command 与 2 个 cancel command，全部 progress payload 携带 jobId；Zustand 忽略陈旧事件和旧 promise settle
- 文件 preview 只取得一次 portability lease；Update Center apply 生成 jobId；可见错误统一 formatBackendError 与中英文映射
- legacy migration 在同一 Local mutation guard 内重查/写 marker，递归 FS 作为一个 run_blocking_fs_with unit
- 新增后端 exclusive-job lifecycle 与前端 job correlation specs，并同步 mutation-lock/spawn-blocking 契约

### Git Commits

| Hash | Message |
|------|---------|
| `ccc70666` | (see git log) |

### Testing

- [OK] just ci 全绿：前端 1497 passed/1 skipped；Rust 主库 1009 passed/6 ignored，全部 bin/E2E 与 production build 通过
- [OK] task.py validate 通过（implement/check 各 10 条），git diff --check 通过

### Status

[OK] **Completed**

### Next Steps

- 父任务 07-24-audit-remediation 现为 12/16，剩余 4 个 planning 子任务


## Session 69: 完成启动恢复状态机与前端恢复页

**Date**: 2026-07-28
**Task**: 完成启动恢复状态机与前端恢复页
**Branch**: `dev`

### Summary

完成启动恢复状态机与完整前端恢复页：分类目录、数据库打开和 schema 初始化故障，串行重试与 DB/WAL/SHM 备份重建；补齐故障注入、启动恢复 code-spec、just ci 与 Windows cold-start smoke。

### Git Commits

| Hash | Message |
|------|---------|
| `98ce97f` | (see git log) |

### Status

[OK] **Completed**


## Session 70: 完成 CI 跨平台与供应链加固

**Date**: 2026-07-28
**Task**: 完成 CI 跨平台与供应链加固
**Branch**: `dev`

### Summary

新增 Ubuntu/macOS 源码门禁、双生态 fail-closed 依赖审计、Action SHA 固定与 Dependabot；完成 just ci、实时审计和 Windows NSIS bundle 验证。

### Git Commits

| Hash | Message |
|------|---------|
| `f2dcc3f78b5ac1dfcfa90476f926d89936261b62` | (see git log) |

### Status

[OK] **Completed**


## Session 71: 完成 Typed IPC 与结构化错误边界迁移

**Date**: 2026-07-28
**Task**: 完成 Typed IPC 与结构化错误边界迁移
**Branch**: `dev`

### Summary

完成 180 个 Tauri command 的结构化 IpcError 迁移、首批 42 个 Rust-derived Typed IPC、parity/codegen 门禁与 Windows NSIS 打包验证。

### Git Commits

| Hash | Message |
|------|---------|
| `9f7a719f` | (see git log) |

### Status

[OK] **Completed**


## Session 72: 清偿 size budget 历史例外

**Date**: 2026-07-28
**Task**: 清偿 size budget 历史例外
**Branch**: `dev`

### Summary

拆分五个超过 800 行的历史模块，恢复无例外的统一 size gate。

### Main Changes

- 保持 Central 更新、collections、seed、页面与唯一技能卡片的公开边界和行为。
- 移除 frozen allowlist，并同步 CI quality gate 与贡献指南。

### Git Commits

| Hash | Message |
|------|---------|
| `a9a337f3` | (see git log) |

### Testing

- [OK] just ci；pnpm sizecheck；任务校验；独立代码审阅。

### Status

[OK] **Completed**


## Session 73: 完成极限审计整改父任务验收

**Date**: 2026-07-28
**Task**: 完成极限审计整改父任务验收
**Branch**: `dev`

### Summary

完成 16 个审计整改子任务的本地集成复核并归档父任务；父任务本身无独立产品提交。

### Main Changes

- 核对全部子任务归档状态、验收映射、P3-01 逐文件计数与统一 size policy。
- 记录最终 just ci 通过及不推送、不创建远程 PR 的交付边界。

### Git Commits

(No commits - planning session)

### Testing

- [OK] 父任务 task.py validate；16/16 archive status；just ci。

### Status

[OK] **Completed**


## Session 74: 优化 Skill Usage 页面体验

**Date**: 2026-07-28
**Task**: 优化 Skill Usage 页面体验
**Branch**: `dev`

### Summary

完成扫描骨架、紧凑 KPI、安装状态筛选、匹配状态提示和 target 切换数据重置；通过 just ci 与 1024/1280/1920 浏览器验证。

### Git Commits

| Hash | Message |
|------|---------|
| `39f8a674` | (see git log) |

### Status

[OK] **Completed**


## Session 75: Repair PR #25 cross-platform CI

**Date**: 2026-07-29
**Task**: Repair PR #25 cross-platform CI
**Branch**: `dev`

### Summary

Fixed the macOS stdin fixture and LF checkout contracts for generated IPC and frozen SQL fixtures; exact-head PR run 30415927278 passed every required check.

### Git Commits

| Hash | Message |
|------|---------|
| `0c49dcf6` | (see git log) |
| `e1a2d6f5` | (see git log) |

### Status

[OK] **Completed**


## Session 76: GitHub 导入分支选择

**Date**: 2026-08-01
**Task**: GitHub 导入分支选择
**Branch**: `dev`

### Summary

新增可选 GitHub 分支输入并贯通 Central/Marketplace、typed IPC 与 Local/SSH/WSL 解析；保持默认分支、CLI 与不可变 preview snapshot 契约，补齐本地化错误和跨层回归测试。

### Git Commits

| Hash | Message |
|------|---------|
| `eaf3035d` | (see git log) |

### Status

[OK] **Completed**


## Session 77: 文档生成完整性与 Pages 部署

**Date**: 2026-08-01
**Task**: 文档生成完整性与 Pages 部署
**Branch**: `dev`

### Summary

完成确定性生成文档门禁、官方 Pages artifact 部署与公网 smoke；PR #27 squash 合入 dev，PR #28 merge commit 合入 main，保留 dev 并删除 legacy gh-pages。

### Git Commits

| Hash | Message |
|------|---------|
| `ba23e5925dc3d4b8f18ca2be69df459d7b2bbc24` | (see git log) |

### Status

[OK] **Completed**


## Session 78: CI 反馈路径提速交付

**Date**: 2026-08-01
**Task**: CI 反馈路径提速交付
**Branch**: `dev`

### Summary

并行化 common 与三平台 Rust、供应链 lanes，保持 fail-closed just-ci 汇总；完成 exact-head PR/手动打包 CI、Windows bundle 验证并 squash 合入 dev。

### Git Commits

| Hash | Message |
|------|---------|
| `4119855d516bd2e91f3a68fa381a7c912d909d9e` | (see git log) |

### Status

[OK] **Completed**


## Session 79: 开发与 PR 体验治理交付收尾

**Date**: 2026-08-01
**Task**: 开发与 PR 体验治理交付收尾
**Branch**: `dev`

### Summary

完成 developer/PR 子任务：固定工具链与 doctor、quick lane、PR 模板和 dev 分支治理；PR #31 squash 合入 dev，exact-head hosted CI run 30690364410 全部 required checks 通过；记录远端 ruleset/merge settings 回读。

### Git Commits

| Hash | Message |
|------|---------|
| `cc8a12bde9394142d5ac6cb100d2f28e596e1451` | (see git log) |

### Status

[OK] **Completed**


## Session 80: 桌面发布可信度提升交付

**Date**: 2026-08-01
**Task**: 桌面发布可信度提升交付
**Branch**: `dev`

### Summary

完成 desktop-release-assurance 实现、PR #33 squash 合入 dev 与 exact-head hosted CI；真实 rehearsal、Azure 和 release environment 仍等待独立授权。

### Main Changes

- 加入 exact-SHA rehearsal、tag-bound publish、Windows Authenticode/updater 分离签名、安装启动卸载 smoke、publish-only attestation 和 staging guard。
- 同步中英文发布文档、质量 spec、任务证据并归档 desktop 子任务。

### Git Commits

| Hash | Message |
|------|---------|
| `f4dadb798acf0bdd22f82818379144de9eefe7eb` | (see git log) |

### Testing

- [OK] just ci；just audit；release/workflow contract tests；release-signature-verifier tests。
- [OK] pnpm docs:gen:check；pnpm docs:build；Windows NSIS 与显式 NSIS/MSI bundle；git diff --check。

### Status

[OK] **Completed**

### Next Steps

- 等待 desktop-signing/desktop-release、Azure OIDC variables/secrets 与非公开 rehearsal 的独立授权；完成后再做父任务集成验收和归档。


## Session 81: 工程交付流程优化父任务集成收尾

**Date**: 2026-08-01
**Task**: 工程交付流程优化父任务集成收尾
**Branch**: `dev`

### Summary

四个子任务均已归档；PR #40 将最终跨子任务验收证据 squash 合入 dev，最终 promotion SHA d68387b 在非公开 rehearsal run 30700955460 通过。父任务已归档，保留 dev、gh-pages 缺失和未授权发布边界。

### Main Changes

- 汇总 PR #38/#39/#40、exact-head hosted CI 与 rehearsal 30700955460 的真实证据。
- 归档 08-01-engineering-delivery-workflow-optimization；未修改远端设置、tag、Release、Azure 或 secrets。

### Git Commits

| Hash | Message |
|------|---------|
| `d68387bffdd2f9e0b9f05d978ed925976913ef42` | (see git log) |
| `d03548d3dbed17a69cef88b5f7ff08e760e6f6ad` | (see git log) |

### Testing

- [OK] just ci、just audit、文档/版本检查、18 个 workflow contract tests、git diff --check 和 Windows pnpm tauri build 全部通过。

### Status

[OK] **Completed**

### Next Steps

- Azure Artifact Signing、desktop-release environment、updater staging、公开 Release 和 tag movement 仍需独立授权。


## Session 82: Marketplace Central 安装一致性

**Date**: 2026-08-03
**Task**: Marketplace Central 安装一致性
**Branch**: `dev`

### Summary

删除 registry-backed Marketplace 的缓存 URL 与展示名写入旁路，复用 pinned snapshot 和 central_update Saga 完成 Local/SSH/WSL 可恢复安装。

### Main Changes

- 以 candidate skill_id 和 GitHub repository provenance 作为安装身份。
- 首次导入使用 central_update + hadTarget=false，同事务提交 skill、repository membership、commit/digest 与 journal phase。
- installed marker 改为 durable Central state 的派生缓存并支持故障后修复。

### Git Commits

| Hash | Message |
|------|---------|
| `a52591c9` | (see git log) |

### Testing

- [OK] Node 22.23.2 下 just ci 通过；Marketplace 22/22；GitHub import 137/137；Rust 1056 passed/6 ignored。
- [OK] Fake SSH 与 Fake WSL 完整 Saga、恶意路径、DB rollback、marker 故障与 UID 保留回归通过。

### Status

[OK] **Completed**

### Next Steps

- 实施 08-03-bounded-github-snapshot-lifecycle。


## Session 83: 完成 GitHub 快照生命周期边界

**Date**: 2026-08-03
**Task**: 完成 GitHub 快照生命周期边界
**Branch**: `dev`

### Summary

以 Arc 和 entry/byte/TTL/LRU policy 约束 Central snapshot cache；以严格 reservation、lease、CleanupPending 与 owning-target generation ack 约束 GitHub preview workspace，补齐 cancellation、cleanup failure、跨 target/kind 和并行测试隔离证据，并在 Node 22 下通过完整 CI。

### Git Commits

| Hash | Message |
|------|---------|
| `c2aaea06774bc77eed7865255ed6509214ec2491` | (see git log) |

### Status

[OK] **Completed**


## Session 84: Complete bounded external text ingestion

**Date**: 2026-08-03
**Task**: Complete bounded external text ingestion
**Branch**: `dev`

### Summary

Bounded external HTTP, SSE, Local, SSH, and WSL text ingestion before allocation while preserving typed errors and redaction.

### Main Changes

- Added shared bounded HTTP/local readers, UTF-8-safe truncation, and a deadline-aware SSE state machine.
- Migrated AI, GitHub, Central Skills, scanner, AI tagging, Central Updates, and targets call sites with Local/SSH/WSL parity.

### Git Commits

| Hash | Message |
|------|---------|
| `c126b3cf` | (see git log) |

### Testing

- [OK] Node 22.23.2 just ci passed; Rust 1103 passed and 6 ignored; frontend 1609 passed and 1 skipped; GitHub import 154 passed.

### Status

[OK] **Completed**

### Next Steps

- Implement and verify 08-03-transactional-metadata-mutations.
