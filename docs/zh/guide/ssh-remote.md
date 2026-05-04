# SSH 远程

SSH 远程模式让 SkillPort 在不离开桌面的前提下管理远程 Linux 或 macOS 主机上的 skills。界面仍跑在本机；后端会打开到目标的 SSH 会话，并对远程用户目录下的 skill 操作。

## 提供什么

- 一键在 Local 与任意已注册 SSH 目标之间切换。
- 远端的中央库位于 `~/.skillsmanage/skills/`，Universal Agents 位于 `~/.agents/skills/`。
- 每个 SSH 目标拥有独立的本地缓存数据库：`~/.skillsmanage/targets/<target_id>/db.sqlite`，扫描结果与元数据按目标隔离。
- 在 Settings → Remote Targets 增删、测试与切换目标。

## 鉴权

| 方式 | 保存方式 |
|------|----------|
| Private key | 仅记录路径；私钥内容不会被 SkillPort 复制保存。 |
| 密码 | 存入系统凭据库（Keychain / Credential Manager / libsecret），不入 SQLite。 |

凭据保存后，UI 不再显示原始值。

## 当前版本支持

- 扫描远程用户目录下的中央与各平台 skill 目录。
- 用 **copy** 模式安装（当前版本对 SSH 目标禁用 symlink 模式）。
- 针对远程 skill 内容浏览详情与生成 AI 解释。

## 当前版本不支持

- 远程目标的 symlink 安装。
- 远程的 Discover（项目级）扫描。
- 文件管理器打开动作改为 **复制远程路径**，因为该路径在远程主机上，不在本机。

## 切回 Local

通过顶部栏的目标切换器切回。切回后恢复本机缓存数据库，并停止往 SSH 发命令。远程模式不会改本机 skills；Local 模式也不会触达远程主机。

## 常见问题

- **连接被拒**：确认 SSH 端口，以及远端 `sshd` 是否允许当前鉴权方式。
- **HOME 探测失败**：确认远程用户有可写 home 目录；SkillPort 登录后读取 `$HOME`。
- **安装时 permission denied**：目标用户对平台目录没有写权限；用 copy 模式并核对目录权限。

## 下一步

- 检查桌面端配置：[Settings → Remote Targets](./settings)。
- 跨机器一键批量安装：搭配 [集合](./collections)。

---

Last reviewed: 2026-05-04
