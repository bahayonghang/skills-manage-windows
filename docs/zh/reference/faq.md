# 常见问题

最常被问到的问题。

## 通用

**SkillPort 是 Anthropic / OpenAI / GitHub 的官方应用吗？**
不是。SkillPort 是 [`iamzhihuix/skills-manage`](https://github.com/iamzhihuix/skills-manage) 的独立非官方 fork，与上述任何平台厂商无任何隶属、背书或合作关系。

**数据存在哪里？**
`~/.skillsmanage/db.sqlite` 是 SQLite 数据库；`~/.skillsmanage/skills/` 是中央技能库。两个目录名沿用历史命名，便于已有安装平滑升级。

**AI 解释会发到厂商吗？**
仅在你点击 "解释" 或 "批量解释" 时才发。请求按设置中配置的 AI 提供方（Claude / GLM / Kimi / DeepSeek / OpenRouter / MiniMax）发出。后台没有任何遥测。

## 安装

**Windows 提示无法创建符号链接怎么办？**
Windows 创建 symlink 需开发者模式或管理员权限。SkillPort 检测到失败会自动回退为 copy 模式。

**macOS 提示应用 "已损坏" 怎么办？**
当前 macOS 包未签名，Gatekeeper 会隔离它。把应用拖进 `/Applications` 后执行：

```bash
xattr -dr com.apple.quarantine "/Applications/SkillPort.app"
```

然后从 Finder 启动。

**需要单独安装 Tauri 运行时吗？**
不需要。安装包是自包含的；Tauri 前置依赖只在自行构建开发版时才需要。

## 技能

**新加的技能为什么没有显示？**
SkillPort 只扫描已配置目录。要么把技能放到 `~/.skillsmanage/skills/`（Central），要么在 设置 → 扫描目录 中加入父目录。

**为什么 Discover 里同一个技能出现两次？**
0.10.0 之前的版本不去重共享根（如 `.agents/skills`）；0.10.0 已合并去重。如果还能看到重复，去 设置 触发一次完整重扫。

**怎样一次性从所有平台卸载某技能？**
在技能详情页点 "从所有平台卸载"。该动作会遍历 `skill_installations` 移除每个 link / copy；如果同时勾选 "从中央删除"，会再删除 Central 行。

## Marketplace 与 GitHub 导入

**同步报 rate limit。**
GitHub 匿名请求每小时 60 次。在 设置 → GitHub 中填入 PAT（公开仓库无需 scope）。

**私有仓库导入失败。**
使用细粒度 PAT 并授予对应仓库读权限；经典 PAT 也行，需 `repo` scope。

## SSH 模式

**为什么远端安装总是 copy？**
跨 SSH 的 symlink 行为依赖文件系统与 shell。SkillPort 默认在远端使用 copy 保持可预期行为。本版本不启用 symlink 与远端 Discover。

**密码会以明文保存吗？**
不会。SSH 密码进入操作系统凭据存储（Keychain / Credential Manager / libsecret）。私钥文件在连接时读取，不会被复制。

## 数据卫生

**GitHub PAT 存在哪里？**
`settings` 表 key `github.pat`，未加密存储。请用 OS 用户权限保护好 `~/.skillsmanage` 目录。

**怎样在两台机器之间迁移？**
设置 → 数据 → 导出，把 JSON 拷到目标机后导入。导出时密钥已被剥离，请在新机器重新填入。

Last reviewed: 2026-05-04
