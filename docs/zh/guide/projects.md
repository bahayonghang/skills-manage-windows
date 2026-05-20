# 项目（Projects）

Projects 提供「按项目逐个管理」的视图，覆盖那些活在代码仓库里、放在 `.claude/skills/`、`.kiro/skills/`、`.agents/skills/` 等位置的项目级 SKILL.md 目录。

和过去启动即全盘扫描的 Discover 不同，Projects 是**手动注册式**：你 add 自己关心的项目根，SkillPort 只在那些根上扫描，扫描时机由你决定。

## 添加项目

1. 侧边栏点 **项目**。
2. 点 **添加项目**，选文件夹，确认。
3. SkillPort 落库后立刻返回，扫描在后台跑，完成后回填 skill 数。

同一个路径添加多次幂等——已存在的项目会被直接重新选中，不会重复写库。

## 扫描范围

每个项目根下，SkillPort 只走**已启用 agent** 各自的项目级 skill 目录：

- Claude Code：`.claude/skills/`
- 其他平台：`.kiro/skills/`、`.codex/skills/`、`.opencode/skills/`，以及 Antigravity 等 Universal-compatible agent 使用的共享 workspace 路径 `.agents/skills/`。Legacy Gemini CLI 行继续兼容；Antigravity 项目 skills 通过 `.agents/skills/` 表示。

未启用的 agent 跳过；`central` agent 永远跳过（项目本身不是中央库）。

每条 SKILL.md 写入 `project_skill_installations`，按 `(project, skill_id, agent)` 唯一。Symlink 标 `symlink`，实目录标 `copy`。

## 视图布局

页面分两栏：

- **左面板** — 项目列表，支持搜索和置顶；hover 行尾露出 pin / rename / remove 三个快捷键。
- **右面板** — 当前项目下的已安装 skill：每张卡片显示所属 agent、安装方式徽章（symlink 绿、copy 琥珀），以及一键卸载按钮。

## 从中央库安装

右面板的 **从中央库安装** 按钮打开一个对话框：

1. 列出可搜索的中央 skill。
2. 列出有项目级目录的启用 agent。Universal workspace 目标会折叠成一组；选择 Universal 会通过代表成员安装到 `.agents/skills/`。
3. 给出 symlink（默认）和 copy 两个安装方式 radio。

确认后调 `install_skill_to_project`，在项目对应的 agent 目录下落 SKILL.md 文件夹，同时写一行 psi。

## 卸载

点 skill 卡片上的垃圾桶。SkillPort 删盘上的目录（或 symlink），清掉 psi 行。中央库的 skill 本体不受影响。

## 项目操作

- **置顶（Pin）** — 排到列表最前，不受最后扫描时间影响。
- **重命名** — 只改显示名，磁盘路径不动。
- **移除** — 弹窗里有 **同时卸载本项目下所有已装 skill** 复选框：
  - 不勾（默认）只删表，磁盘保留。
  - 勾上则先遍历 psi 删盘上文件，再删项目记录。

## Projects vs Marketplace

| 场景 | 推荐入口 |
|------|----------|
| 本地仓库自带的 skills | Projects |
| 厂商或社区发布的 skills | [Marketplace](./marketplace) |
| 想镜像某个 GitHub 仓库 | [GitHub 导入](./github-import) |

## 从 Discover 迁移

旧的 Discover 页是全盘爬，扫描候选根写死在代码里，用户加不了任意路径，深度上限还会漏掉嵌套深的项目。0.10.x 起被 Projects 整体替换。

访问 `/discover` 会重定向到 `/projects`，并展示一次性顶部条幅。旧的 discovered 表在首次升级时被清空，请重新 add 你关心的项目。

## 下一步

- 把项目 skill 提升进中央库：[中央技能库](./central-skills)。
- 配置哪些 agent 启用：[平台](./platforms)。

---

Last reviewed: 2026-05-14
