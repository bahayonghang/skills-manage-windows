# 五套 harness 对齐审查

日期：2026-09-06。基线：`dev` / `a81b7c2d`。范围仅含 Claude Code、Codex、Grok Build、Kimi Code、Oh My Pi（OMP）的规则加载、技能、子代理、权限与模型分工；本文件是批准前的只读审查材料，不代表已经实施。

## 结论与优先级

### HARNESS-001 · P1 · 当前接线未进入基线提交，新检出可复现性存在缺口

`.gitignore:11-13,38-40` 排除了 `.agents/`、`.claude/`、`.codex/`、`.omp/`、`.grok/` 和 `.kimi-code/`。`git ls-tree -r --name-only HEAD -- .claude .codex .grok .kimi-code .omp .agents` 在基线提交无输出，而本机这六个目录包含 hooks、agents、skills 和配置。`.trellis/scripts/tests/test_runtime_resilience.py:54-65` 指向 `.codex/hooks/`、`.claude/hooks/`；但 `:304-314` 会在这两个文件缺失时 `skipTest`，所以新 clone 的风险是静默减少覆盖，并非 import 失败。`scripts/check/run-ci.mjs:10-62` 的 canonical lanes 只编排 pnpm/cargo 步骤，也没有执行 Trellis Python tests。由源码和 Git 树可以推断：仅凭当前提交无法取得被测 hooks，常规 CI 也不会暴露该缺口；本轮没有实际创建全新 checkout，因此 fresh-checkout 结果仍为 **UNVERIFIED**。

仓库 `.trellis/.version` 与本机 `trellis --version` 都是 `0.7.0-beta.3`，且 `trellis init --help` 已确认支持 `--claude --codex --grok --kimi --omp --skip-existing`。最小修复优先用这一 pinned bootstrap 生成标准接线；Sep 2 已定制的 `.claude/hooks/inject-subagent-context.py` 与 `.codex/hooks/inject-subagent-context.py` 应精确纳入版本控制，配合 `--skip-existing` 防止初始化覆盖其修复。其余必要非私有文件清单在实施前逐项核对，不新建模板/生成层，也不纳入用户级 enable、trust、review、provider 配置或凭据。

验收：在不读取当前工作区被忽略目录的临时 Git checkout 中执行 pinned bootstrap；随后运行 context injection、agent discovery 和 `test_runtime_resilience.py`，并断言 `TestHookBudgetConsumption` 实际执行而非 skip。另把 Trellis Python tests 纳入明确的仓库门禁，避免 `run-ci.mjs` 全绿却未覆盖这些 hooks。失败必须指出缺少的源或生成步骤，而不是回退到开发者本机目录。

### HARNESS-002 · P2 · 根规则出现两套事实源，存在上下文重复与漂移风险

`AGENTS.md:3-47` 是 47 行的稳定项目合同，包含工具链、关键责任边界、交付证据和 PR 基线。根 `CLAUDE.md` 是 167 行、17138 字节的结构快照，并硬编码 171 个 command/24 文件（`:48,95`）、12 个 service/17 个 repo/9 个 schema 模块（`:50-54`）、36 个内置 agent（`:100`）和 70 个 source/22 个推荐 skill（`:129`）。这些计数具有随产品演进漂移的风险；本轮没有重新统计对应源码，因此不判定这些具体数字当前已经过时。

本机 `grok inspect --json` 显示 Grok 同时加载根 `AGENTS.md` 与 `CLAUDE.md`；Claude Code加载根 `CLAUDE.md`；Codex 以 `AGENTS.md` 为首选（`.codex/config.toml:9-10`）。因此同一仓库对三套 harness 呈现不同且可能冲突的项目事实。最小修复是让根 `CLAUDE.md` 通过 Claude Code 支持的 import 引用 `AGENTS.md`，只保留 Claude 专属增量；会频繁变化的计数留在生成文档或源码查询，不进入常驻提示。

验收：分别打印/inspect 三套工具的有效项目指令，确认唯一的通用规则源为 `AGENTS.md`，Claude 专属内容仅出现一次，Grok 不再摄入一份重复的产品架构快照。

### HARNESS-003 · P2 · research 的语义边界需与工具能力区分，Kimi 能力描述已过时

Claude 的 research agent 明确拥有 `Write` 与 `Bash`（`.claude/agents/trellis-research.md:4-5`）；Codex research 使用 `workspace-write`，再以文字限制到 task `research/`（`.codex/agents/trellis-research.toml:1-3,51`）；Grok 与 OMP 也以 agent 文本约束允许写入的位置（`.grok/agents/trellis-research.md:4-7,143-146`；`.omp/agents/trellis-research.md:4-7,27-30`）。这是当前已授权的角色语义承诺；它与 harness 的工具 allowlist、capability 或 sandbox 属于不同层次，不要求为此新增 OS 级目录隔离。

Kimi skill 进一步声称“不支持项目级 custom sub-agent”，因此用具备写能力的内置 `coder` 承担 research（`.kimi-code/skills/trellis-research/SKILL.md:7-8,20-26`）。当前官方 Kimi Code 文档已经提供 project custom agents，并列出 `coder`、`explore`、`plan` 的不同工具边界；本机 0.40.1 的 `--agent` / `--agent-file` 也与此一致。这里应先删掉错误断言。最薄修复是让只读发现走 `explore`，由主会话在已知 task `research/` 下持久化返回结果；只有在验证 custom agent 的项目发现与路径写约束后，才切换为专用 research agent。

验收：用聚焦 smoke 要求 research 修改源码，记录各 harness 是由能力层阻断还是由角色语义拒绝，不把提示词拒绝表述为 sandbox 证明；正常研究仍能由 read-only agent 返回结果并由主线程写入当前 task 的 `research/`。Kimi smoke 还要证明实际选中的 agent/工具集，不能只检查 skill 文本。

### HARNESS-004 · P2 待验证信息 · OMP 的 `pi/task` 解析状态尚无证据

`.omp/agents/trellis-research.md:6-7` 与 `trellis-implement.md:5-6` 声明 `model: pi/task`，但已查到的 OMP agent/model/task 官方说明不足以证明它在本机 18.1.11 / Pi 0.85.1 中解析为何种 provider/model；本轮未调用 provider，故状态为 **UNVERIFIED**。这是待补证的信息项，不是已证缺陷；也不据此要求新增 alias 或强制硬钉模型。

模型分工应记录职责而不是产品排名或价格：架构与 source-of-truth 判断、跨层/凭据/发布修改方案、失败根因裁决、独立验收由强推理模型负责；已给定文件和断言的检索、清单、机械编辑、格式化、docs 生成和测试命令可下放给便宜模型。任何模型都不得通过角色名称扩大权限；planning 保持只读，research 只写当前 task 的 `research/`，implement/check 才可在批准范围改代码。

验收：每个平台尽量通过 agent discovery 输出说明最终模型（显式或继承）、工具/能力和写边界；OMP 可做一次无敏感数据、无代码写入的 task-agent smoke，记录解析后的 provider/model。无法得到该输出时继续标记 `UNVERIFIED`，保持继承或现有声明，不新增别名兼容层。

## 五套 harness 对照

### Claude Code 2.1.263

- 规则：官方支持 `CLAUDE.md` 与文件 import；当前根 `CLAUDE.md` 是重复且易失效的产品快照，需收敛到 `AGENTS.md` 的薄入口。
- Hooks：`.claude/settings.json:1-94` 已接 SessionStart/UserPromptSubmit/SubagentStart/PreToolUse，但目录未入库，只有本机证据。
- 子代理：`.claude/agents/trellis-research.md:1-5` 被本机发现；未指定 model，因而继承主会话模型。
- 权限：research 拥有 Write/Bash，目录边界是当前角色语义承诺；Claude 权限规则可缩小工具调用，原生 Windows sandbox 状态需单独验证。
- 最小动作：薄化 `CLAUDE.md`；优先通过现有 Trellis bootstrap 分发必要 agent/hook，或精确追踪必要非私有文件；用 smoke 区分工具阻断与语义拒绝。
- 官方依据：[memory/import](https://code.claude.com/docs/en/memory)、[hooks](https://code.claude.com/docs/en/hooks)、[subagents](https://code.claude.com/docs/en/sub-agents)、[permissions/sandboxing](https://code.claude.com/docs/en/sandboxing)、[MCP](https://code.claude.com/docs/en/mcp)。

### Codex 0.153.4

- 规则：Codex 原生读取 `AGENTS.md`；`.codex/config.toml:9-10` 将 `AGENTS.md` 再列为 fallback，没有提供额外覆盖价值。
- Hooks：`.codex/hooks.json:3-49` 接入四个时点；`.codex/config.toml:12-18` 正确说明 user feature enable 与一次性 review 是生效前提。
- 子代理：`.codex/config.toml:30-39` 固定最大深度 1；research 使用 `workspace-write`，implement/check 的强模型字段目前只是注释，实际模型继承需由 inspect 证明。
- 权限：Codex 能为 agent 配 sandbox/model/reasoning；当前 research 的路径限制是已授权的角色语义合同，不把它表述为目录级 sandbox 证据。
- 最小动作：追踪/生成 hooks 与 agents；删除冗余 fallback；bootstrap 验收必须包含 trusted project、hook enable/review 的明确人工前置状态。
- 官方依据：[AGENTS.md discovery](https://developers.openai.com/codex/guides/agents-md)、[subagents](https://developers.openai.com/codex/subagents)、[configuration](https://developers.openai.com/codex/config-reference)、[MCP](https://developers.openai.com/codex/mcp)。

### Grok Build 1.0.21

- 规则：本机 `grok inspect --json` 证明它同时摄入根 `AGENTS.md` 和 `CLAUDE.md`，当前因重复事实源而承担最大上下文漂移风险。
- Hooks：inspect 显示当前配置没有启用 Claude-compatible hooks；不能把 `.grok/agents` 可见等同于上下文 hook 已生效。
- 子代理：`.grok/agents/trellis-research.md:21-27` 使用 `spawn_subagent`，但 agent frontmatter 没有 model、tool 或 capability mode。
- 权限：官方当前支持 role/capability modes；本仓库当前以角色语义约束写边界，能力层是否阻断越界为 **UNVERIFIED**。
- 最小动作：先消除根规则重复；再在 project agent 中明确能力模式与模型继承策略，并用 `grok inspect --json` 作为发现验收。
- 官方依据：[Grok Build repository](https://github.com/xai-org/grok-build)、[subagents guide](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/16-subagents.md)、[settings](https://docs.x.ai/build/settings)。

### Kimi Code 0.40.1

- 规则/技能：项目 `.kimi-code/skills/` 在本机存在但未入库；全新 clone 不会获得 Trellis skills。
- 子代理：`.kimi-code/skills/trellis-research/SKILL.md:22` 的“不支持 project custom agent”已与当前官方文档冲突。
- 权限：内置 `explore` 只读，`coder` 可写；当前为满足持久化而直接调用 coder，使 research 边界依赖 prompt。
- 模型：官方配置支持 model pool/secondary model；本仓库没有可复现的角色模型决策，实际继承为 **UNVERIFIED**。
- 最小动作：删除过时能力声明，使用 `explore -> 主会话持久化` 的薄流程；custom agent 只有在本机 discovery smoke 通过后再采用。
- 官方依据：[custom agents](https://moonshotai.github.io/kimi-code/en/customization/agents)、[configuration](https://moonshotai.github.io/kimi-code/en/configuration/config-files.html)、[CLI](https://moonshotai.github.io/kimi-code/en/reference/kimi-command.html)。

### Oh My Pi 18.1.11 / Pi 0.85.1

- 规则：OMP 的 context-file provider 支持项目规则发现；当前 `.omp/` 没有单独 AGENTS 文件并非缺陷，根 `AGENTS.md` 应作为通用事实源。
- 子代理：`.omp/agents/trellis-{research,implement,check}.md` 定义三种职责；research 和 implement 固定 `pi/task`，check 未固定模型。
- 权限：tool allowlist 能减少暴露工具，但 research 仍同时拥有 write/bash（`.omp/agents/trellis-research.md:6`），审批模式与目录写隔离不能从 frontmatter 推定。
- 模型：`pi/task` 的实际解析结果无本轮 provider smoke，明确为 **UNVERIFIED**。
- 最小动作：优先核对现有 bootstrap 是否已包含 agent/extension 来源；按需验证 agent discovery、审批模式和解析后的模型，未获证据时保持 `UNVERIFIED`，不强制改变模型声明。
- 官方依据：[context files](https://github.com/can1357/oh-my-pi/blob/main/docs/context-files.md)、[task-agent discovery](https://github.com/can1357/oh-my-pi/blob/main/docs/task-agent-discovery.md)、[approval mode](https://github.com/can1357/oh-my-pi/blob/main/docs/approval-mode.md)、[task tool](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/task.md)。

## 建议实施顺序与可判定门禁

1. **P1 可复现接线**：精确追踪两个已定制的 inject-subagent-context hooks，以 pinned `trellis init --claude --codex --grok --kimi --omp --skip-existing` 生成其余标准接线；清单实施前核对。临时 checkout 必须证明 hook tests 未被 skip，并补齐 Trellis Python tests 的仓库门禁。
2. **P2 唯一规则源**：薄化 `CLAUDE.md` 并核对 Claude/Codex/Grok 的有效上下文；不把产品清单复制回常驻规则。
3. **P2 Kimi 薄修复与权限 smoke**：改正 custom-agent 事实，优先采用 `explore -> 主会话持久化`；五套 smoke 区分能力阻断和语义拒绝。
4. **P2 模型职责与补证**：按风险表达强推理与有限执行职责；不默认固定 model，OMP `pi/task` 只补充解析证据。
5. **P2 已验收知识回写**：仅在实施和独立检查通过后，把稳定的“规则源、角色边界、bootstrap 命令”回写 `AGENTS.md`/`docs/agents/` 或项目 skill；版本号、模型排名、provider 可用性保留为运行时证据，不写成常青事实。

建议聚焦检查：

```text
git ls-tree -r --name-only HEAD -- .claude .codex .grok .kimi-code .omp .agents
python -X utf8 -m unittest discover -s .trellis/scripts/tests -p "test_runtime_resilience.py"
claude --version; codex --version; grok --version; kimi --version; omp --version; pi --version
grok inspect --json
```

未运行五个 provider 的真实模型请求，也未在全新 Windows 用户配置下完成 trust/review/approval 流程，因此模型解析、外部服务可用性和首次安装体验均为 **UNVERIFIED**。本报告没有执行 `task.py start`、修改产品代码/现有 harness、安装依赖、提交、归档或远程写入。
