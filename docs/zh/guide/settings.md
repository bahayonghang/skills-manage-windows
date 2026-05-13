# 设置

Settings 是唯一的配置面板，所有配置项都在这里。各区段彼此独立；改动立即落库，并对当前目标（Local 或当前 SSH 目标）生效。

## 区段

| 区段 | 用途 |
|------|------|
| 扫描目录 | 增删 Discover 要扫的项目路径。 |
| 自定义平台 | 定义新平台（id、显示名、skills 目录、分类）。 |
| 平台可见性 | 隐藏不用的平台；后台仍扫描。 |
| Remote Targets | [SSH 远程](./ssh-remote) 用到的 SSH 目标。 |
| GitHub PAT | 用于 [Marketplace](./marketplace) 与 [GitHub 导入](./github-import) 的个人访问令牌。 |
| AI | [AI 解释](./ai-explanation) 的提供商与 key。 |
| 关于 | 应用版本、数据库路径、更新日志与安全说明链接。 |

## 行为约定

- **本地优先**：所有值保存在 `~/.skillsmanage/db.sqlite`（或对应 SSH 目标缓存库 `~/.skillsmanage/targets/<id>/db.sqlite`）。
- **原子更新**：设置改动立即提交，没有独立的「保存」按钮；关闭对话框不会回滚。
- **不做隐式迁移**：切换 SSH 目标时换的是缓存文件，不会修改原文件。
- **隐藏字段**：密钥（GitHub PAT、AI key、SSH 密码）保存后只显示掩码，可以清除但无法再读取明文。

## 全新安装的推荐顺序

1. 准备用 Marketplace 或 GitHub 导入：先加 **GitHub PAT**。
2. 想生成解释：配置 **AI** 提供商。
3. 想被 Discover 扫到的项目目录：在 **扫描目录** 里加。
4. 用不到的平台：在 **平台可见性** 里隐藏。
5. 管理 SSH 主机：加 **Remote Targets**。

以上配置项都可以随时再调整。基础的中央 → 平台安装工作流不要求任何前置配置。

## 下一步

- 主题与语言：[国际化与主题](./i18n-and-themes)。
- 排查平台相关问题：[故障排查](./troubleshooting)。

---

Last reviewed: 2026-05-04
