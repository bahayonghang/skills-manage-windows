# 状态导入 / 导出

SkillPort 可以把完整本地状态——Central 技能元数据、集合、自定义平台、设置、扫描目录——序列化为单个 JSON 文档。用于跨机器迁移，或为远端 SSH 目标做种子。

## 命令

| 方向 | IPC | UI |
| --- | --- | --- |
| 导出 | `export_skillport_state` | 设置 → 数据 → 导出 |
| 预览导入 | `preview_skillport_state_import` | 拖入文件 → 差异对话框 |
| 应用导入 | `import_skillport_state` | 差异对话框 → 应用 |

预览步骤强制；SkillPort 计算 add / update / delete 后才允许写入。

## 文档结构

```json
{
  "version": 1,
  "exportedAt": "2026-05-04T08:30:00Z",
  "skillport": { "version": "0.10.0" },
  "tables": {
    "skills": [{ "id": "...", "name": "...", "is_central": true, "canonical_path": "..." }],
    "skill_installations": [{ "skill_id": "...", "agent_id": "claude-code", "link_type": "symlink" }],
    "agents": [{ "id": "claude-code", "display_name": "Claude Code", "category": "coding", "is_enabled": true }],
    "collections": [{ "id": "...", "name": "...", "description": "..." }],
    "collection_skills": [{ "collection_id": "...", "skill_id": "..." }],
    "skill_repositories": [],
    "skill_repository_members": [],
    "skill_tags": [],
    "skill_tag_links": [],
    "scan_directories": [],
    "settings": [{ "key": "ui.locale", "value": "en" }]
  }
}
```

## 字段说明

- `skills.canonical_path` / `skills.file_path` 为源机器绝对路径。导入时按目标机器的 `~/.skillsmanage/skills/` 重写。
- `skill_installations.installed_path` 在目标机重新计算；只有 `(skill_id, agent_id, link_type)` 是可移植的。
- AI 解释（`skill_explanations`）与操作日志（`operation_logs`）**不导出**。
- 含密设置（`github.pat`、`ai.<provider>.api_key`）在导出时被剥离，请在目标机重新填写。

## 兼容

- `version: 1` 是当前文档版本。老 JSON 通过忽略未知字段做向前兼容。
- 导入器要求文档主版本与 SkillPort 一致，或仅低一版本但有迁移路径。否则弹出阻塞对话框。

## 文件命名

默认 `skillport-state-YYYY-MM-DD.json`。文件名随意，导入器只读 `version` 字段。

## SSH 目标

每目标库（`~/.skillsmanage/targets/<id>/db.sqlite`）通过切换活动目标分别导出。导入器始终写入当前活动目标——跨主机恢复时务必显式确认目标。

Last reviewed: 2026-05-04
