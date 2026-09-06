# 恢复 Central 筛选到批量安装的交互回归

## Goal
P2。替换唯一因旧UI入口消失而skip的前端交互用例，覆盖当前筛选→选择→批量安装请求；对应父任务第1、2、4项关于证据、根因和工具分工的要求。

## Evidence
`src/test/pages/CentralSkillsView.repositories-and-installs.test.tsx:541` 的it.skip仍引用已移除installed-filter-*；注释已指出新入口ToolbarViewMenu/checkbox/BulkActionBar。当前2057 pass/1 skip不证明这段串联交互通过；既有menu/batch outcome单项通过不替代串联。

## Requirements
- R1：通过现有ToolbarViewMenu已安装筛选、卡片checkbox和BulkActionBar完成等价当前用户操作，不恢复旧UI。
- R2：断言被筛除skill不进入batchInstallSkills，所选skill、目标agent和安装方式正确，保留既有batch结果反馈覆盖。
- R3：只改该测试与必要同域fixture，不加生产feature/依赖/测试框架。

## Acceptance Criteria
- [ ] AC1（R1）：原skip被当前交互测试取代并实际执行通过，无旧installed-filter-*选择器。
- [ ] AC2（R2）：至少一个安装与一个未安装skill的夹具，筛选后仅正确对象可选且batch调用参数精确；意外包含被过滤skill会让断言失败。
- [ ] AC3（R3）：目标文件测试、typecheck和全Vitest通过；如暴露真实产品失败，回主线程修订范围，不降低断言或直接扩改产品。

## Scope / Approval
planning。独立低风险测试交付，用户批准后执行；结果不证明原生WebView可用性。
