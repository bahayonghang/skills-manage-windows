# 中央技能库

中央技能库（Central Skills）是 SkillPort 管理的所有 skill 的唯一真实源。各平台安装、集合、Marketplace 导入，最终都向中央库汇总或从中央库分发。

## 为什么要有中央库

多个 AI 编码工具想读同一份 SKILL.md。没有真实源就会出现互相不同步的副本。SkillPort 走相反方向：

- 唯一规范目录：`~/.skillsmanage/skills/`。
- 各平台安装要么是指回中央目录的 symlink，要么是被追踪的副本。
- Universal Agents 路径（`~/.agents/skills/`）只是普通平台目标，**不是**真实源。

这意味着任何时候都可以从中央库重建整套平台安装。

## 中央技能库 V2 布局

中央技能库 V2 是默认的 Central Skills 体验。它保留原有卡片、安装、详情、AI 解释和删除流程，同时为更大的本地技能库补上更清晰的信息架构。

- **三段式侧边栏**：Smart Views、Repositories、Tags 始终作为一等筛选入口展示。
- **多维 facet 筛选**：仓库、owner、来源、标签、平台、状态可以组合，而不是互斥替换。
- **结构化搜索**：可以直接在搜索框输入 `tag:writing repo:anthropics/* owner:anthropics has:update` 这类查询。
- **URL-as-state**：搜索词、已选 facet、排序、分组和 saved view 身份都会编码到 URL query。
- **保存视图**：把“待升级”“anthropics 写作技能”等常用组合保存下来，可从侧边栏或命令面板再次打开。
- **标签分组**：把大量 tag 收拢到一级分组里，避免标签列表失控。
- **分组视图**：支持不分组、按仓库、按 owner、按标签、按更新状态分组。
- **命令面板**：按 `Ctrl+K` 可保存当前视图、创建标签分组、切换分组模式，或退回经典布局。

如果 rollout 期间需要旧版界面，可点击 V2 徽章附近的 **切回经典布局**。开发者也可以在 DevTools localStorage 中设置 `featureFlag.central.newLayout=off`，再派发 `feature-flag-change`。

## 搜索语法

搜索 key 大小写不敏感（`TAG:` 与 `tag:` 等价），value 保留原始大小写。

| 语法 | 含义 | 示例 |
|------|------|------|
| `tag:` / `-tag:` | 包含或排除标签 | `tag:writing -tag:wip` |
| `repo:` | 匹配 `owner/name`，支持 `*` 通配 | `repo:anthropics/*` |
| `owner:` | 匹配仓库 owner | `owner:anthropics` |
| `source:` | 匹配仓库来源类型 | `source:github` 或 `source:local` |
| `has:` | 匹配派生状态 | `has:update`、`has:no-tag`、`has:ai-review` |
| `platform:` | 匹配已链接的平台 id | `platform:claude-code` |
| `created:` / `updated:` | 匹配粗粒度日期或相对时间 | `updated:<30d`、`created:>2026-01-01` |

没有被解析成结构化筛选的自由文本，仍然走 skill 的 searchable text 匹配。

## 保存视图与标签分组

Saved Views 和 Tag Groups 都存储在本地 SkillPort 数据库中。它们只是中央库上的元数据，不会移动或改写 skill 文件夹。

- 删除 saved view 只会删除保存的查询，不会删除匹配到的 skills。
- 删除 tag group 会保留组内 tags，并把它们移回 **未分组**。
- 可以直接在 V2 侧边栏把 tag 分配到分组或移出分组。
- 后端 IPC 与 store 已提供 reorder 能力，后续可接拖拽 UI；当前界面依赖创建顺序和 pinning。

## 可以做什么

- **浏览**所有中央 skills，启用虚拟渲染和延迟搜索。
- **搜索和筛选**：使用结构化语法、多选 facet、保存视图和分组模式。
- **安装 / 卸载**到任意已启用平台，点击卡片上平台图标行即可切换。
- **打开 SKILL.md** 详情：渲染 Markdown 与原始源码两种视图。
- **生成 AI 解释**，每个 skill 缓存一次。
- **加入集合**用于批量安装。
- **删除**中央 skill（带确认），同时清理它的安装。

## 截图

文档站复用 README 的同一组截图资源。

![中央技能库视图](/images/01.png)

![技能详情与平台安装状态](/images/02.png)

![集合与批量工作流](/images/03.png)

## 自动中央化

当你把仅存在于某平台的 skill（比如 Discover 扫到的项目级 Claude skill）安装到另一个平台时，SkillPort 会先把它提升到中央库。流程：

```text
[Skill 只存在于 ~/.<platform>/skills/<name>]
        │
        ├─ ensure_centralized：复制 SKILL.md 目录树到
        │   ~/.skillsmanage/skills/<name>
        │
        ├─ DB 更新：canonical_path、is_central
        │
        └─ 继续正常安装：symlink 或 copy 到其余选中平台
```

整个过程对调用方透明。下一次列表刷新就能看到该 skill 出现在中央视图。

## Symlink 与 Copy

| 模式 | 行为 | 何时选 |
|------|------|--------|
| Symlink | 每个 skill 在平台目录下指回中央。 | 默认。最快，真实源唯一。 |
| Copy | 在平台目录下生成副本目录树。 | 平台或文件系统不支持 symlink（部分 Windows、受限 SSH 目标）。 |

切换模式触发的是一次卸载 + 一次安装，不会污染中央数据。

## 下一步

- 看各平台视图：[平台](./platforms)。
- 用集合做批量操作：[集合](./collections)。
- 从外部拉取：[Marketplace](./marketplace)、[GitHub 导入](./github-import)。

---

Last reviewed: 2026-05-11
