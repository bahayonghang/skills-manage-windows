# 五套 harness 的可复制检查与证据上限

2026-09-06 补充只读 CLI 实测。这里只定义检查，不新增通用验证器；在 bootstrap 后的隔离检出执行，不读取用户级凭据。不调用 provider、不启用 hooks、不自动接受 trust。

| 工具 | 已验证的只读命令 | 可判断内容 | 不能据此判断 |
|---|---|---|---|
| Claude | `claude --version`、`claude --help`、`claude agents --help` | CLI版本；--agent/--agents存在；agents实际管理后台会话 | custom agents已被当前主/子会话加载；hook实际触发 |
| Codex | `codex --version`、`codex --help`、`codex features list` | hooks/multi_agent的stage/enabled | 项目agent registry、受信任hook已执行、最终provider/model |
| Grok | `grok --version`、`grok inspect --json` | projectRoot、projectTrusted、projectInstructions、hooks、skills、agents、configWarnings | 每个hook/subagent执行成功、provider可用 |
| Kimi | `kimi --version`、`kimi --help` | --skills-dir/--agent/--agent-file/--plan存在 | 没有只读list/inspect；--agent*会开会话，不属于本轮检查 |
| OMP | `omp --version`、`omp agents --help`、`pi --version`、`pi --help` | OMP/Pi身份；agents只有unpack | `omp agents list`实测exit 1，不能写进验收；unpack有写入副作用不执行；pi/task解析仍UNVERIFIED |

## 不调用模型的静态检查

在项目根运行以下现有命令，并人工比较预期。`rg` 的字段匹配只是阅读辅助，不是新建封闭schema，也不能证明运行时能力。

```powershell
Get-Content CLAUDE.md
rg -n --hidden '^(name|description|tools|model):' .claude/agents/trellis-research.md .claude/agents/trellis-implement.md .claude/agents/trellis-check.md
rg -n --hidden 'SessionStart|UserPromptSubmit|SubagentStart|PreToolUse|inject-subagent-context' .claude/settings.json .codex/hooks.json
rg -n --hidden '^(name|description|sandbox_mode|model|model_reasoning_effort|max_depth)\s*=' .codex/config.toml .codex/agents/trellis-research.toml .codex/agents/trellis-implement.toml .codex/agents/trellis-check.toml
rg -n --hidden '^(name|description):|explore|main session|no project-level custom' .kimi-code/skills/trellis-research/SKILL.md .kimi-code/skills/trellis-implement/SKILL.md .kimi-code/skills/trellis-check/SKILL.md
rg -n --hidden '^(name|description|tools|model):' .omp/agents/trellis-research.md .omp/agents/trellis-implement.md .omp/agents/trellis-check.md
Get-ChildItem .omp/extensions/trellis -Name
```

预期：CLAUDE用`@AGENTS.md`保留唯一通用合同；agent名称/角色与文件一致；工具和sandbox字段如实表达权限，模型字段缺省只记录“请求继承”，不冒充最终模型。配置指向存在的hook文件；Kimi research使用explore返回主线程持久化，三个技能不再包含错误“no project-level custom”断言；OMP必要extension/source存在，pi/task仅记录配置字符串。

Grok inspect JSON预期字段：projectInstructions项为`path,scope,fileType,sizeBytes,approxTokens`；hooks项为`event,hookType,target,source,matcher`；skills项为`name,description,source,userInvocable`；agents项为`name,description,source`。核对新检出projectRoot、AGENTS/薄CLAUDE路径、Trellis skills/agents来源。没有启用hook可记录“未启用”，不能判为触发成功。configWarnings逐项解释，不把空warnings当全面PASS。

## 验收状态

本计划必须通过：受控文件/导入路径/必要字段静态一致、Grok实际inspect来源一致、现有Python回归。其余四套真实session发现、hook事件、研究拒写演练、最终模型与provider结果属于单独的UNVERIFIED栏；只在明确获准的真实会话执行后改变状态。静态检查成功即可完成文档接线范围，不能勾选运行时或产品发布验收。

来源沿用同目录harness-alignment.md的官方链接；本文件补充本机CLI发现边界，避免将CLI命令存在误认为agent实际运行。
