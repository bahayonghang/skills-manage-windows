# 会话隔离设计

## Boundary
身份与 active task 的权威是 common/active_task.py，共享边界一次修正，平台不各自补过滤器。

## Mechanisms
- R1：确定 context_key 而无自身任务时返回“本会话无任务”，不进入 single-session inference。无身份子代理显式传递 task/context，不默认猜其他会话。
- R2：clear 使用本次确定 key；workflow mutation 使用同一身份边界。无身份/无自身任务返回无操作或明确失败，不能从读取 fallback 获得写权。
- R3：共享解析层一次判断目录、task.json 状态及归档位置，保留 stale 诊断；text/json/record 消费该结果，不重复探测。失效路径只能显示为诊断。
- R4：新增 unittest + TemporaryDirectory + mock 行为回归，无新依赖；保留正常会话和显式任务注入。

失效输出合同：get_context 的 text 与 record 不在 CURRENT TASK 下输出可执行任务，另列失效pointer诊断；其JSON不返回有效current-task对象。task.py current默认输出不得只打印失效路径并exit 0，应明确无有效任务并非零返回；current --json保留结构化诊断、有效current_task为空，不把失效元数据混作活动任务。正常同会话输出和成功退出保持既有语义。所有呈现消费同一次ActiveTask解析结果，四类入口分别测试缺失、归档与正常情况。

## Files
必改 `.trellis/scripts/common/active_task.py`、`.trellis/scripts/common/session_context.py`、`.trellis/scripts/task.py`、`.trellis/scripts/common/paths.py`。共享解析结果区分有效 task、失效 pointer、无任务；paths 的简化 getter 不把失效 pointer 当作有效任务，session_context 的 text/JSON/record 与 task current 消费同一结果并显示诊断，workflow mutation只接受有效自身任务。各消费者不重复探测或判断归档。新增 `.trellis/scripts/tests/test_active_task_isolation.py`，每个输出入口都有缺失/归档/正常断言。不改 ignored hooks 掩盖共享根因，接线问题交给 bootstrap child。

## Harness / Model
Codex/Claude Code 强模型做语义设计和独立审查。便宜模型只补明确 fixture/断言，不自主决定 ownership/fallback。Grok/Kimi pull-context 与 OMP extension 分别做显式任务 smoke，不能从 Codex 推断成功。

## Tradeoff / Rollback
取消跨会话猜测，允许无身份调用明确无任务，接受该行为变化；不加兼容层。回退只限本 child 源码/测试 diff，不恢复或搬运真实 session。
