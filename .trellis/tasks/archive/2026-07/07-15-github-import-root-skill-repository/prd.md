# 支持根级 SKILL.md 仓库完整导入

## Goal

修复 GitHub 导入与中央更新对根级 skill 仓库的内容截断：当仓库根目录包含 `SKILL.md` 时，预览仍将它识别为一个 skill，实际导入、哈希比较和后续更新则以完整仓库快照为内容边界，保留所有后代文件及其相对路径。

## User Value

用户从 GitHub 导入 `huashu-design`、`yao-meta-skill` 这类根级 skill 仓库后，可以直接使用其 `assets/`、`references/`、`scripts/` 等配套资源；中央更新也不会再把缺少这些目录的损坏副本误报为 `up_to_date`。

## Confirmed Facts

- `alchaincyf/huashu-design` 的 `master` 分支根目录包含 `SKILL.md` 和 `assets/`、`demos/`、`references/`、`scripts/`；2026-07-15 核对的 HEAD 为 `0e7ec8aca0058184c1a9e06e57697e84f68a3f0f`，完整树含 177 个文件。
- 本地 Central 的 `huashu-design` 与上游根 `SKILL.md` Git blob hash 同为 `e86780d280ea57bb8e67cc2d5a5dbe32e913e056`，但只剩 9 个根级文件，上述 4 个目录全部缺失。
- `yaojingang/yao-meta-skill` 的 `main` 分支根目录包含 `SKILL.md` 和 20 个顶层目录；2026-07-15 核对的 HEAD 为 `4eb11f923dc71173736ebf541a7eebfff942d10e`，完整树含 650 个文件。
- 本地 Central 的 `yao-meta` 与上游根 `SKILL.md` Git blob hash 同为 `a2c7a6e8974d698fd8716daa642d3e0036727820`，但只剩 9 个根级文件；本地 `SKILL.md` 引用的 `references/`、`scripts/` 均不存在。
- 数据库把 `huashu-design` 和 `yao-meta` 正确关联到各自 GitHub 仓库并记录 `source_path = "."`，但两者当前更新状态仍为 `up_to_date`。
- `vercel-labs/agent-browser` 不是根级 skill 仓库：仓库根目录没有 `SKILL.md`，已安装 skill 的来源是 `skills/agent-browser`；该子树只有一个 `SKILL.md`，其上游与本地 Git blob hash 同为 `bdd73cc60a51261b0d18e3d3d646cba9e6280bc2`。
- 候选发现已经为根清单生成 `sourcePath = "."`，并以仓库名生成稳定 ID；首次导入和更新分别在 `src-tauri/src/services/github_import/progress.rs:26` 与 `src-tauri/src/services/central_updates/fs.rs:123` 把根来源错误限制为不含 `/` 的顶层文件。
- SSH/WSL GitHub 直接导入在 `src-tauri/src/services/github_import/remote.rs:408` 将根来源映射为整个远端仓库目录，并通过 `cp -a "$source_dir/."` 递归复制，当前缺陷造成 local snapshot 与 remote direct import 语义不一致。
- 当前测试覆盖根候选预览和嵌套 skill 的递归复制，但没有以 `sourcePath = "."` 验证根级后代文件的导入、哈希或更新。

## Requirements

- 根级 `SKILL.md` 候选的内容范围必须包含 GitHub 仓库快照中的全部文件，而不只是根级文件；相对路径必须保持不变。
- 显式或发现得到的非根 `sourcePath` 必须继续只包含该 source 子树，不能把仓库其他内容带入 skill 目录。
- 首次完整导入、部分导入、普通更新、强制更新/仓库镜像与远端目标更新必须遵守同一 sourcePath 内容范围契约。
- 根级后代文件必须参与远端哈希；已有截断副本在下一次新鲜检查中应变为可更新，并可通过现有原子更新流程修复。
- 更新后删除上游已经移除的根级文件，继续保留现有 staging、backup、回滚和 copy-install refresh 语义。
- 不改变根候选身份、数据库 schema、repository assignment、IPC DTO、导入选择 payload、冲突处理或前端交互。
- 保持 `skill/SKILL.md` 仓库级容器、`skills/<name>/SKILL.md`、`.agents/skills/<name>/SKILL.md`、插件 manifest 分组、深层泛化目录过滤和 `agent-browser` 嵌套单文件 skill 的既有行为。
- 根级整仓复制继续受现有 GitHub archive 资源预算与路径安全校验约束，不新增按仓库名特判或未声明的文件排除表。

## Acceptance Criteria

- [x] 一个包含根 `SKILL.md`、`references/guide.md`、`scripts/run.py` 和 `assets/example.txt` 的 snapshot 只产生一个根候选，导入后四类文件均按原相对路径存在。
- [x] 根来源的文件收集、进度统计和写入包含所有后代文件；嵌套来源仍排除 source 子树之外的仓库文件。
- [x] 根来源的远端 hash 包含后代文件；仅新增或修改 `references/`、`scripts/` 等后代文件即可触发 `update_available`。
- [x] 对已有顶层文件相同、后代目录缺失的 Central skill 执行普通或强制更新后，完整远端树落盘，旧的多余文件被移除，repository assignment 仍为 `source_path = "."`。
- [x] 更新写入失败时恢复原目录，不留下 `.skillport-import-*`、`.skillport-update-*` 或 `.skillport-backup-*` 临时目录。
- [x] `agent-browser` 形状的 `skills/agent-browser/SKILL.md` 仍只导入该子树，不复制仓库根、`skill-data/` 或其他源码目录。
- [x] 根级 `skill/` 容器、命名 skill 目录、多 skill 仓库、插件分组、深层泛化过滤以及 SSH/WSL 直接递归复制的既有测试保持通过。
- [x] 本任务 Rust 文件的 scoped `rustfmt --check`、GitHub import 定向测试、Central updates 定向测试、`cargo clippy -- -D warnings`、`git diff --check` 和最终 `just ci` 全部通过。

## Out of Scope

- 修改三个上游仓库或直接修补当前 `~/.skillsmanage/skills/` 内容。
- 把没有根 `SKILL.md` 的任意仓库当作单个整仓 skill。
- 为根级 skill 增加文件挑选 UI、ignore 规则或仓库级白名单。
- 重构 GitHub 导入 UI、数据库 schema、错误协议、资源预算数值或 Tauri 打包链路。

## Notes

- 本任务按一个复杂后端缺陷处理：候选身份不变，修复的是 import/update 共享的内容范围契约，因此不拆父子任务。
- 详细证据见 `research/bug-analysis.md`；技术方案和执行顺序分别见 `design.md`、`implement.md`。
- 相关历史任务：`.trellis/tasks/archive/2026-07/07-10-github-import-skill-directory-false-positive/`。
- 仓库级 `cargo fmt --check` 仍会报告本任务范围外的既有格式漂移；为避免改动用户工作，本任务只对 7 个变更 Rust 文件执行并通过 scoped rustfmt 校验，详见 `check.md`。
