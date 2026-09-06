# 统一五套 harness 项目规则与模型分工说明

## Goal
P2。让五套harness读取同一项目事实，保留工具真实边界，并在实施验收后回写批准项；父要求R3/R4/R5。

## Evidence
根CLAUDE.md约17KB且含多处高变计数，Grok inspect同时摄入AGENTS/CLAUDE；计数存在漂移风险，未逐项证明全部错误。三个Kimi技能的“无项目custom sub-agent”陈述已与当前官方能力冲突。docs/agents/git-and-release.md把已contract-wontfix的REL事项写作open；task create默认base可能main而项目要求dev。详见父research。

## Requirements
- R1：AGENTS.md为项目通用事实源；CLAUDE.md用官方import保留最薄入口和必要Claude增量，高变结构计数回归code_map或生成文档。
- R2：明确Claude/Codex/Grok/Kimi/OMP的规则发现、子代理、hooks/pull、权限、模型继承及本机/官方/运行时证据差异；修正Kimi过时断言。
- R3：每类整改指定强模型规划/独立审查和便宜模型可执行范围；不把工具品牌等同模型能力、价格或授权。
- R4：已批准且验收的结论回写项目说明/项目技能，带五工具适用标签；未批准的全局知识库写入、REL风险重开、provider调用不在本范围。
- R5：统一只读审查入口、标准实施门禁、--base-branch dev创建示例和REL wontfix残留风险状态，不改历史报告或签名工作流。

## Acceptance Criteria
- [ ] AC1（R1）：按父research/harness-checks.md，静态验证CLAUDE.md导入AGENTS.md且不再复制高变计数，Codex/OMP保留AGENTS路径；Grok inspect的实际projectInstructions指向该合同和薄CLAUDE。Claude/Codex/OMP真实会话加载未运行时单列UNVERIFIED。
- [ ] AC2（R2）：三Kimi skill静态内容删除错误能力断言，research指定explore返回→主线程限定目录持久化；implement/check仍限批准scope。五工具矩阵逐一标权限是强制能力还是提示词约定；Kimi实际agent发现与研究持久化演练未运行时单列UNVERIFIED，不以文案通过冒充执行通过。
- [ ] AC3（R3）：每个child与最终指南均有强模型owner、廉价执行白名单及升级条件；OMP pi/task无法证明可用时保留UNVERIFIED，不凭YAML字符串通过就换model。
- [ ] AC4（R4, R5）：指南仅收录批准且实际通过的机制，附适用工具/来源/检查入口；docs/dev合同测试和文档构建通过，REL不再被误导为待执行或fixed。未获证据保留缺失项。
- [ ] AC5（R1, R5）：现有developerExperienceContract覆盖唯一规则入口、dev PR示例、只读审查入口与保留的Windows验收边界；公共README若改变则中英文同步。

## Approval / Out of Scope
planning。最后执行，消费其他children证据。选项目说明和项目技能作为本次回写目标，不写Basic Memory或用户native记忆、不改C:/Users/...全局规则；不新建治理schema/模型评分表/自动选模器。
