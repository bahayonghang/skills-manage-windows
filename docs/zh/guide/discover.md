# Discover

Discover 在本地磁盘上扫描项目级 skill 库，也就是那些含有 `SKILL.md` 但又不在各平台全局目录里的目录。它是把仓库自带或团队共享 skills 引入应用的主要入口。

## 扫描范围

- **设置 → 扫描目录** 中登记的所有目录。
- 这些目录下的常见项目级 skill 路径：`.claude/skills/`、`.cursor/skills/`、`.agents/skills/`、`.factory/skills/`，以及其他平台对应路径。
- macOS 上还会扫描 `/Applications`，覆盖把 skill 打进应用包的场景。

扫描全程只读，绝不修改源文件，仅读取 `SKILL.md` 并记录发现的内容。

## 视图布局

页面分两栏：

- **左面板** — 检测到的项目列表，按根目录分组，每项显示该项目下的 skill 数量。
- **右面板** — 所选项目的详情：发现的 skills、推断的所属平台、快捷操作。

## 导入发现的 skill

导入一个项目 skill 时，SkillPort 会：

1. 如果尚未中央化，先把它提升进中央库（`ensure_centralized`）。
2. 写入一条安装记录，让原平台仍然能看到它。
3. 可选地通过标准安装对话框再装到其他平台。

原始项目文件保持不动；只有副本进入中央库。

## 刷新

Discover 是按需的。在以下场景手动点页面刷新：

- 在 Settings 里增删了扫描目录。
- 新克隆了一个含 skills 的项目。
- 应用之外编辑了 skills（文件监听不会覆盖任意外部路径）。

## Discover vs Marketplace

| 场景 | 推荐入口 |
|------|----------|
| 本地项目自带的 skills | Discover |
| 厂商或社区发布的 skills | [Marketplace](./marketplace) |
| 想镜像某个 GitHub 仓库 | [GitHub 导入](./github-import) |

## 下一步

- 配置扫描目录：[设置](./settings)。
- 把项目 skills 提升进中央库：[中央技能库](./central-skills)。

---

Last reviewed: 2026-05-04
