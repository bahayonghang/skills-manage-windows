# 父执行计划（待批准）

## 本轮规划交付
1. 完成当前结构/关键文件和五工具官方/本机接线审查。
2. 跑现有tests并追历史失败；记录doctor失败、direct CLI全套、相同树远程CI与缺失发布证据。
3. 建5个planning child，PRD/design/implement +真实context manifests；root/children均base_branch=dev。
4. 运行递归plan_precheck、task.py validate；强模型独立审查整个scope，修正规划问题。不得task.py start。

## 批准后的顺序
1. session-isolation（P1）、doctor-pnpm-readonly（P1）、bootstrap-and-gates（P1）按独立文件并行或顺序实施；Central regression（P2）可独立下放便宜模型。
2. 各child先focused regression，再必要模块suite；失败/新增风险返回强模型，不削弱assertion。
3. rules-and-handoff（P2）消费四项结果，最后回写项目说明与三Kimi skill；没有通过的结果保留UNVERIFIED。
4. 父统一集成检查：`just ci`；涉及供应链仅复跑相关audit；Python专项随改造后lane执行。doc检查按质量合同。缺匹配pnpm时明确BLOCKED，不擅自安装。
5. 独立强模型核对用户需求1-6逐项有交付，五工具能力/模型职责没有混淆，modified files与child所有权一致；记录passed/failed/skipped/missing证据。
6. `python -X utf8 .trellis/scripts/task.py validate <task-dir>` 对6成员；`git diff --check`。提交/归档/远程发布等待对应授权，不以本次规划批准自动执行。

## Final Evidence
报告必须包含修前反例/修后验证、五工具适用范围、实际模型/工具集可见性、产品与运行时外部验收边界，以及每个批准项的最终写回路径。
