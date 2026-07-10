# 修复 GitHub 导入 skill 目录误判

## Goal

让 GitHub 仓库中合法的顶层 `skill/SKILL.md` 能被预览和导入，同时保留对深层泛化 `.../skill/` 包装目录的既有保护，并消除错误文案触发无关 PAT 指引的问题。

## User Value

用户可以直接导入 `yetone/kill-ai-slop` 这类“网站或应用 + 顶层单数 `skill/`”仓库，得到稳定、可识别的技能 ID，而不会看到与实际失败原因无关的 GitHub 令牌建议。

## Confirmed Facts

- `yetone/kill-ai-slop` 的默认分支为 `main`，仓库中唯一的技能清单是 `skill/SKILL.md`。
- 该文件包含合法 frontmatter：`name: kill-ai-slop` 与非空 `description`；失败不是 YAML、UTF-8、下载、权限或速率限制问题。
- 当前发现器能找到 `skill/SKILL.md`，但嵌套候选的 `skill_id` 取自目录 basename，因此得到 `skill`。
- `is_generic_remote_skill_candidate` 对所有 `source_path != "." && skill_id == "skill"` 的候选执行静默过滤，导致唯一候选消失。
- 该过滤最初用于排除类似 `agent_reach/skill/SKILL.md` 的深层泛化包装目录，已包含在 `v0.10.12`，不能无条件删除。
- 根目录 `SKILL.md` 已使用仓库名生成技能 ID；上游 `kill-ai-slop` 的安装说明也要求把 `skill/` 内容安装到名为 `kill-ai-slop/` 的目标目录。
- 直接子路径 URL 仍保留仓库相对 `source_path`，因此 `.../tree/main/skill` 当前同样命中过滤。
- 前端认证提示使用裸正则分支 `pat`；通用错误文案中的 `subpaths` 会误命中该分支并显示 PAT 建议。
- 2026-07-09 的插件清单分组是 preview-only、additive 行为，不是本次回归来源。

## Requirements

- GitHub 仓库根下的单数 `skill/SKILL.md` 通过现有合法性校验后必须成为可预览、可导入候选。
- 顶层 `skill/` 候选不能以泛化 ID `skill` 导入；它应沿用根技能语义，以仓库名生成稳定的技能 ID。
- `https://github.com/yetone/kill-ai-slop` 和显式 `.../tree/main/skill` 两种输入都必须得到同一个候选身份与内容范围。
- 深层 `agent_reach/skill/SKILL.md`、`packages/example/skill/SKILL.md` 等既有泛化包装过滤保持不变，除非后续显式选择另立需求。
- 不改变常规 `skills/<skill-name>/SKILL.md`、`.agents/skills/<skill-name>/SKILL.md`、根 `SKILL.md` 或插件 manifest hint 的候选 ID 与发现顺序。
- 本次修复不得改变导入选择 payload、Central 数据库 schema、更新来源元数据或插件分组持久化契约。
- 预览无候选时仍返回当前领域错误；本任务不扩展为完整的结构化 preview diagnostics 重构。
- 认证指引只能对真实的 GitHub 鉴权、权限、限流或已配置令牌失败显示，不能因普通单词子串出现 `pat` 而显示。
- 所有新增或修改的用户可见文本必须通过中英文 i18n；若无需新增文案则保持现状。

## Acceptance Criteria

- [x] 以真实仓库同构 snapshot（`skill/SKILL.md`，frontmatter 名为 `kill-ai-slop`）执行候选构造时返回且仅返回一个有效候选。
- [x] 该候选的 `sourcePath` 为 `skill`，`skillId` 为 `kill-ai-slop`，并保留 frontmatter 的显示名称和描述。
- [x] 仓库根 URL 与 `tree/main/skill` URL 的预览结果具有相同候选身份。
- [x] 现有深层泛化 `.../skill/` 过滤测试继续通过；包含真实命名技能的混合仓库不会被隐藏。
- [x] 根技能、命名技能目录、插件 manifest 分组、local snapshot 与 SSH/WSL remote workspace 的候选语义保持一致。
- [x] 导入选择、冲突处理、目录复制和更新来源行为没有回归。
- [x] `No importable ... subpaths ...` 不显示 PAT 指引，真实限流/权限错误仍显示正确指引。
- [x] 增加覆盖上述后端假阳性与前端字符串误分类的定向回归测试。
- [x] `git diff --check`、定向 Rust/前端测试和 `just ci` 全部通过。

## Out of Scope

- 全面移除对任意深度 `skill/` 目录的泛化候选保护。
- 重写 GitHub 导入发现器、改变递归深度或优先根列表。
- 把所有字符串错误迁移为新的跨 IPC 结构化错误协议。
- 修改目标仓库内容、要求上游移动目录或创建 plugin manifest。
- 数据库迁移、插件分组持久化或新的安装目标交互。

## Open Questions

- None. The user confirmed that fixing the misleading PAT guidance is a required deliverable in this task.

## Notes

- 诊断证据来自 2026-07-10 的 GitHub 官方仓库树和源码、当前 `dev` 分支实现、提交历史及现有定向测试。
- 本任务按跨后端发现/身份与前端错误提示的复杂修复处理；范围确认后补齐 `design.md` 与 `implement.md`，再等待实施审批。
