# Scanning

Two services handle "find SKILL.md files on disk": `services::scanner` for the Central store and `services::discovery` for project / Obsidian sources. Both produce typed records and never touch the DB directly.

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

## Project Discovery

Source: `src-tauri/src/services/discovery/`.

```text
discovery/
├── roots.rs     resolve project skill patterns per platform
├── scan.rs      walk roots, collect SKILL.md candidates
├── query.rs     Obsidian vault scan (source-only)
├── import.rs    promote a discovered skill into Central / a platform
└── types.rs     ScanRoot, DiscoveredSkill, ImportRequest
```

Discovery solves a different problem from the scanner: scanner reads agent-installed skills, discovery reads project-level SKILL.md files that have not been promoted to a Central location yet.

### Roots and Patterns

`roots.rs` collapses each platform's project skill patterns into a deduplicated list. Shared patterns (`.agents/skills`) are emitted once even when several platforms claim them — preventing the same SKILL.md from appearing N times in the UI.

### Project Scan

`scan.rs` walks each root, parses frontmatter, and writes rows into `discovered_skills` keyed by `(project_path, platform_id)`. The UI renders left-panel project list + right-panel skill detail off this table.

### Obsidian Source

`query.rs` reads Obsidian vault registries. Vault-level `.skills > .agents/skills > .claude/skills` precedence picks the canonical SKILL.md when a vault is structured for multiple platforms. Source skills do not pass through `discovered_skills`; `commands::discover::import_source_skill_to_central` and `import_source_skill_to_platform` accept the raw file/dir path so vault data does not pollute the persistent cache.

### Import Pipeline

`import.rs` is the only writer for the discovered → installed transition:

| Method | Effect |
| --- | --- |
| `symlink` | Default; install via OS symlink. |
| `auto` | Symlink with copy fallback when permissions block links (Windows). |
| `copy` | Force a directory copy. |

When promoting to Central first, the import service calls `installation::centralize::ensure_centralized` so `canonical_path` and `is_central` are correct before the install path is taken.

## Re-scan Performance

- Scanner reuses `agent_skill_observations` rows so a clean re-scan is O(file count), not O(agent × file count).
- Discovery prunes rows for `(project_path, platform_id)` keys that no longer exist on disk to keep the UI list bounded.

Last reviewed: 2026-05-04
