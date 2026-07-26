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
