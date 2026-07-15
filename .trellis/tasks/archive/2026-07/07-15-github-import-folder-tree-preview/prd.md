# GitHub 导入预览文件树

## Goal

让用户在确认 GitHub skill 导入前，不只阅读 `SKILL.md`，还能核对当前技能实际将写入 Central 的完整目录与文件结构，及时发现 `assets/`、`references/`、`scripts/` 等配套资源是否被纳入导入范围。

本任务针对重度用户的“导入前可信检查”场景：界面必须如实呈现后端导入边界，不能由前端根据 `sourcePath` 猜测或展示与实际复制范围无关的整个仓库。

## Background And Confirmed Facts

- 2026-07-15 已修复根级 skill 仓库漏导入后代目录的问题；修复前 `huashu-design` 上游有 177 个文件，本地只留下 9 个根文件，`assets/`、`demos/`、`references/`、`scripts/` 全部缺失。证据见 `.trellis/tasks/archive/2026-07/07-15-github-import-root-skill-repository/research/bug-analysis.md:11`。
- 当前共享内容边界已由 `repo_file_relative_to_source` 定义：`sourcePath = "."` 保留完整仓库相对路径，嵌套 source 只保留所选子树并去掉 source 前缀（`src-tauri/src/services/github_import/source.rs:680`）。本地导入文件收集复用该映射（`src-tauri/src/services/github_import/progress.rs:18`）。
- 本地预览已经下载完整 `GitHubRepoSnapshot`，但只把候选元数据传给 `build_preview_skills`，快照内的文件清单未进入 preview DTO（`src-tauri/src/services/github_import/source.rs:183`、`src-tauri/src/services/github_import/preview.rs:12`、`src-tauri/src/services/github_import/import.rs:637`）。
- 远程预览工作区已经包含解压后的完整仓库，并在导入时复用同一 `previewWorkspaceId`；当前候选发现只枚举 `SKILL.md`，尚未为每个候选生成内容清单（`src-tauri/src/services/github_import/source.rs:282`、`src-tauri/src/services/github_import/source.rs:431`、`src-tauri/src/services/github_import/remote.rs:197`）。
- 当前 Rust/TypeScript `GitHubSkillPreview` 只含身份、描述、路径、下载地址与冲突信息，没有文件条目或汇总字段（`src-tauri/src/services/github_import/types.rs:31`、`src/types/index.ts:737`）。
- 当前详情区只有 `overview | ai` 两个 tab，`overview` 实际只渲染 `SKILL.md`（`src/components/marketplace/githubImportWizardUtils.ts:23`、`src/components/marketplace/GitHubRepoImportWizardPreview.tsx:598`、`src/components/marketplace/GitHubRepoImportWizardPreview.tsx:635`）。

## Requirements

- 每个 `GitHubSkillPreview` 必须携带该候选实际导入范围内的完整文件清单；每个条目至少包含相对目标路径与字节数，目录节点和目录计数由前端从路径确定性派生。
- 本地 preview 文件清单必须由本次已下载 snapshot 通过导入共用的 `repo_file_relative_to_source` / collector 语义生成；不得再下载一次仓库，也不得在前端复制 source-path 规则。
- SSH/WSL preview 必须从已创建的远程 preview workspace 枚举同一候选 source 目录；根候选与嵌套候选的范围必须分别与远程 `cp -a` 的完整根目录/精确子树语义一致。
- 文件清单必须表示“将导入的技能包”，而不是无条件展示整个仓库：根 `SKILL.md` 候选显示完整仓库；`skills/<name>` 等嵌套候选只显示该子树；相对路径以最终 skill 目录为根。
- 详情 tab 将现有“概览”明确命名为 `SKILL.md`，并新增“文件树”；AI 摘要保留为第三个 tab。
- 文件树顶部显示文件数、目录数和总大小。树以最终导入 skill id（含 rename 决策）为视觉根节点，默认展开根节点和第一层目录；更深层级可逐级展开/折叠，目录行显示后代文件数以便不展开也能判断范围。
- 文件树仅用于 Preview 阶段的“预览快照内容”核对；Result 阶段不新增落盘后枚举或二次完整性校验。界面文案不得把未固定 commit 的本地 preview 描述为强一致证明。
- 文件树只负责结构与范围核对；任意资源文件内容预览、文件选择/排除、ignore 规则不进入本任务。
- 文件树在约 1 个、177 个和 650 个文件的真实量级下保持可扫描、可滚动且不撑大现有 modal；大树不能默认全部展开。
- 所有新增文案同步中英文 i18n；目录 disclosure、tab 与滚动区支持键盘操作和清晰的 `focus-visible` 状态，颜色不作为唯一状态载体。
- preview 数据缺失或枚举失败不得伪装为空目录。预览整体应返回明确错误，避免用户在不完整信息下继续确认导入。
- 保持现有技能选择、冲突处理、rename/overwrite/skip、AI 摘要、`previewWorkspaceId` 生命周期与扁平导入 selection payload 不变。

## Acceptance Criteria

- [x] 根级 fixture（`SKILL.md` + `assets/` + `references/` + `scripts/`）的 preview DTO 包含全部文件，并与 `collect_snapshot_source_files(..., ".")` 的相对路径及字节数一致。
- [x] 嵌套 fixture 只返回选中 source 子树，路径去掉 source 前缀，仓库根文件和兄弟 skill 不出现在树中。
- [x] 本地与 SSH/WSL preview 对同一仓库形状生成等价的技能相对文件清单；远程预览继续复用/清理既有 workspace。
- [x] Preview 详情出现 `SKILL.md / 文件树 / AI 导入摘要` 三个 tab；切换技能或 tab 后滚动行为稳定，不影响选择和冲突决策。
- [x] 文件树正确显示最终 skill 根名、文件/目录/总大小统计、第一层结构和可展开的深层节点；177/650 文件 fixture 不默认展开全部节点。
- [x] 文件清单失败显示明确错误并阻止进入不可信的 preview；有效 skill 的空数组或缺少根相对 `SKILL.md` 均视为 preview 契约错误。
- [x] 前端测试覆盖根/嵌套树、展开折叠、rename 后根名、技能切换、大树默认折叠、键盘与错误状态；现有 wizard selection payload 断言保持无新增显示字段。
- [x] 后端测试覆盖 snapshot 清单、远程 workspace 清单、根/嵌套边界、稳定排序、字节统计与资源/路径错误。
- [x] `pnpm typecheck`、`pnpm lint`、相关 Vitest、GitHub import Rust 定向测试、`cargo clippy -- -D warnings`、`git diff --check` 和最终 `just ci` 全部通过。

## Out Of Scope

- 再次修复 2026-07-15 已完成的根级导入内容边界缺陷，或直接修补用户当前 Central 目录。
- 点击树中任意文件后预览其内容；`SKILL.md` 继续使用现有 Markdown preview 专用链路。
- 导入前勾选/排除单个文件、配置 ignore 规则、比较本地旧树与远端新树。
- Result 阶段落盘后重新枚举、文件哈希比对、固定 preview commit 或阻止 preview/import 之间的分支漂移。
- 修改数据库 schema、repository assignment、导入 selection/result 持久化契约或 GitHub archive 资源预算。

## Notes

- 本任务跨 Rust snapshot/remote workspace、Tauri DTO、TypeScript 类型、wizard 组件、i18n 与测试，是复杂任务；`design.md` 与 `implement.md` 已补齐，状态保持 `planning`，不执行 `task.py start`。
- 用户已确认文件树只进入 Preview，不扩展 Result 二次核验。
- 当前工作树中 `package.json`、`src-tauri/Cargo.lock`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 的既有未提交改动不属于本任务。
