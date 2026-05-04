# State Import / Export

SkillPort can serialize the full local state — Central skills metadata, collections, custom platforms, settings, scan directories — into a single JSON document. Use this to migrate between machines or to seed a remote SSH target.

## Commands

| Direction | IPC | UI |
| --- | --- | --- |
| Export | `export_skillport_state` | Settings → Data → Export |
| Preview import | `preview_skillport_state_import` | Drop file → diff dialog |
| Apply import | `import_skillport_state` | Diff dialog → Apply |

The preview step is mandatory. SkillPort computes adds / updates / deletes and shows a per-table diff before any write.

## Document Shape

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

## Field Notes

- `skills.canonical_path` and `skills.file_path` are absolute paths on the source machine. The importer rewrites them to point at the target's `~/.skillsmanage/skills/` location.
- `skill_installations.installed_path` is recomputed on the target; only `(skill_id, agent_id, link_type)` is portable.
- AI explanations (`skill_explanations`) and operation logs (`operation_logs`) are intentionally **not** exported.
- Settings rows containing secrets (`github.pat`, `ai.<provider>.api_key`) are stripped on export. Re-enter them in Settings on the target machine.

## Compatibility

- `version: 1` is the current document version. Older `.json` exports are forward-compatible by ignoring unknown fields.
- The importer requires the same major SkillPort version as the document, or one version newer with a documented migration. A mismatch surfaces a blocking dialog.

## File Naming

Exports default to `skillport-state-YYYY-MM-DD.json`. Pick any name on save; the importer reads `version` from the body, not the filename.

## SSH Targets

Per-target databases (`~/.skillsmanage/targets/<id>/db.sqlite`) are exported separately by switching the active target before exporting. The importer always writes to the currently active target — be deliberate when restoring across hosts.

Last reviewed: 2026-05-04
