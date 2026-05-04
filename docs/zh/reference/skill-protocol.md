# 技能协议

技能是包含 `SKILL.md` 的目录，文件由 YAML frontmatter 与 markdown 正文组成。SkillPort 遵循 [Agent Skills](https://github.com/anthropics/agent-skills) 开放格式。

## 目录布局

```text
my-skill/
├── SKILL.md              必需：frontmatter + markdown 正文
├── reference/            可选：agent 可读的参考文档
│   └── api.md
├── scripts/              可选：可执行 helper
│   └── lint.sh
└── assets/               可选：图片、fixture、prompt
    └── prompt.txt
```

技能 ID 即目录名。展示名与描述写在 frontmatter。

## SKILL.md

```markdown
---
name: my-skill
description: 一句话说明何时使用此技能。
version: 1.0.0
---

# My Skill

agent 会读取的 markdown 正文，作为工作指令。
```

### 必填字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `name` | string | 展示名；建议与目录名一致 |
| `description` | string | 短句，作为搜索和卡片摘要 |

### 可选字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `version` | string | semver 风格；详情页展示 |
| `tags` | string[] | 自由标签；与 SkillPort 本地标签合并 |
| `author` | string | 仅展示 |
| `homepage` | string | 仅展示 |

其他字段在 SkillPort 写回磁盘时保留，但 UI 不解读。

## 正文

正文通过 `react-markdown` + GFM 渲染。代码块按语言 fence 着色。

## 校验

| 失败 | 行为 |
| --- | --- |
| 没有 `SKILL.md` | 目录被忽略 |
| YAML 损坏 | 用文件名作为 `name`，描述回退到第一段 |
| 缺 `name` / `description` | 用目录名替代；设置 → 诊断 中显示告警 |

## 更新身份

远端更新（Marketplace / GitHub 导入）按 `(repository_id, source_path)` 匹配，目录改名不会产生重复行。

## 删除安全

删除 Central 技能时，先遍历 `skill_installations` 移除每个 symlink / copy；只有所有平台侧成功后，才删除 DB 行。

Last reviewed: 2026-05-04
