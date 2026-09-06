# 执行计划

依赖：用户批准父计划。不依赖其他 child 产品改动。运行现有 runtime suite 时禁止并发增删真实 tasks（已有 fixture 完整性断言）。

1. 强模型核对 SES-001 与当前 HEAD，用父 probe 在临时目录重现。
2. 先写 `test_active_task_isolation.py` 覆盖 AC1-AC4 的读取、clear、workflow、stale 和正常路径，确认旧实现失败。
3. 在 active_task 修复权威边界；同步修改 paths 的有效任务getter、session_context 的 text/JSON/record 和 task.py 的 current/workflow消费者，逐项验证失效只诊断、不跨会话写入；不加配置、不清理真实 session。
4. `python -X utf8 -m unittest discover -s .trellis/scripts/tests -p test_active_task_isolation.py -v`。
5. `python -X utf8 -m unittest discover -s .trellis/scripts/tests -p 'test_*.py' -v`，分别报告通过/跳过，POSIX 未验证保留 UNVERIFIED。
6. 现有 context-injection tests 必须通过；干净 checkout 接线缺失不得跳过算成功，交 bootstrap child 提供可复现性。
7. 强模型独立复核 mutation 不跨会话；规则 child 收录合同和五工具 smoke 状态。
8. 父任务集成统一跑 `just ci`（实施阶段已有写权限）和专项检查。工具链阻塞先解除，不拿 Python PASS 冒充全门禁 PASS。

回滚仅本 child 源码/测试 diff。提交、归档、发布仍依独立授权。
