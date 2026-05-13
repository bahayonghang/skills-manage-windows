# GitHub 导入

GitHub 导入是一个向导，用于从任意指定的 GitHub 仓库拉取 skills。它与 Marketplace 维护精选列表的方式不同，更适合一次性引入——可以是你自己的仓库，也可以是同事的实验项目。

## 向导流程

```text
[向导] ──┬── 步骤 1：粘贴仓库 URL 或 owner/repo
         ├── 步骤 2：选择分支与源路径
         │           （根目录、skills/ 或任意子目录）
         ├── 步骤 3：预览发现的 SKILL.md
         │           （name、description、目标中央路径）
         └── 步骤 4：确认导入 → 写入
                     ~/.skillsmanage/skills/
```

预览阶段只读，确认前不写任何文件。

## 鉴权

- **GitHub PAT（推荐）**：保存在本地 settings 表。配额 5000 次/小时，并支持私有仓库。
- **匿名兜底**：未配置 PAT 时使用。配额 60 次/小时，仅支持公开仓库。向导在 rate limit 错误下自动重试。

PAT 只发送到 `api.github.com`。在 Settings → GitHub PAT 中增删。

## 导入什么

对发现的每个 SKILL.md，向导会把*整个 skill 目录*一起拷过去——SKILL.md 同级的所有文件（scripts、references、assets）都进入中央存储。`.git` 等隐藏文件会跳过。

如果同名 skill 已存在：

- **跳过** — 保留现有版本。
- **替换** — 用新目录树覆盖。
- **重命名** — 用另一个名字保存新版本。

## 导入之后

导入的 skills 在中央视图带来源标签，指向原 GitHub URL。更新通过与 Marketplace 共用的 Updates 面板展示。

## 常见坑

- **找不到 SKILL.md**：向导只在选定源路径下递归一层。仓库嵌套较深时，直接把向导指向那个子目录。
- **API 403**：仓库私有但缺 PAT 或 PAT 没有 `repo` scope。补一份带正确 scope 的 PAT。
- **frontmatter 解析错误**：向导跳过无效条目并在预览中给出警告；修好源仓库再跑一次。

## 下一步

- 跟踪导入来源的更新：[Marketplace](./marketplace) → Updates 面板。
- 快速看清一个 skill 的内容：[AI 解释](./ai-explanation)。

---

Last reviewed: 2026-05-04
