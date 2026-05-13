# 故障排查

本页汇总常见问题。每条给出症状、最可能原因、修复方式。

## 扫描结果为空

- **原因**：还没有任何平台 skills 目录，或中央路径为空。
- **修复**：至少创建一个平台 skills 目录（例如 `~/.claude/skills/`），或先从 Marketplace 导入一个 skill。SkillPort 不会替你创建平台目录。

## 用 symlink 安装的 skill 「消失」

- **原因**：symlink 目标被 SkillPort 之外的方式删了。
- **修复**：重新触发一次扫描，断链的安装记录会被清理。再从中央视图重新安装即可——规范文件仍在 `~/.skillsmanage/skills/`。

## Windows 提示 symlink 模式不可用

- **原因**：未启用开发者模式，或文件系统不允许非管理员创建 symlink。
- **修复**：把该平台的安装方式切换为 **copy**，或在 Windows 设置 → 隐私和安全 → 开发者选项 启用开发者模式。

## GitHub Marketplace 同步 403

- **原因**：匿名速率限制，或 PAT 缺少 scope。
- **修复**：在 Settings → GitHub PAT 添加 PAT，或等速率限制窗口重置。私有仓库需要 PAT 带 `repo` scope。

## AI 解释一直没返回

- **原因**：base URL 配错、API key 过期、提供商不可用。
- **修复**：进 Settings → AI 重新测试凭据。请求一直挂起时，切换提供商或模型再试。

## SSH 目标已连上但扫描结果为空

- **原因**：远程用户还没创建任何平台目录（新服务器常见），或 `$HOME` 解到非预期路径。
- **修复**：确认远程登录后落到预期 home；检查主机上 `~/.<platform>/skills/` 是否存在。

## 升级后数据库看起来是空的

- **原因**：数据库文件被移动，或已切换到了 SSH 目标。
- **修复**：确认 `~/.skillsmanage/db.sqlite` 存在且可读。如果切换过目标，切回 Local。SkillPort 升级不会做破坏性迁移。

## 下一步

- 平台相关固定说明：[平台](./platforms)。
- 配置扫描路径与可见性：[设置](./settings)。

---

Last reviewed: 2026-05-04
