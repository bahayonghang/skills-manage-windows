# 未使用技能 unlink 弹窗化:行右统一入口,支持全量/按 Agent 移除

## Goal

重新设计 Skill Usage 页「Unused skills」面板的 unlink 交互:撤销上一任务(08-15-unused-skills-unlink-actions)
引入的行内散点入口(Central 条目第二排 per-agent chip 各带一个内联两段式按钮、平台小节操作区内联按钮),
改为每行最右侧一个统一的 Unlink 按钮,点击打开弹窗;弹窗内支持一键 unlink 全部 Agents 平台,或勾选
单个/部分 Agent 单独 unlink。后端命令与守卫逻辑不变。

## Background

- 用户反馈(2026-08-16 截图):红框标注的 per-agent chip + 内联 unlink 按钮设计不对——按钮散落在行下方
  第二排,多个 Agent 多个按钮,视觉噪音大且入口不统一。
- 现状代码:`src/components/usage/UnusedSkillsPanel.tsx`
  - `CentralAgentChip`(行下方第二排):每个 Agent 一个 chip,可卸载的附 `InlineConfirmAction` 内联按钮;
  - `PlatformUnlinkAction`(操作区):平台散件行内按钮,且每行只针对小节 Agent 的一个 install
    (`preferredPlatformInstall` 只挑一个),同技能装多平台时无法在本行一次清理。
- 弹窗化后可以顺带修复该限制:平台散件弹窗内列出该技能在**所有** Agent 上的 installs,一次勾选清理。

## Requirements

- R1 统一入口:每个未使用技能行(Central 小节与平台小节)的操作列最右侧(打开技能按钮之后)放一个
  Unlink 图标按钮,点击打开弹窗;原 `CentralAgentChip` 内联按钮与 `PlatformUnlinkAction` 内联按钮移除。
- R2 弹窗内容:标题含技能名;列出该技能的全部 Agent 安装项(Central 条目取 `entry.agents`,
  平台条目取 `entry.installs` 全量,不限于小节 Agent),每项带勾选框。
- R3 全量/单选:列表头部提供「全选」勾选框;单个 Agent 可独立勾选;确认按钮展示选中数量并在
  选中数 ≥1 时可用,否则禁用。
- R4 禁用态与原因:不可卸载的 Agent 项(pending recovery、shared-root/central 自身、read-only、
  非 user 来源、缺 rowId)勾选框禁用,悬停/说明展示既有 i18n 原因文案;禁用项不计入全选范围。
- R5 批量执行:确认后按选中项逐个调用既有 `uninstall_skill_from_agent`(组件不直接 invoke,走 store);
  执行期间弹窗内展示进行中状态并阻止重复提交;全部结束后调用一次 `refreshUnused()`。
- R6 结果反馈:全部成功 → 成功 toast 并关闭弹窗;部分失败 → 弹窗内保留并逐项标出失败原因,
  成功项正常清理;整体失败 → 既有错误 toast 通道(formatBackendError)。
- R7 Central 副本保护语义不变:unlink 仅移除 Agent 侧安装,Central 库副本不受影响;弹窗文案延续
  「保留 Central 副本」说明(仅 Central 条目)。
- R8 i18n:所有新增文案进 `src/i18n/locales/en.json` / `zh.json`(`skillUsage.unused.unlink.dialog.*`);
  清理不再使用的旧 key(如内联 `actionLabel`/`confirm`),保留仍被引用的禁用原因 key。
- R9 可访问性:弹窗遵循既有 Dialog 焦点管理;勾选框与全选有可编程关联标签;行右按钮沿用
  40px 热区基准(icon-control-hit-area)。

## Non-Goals

- 不改后端命令、Rust 服务/仓库层与 `unused` 报告契约(数据字段已够用)。
- 不引入「删除 Central 库技能」能力(那是 Central 面板的既有职责)。
- 不改面板的过滤/阈值/排序逻辑。

## Acceptance Criteria

- [ ] 每行最右操作区只有一个 Unlink 入口(打开按钮之后),打开统一弹窗;行内/行下不再出现任何
      unlink 确认按钮。
- [ ] Central 条目弹窗列出全部 `agents`,平台条目弹窗列出全部 `installs`(跨 Agent),勾选框可用。
- [ ] 全选 → 确认,等价于对每个可卸载 Agent 依次执行 unlink,成功后列表刷新、无残留行。
- [ ] 勾选部分 Agent → 确认,只清理所选项;未选项在刷新后仍可见(或按新报告正确呈现)。
- [ ] 禁用项(五种原因)不可勾选且展示原因;全选不会选中禁用项;确认按钮计数只含可卸载项。
- [ ] 执行中重复点击确认无效;完成后弹窗关闭,`pendingUnlinkKeys` 无泄漏。
- [ ] 部分失败场景:失败项在弹窗内可见原因,成功项已清理,报告已刷新。
- [ ] en/zh 双语完整,无硬编码文案;旧的内联确认相关 key 已清理。
- [ ] 前端测试覆盖:弹窗打开/全选/单选/禁用原因/确认调用 store/部分失败呈现;store 批量方法
      的成功、部分失败、pending key 生命周期。
- [ ] `just ci` 通过(前端 + Rust;本任务预期仅前端代码与测试变更)。

## Notes

- 复用组件:`@/components/ui/dialog`、`@/components/ui/checkbox`;交互范式参考
  `BatchUninstallCentralSkillsDialog`(批量选择 + footer 确认)。
- Store 契约细节与守卫原因映射见 `design.md`;执行清单见 `implement.md`。
