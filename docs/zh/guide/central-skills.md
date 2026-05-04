# 中央技能库

中央技能库（Central Skills）是 SkillPort 管理的所有 skill 的唯一真实源。各平台安装、集合、Marketplace 导入，最终都向中央库汇总或从中央库分发。

## 为什么要有中央库

多个 AI 编码工具想读同一份 SKILL.md。没有真实源就会出现互相不同步的副本。SkillPort 走相反方向：

- 唯一规范目录：`~/.skillsmanage/skills/`。
- 各平台安装要么是指回中央目录的 symlink，要么是被追踪的副本。
- Universal Agents 路径（`~/.agents/skills/`）只是普通平台目标，**不是**真实源。

这意味着任何时候都可以从中央库重建整套平台安装。

## 可以做什么

- **浏览**所有中央 skills，启用虚拟渲染和延迟搜索。
- **安装 / 卸载**到任意已启用平台，点击卡片上平台图标行即可切换。
- **打开 SKILL.md** 详情：渲染 Markdown 与原始源码两种视图。
- **生成 AI 解释**，每个 skill 缓存一次。
- **加入集合**用于批量安装。
- **删除**中央 skill（带确认），同时清理它的安装。

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

Last reviewed: 2026-05-04
