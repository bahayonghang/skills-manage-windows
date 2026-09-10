# 接线来源与门禁设计

## Minimal Source
R1：精确纳入 `.claude/hooks/inject-subagent-context.py` 与 `.codex/hooks/inject-subagent-context.py`，保留现有定制与逐字一致合同；通过 `.gitignore` 的逐层例外只开放这两文件。rules child需要持久化的三个Kimi技能同样通过精确例外开放，内容由该child持有。禁止全量追踪生成目录。

R2：复用现有入口 `trellis init --claude --codex --grok --kimi --omp --skip-existing --yes`；只对隔离检出运行并先核对CLI版本与现有help。不使用 --force，不拉取远程spec。现有 .trellis/.version 是版本记录，不新加manifest/registry。初始化后核对受控源码未被改写；安装Trellis或用户trust不是该命令已证明的结果。

隔离输入在批准实施时按以下方式构造，不触碰真实Git index：用 `git ls-files -z` 枚举当前受控路径，从工作区复制实际字节（即包括tracked diff；明确删除的文件不复制），再只加入批准的新增文件清单：两个inject hooks、三个Kimi skill，以及session child新增的test_active_task_isolation.py；rules child的新harness-guide.md完成后在最终集成快照加入。禁止从六套ignored目录递归复制，禁止纳入runtime/cache/local settings或无关untracked文件。在自有临时目录 `git init` 建立repo身份，仅此临时repo允许建立测试所需index。运行bootstrap前后对输入文件做hash比较，并记录遗漏/删除和实际新增入口。无需主工作区git add/commit，也不使用只含旧HEAD的export。该机制是本次验收步骤，不新增持久快照工具或manifest。

## CI Placement
R3：在 `scripts/check/run-ci.mjs` 的现有 rust-platform lane中加入 Python unittest discover（Windows `python`、POSIX `python3`），让同一入口覆盖三真实host及本机。现有任务安全测试大多标准库，不添加Python依赖。移除必要hook缺失的success skip，改为明确失败；POSIX/Windows不可适用跳过仍保留。
修改 `src/test/scripts/runCi.test.ts` 与 `src/test/contracts/ciWorkflowContract.test.ts`，断言顺序/失败传播/既有lane不变。既有GitHub hosted runners所需Python命令在现有job中检查，若不可用明确环境阻塞，不能在CI静默跳过测试；不新增第三方Action依赖。

R4：fixture或受控检出验证静态可重放，harness真实发现按各自CLI/扩展可用能力记录；未调用模型或trust流程一律未验证。不通过版本文本声称provider可用。

## Owned Files
.gitignore；两个inject hooks；.trellis/scripts/tests/test_runtime_resilience.py；scripts/check/run-ci.mjs；src/test/scripts/runCi.test.ts；src/test/contracts/ciWorkflowContract.test.ts。必要的bootstrap行为回归放在现有Python tests中，不另建检查框架。规则文档/三个Kimi skill/quality spec由rules child所有。

## Model / Rollback
Codex/Claude Code强模型审查受控源、ignore最小范围及CI失败传播。便宜模型可录入确定的allowlist或fixture，不能决定暴露哪些个人配置。初始化只操作自有临时目录；回滚按child diff，禁止递归移除真实harness目录。
