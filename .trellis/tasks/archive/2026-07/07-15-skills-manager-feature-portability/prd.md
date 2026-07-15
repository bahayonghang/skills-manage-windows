# SkillPort 共享核心 CLI 与稳定身份/并发基础

## Goal

在不引入 Git 备份、版本快照或多设备合并的前提下，为 SkillPort 建立两项可独立验收的能力：

1. 桌面端与 CLI 共用 Rust use case/service 的 `skillport-cli`。
2. 与目录 slug/path 分离的稳定技能身份，以及 GUI/CLI 共享的中央库跨进程 mutation 锁。

本父任务负责跨子任务契约、依赖顺序和最终集成验收，不直接承载产品代码实现。

## Background

- SkillPort 已是 `rlib`，业务逻辑主要位于 `src-tauri/src/services/`；CLI 应复用这些服务，而不是调用依赖 `tauri::State` 的 command 或复制实现。
- 当前 `skills.id` 来自目录名，适合作为兼容 slug，但不适合作为跨接口稳定引用；移除 Git 合并范围后，无需把全库主键一次性迁成 UUID，可用新增不可变 `uid` 的增量方案降低风险。
- SQLite 能串行化数据库写入，但不能保护两个进程对同一中央技能目录执行 rename/swap/delete；开放可变更 CLI 前必须建立跨进程文件锁。

## Task Map

| Child | Deliverable | Dependency |
| --- | --- | --- |
| `07-15-stable-skill-identity-mutation-lock` | 不可变 skill `uid`、兼容 resolver、中央 mutation lock、现有 GUI 写路径接入 | 先完成；为 CLI 的稳定引用和安全写入提供契约 |
| `07-15-shared-core-cli` | `skillport-cli`、CLI façade、JSON/exit-code、查询/搜索/安装/同步 | 依赖上一个子任务的 resolver 与 mutation lock；不得自行实现第二套锁 |

## Requirements

- **P1 Shared Core**：Tauri command 与 CLI 必须调用同一 Rust service/use case；command 层只做 IPC 适配，CLI binary 只做参数、输出与退出码适配。
- **P2 Stable Reference**：CLI/API 返回并接受不可变 skill `uid`，同时兼容现有 slug/id 和唯一 name；目录 slug 仍负责路径与展示，不被误称为稳定身份。
- **P3 Safe Mutation**：GUI 与 CLI 对中央库的 install/update/delete/centralize/sync 等最终 mutation 必须经过同一跨进程锁和现有原子文件置换边界。
- **P4 Local MVP**：CLI 首发仅操作本机 Local target，并可把中央技能同步到本机已启用 Agent；SSH/WSL CLI target 后置。
- **P5 Windows First**：路径、锁、symlink/copy fallback、CLI 安装和 Tauri Windows bundle 都是验收范围。
- **P6 Compatibility**：现有 DB、中央目录名、前端路由、collections/tags/repository assignment、portable state 和安装关系不得因新增 `uid` 失效。
- **P7 Scope Boundary**：不实现 Git 仓库、Git 备份、历史快照、恢复、自动备份、三方合并或冲突三选一。

## Acceptance Criteria

- [x] 两个子任务的 `prd.md`、`design.md`、`implement.md` 均通过 Trellis 校验且依赖写明。
- [x] 稳定身份/并发锁子任务完成并通过其定向验收。
- [x] 共享核心 CLI 子任务完成并通过其定向验收。
- [x] CLI mutation 通过共享锁与 GUI 串行化；不存在 CLI 专属文件写实现或重复业务编排。
- [x] 旧数据库升级后原 slug/id 仍可解析，所有既有中央技能均获得唯一不可变 `uid`。
- [x] exact shorthand 等价离线 fixture 链路可用，并可选同步到本机已启用 Agent。
- [x] `just ci` 与 Windows `pnpm tauri build` 通过，desktop exe、CLI exe 与 NSIS 均生成。
- [x] `ref/skills-manager` 保持只读；已有无关 `.trellis` 修改未被产品实现覆盖。

## Out Of Scope

- Git backup、snapshot/restore、multi-device sync、skill-aware Git merge。
- CLI 管理 SSH/WSL target、Git remote、backup history 或 conflict resolution。
- 通过内容哈希猜测用户绕过 SkillPort 执行的任意外部目录重命名。
- 照搬参考项目的 preset/scenario、rusqlite store 或 Tauri command 结构。

## Review Gate

用户于 2026-07-15 批准实施；两个子任务已按 identity/lock → shared CLI 顺序完成并归档。
