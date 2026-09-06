# SkillPort 常青项目与五套 harness 审查

日期：2026-09-06；分支 dev；基线 a81b7c2d；起始工作区干净。结论：**未发现需要本轮大改产品架构的实测失败；应优先修复任务隔离、只读诊断与接线测试覆盖，再收敛五工具规则。** 本轮只新增审查/规划材料，父任务与5个子任务均保持planning，待用户批准。

## 1. 优先发现与责任层

| ID / 优先级 | 已确认事实、根因与影响 | 改造归属 |
|---|---|---|
| SES-001 / P1 | `.trellis/scripts/common/active_task.py:584` 对已知新session无pointer仍借唯一旧session；`:689` 的clear再删除旧session文件。临时fixture实测other_session_survives=false。 | harness-session-isolation |
| ENV-001 / P1 | `scripts/check/doctor.mjs:46` 在项目cwd探测pnpm，继承自动版本管理；全局12.3.4与项目10.34.5错配，5秒超时并触发用户缓存获取尝试。宣称只读的诊断不满足合同。 | doctor-pnpm-readonly |
| HARNESS-001 / P1 | `.gitignore:11` 起排除harness目录，必要定制hook不在HEAD；`test_runtime_resilience.py:314` 缺hook时skip，且`scripts/check/run-ci.mjs:33` 的lane未包含Python suite。新clone减少覆盖是源码确认推断，本轮未实跑全新clone。 | harness-bootstrap-and-gates |
| TEST-001 / P2 | `src/test/pages/CentralSkillsView.repositories-and-installs.test.tsx:541` 因旧UI入口消失而skip；当前工具栏筛选→选择→批量安装链缺该回归证据。未宣称产品功能损坏。 | central-bulk-install-regression |
| HARNESS-002/003 / P2 | CLAUDE.md约17KB的结构快照与AGENTS双源，Grok实测两者均加载；三个Kimi skill仍声称不支持project custom agents，已与当前官方能力冲突。 | harness-rules-and-handoff |
| DOC-STATE / TASK-DOC / P2 | 项目指南仍称REL事项open，归档合同已wontfix且保留风险；task create默认推导main，而项目task PR要求dev。应修当前指南/创建示例，不改通用分支引擎或历史ledger。 | harness-rules-and-handoff |
| HARNESS-004 / 信息项 | OMP配置pi/task缺本轮provider/model解析证据；这是UNVERIFIED，不是已证故障。 | 在规则指南列明，验证后再决定，不新增别名层 |

SES-001 实证、结构图和原始输出见 [项目与会话审查](research/project-and-session-audit.md)、[隔离probe](research/probe-active-task.py)。harness本机版本、具体文件行号及官方链接见 [五工具证据](research/harness-alignment.md)。

## 2. 项目结构与已检视关键面

`src/App.tsx → pages/components → stores → src/lib/ipc` 为renderer数据路径；`src-tauri/src/lib.rs → commands → services/repositories → SQLite/FS/SecretStore/target transports` 为后端责任路径。`code_map.md`、CONTEXT.md及backend/frontend/quality spec定义了边界。构建检查由package.json、justfile、scripts/check/run-ci.mjs和.github/workflows负责。开发harness通过根规则、平台生成目录及共享.trellis scripts接入；产品“支持某平台skills”不能证明开发工具的hooks已接好。

本轮已读导航/入口/服务样本、CI/发布指南与脚本、Trellis会话解析和测试、五工具规则/agent/skill配置及相关历史整改合同；不是逐行穷尽全库安全审计，不复用旧finding数量冒充当前扫描。

## 3. 当前测试与证据

| 检查 | 结果 | 边界 |
|---|---|---|
| just doctor | **FAIL，exit 1** | pnpm bootstrap超时；获取尝试已中断，不清用户cache |
| Vitest（现有本地Node CLI） | **178文件，2057 pass / 1 skip** | 直接CLI，不是canonical pnpm门禁 |
| Rust locked tests | **1553 pass / 7 ignored，exit 0** | 当前Windows本机；7项ignored不当作通过 |
| Trellis unittest | **35项：31 pass / 4 skip，exit 0** | Windows无法执行的POSIX场景跳过；无SES-001现有回归 |
| Typecheck/lint/version/docs drift/capability/size/entrypoint | **PASS** | 用现有本地工具，无新依赖 |
| Rust fmt/Clippy/IPC codegen check | **PASS** | 只读检查 |
| 前端Vite和VitePress构建 | **PASS** | 不证明Windows安装包/原生WebView |
| node scripts/check/run-ci.mjs / just ci | **本轮无标准全门禁PASS** | pnpm原入口阻塞，just ci另有版本写入；逐项direct检查不能替代它 |
| 当前真实发布/五provider请求/全新用户trust流程 | **UNVERIFIED** | 未触发远程执行或付费请求 |

详见 [测试工作流报告](research/test-workflow-audit.md) 与其 test-logs。doctor并非“没有任何副作用”：项目文件没变，但包管理器自动引导写了用户缓存；本轮没有主动安装或修改全局配置。

## 4. 失败工作流根因追踪

- 2026-08-28 CI run `33171369538`：供应链例外过期与新advisory，以及三个skills_cli“缺npx”测试混入宿主well-known路径。修复已分别由 `d0d7b239`（依赖/精确例外）与 `3308c0aa`（注入fallback roots，使缺失fixture使用空root）交付；不作为当前整改。
- 2026-08-09 Release run `31308665794`：macOS stdout-overflow终止测试返回TerminationFailed/PermissionDenied，aggregate正确fail-closed，后续build/publish跳过；后续process-runner测试修复及当前macOS lane通过。不能从这次旧失败推断今天的发布失败。
- 最新列出CI `33710069460`（2026-09-03）五条required lanes与aggregate成功；其PR head `9aca456d` 与当前merge HEAD的tracked tree差异为空。**这是相同树内容的远程CI证据，不是merge SHA本身跑过CI。**
- 最近查询未见9月3日流程变更后的Release Desktop执行。当前安装/签名/发布端到端仍UNVERIFIED。REL-001/002既有不实施合同继续有效，不静默重开。

run链接、源位置和修复链见 [GitHub只读回查](research/test-logs/github-workflows.md)。

## 5. 五套工具边界与任务分配

以下是依据本仓库接线的工程建议，不是产品能力排名，也不假定harness等于固定模型/固定价格。

| Harness（本机探测版本） | 适合强模型规划/审查 | 适合较便宜模型执行 | 必须保留的边界 |
|---|---|---|---|
| Claude Code 2.1.263 | CLAUDE导入/规则审查；跨层根因与hook审查 | 明确文件的文档薄化、fixture、测试运行 | CLAUDE导入AGENTS；Write/Bash权限不等于research路径sandbox |
| Codex 0.153.4 | SES-001共享会话与Windows进程边界；CI独立审查 | 有固定断言的Python/TS回归、日志清单 | AGENTS原生链；hook启用/trust与模型覆盖须实际有效 |
| Grok Build 1.0.21 | 用inspect核对双规则加载、pull-context语义 | 限定文件编辑和测试回放 | 本机未启用compatible hooks；spawn_subagent不会自动证明注入 |
| Kimi Code 0.40.1 | 审查旧Trellis说明与当前agent能力差异 | explore只读定位；已授权coder执行单文件测试 | research结果由主线程持久化；不声称仅有built-in或伪装路径隔离 |
| OMP 18.1.11 / Pi 0.85.1 | extension/agent发现和模型路由审查 | 已验证model role下的有限task工作 | OMP=Oh My Pi；pi/task解析未证，不自动替换或发provider请求 |

强模型必须保有ownership、权限、跨层语义、发布风险和独立验收判断；廉价执行限定输入文件、允许操作、AC和停止条件。失败断言变化、扩大权限/文件或新根因出现时回强模型。研究报告提供[Claude官方](https://code.claude.com/docs/en/sub-agents)、[Codex官方](https://developers.openai.com/codex/subagents)、[Grok官方源码](https://github.com/xai-org/grok-build)、[Kimi官方](https://moonshotai.github.io/kimi-code/en/customization/agents)、[OMP官方源码](https://github.com/can1357/oh-my-pi/blob/main/docs/task-agent-discovery.md)及逐能力来源。

## 6. 可批准计划与写回

| 顺序 | 子任务 | 主要改动文件 | 必须通过 |
|---|---|---|---|
| P1 | [会话隔离](../09-06-harness-session-isolation/prd.md) | active_task.py、必要消费者、新isolation unittest | 跨会话读/clear/workflow反例、正常会话、stale摘要、Python全套 |
| P1 | [只读doctor](../09-06-doctor-pnpm-readonly/prd.md) | doctor.mjs、doctor.test.ts | 实际错配probe无引导/cache写；匹配pin探测；有界失败/脱敏 |
| P1 | [接线与门禁](../09-06-harness-bootstrap-and-gates/prd.md) | .gitignore、两个inject hooks、runtime tests、run-ci及合同测试 | 必要hook缺失非零、隔离检出bootstrap、Python接入现有三host lane |
| P2 | [Central交互回归](../09-06-central-bulk-install-regression/prd.md) | CentralSkillsView.repositories-and-installs.test.tsx | 当前toolbar→选择→batch参数；focused Vitest/typecheck/全Vitest |
| 最后P2 | [规则与回写](../09-06-harness-rules-and-handoff/prd.md) | AGENTS/CLAUDE、docs/agents/harness-guide.md、相关指南/spec、三Kimi skill | 规则实际加载、无旧Kimi断言、docs/dev合同测试、docs build |

每个child已有PRD/design/implement及context manifests。执行依赖、明确文件所有权、rollback和UNVERIFIED边界见各child。P1前三项可在各自文件内独立实施；规则回写最后执行，集成门禁由父统一负责。

批准后的知识目标选择**项目说明和项目技能库**，明确适用五工具，不写用户全局memory/vault。本轮不实施、commit、archive、push、安装工具或改变用户trust；标准pin缺失需独立授权恢复，不能拿本计划擅自全局安装。REL既有残留风险不进入本次改造范围。
