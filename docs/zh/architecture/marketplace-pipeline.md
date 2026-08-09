# Marketplace 流水线

Marketplace 表面整合四个后端服务：`marketplace`（同步 + 缓存）、`github_import`（预览 + 导入）、`central_metadata`（标签 + AI 建议）、`ai_provider`（技能解释流式）。

## 同步循环

```text
[用户点击同步]
       │
       ▼
commands::marketplace::sync_registry
       │
       ▼
services::marketplace::sync — GitHub API：列出仓库 tree
       │
       ▼
解析 SKILL.md frontmatter（name、description、downloadUrl）
       │
       ▼
upsert 到 marketplace_skills（带 cache_updated_at）
       │
       ▼
更新 skill_registries.last_synced / etag / last_modified
```

条件请求：源记录上一次响应的 `etag` 与 `last_modified`，下次同步带条件头，命中 `304 Not Modified` 时跳过解析。

## 缓存表

| 表 | 作用 |
| --- | --- |
| `skill_registries` | 每个源（GitHub repo 或镜像）一行；状态、错误、ETag、过期。 |
| `marketplace_skills` | 远端技能元数据缓存，按 registry_id 聚合。 |
| `skill_explanations` | AI 生成的解释，按 `(skill_id, lang)` 复合主键。 |

完整字段见[数据模型](./data-model.md)。

## 从 Marketplace 安装

```text
Marketplace 技能 id + 已启用 registry source
       │
       ▼
解析 GitHub source 并固定一个 commit/snapshot
       │
       ▼
重新构建候选并要求唯一精确匹配 Marketplace id
       │
       ▼
把候选完整目录投影为 CentralSkillWrite
       │
       ▼
target lock → pending recovery → durable stage/swap
       │
       ▼
同一事务提交 skill + repository provenance + db_committed
       │
       ▼
finalize journal → best-effort installed-cache repair
```

缓存的 `download_url` 与 frontmatter 展示名称都不是请求或路径 authority。
候选 `skill_id` 决定 Local/SSH/WSL 目标目录，三种 target 都从同一个 pinned
snapshot 接收 `SKILL.md`、references、scripts、assets 与其他同目录文件。首次安装使用
既有 `central_update` journal 且 `hadTarget=false`；覆盖安装通过同一可恢复 swap，
此时 `hadTarget=true`。

`marketplace_skills.is_installed` 是派生缓存。只有 Central 文件系统、skill row、
repository assignment、commit/digest provenance 与 journal commit 全部成功后才写入。
缓存 marker 写入失败不会把已提交安装伪装成失败；Marketplace 查询会从 Central
实时状态派生结果并重试缓存修复。

## GitHub 导入

`services::github_import/` 处理任意 GitHub 仓库的批量导入：

```text
github_import/
├── source.rs             解析用户输入（owner/repo[@ref][:path]）
├── raw_http.rs           带 PAT 鉴权的轻量 reqwest 包装
├── archive.rs            拉取 zipball 并解压
├── preview_workspace.rs  scratch 目录 + 清理
├── preview.rs            枚举 SKILL.md 候选
├── remote.rs             直接抓取单个 SKILL.md
├── import.rs             把选中预览提升到 Central
└── pat.rs                GitHub PAT 存储
```

预览返回 workspace id；UI 列出候选，用户挑要导入的。未选中的由 `discard_github_repo_preview_workspace` 清理。

## AI 解释

`services::ai_provider/` 流式生成技能解释：

| 文件 | 作用 |
| --- | --- |
| `mod.rs` | 提供方路由（Anthropic / OpenAI 兼容） |
| `claude.rs` | Anthropic API messages 格式 |
| `stream.rs` | 服务端推送事件解析 |
| `prompt.rs` | Prompt 模板 + locale |
| `cache.rs` | `skill_explanations` 读写 |
| `error.rs` | 错误映射为可见字符串 |

`explain_skill_stream` 命令 emit `ai-explain://{job_id}` 事件，UI 一次订阅持续渲染流式 chunk。取消通过 `AiTagJobRegistry::cancel`。

## 标签建议

`commands::central_metadata::*` 驱动标签抽屉。AI 建议写入 `skill_ai_tag_reviews`（`status='pending'`），UI 一条条接受或跳过；接受的行落到 `skill_tag_links`，`source='ai'`。

Last reviewed: 2026-08-03
