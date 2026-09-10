# 修复 Trellis 当前会话任务隔离

## Goal
消除新会话借用、清理或修改其他会话任务的路径，让五套 harness 获得可追踪的任务归属。P1；父要求 R2/R3/R4。

## Confirmed Facts
- SES-001 已在父任务 research/probe-active-task.py 的隔离临时目录复现：有新 context_key 但无自身 pointer 时借唯一旧 pointer，clear 后旧文件消失。
- 根因入口：`.trellis/scripts/common/active_task.py:584`、`.trellis/scripts/common/active_task.py:689`。
- 摘要 `.trellis/scripts/common/session_context.py:622` 消费简化任务路径，没有呈现 stale。
- 现有 Python suite 35 tests / 31 passed / 4 skipped，未覆盖该缺陷。

## Requirements
- R1：已有确定会话身份时，只读取该身份任务；新会话缺失 pointer 时不得借其他会话任务。
- R2：清理和当前任务 workflow 变更只作用于调用者身份；无身份不得从其他会话推断写入授权。
- R3：缺失或归档任务只作诊断，不作为可执行 current task；text、JSON、record 与 task CLI 一致呈现失效状态。
- R4：保留同会话正常操作和明确 Active task 子代理交接；纳入 Python 行为回归及五工具适用说明。

## Acceptance Criteria
- [x] AC1（R1）：临时 fixture 中，新身份+唯一旧任务、新身份+缺失旧任务、新身份+两个旧会话均不读取旧任务，旧 session 字节不变。
- [x] AC2（R2）：clear/workflow 修改在无自身 pointer 或无身份时不改变他人 session/task.json；同身份正常操作成功。
- [x] AC3（R3）：自身缺失/归档 pointer 在 text、JSON、record 和 current CLI 明确非活动，不引导执行，也不自动删除真实 session。
- [x] AC4（R4）：全 Python suite 通过，原 probe 转为断言回归后通过；明确子任务 context 注入测试继续通过，五工具真实 smoke 未跑时逐项 UNVERIFIED。

## Out of Scope
不建 session 迁移、别名表或清理服务；不改真实 session 数据、全局 Trellis 安装或上游仓库；不以 fixture 冒充五工具真实会话通过。

## Approval
保持 planning，父计划批准后再开始。知识回写由 harness-rules-and-handoff child 统一持有。
