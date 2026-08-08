# 诊断更新检查 tracked source 冲突

## Goal

确定 Update Center 在常规检查中对
`bahayonghang/my-claude-code-settings` 报出
`central_updates.skill_source_missing`，随后按界面提示以增量模式重查却返回
`resource.conflict` 的真实原因；说明为何此前不报错，并给出兼顾现有技能数据、
repository provenance 与 Windows 发布链路的最小解决方案。

## Requirements

- 建立一条快速、确定、可由 agent 自动运行的反馈命令，命中截图中的源路径缺失与
  增量重查冲突链路，而不是只验证相邻功能。
- 从前端失败行按钮、store、Tauri command、Central Updates service、SQLite 持久化
  逐层追踪请求与错误映射。
- 只读检查与该仓库技能相关的本地状态；不得修改真实数据库、Central 目录或凭据。
- 对比相关 Git 历史和已归档 Trellis 任务，区分远端仓库内容变化、历史 provenance
  漂移、近期重试功能回归及并发/陈旧 inventory 等候选原因。
- 给出可立即恢复检查的用户操作方案，以及能防止复发的代码修复方案；标明各方案的
  数据安全边界和验证方式。
- 本任务只做诊断和方案，不实现修复、不发布安装包、不推送远端。

## Acceptance Criteria

- [x] 记录一条已经运行的快速确定性命令，其失败信号与截图症状一致。
- [x] 用代码、测试、只读运行时状态和 Git 历史证据确认单一主根因，并排除主要替代
      假设。
- [x] 解释“常规检查为何产生失败项”以及“增量重查为何又变成 resource.conflict”。
- [x] 给出不丢失 Central 技能的即时恢复步骤、永久修复位置及建议回归测试。
- [x] 最终报告明确证据缺口；若无法读取实机状态，不把推断写成已确认事实。

## Notes

- 用户截图时间为 2026-08-08 20:38:32，失败 repository 为
  `bahayonghang/my-claude-code-settings`。
- 截图显示常规结果有 24 个可更新项、1 个失败项；失败项建议以增量模式重查，但
  Retry toast 为通用 `resource.conflict`。
