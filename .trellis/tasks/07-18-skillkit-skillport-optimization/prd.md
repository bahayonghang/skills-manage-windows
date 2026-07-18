# SkillKit 对标与 SkillPort 优化路线图

> 任务类型：父任务。只维护来源要求、任务地图、跨子任务验收和最终集成审查，不直接承载产品代码实现。

## Goal

深入审阅 `ref/skillkit` 与当前 SkillPort 的真实实现，识别可迁移的界面、交互逻辑、业务机制和性能优化，并形成符合 SkillPort“高密度、本地优先、主题即身份、Windows 构建优先”定位的分阶段 Trellis 路线图。

目标不是复刻 SkillKit，而是吸收其有效机制、保护 SkillPort 已经更成熟的能力，并明确拒绝会削弱中央目录唯一真源、远程目标、可观测错误或设计系统一致性的做法。

## Background

- 参考快照固定为 `ref/skillkit` commit `9e89e03a0ea1f7a5e3551d7abc7ea98a7433eb77`（2026-07-16，`v0.4.0`）。
- SkillKit 是 Electron + React 18 + better-sqlite3；SkillPort 是 Tauri + React 19 + Zustand + SQLite。
- SkillPort 已有虚拟化列表/网格、分组、保存视图、批量操作、GitHub 导入预览文件树、插件分组、远程目标、资源预算、并发扫描、事务化持久化和完整测试。这些能力不因对标而降级。
- 所有关键判断与证据见 `research/skillkit-skillport-comparison.md`。
- 四个子任务曾于 2026-07-18 全部归档；2026-07-19 最终验收后，`07-18-unified-skill-import` 已恢复为 `planning` 以修复原始契约缺口，其他三个子任务保持归档。父任务继续保持 `planning`，只承载最终跨子任务验收，不启动、不承载产品代码。
- 父任务验收结果与命令证据见 `research/final-cross-child-acceptance.md`。当前统一 ZIP 导入仍有原子恢复、Operation Log、Zustand 状态所有权、前端/端到端测试和错误脱敏/i18n 缺口，因此父任务尚不可归档。

## Requirements

### R1. 证据化双向盘点

- 覆盖技术栈、信息架构、安装/导入链路、状态与持久化、性能、可靠性、可访问性、i18n、主题和 Windows 兼容性。
- 关键结论必须保留 `file:line` 锚点，并区分已确认事实、合理推论和需要实施阶段测量的假设。

### R2. 采纳判断

- 每个候选项标注“直接吸收 / 适配后吸收 / 保持现状 / 明确拒绝”。
- 采纳项必须说明用户价值、实现成本、风险、依赖、度量与回滚方式；没有基线的数据不得宣称性能提升已经成立。
- 不以暖白大留白、固定四列卡片、横向 agent chip 导航、通用 SaaS 卡片化或装饰性动效作为优化方向。

### R3. SkillPort 不变量

- 技能卡继续只用 `UnifiedSkillCard`。
- React 组件继续通过 Zustand store 进入 IPC，不直接调用 Tauri `invoke()`。
- 中央目录继续是唯一真源；安装/链接链路继续复用 `ensure_centralized` 语义。
- 保留 GitHub 导入现有 preview-only `pluginName`、文件清单、扁平 selection payload、PAT、镜像、资源预算、远程目标与安全路径契约。
- 所有用户可见文本同步中英文 i18n；平台与打包验证以 Windows/PowerShell 和 Tauri bundle 为必需范围。
- 不重做 2026-07-16 已完成的技能详情视觉层级工作。

### R4. 父子任务边界

- 父任务不启动、不直接实现；实际工作由下列子任务独立实施、验证和归档。
- 有依赖的子任务必须在自身 `prd.md` / `implement.md` 中记录顺序，不能把父子链接误当依赖系统。
- 当前 Codex workflow 为 inline；实施按任务地图串行，一次只启动一个子任务。并行只表示可独立规划，不授权同时修改共享工作树。

## Task Map

| 顺序 | 子任务 | 优先级 | 采纳判断 | 依赖 |
| --- | --- | --- | --- | --- |
| 1 | `07-18-unified-skill-import` | P1 | 适配后吸收：统一添加入口 + 安全 ZIP 预览/导入 | 无 |
| 2 | `07-18-github-import-manifest-fast-path` | P1 | 适配后吸收：tree manifest 快路径 + archive 回退 + 性能基线 | 无产品依赖；inline 实施排在 1 之后，不得破坏现有导入契约 |
| 3 | `07-18-dense-typography-wcag` | P2 | SkillPort 自身优化：语义排版 token + WCAG 治理 | 无产品依赖；排在 1/2 之后，避免与 1 同改 `CentralSkillsShell.tsx` |
| 4 | `07-18-skillport-import-deep-link` | P2 | 可选吸收：只传递 GitHub 导入意图的 `skillport://` 深链 | 必须等待 1 的统一入口稳定 |

推荐 inline 顺序固定为 1 → 2 → 3 → 4。这里的 2/3 排序是共享工作树与修改面协调，不新增产品层依赖；4 仍有对 1 的真实依赖。新生产依赖（ZIP 与 deep-link/single-instance 插件）必须在对应子任务启动前单独确认。

## Cross-Child Acceptance Criteria

- [ ] 4 个子任务分别通过审批、实施、验证和归档。统一 ZIP 子任务因约定测试与行为缺口已恢复为 `planning`；其他三项保持归档。
- [x] 统一入口没有复制或包裹现有 GitHub wizard 的状态机，GitHub 与 ZIP 都保持“预览后写入”。
- [x] GitHub 快路径在受支持场景减少整包下载，且所有失败在任何 Central 写入前回退 archive；现有 DTO、selection、路径与持久化契约不变。
- [x] 深链不携带凭据、文件路径、冲突决策或目标平台，不绕过预览与确认；Windows 冷启动和已运行实例均可用。
- [x] 排版治理保留调度台密度、6 主题和 14 accent，并用实测对比度与 0.875/1/1.125 字号缩放验证，而非机械放大全局字号。
- [ ] 每个子任务最终通过定向测试、`git diff --check` 和 `just ci`；涉及打包的深链任务额外通过 Windows `pnpm tauri build` 并验证安装产物。统一 ZIP 导入缺少约定的前端状态覆盖与后端原子回滚集成测试。
- [ ] 父任务完成最终跨子任务回归，确认 Central、Marketplace、Usage、Operation Logs、远程目标和技能详情无语义或视觉回退。现有跨表面测试通过，但 ZIP overwrite 回滚、持久化 Operation Log、错误脱敏/i18n 和 Operation Logs/技能详情最终视觉证据仍未闭环。

## Planning Readiness

- [x] SkillKit 与 SkillPort 的功能、架构、界面和性能证据已盘点。
- [x] 候选项已形成采纳/拒绝矩阵并记录价值、成本、风险与验证方式。
- [x] 4 个可独立验收的 Trellis 子任务已定义依赖并分别实施；统一 ZIP 子任务在最终验收后按原范围重新打开，父任务始终保持 `planning`。
- [x] 父任务和每个子任务均具备 `prd.md`、`design.md`、`implement.md`。
- [x] 最终 PRD 已完成收敛检查，无临时问题、重复事实或无证据的通过结论。

## Out of Scope

- 账号/OAuth、云同步、7 天分享短链服务及其后端。
- 复制 SkillKit 品牌、文案、资产或视觉语言。
- 把 session-only Recent installs 移植到 SkillPort；现有持久化 Operation Logs 是更强的机制。
- 搬运同步文件系统扫描、逐行非事务写入、React `key` 强制重挂载、原生 `confirm`、巨型 CSS 或手绘 SVG 图标。
- 由父任务直接承载任何产品代码、依赖、数据库 schema、打包或发布变更；这些变更只能由明确的执行范围拥有。
