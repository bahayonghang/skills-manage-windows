# 项目事实与五工具分工设计

## Source of Truth
R1：AGENTS.md保持简短项目合同；CLAUDE.md使用`@AGENTS.md`导入并链接code_map、相关docs、Trellis工作流；移出快照式计数，不维护第二份架构事实。Grok也会读取CLAUDE，薄入口即消除冗余大快照，无需工具识别规则引擎。

R2：新增单一 `docs/agents/harness-guide.md` 记五工具矩阵、已验收bootstrap、任务交接与证据分层。Kimi三个project skill（research/implement/check）去掉“不支持project custom agent”的绝对断言；research默认read-only explore，结果返回主线程写已知task research路径。其他工具现有提示词写范围可继续使用，但不声称OS路径隔离。新增custom-agent框架没有必要。

## Routing
R3：身份/路径/凭据/FS+DB/CI发布/能力争议由强模型作决定和独立review；确定文件与断言后的fixture、清单、文档薄化、命令执行可交便宜模型。执行发现新失败、需要扩大文件/权限、改变断言或跨层语义时交回强模型。版本与预算是执行时的实际选择，不写死“工具X永远更便宜”。OMP pi/task是待验证信息项，不是已证故障。

R4/R5：其他children完成后，只将已验收合同回写本repo。docs/agents/build-and-test.md加入只读审查入口（直接run-ci且版本check）、Python门禁/前置条件与direct CLI证据边界；git-and-release.md写dev示例和REL contract-wontfix链接。quality/ci-quality-gate.md同步最终lane与doctor实际能力。全局Basic Memory本次不选为写入目标。

## Owned Files
AGENTS.md、CLAUDE.md、docs/agents/harness-guide.md（新）、docs/agents/build-and-test.md、docs/agents/git-and-release.md、.trellis/spec/quality/ci-quality-gate.md、src/test/contracts/developerExperienceContract.test.ts；
.kimi-code/skills/trellis-research/SKILL.md、trellis-implement/SKILL.md、trellis-check/SKILL.md（bootstrap child拥有ignore例外）。
README.md/README_CN.md仅当新增公共入口或承诺时同步，否则不改。不改管理型Trellis bundled skill、template hashes或历史ledger。

## Evidence / Rollback
父research/harness-checks.md是检查命令、字段和证据上限的清单：除Grok inspect外，本机其余四套没有provider-free项目agent registry。help/features与静态导入只能证明配置/接口，实际会话发现、hook触发、最终模型与provider未跑时逐项UNVERIFIED。拒写演练只能证明该场景表现，不证明通用sandbox。该child文档范围的PASS与运行时未验证栏分开，不能由文档PASS替代产品验收。回滚限该child文档/skill/test diff。
