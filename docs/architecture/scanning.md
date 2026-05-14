# Scanning

Two services handle "find SKILL.md files on disk": `services::scanner` for the Central store and `services::projects` for project-level libraries. Both produce typed records and never touch the DB directly.

## Central Scanner

Source: `src-tauri/src/services/scanner/`.

```text
[command::scan_all_skills] ──► services::scanner::scan_all
                                       │
                                       ▼
              walk every registered scan directory
                                       │
                                       ▼
              parse SKILL.md YAML frontmatter (name, description)
                                       │
                                       ▼
              upsert skills + observations (per agent) via repos
```

- Scan directories live in `scan_directories`; both built-in (resolved from each agent's `global_skills_dir`) and user-added rows are honored.
- The Claude plugin variant (`claude_plugin.rs`) understands the nested `~/.claude/plugins` layout where each plugin has its own `skills/` folder.
- Observations are written per agent so the UI can show "this skill is also installed in X / Y / Z" without re-walking disk.

## Project-Level Scan

Source: `src-tauri/src/services/projects/`.

```text
projects/
├── crud.rs   add / list / rename / pin / remove + install / uninstall
├── scan.rs   walk enabled-agent skill dirs under a project root
├── types.rs  ProjectDto, ProjectSkillDto
└── tests.rs  unit coverage
```

Project scan solves a different problem from the Central scanner: scanner walks global agent paths to populate the central library, while project scan walks per-project agent paths (`<project>/.claude/skills/`, etc.) for project-local SKILL.md files.

### Roots

There are no implicit roots. Each project root is added explicitly by the user via `pick_project_folder` → `add_project`. Paths are canonicalised, separators normalised, and `sha256` hashed (first 16 hex chars) into a stable `project_id`.

### Project Scan

`scan.rs` iterates **enabled agents** (`SELECT id, project_skills_dir FROM agents WHERE is_enabled = 1 AND id != 'central'`), resolves each agent's per-project skill directory, runs the shared `services::scanner::scan_directory` walker, and upserts rows into `project_skill_installations` keyed by `(project_id, skill_id, agent_id)`. `symlink_metadata` decides `link_type` (`symlink` vs `copy`); orphan psi rows for SKILL.md folders that no longer exist on disk are deleted in the same pass.

### Install / Uninstall

`crud::install_skill_to_project_impl` and `uninstall_skill_from_project_impl` are the only writers for the install / uninstall transition. Install requires a centralised skill (`is_central = true && canonical_path IS NOT NULL`) and accepts a `method` of `symlink` (default) or `copy`. Symlink failure (Windows without Developer Mode) is bubbled up as a plain error string for the front-end toast.

### Removal

`remove_project_impl(id, uninstall_skills)` deletes the project row (and its psi rows). When `uninstall_skills = true`, the function walks every psi row first and removes the on-disk symlink / copy before deleting the project; per-row removal failures are logged but never abort the project deletion.

## Migration from Discovery

The legacy `services::discovery` module — full-disk crawl, hard-coded scan roots, `discovered_skills` table — was removed in 0.10.x. Schema migration drops `discovered_skills` and the `discover_scan_roots_config` setting on first launch. Obsidian vault scanning that previously shared the discovery module was extracted into `services::obsidian/`.

## Re-scan Performance

- Scanner reuses `agent_skill_observations` rows so a clean re-scan is O(file count), not O(agent × file count).
- Project scan prunes psi rows for `(skill_id, agent_id)` pairs that no longer exist on disk to keep the UI list bounded; the IPC layer emits `project:scanned` so the front-end refreshes without a full reload.

Last reviewed: 2026-05-14
