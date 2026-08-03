# 诊断检查更新失败

## Goal

在不修改 GitHub 凭据、应用数据库、Central 技能目录或远端仓库的前提下，定位桌面端“检查更新”失败的实际根因，解释为什么用户更新 GitHub token 后故障仍存在，并给出有证据支撑的处置或修复建议。

## Background

- 2026-08-03，Central Skills 的 `Check all (141)` 常规检查弹出 `Update check failed: The operation failed. See runtime logs for details.`。
- Operation Logs 记录 `update_center.refresh` 失败，摘要为 `Failed to refresh skill update inventory`，外层错误为 `Update Center action failed`。
- 该条 operation log 的 details JSON 仅含 `requestedAgents: 0`、`requestedRepositories: 0`、`requestedSkills: 141`、`scopeKind: "skills"`，未包含底层异常。
- 同期 `scan.all` 成功；因此当前证据只证明 Update Center refresh 路径失败，不能直接证明扫描、token 或 GitHub API 本身失败。
- 用户已更新 GitHub token，但尚未确认运行中的应用实际读取了哪个凭据来源、何时加载、是否需要重启，以及 token 的权限/有效性。

## Requirements

- 全程只读取证；不得输出、复制或持久化 token、密码或其他 secret。
- 建立一个可由代理重复运行、能命中用户所见故障的最小失败信号；若本机缺少无交互入口，必须明确记录该限制及替代证据。
- 追踪 `refresh_skill_update_inventory` 从前端调用到 Tauri command、service/repository、GitHub 客户端和 operation/runtime logging 的错误传播链。
- 识别应用实际使用的 GitHub 凭据来源及加载/缓存生命周期，但只验证来源、存在性和响应状态，不读取或展示 secret 值。
- 至少比较以下候选原因：应用仍使用旧/不同凭据、token 权限或认证失败、API 限流、网络/TLS/代理、单个仓库或技能元数据异常、并发/锁/数据库失败、底层错误被外层错误吞掉。
- 根因结论必须区分“已证实”“高概率但未证实”和“已排除”，并引用具体日志、状态码、调用路径或可重复命令。
- 在用户另行批准实现前，不修改产品代码，不执行 `task.py start`，不应用任何修复。

## Acceptance Criteria

- [ ] 有一个已实际运行的、快速且可重复的失败检查，或明确说明为什么当前只能依赖用户交互复现。
- [ ] 找到与截图时间和动作对应的底层 runtime error；若当前日志设计无法保留底层异常，明确定位丢失异常的代码边界。
- [ ] 确认应用本次更新检查使用的凭据来源和加载时机，且诊断输出不包含 secret。
- [ ] 对主要候选根因逐项给出证据与判定，不把“用户已更新 token”当成“运行进程已使用新 token”。
- [ ] 给出最小临时处置、建议的永久修复及验证方法；任何代码修复仍需后续规划批准。

## Out of Scope

- 修改、迁移或清除 GitHub token。
- 自动安装更新、删除技能或处理远端仓库增删。
- 修改 Central 文件、SQLite 数据、GitHub 仓库或其他外部状态。
- 在根因未证实前重构 Update Center 或凭据系统。

## Notes

- 当前任务处于 Trellis planning；本阶段只进行需求记录和只读取证。
