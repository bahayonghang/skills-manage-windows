# Design

## Scope

在共享 GitHub 导入向导的 Preview 详情区增加只读文件树。后端 preview DTO 携带当前候选技能包的扁平文件清单，前端确定性派生目录节点、统计和展开状态。

本任务不改变候选发现、导入 selection、冲突决策、持久化、实际复制逻辑或 Result 页面。它把已经存在的内容边界显式呈现出来，而不是新增第二套“哪些文件应该导入”的判断。

## UX Direction

### User And Scene

独立开发者在桌面端准备把不熟悉或刚修复过的 GitHub skill 导入中央库，处于谨慎核对状态；他们需要快速确认 `assets/`、`references/`、`scripts/` 等运行资源是否在预览范围内，再进入 Confirm。

### Visual Lane

- 沿用 SkillPort 的 Restrained 调度台风格、当前主题 token、等宽正文和高信息密度。
- 参考 Raycast 的紧凑检查面板、VS Code Explorer 的目录 disclosure、GitHub 文件树的目录优先排序；不引入新配色、玻璃面板或嵌套卡片。
- 现有详情标题、冲突按钮和 tab 布局不重排。只把 tab 明确为 `SKILL.md / 文件树 / AI 导入摘要`，让“内容说明”和“包结构”成为并列证据。

## User Flow

```text
Preview repository
  -> select a discovered skill on the left
  -> SKILL.md tab explains behavior
  -> Files tab shows preview-snapshot boundary and totals
       root label follows current final skill id
       root + first directory level visible
       deeper folders expand on demand
  -> user resolves skip / overwrite / rename
  -> Review import (unchanged)
```

切换 skill 时保留当前 tab 选择，但重置该技能文件树的滚动位置；rename 只更新视觉根名，不改文件相对路径。重新 Preview 会用新 DTO 替换整棵树。

## DTO Contract

新增只用于展示的文件条目：

```rust
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubSkillPreviewFile {
    pub path: String,
    pub byte_len: u64,
}

pub struct GitHubSkillPreview {
    // existing fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<GitHubSkillPreviewFile>>,
}
```

TypeScript 镜像为 `files?: GitHubSkillPreviewFile[] | null`。`path` 是相对最终 skill 根目录的 `/` 分隔文件路径；不发送目录条目，避免同一目录在后端和前端分别维护。`byteLen` 是未压缩文件字节数。

`files` 保持 optional，而不是给所有 `build_preview_skills` 调用默认空数组：

- GitHub 导入 preview 命令必须填充 `Some(files)`，且至少包含根相对 `SKILL.md`；无法证明清单完整时 preview 整体失败。
- CLI、skills.sh、Marketplace 同步和 Central remote-added 等共享 `GitHubSkillPreview` 的调用继续得到 `None`，序列化时省略该字段，不承担文件枚举和大 DTO 成本。
- `None` 表示“该调用没有请求文件清单”，`Some([])` 不用于有效 skill。前端向导把缺失清单视为契约错误，而不是空状态。

清单按 `path` 稳定排序。文件计数和总字节数由条目直接汇总；目录数、目录后代文件数和目录树由前端确定性派生，不向 DTO 添加重复汇总字段。

## Local Preview Data Flow

当前 local preview 已经通过 `fetch_repo_skill_candidates_from_source` 下载完整 archive，但 helper 丢弃 snapshot 后只返回候选。调整后的导入 preview 路径直接保留同一次 snapshot：

```text
resolve repo
  -> download_repo_snapshot once
  -> build_repo_skill_candidates_from_snapshot_at_path
  -> build_preview_skills (identity/conflict; files=None)
  -> for each preview skill:
       collect/map snapshot entries through repo_file_relative_to_source
       convert to GitHubSkillPreviewFile
       validate SKILL.md exists
       set files=Some(sorted manifest)
  -> GitHubRepoPreview
```

`fetch_repo_skill_candidates_from_source` 继续服务 Marketplace 等现有调用，不为了本任务改成携带大 snapshot 的通用返回类型。Preview 不再追加第二次下载。

本地文件 membership 必须复用 `repo_file_relative_to_source`，或复用内部同语义 collector 后只转换 DTO：

- `sourcePath = "."`：保留 snapshot 内所有文件及原相对路径；
- 嵌套 source：只保留精确子树并去掉 source 前缀；
- 兄弟目录和仓库根文件不进入嵌套 skill 清单。

## SSH / WSL Preview Data Flow

远程 preview workspace 已经解压完整仓库，并通过 `previewWorkspaceId` 在后续 import 中复用。候选发现完成后新增一次有界的递归文件 inventory：

```text
remote preview workspace
  -> one remote command enumerates regular repo files + byte sizes
  -> parse to repo-relative flat entries
  -> for each candidate apply repo_file_relative_to_source
  -> validate + sort + attach files
```

只枚举一次完整 workspace，再在 Rust 内为多个候选切分；禁止每个候选各跑一次 `find`。命令必须通过 `ConnectedRemoteTarget::run_script` 和参数传递，不拼接未转义路径。输出协议需能明确分隔路径与大小，解析失败、非 UTF-8、非法/空路径、超出既有 archive file budget 或缺少 `SKILL.md` 都使 preview 失败。

远程 inventory 不读取文件内容，也不新增网络下载。根/嵌套 membership 仍调用共享 `repo_file_relative_to_source`；它与 import 的 `remote_skill_source_dir` + `cp -a` 语义必须通过邻接测试锁定。

## Frontend Tree Model

优先复用 `src/lib/fileTree.ts` 的路径归一化、目录优先排序和 `DirectoryTreeEntry` 约定；若现有 builder 的 `SkillsShFileEntry` 输入过窄，则抽取一个小型纯 builder 接受 `{ path, byteLen }`，让 skills.sh 和导入预览分别适配输入。不要复制一份目录构建算法到 wizard 文件。

现有 `SkillDetailFileTree` 不直接复用为导入 UI：它把文件节点作为“打开内容”按钮，并递归渲染当前展开子树；本任务明确不打开任意文件，且后端归档上限允许 20,000 个文件。导入向导使用专用的只读展示壳，但复用共享 tree model、Lucide 图标、按钮/focus 样式和排序规则。

派生节点至少包含：

```ts
type GitHubImportTreeNode = {
  name: string;
  path: string;
  kind: "directory" | "file";
  byteLen: number;
  descendantFileCount: number;
  children: GitHubImportTreeNode[];
};
```

目录 `byteLen` 为后代文件大小汇总；只展示后代文件数，避免信息过密。文件行右侧显示格式化大小。路径分段不得接受空段、`.`、`..` 或反斜杠；后端已保证安全，前端校验是 DTO 防御，不是另一套导入安全边界。

## Layout And Interaction

Files tab 使用一个无嵌套卡片的纵向面板：

1. 紧凑摘要行：`177 个文件 · 4 个目录 · 2.8 MB`，辅以“预览快照”文本，避免暗示 Result 已校验。
2. 单一根行：folder icon + 当前最终 skill id；rename 时即时更新。
3. 可展开树：目录优先、名称稳定排序，root 默认展开，第一层目录默认展开，更深目录折叠。

目录 disclosure 使用原生 `button`、`aria-expanded`、明确的展开/折叠 label 与 `focus-visible` ring；Enter/Space 可操作。文件行不伪装成按钮。颜色只辅助 icon，不承担文件/目录区分。

树视口在 tab panel 内占满可用高度并独立滚动。先把展开状态转换成固定行高的 visible rows，再复用现有 `VirtualizedList` 或等价仓库内机制，保证最坏 20,000 个根级文件不会一次挂载全部 DOM。虚拟列表只渲染可见行，缩进由 depth 决定；不新增前端依赖。

摘要、树根和滚动树之间使用分隔线与背景层级，不再套卡片。详情 modal 尺寸、左右栏比例和 footer 保持不变。

## States

| State | Behavior |
| --- | --- |
| Ready | 显示统计、视觉根和目录树 |
| Single-file skill | 显示 `1 个文件 · 0 个目录` 与 `SKILL.md`，不制造空状态 |
| Large tree | 第一层可见、深层折叠、visible rows 虚拟化 |
| Skill switch | 新树替换旧树，Files tab 保留，scroll 回顶 |
| Rename | 只更新视觉根 id，统计与相对路径不变 |
| Re-preview | 整体 preview DTO 原子替换，不混用旧清单 |
| Missing / invalid manifest | 不进入 Preview，复用外层 preview error 与 Retry |
| Browser fixture | fixture 提供文件清单，可演示树；不依赖 Tauri 读取文件 |

不增加 Files tab 内的 loading spinner：文件清单随 preview DTO 一次性返回，外层现有 Preview loading/skeleton 已经覆盖等待过程。

## Compatibility And Honest Limits

- `GitHubSkillImportSelection`、`ImportedGitHubSkillSummary`、`GitHubRepoImportResult`、数据库行和 repository assignment 不增加文件字段。
- `pluginName` 仍是 preview-only grouping；文件清单同样仅是 import preview display metadata，不进入选择 payload。
- local preview 与 import 目前会分别获取分支快照。本任务展示“预览时检测到的快照”，不固定 commit，也不承诺仓库在 Preview 与 Confirm 之间变化时清单仍相同；用户可用现有 Re-preview 刷新。
- remote import 继续复用 preview workspace；本任务不改变其 TTL、清理或 fallback 行为。
- 文件清单受现有 archive 20,000 文件 / 256 MiB expanded budget 约束，不提高资源预算。

## Spec Update

实现时更新 `.trellis/spec/backend/github-import-preview-contract.md`，增加 Preview File Manifest 场景，固定：

- optional DTO 字段与仅 import wizard 强制填充的边界；
- root / nested membership；
- local snapshot 与 remote workspace 等价性；
- display-only，不进入 selection/result/persistence；
- 缺失清单必须 fail closed。

## Rollback

回滚时一起移除 preview file DTO、local/remote 清单填充、前端 Files tab 与 i18n。因为没有 schema、selection 或落盘行为变化，不需要数据迁移；旧客户端会忽略新增字段，新后端也可继续服务不认识文件字段的旧前端。
