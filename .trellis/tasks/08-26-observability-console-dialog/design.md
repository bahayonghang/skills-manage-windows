# Observability Console UX Brief

## Job and Audience

Mode: **Operate**。用户通常在刚看到失败Toast、检查操作历史或诊断异常退出时进入；首要任务是在数秒内
确认“哪个操作、什么结果、为什么、下一步”，然后按需查看开发诊断。

## Outcome and Proof

- Primary proof: stable operation ID、action/status、public reason、phase/retryability和关联Runtime evidence；
- Success: 不阅读JSON、不按时间猜测，也能定位同一次操作；
- Product truth: Operation是长期用户事实，Runtime是短期开发证据，UI相邻但不混淆存储语义。

## Selected Direction

保留现有桌面应用视觉与高密度日志控制台，不做品牌重设。结构焦点从“大块raw JSON drawer”转成“小型诊断
工作台”：摘要和下一步在首屏，稳定诊断键紧随其后，raw safe JSON退到disclosure。

## Layout and Interaction

- centered modal，typical 560px，max width/height受viewport约束；mobile/narrow使用12–16px安全边距近全宽；
- sticky compact header：title、copy operation ID、close；body内部滚动；
- summary card -> diagnosis/next action -> diagnostic key grid -> metadata -> collapsed JSON；
- Operation/Runtime页面工具栏增加ID filter；关联跳转使用显式按钮，不靠链接样式猜测；
- backend/frontend Runtime rows共享ID但保留source badge；可折叠成group，默认仍能看到两条来源；
- dialog由Base UI管理topmost Escape/focus；关闭后回到触发行按钮。

## States and Ranges

- empty/loading/error；legacy row，无ID/无details；unknown code；invalid JSON；
- 0–50 failure items与truncation；36-char UUID；长action/code/category；中英文长next action；
- started/interrupted/cancelled/partial；Runtime只有backend、只有frontend或双视角；
- 100/125/150% Windows scaling与窄窗口。

## Boundaries

No gradients/glass/decorative oversized cards, no new dependency, no raw secret/path/host/stack rendering, no animation
beyond existing dialog transitions. Existing Operation filters/KPI/heatmap and Runtime file/list mechanics remain functional.
