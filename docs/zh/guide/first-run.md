# 首次启动

首次启动 SkillPort 时，应用会准备好本地数据存储并触发一次全盘扫描，让界面有东西可展示。本页解释幕后发生了什么，以及你应该看到什么。

## 会创建什么

| 路径 | 用途 |
|------|------|
| `~/.skillsmanage/` | 应用数据根目录 |
| `~/.skillsmanage/db.sqlite` | SQLite 数据库（WAL 模式） |
| `~/.skillsmanage/skills/` | 中央技能库 — 唯一真实源 |
| `~/.skillsmanage/targets/<id>/db.sqlite` | SSH 目标的独立缓存（按需创建） |

数据库会自动初始化。该目录名保留是为了让已有安装继续使用旧数据。

## 启动扫描

```text
[应用启动] ──┬── 读取内置 agent 注册表
             ├── 遍历每个启用平台的 ~/.<platform>/skills/
             ├── 遍历 ~/.skillsmanage/skills/ 作为中央库
             ├── 读取已配置的项目扫描目录
             ├── 解析 SKILL.md frontmatter（name、description 等）
             ├── 通过 lstat 检测 symlink 关系
             └── 写入 skills 与 skill_installations 表
```

扫描是幂等的，可以反复触发。顶部栏提供随时重扫的入口。

## 你应该看到什么

- 左侧导航出现 **中央技能库** 视图，列出 `~/.skillsmanage/skills/` 下的 skills。
- 检测到的平台分到 **Coding** 与 **Lobster** 两组。已存在 skills 目录的平台变成可点击行；其余保持灰色，需要你显式启用。
- 顶部搜索框使用延迟查询，输入不会阻塞扫描。

## 扫描结果为空

- 确认至少有一个平台的 skills 目录存在（比如 `~/.claude/skills/`）。应用本身不会创建平台目录。
- 进入 Settings → 扫描目录。中央路径与内置平台路径默认存在；自定义项目路径只有你之前添加过才会出现。
- 创建目录后从顶部栏重新触发扫描。旧结果会原子替换，不保留中间态。

## 下一步

- 浏览和安装：[中央技能库](./central-skills)。
- 管理各平台安装：[平台](./platforms)。
- 从网络拉取：[Marketplace](./marketplace) 或 [GitHub 导入](./github-import)。

---

Last reviewed: 2026-05-04
