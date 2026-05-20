# 简介

`SkillPort` 是一个基于 Tauri 的桌面应用，在一个界面里统一管理多平台 AI coding agent skills。

## 项目定位

`SkillPort` 遵循 [Agent Skills](https://github.com/anthropics/agent-skills) 的开放模式，但中央技能库使用私有目录 `~/.skillsmanage/skills/`。共享的 Universal Agents 目标仍然是 `~/.agents/skills/`，只有显式安装到这里的 skills 才会被 Codex CLI、Cursor、OpenCode、Amp、Copilot 等读取该目录的工具看到。Google 当前推荐平台是 Antigravity：SkillPort 会把它的全局 skills 安装到 `~/.gemini/antigravity/skills/`，项目级 skills 使用 `.agents/skills/`。Gemini CLI 仍保留为 legacy / enterprise 兼容目标。

应用在一个窗口里同时承担四个角色：

- **管理器**：管理本地中央技能库与各平台安装。
- **浏览器**：浏览 Marketplace 发布者，导入 GitHub 仓库。
- **发现器**：扫描磁盘上的项目级 skill 库。
- **远程代理**：通过 SSH 管理远程 Linux 或 macOS 主机的 skills。

## 核心能力

- 中央技能库与按平台安装、卸载工作流。
- Claude Code 视图同时显示原生 skills 与只读的 marketplace plugin skills。
- 完整技能详情视图：Markdown 预览、原始源码视图、AI 解释生成。
- 通过技能集合整理与批量安装 skills。
- 支持扫描本地项目级 skill 库的 Discover 能力。
- 支持 Marketplace 浏览，以及带鉴权请求和重试回退的 GitHub 仓库导入。
- 通过延迟查询、懒加载索引和虚拟列表提升大规模 skill 库搜索体验。
- 中英双语界面、Catppuccin 主题、强调色、首启引导和响应式导航。

## 隐私与安全

- **本地优先**：元数据、集合、扫描结果、设置、AI 解释缓存都保存在 `~/.skillsmanage/db.sqlite` 或你自己管理的本地 skill 目录。
- **无遥测**：应用不包含分析、崩溃上报或使用追踪。
- **网络访问由功能触发**：只有显式使用 marketplace 同步、GitHub 导入或 AI 解释时才会发起外部请求。
- **SSH 只作用于当前目标**：只有当前远程目标会建立 SSH 连接；远程改动只发生在该远程用户的 skills 目录内。
- **凭据仅本地存储**：GitHub PAT 和 AI API key 保存在本地 SQLite settings 表中。

## 免责声明

`SkillPort` 是一个独立的非官方桌面应用，用于管理本地 skill 目录并导入公开 skill 元数据。它与 Anthropic、OpenAI、GitHub、MiniMax 或其他受支持平台、发布方、商标所有者均无隶属、背书或赞助关系。

## 下一步

- 安装应用：[安装](./installation)。
- 在 [GitHub](https://github.com/bahayonghang/skills-manage-windows) 阅读项目 README。

---

Last reviewed: 2026-05-04
