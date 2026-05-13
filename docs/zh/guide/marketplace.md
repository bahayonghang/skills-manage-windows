# Marketplace

Marketplace 视图聚合远端 skill 目录，让你不离开桌面就能发现并安装。它直接走 GitHub，因此任何遵循 SKILL.md 约定的公开仓库都能当作来源。

## 三个 Tab

| Tab | 内容 | 数据源 |
|-----|------|--------|
| 推荐 | 精选 skills，按 tag 分组。 | 内置在应用里（`src/data/officialSources.ts`）。 |
| 官方源 | 知名发布者及其仓库目录。 | 内置列表，随应用更新刷新。 |
| 我的源 | 你自己加入的远端来源（GitHub 仓库）。 | 本地保存在 SQLite settings 表。 |

「推荐」和「官方源」两个 Tab 在你不主动安装之前不需要网络。

## 同步过程

同步一个来源时，SkillPort 会：

1. 如果你在 Settings → GitHub PAT 配过 PAT，则带 PAT 鉴权。
2. 遍历仓库根目录和 `skills/` 子目录。
3. 解析每个 `SKILL.md` 的 frontmatter，取出 `name` 与 `description`。
4. 写入 `marketplace_skills` 表用于后续浏览。
5. 没 PAT 或 rate limit 时，自动降级为匿名请求并带重试。

缓存按来源隔离，清理某个来源不会影响其他。

## 从 Marketplace 安装

任意 Tab 选中一个 skill 后点 **安装**。SkillPort 会：

1. 把 SKILL.md 目录树下载到 `~/.skillsmanage/skills/<name>/`。
2. 记录来源，便于后续更新拉取。
3. 可选地继续走标准安装对话框，分发到各平台。

## 更新

中央视图会在 Marketplace 来源的 skill 上游有更新时显示徽标。从 Updates 面板预览改动并应用更新。原目录会被原子替换。

## 添加自定义源

在 **我的源** 中粘贴一个 GitHub URL。该来源会保存在本地，后续搜索可见。随时删除来源会清除其缓存条目。

## 下一步

- 拉取单个一次性仓库：[GitHub 导入](./github-import)。
- 安装前先看明白：[AI 解释](./ai-explanation)。

---

Last reviewed: 2026-05-04
