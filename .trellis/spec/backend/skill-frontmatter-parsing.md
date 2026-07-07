# SKILL.md frontmatter 解析约定

## 契约

- SKILL.md 的 `---` 栅栏剥离全仓只有一份实现：`services/scanner/frontmatter.rs` 的 `extract_frontmatter_block`。
- 任何新入口（本地扫描、远程扫描、marketplace/github 导入、obsidian 导入等）需要解析 SKILL.md frontmatter 时，必须调用它拿到 YAML 块，再自行 `serde_norway::from_str` 映射到本域类型；**禁止手抄栅栏剥离逻辑**（`strip_prefix("---")`、`find("\n---")` 之类）。
- 统一语义（以历史上更严谨的 github_import 版为准）：去 UTF-8 BOM、容忍前导空白、开栅栏为首行且 trim 后恰为 `---`、闭栅栏必须是独立成行的 `---`（行内 `---` 不闭合）、兼容 CRLF。

## 现有调用方

- `services/scanner/mod.rs` `parse_skill_md_content`（本地扫描；`ssh_batch.rs` remote 扫描 shell 侧只 `cat` 内容，解析同走此函数）。
- `services/github_import/source.rs` `parse_frontmatter`（typed `SkillFrontmatter`）。

## 巡检命令

```bash
# 除 frontmatter.rs 外不应再出现栅栏剥离手抄
rg -n 'strip_prefix\("---|find\("\\n---' src-tauri/src --glob '!**/scanner/frontmatter.rs'
```

## 背景

2026-07 架构评审发现 scanner 与 github_import 各持一份解析且边界已分叉（BOM、闭栅栏严格性），带 BOM 的 SKILL.md 两条入口结果不一致。任务 `07-04-unify-frontmatter-parsing` 收敛为单一实现。
