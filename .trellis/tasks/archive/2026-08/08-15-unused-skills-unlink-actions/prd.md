# 未使用技能面板 unlink 操作与徽章优化

## Goal

在未使用技能面板中，让用户能直接把技能从本地各 Agent 的 skills 目录移除（unlink/uninstall，不动 Central 库），并改善匹配/状态徽章的可读性与交互性。

## Background / Confirmed Facts（代码调查结论）

- 可复用命令：`uninstall_skill_from_agent(skill_id, agent_id, row_id?)` 与 `batch_uninstall_skills_from_agent`（`commands/linker.rs:99-220`；`services/installation/install.rs:109-158`）。前端惯例：`skillStore.uninstallSkillFromAgent`（`src/stores/skillStore.ts:82-129`）+ `PlatformView.handleUninstall`（`PlatformView.tsx:213-227`）。
- 确认 UX 惯例：`InlineConfirmAction`（`src/components/ui/inline-confirm-action.tsx`，两段式 arm/confirm，destructive 样式）；语义文案已有 `detail.uninstallPlatformTitle/Desc`（"只从该平台移除，Central 保留"）与 `central.batchUninstallSafety*`。
- **数据缺口**：`UnusedSkillEntry` 平台条目只有 name/agents/installedPath，缺 `row_id`、per-agent skill id、`source_kind`/`is_read_only`/`link_type` —— 现有报告形状不足以直接调 uninstall。
- 能力边界：
  - Central 条目（skillId 存在）：可按 agent unlink；shared-root agent 报 `SharedCentralUninstall`，`central` 自身拒绝，pending-recovery 拒绝。
  - claude-code 散件：`row_id` 路径可删 native 真目录并清 observation 行。
  - 非 claude 散件（如 codex 的 native 真目录）：现有命令**不支持**（`remove_install_local` 对 native 真目录报 `NotASymlink`）。
- 刷新缺口：unlink 后无事件、无自动重扫；非 claude 路径只删 `skill_installations` 不删 observation 行，`refreshUnused()` 后平台分组仍显示旧条目，需扫描器重跑。
- 徽章现状：State 列 5rem 宽导致 "Never us…" 截断；无通用 Badge 组件，徽章由 `src/lib/statusTone.ts`（`statusChipClass`/`statusFillClass` 等）+ Tailwind 内联组合；tooltip 惯例为 `title` + i18n。
-  spec 约束：`icon-control-hit-area.md`（<40px 图标按钮需 `after:` 扩大热区、相邻按钮中心距 ≥40px、hover 显现需配 focus-visible）；`async-error-feedback.md`（toast + `formatBackendError`）；`skill-usage-state.md`（组件不直接 invoke，新动作须进 store）。

## Requirements

- R1: 面板行内新增 unlink 操作（per-agent 维度），两段式确认（InlineConfirmAction），文案明确"仅从该 Agent 目录移除，Central 保留"。
- R2: 后端扩展 `UnusedSkillEntry`：平台条目携带 per-agent `row_id`、skill id、`link_type`、`source_kind`、`is_read_only`，使前端能判定可 unlink 性并调用现有命令。
- R3: Central 条目行支持按 agent unlink（shared-root/central/pending-recovery 等不可行情形禁用并给 tooltip 说明）。
- R4: unlink 成功后刷新面板数据且不留陈旧行（需要时删除对应 observation 行或触发局部重取）。
- R5: 徽章优化：State 列不再截断（用 `statusChipClass` 紧凑 chip 或调宽），匹配状态徽章统一样式，图标按钮满足热区/focus-visible 规范，hover/tooltip 提供可发现性。
- R6: i18n 中英文齐备；组件不直接 invoke。

## Acceptance Criteria

- [ ] AC1: Central 条目可按 agent unlink，成功后面板刷新、该行 agent 标记消失；不可行 agent 显示禁用态+原因 tooltip。
- [ ] AC2: claude-code 散件可 unlink（含 native 真目录）；unlink 后 observation 行清除、面板不再显示。
- [ ] AC3: 非 claude 散件 native 真目录可 unlink（后端扩展的 observation 删除路径），含 dir_path 一致性校验；read-only / 非 user 来源等不可行情形禁用+tooltip。
- [ ] AC4: State 徽章全文可读、无截断；新图标按钮符合 hit-area/focus-visible spec。
- [ ] AC5: 失败路径 toast + `formatBackendError`；`just ci` 通过。

## Out of Scope

- 从 Central 库删除 skill（现有入口已覆盖）。
- 批量 unlink（先做单条，批量留后续）。

## Key Decisions（已确认）

- D1（用户已确认）: **全部可 unlink** —— 本期扩展后端，支持删除非 claude 散件的 native 真目录（推广 claude 的 observation 行 unlink 路径到所有 agent，保留 read-only / source_kind / pending-recovery 守卫）。
- D2: unlink 后平台分组刷新 = 删除对应 observation 行 + `refreshUnused()`；不触发完整重扫。
- D3: 平台条目报告形状改为 per-agent 行（每条含 agent_id、row_id、skill_id、link_type、source_kind、is_read_only、installed_path），前端按 agent 分组渲染。

## Risks

- unlink 是真目录删除操作，必须两段式确认 + 复用现有守卫（mutation lock、pending-recovery 拒绝）；真目录删除前必须校验 observation 行的 dir_path 与目标一致，防误删同名目录。
- 扩展 installation 域删除路径需遵循 `skill-deletion-integrity.md`（实现前阅读）。
