# Design — 未使用技能面板 unlink 操作与徽章优化

## 后端

### 1. 报告形状扩展（R2/R3，D3）

`UnusedSkillEntry` 平台条目改为 per-agent 行（`services/usage/mod.rs` build_unused_report 平台段 + `usage_unused_repo.rs` 查询）：

```ts
interface UnusedPlatformInstall {
  agentId: string;
  rowId: string | null;        // agent_skill_observations.row_id
  skillId: string;             // 散件的 skills 行 id（scanner 已持久化）或 Central id
  linkType: string;            // native | symlink | copy …
  sourceKind: string | null;
  isReadOnly: boolean;
  installedPath: string;
}
// UnusedSkillEntry 平台条目: installs: UnusedPlatformInstall[]（替代 agents: string[]）
```

Central 条目保留 `agents`（来自 skill_installations）并附带 per-agent linkType，用于判定 shared-root 不可 unlink。

### 2. 全 agent observation unlink 路径（D1）

推广 claude 专属 `uninstall_claude_observation_from_agent_impl`（`installation/native.rs:252-284`）：
- 新路径对任意 agent：按 (agent_id, row_id) 校验 observation 行 → 校验 `dir_path` 与期望一致 + `is_read_only=false` + source_kind 守卫（沿用 claude 的 `user` 语义，其他 agent 按 scanner 写入值放行可管来源）→ 删除盘上条目（native 真目录允许，`allow_native_dir=true`）→ 删 observation 行 + 对应 `skill_installations` 行。
- 实现前必读 `.trellis/spec/backend/skill-deletion-integrity.md`；复用 central mutation lock 与 `reject_pending_recovery`。
- 命令层：优先复用 `uninstall_skill_from_agent(skill_id, agent_id, row_id)` —— 将其 Local + row_id 分支从 claude-only 推广（保持 claude 行为不变），不改签名则前端零新命令；若推广会破坏 claude 语义，则新增 `unlink_skill_observation(agent_id, row_id)`。实现时先读 linker.rs/install.rs 现状再定，倾向"推广不新增"。
- 非 claude 的 symlink/copy 安装（generic 路径）unlink 成功后**补删 observation 行**，消除面板陈旧显示（R4）。

## 前端

- `usageStore`：`unlinkUnusedSkill(entry, install)` action（组件不 invoke）→ 调 store 层既有 `skillStore.uninstallSkillFromAgent` 或直接 typed ipc（与现有 store 边界一致：usage store 内 invoke typed ipc）→ 成功后 `refreshUnused()`；失败 toast + `formatBackendError`（`async-error-feedback.md`）。
- 面板行操作区：现有 open-skill 图标按钮旁加 unlink 图标按钮（`InlineConfirmAction` 两段式、destructive 样式）；遵守 `icon-control-hit-area.md`：`size-8` + `after:size-10` 伪元素扩热区、相邻按钮 `gap-2`、hover 显现配 `focus-visible:opacity-100`。
- 确认文案复用 `detail.uninstallPlatform*` 语义（"仅从 {{platform}} 移除，Central 保留"）；Central 条目 shared-root agent 禁用 + tooltip 原因。
- **徽章（R5）**：
  - State 列：`Never us…` 截断修复——改用 `statusChipClass` 紧凑 chip 且列宽自适应/加宽，全文可读；
  - 匹配状态沿用 `UsageMatchStatus` 点+文，保持无色可读契约；
  - 行 hover 提升操作可发现性（操作区默认低透明度、hover/focus-visible 全显）。
- i18n：`skillUsage.unused.unlink.*`（en + zh）。

## 兼容 / 回滚

- 报告字段为增量（TS 类型同步更新）；后端删除路径推广不动 claude 既有行为（测试锁定）。
- 回滚 = 回退命令分支与面板操作区，无迁移。

## 测试

- 后端：非 claude observation unlink（native 真目录删除 + observation/installations 行清除）、dir_path 不匹配拒绝、read-only 拒绝、claude 路径回归不变、symlink/copy 路径补删 observation。
- 前端：unlink 两段式确认流转、成功后 refreshUnused 重取、失败 toast、禁用态 tooltip、徽章无截断（文本渲染断言）、hit-area class 存在。
