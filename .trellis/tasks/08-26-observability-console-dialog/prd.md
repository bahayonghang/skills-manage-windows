# Observability Console 与居中日志详情弹窗

状态：**planning**。依赖：core contracts 与 Runtime diagnostics children 完成。

## Goal

让用户从一次成功、失败或中断操作快速看到可执行原因，并能在 Operation/Runtime 两层间按 operation ID
定位证据；把右侧全高详情 drawer 改成居中、紧凑、响应式且无障碍的诊断 Dialog。

## Requirements

- U1：Operation 与 Runtime mode 均支持 operation ID 精确筛选；详情提供关联层跳转并保持当前查询上下文。
- U2：Operation list/detail显示 started/interrupted/succeeded/partial/failed/cancelled，状态不只靠颜色。
- U3：详情层级固定为 status/summary、localized reason/next action、code/category/phase/retryable/operation ID、
  target/time/duration/batch、默认折叠 safe JSON。
- U4：`OperationLogDetailDrawer` 重构/重命名为约560px centered Dialog；窄窗安全边距、视口内滚动，
  不再右侧全高占屏。
- U5：保留overlay/Close/Escape、focus trap/restore、copy ID/JSON；长code/ID/JSON不撑破外窗。
- U6：Runtime backend/frontend双视角按同一ID显示清晰source，可分组但不删除证据；legacy/no-ID行诚实退化。
- U7：invalid JSON、unknown code、missing details、loading/empty/error、长中英文、100–150% scaling有明确状态。
- U8：所有新增copy走en/zh i18n，backend动态Display、path、host、secret或raw Runtime reason不进DOM。

## Acceptance Criteria

- [ ] 从failed Operation detail一次动作可切到相同ID的Runtime结果，再返回时保留Operation筛选。
- [ ] centered Dialog在桌面、窄窗和长内容下不溢出；Close/Escape/overlay/focus restore/copy均通过。
- [ ] 用户无需展开JSON即可知道发生什么、在哪个phase、是否可重试和下一步。
- [ ] started/interrupted与backend/frontend source均有图标+文本语义，不能仅靠颜色。
- [ ] legacy/unknown/invalid JSON与no-correlation fixture不崩溃、不伪造不存在的原因。
- [ ] component/store/page/i18n tests通过；Windows Tauri视觉与焦点证据单独报告。

## Out of Scope

- 重做整个Observability Console视觉世界、改变既有Catppuccin/token/font体系。
- 修改日志存储、retention、业务错误code或operation policy。
- 把Runtime raw line默认全部展开，或增加装饰性动画/大面积视觉效果。
