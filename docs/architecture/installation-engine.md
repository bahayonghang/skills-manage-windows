# Installation Engine

`services::installation` is the only writer that materializes a Central skill into a platform directory. Five files cover the full pipeline.

## Module Layout

```text
services/installation/
├── types.rs       InstallRequest, InstallResult, InstallMethod
├── fs_util.rs     symlink / copy / dir walks shared by all paths
├── centralize.rs  ensure_centralized: copy a non-central skill into ~/.skillsmanage/skills
├── native.rs      install into a platform's global_skills_dir
├── project.rs     install into a project-level skill directory
├── remote.rs      install over SSH via targets::exec
└── batch.rs       install one skill across many agents in a single transaction
```

## Decision Tree

```text
[install_skill_to_agent] ──┬── skill is_central? ──no──► centralize::ensure_centralized
                           │                              │ copies into ~/.skillsmanage/skills
                           │                              ▼
                           │                         updates skills.canonical_path / is_central
                           │
                           ├── target == Local ──► native | project (by agent.project_skills_dir presence)
                           └── target == SSH   ──► remote (uses targets::exec)
```

The contract is uniform: every entry point first guarantees a Central source, then writes a symlink or copy, then records a row in `skill_installations` with `link_type` and `symlink_target`.

## Symlink vs Copy

| Method | When |
| --- | --- |
| `symlink` | Default. One canonical source. Updates propagate to all installs. |
| `copy` | Windows users without Developer Mode; SSH hosts where symlinks span FS boundaries; Discover imports from read-only locations. |
| `auto` | Try symlink first; fall back to copy on permission errors. Used by Discover. |

The choice flows in from the IPC layer (UI sends `method`), but every service path validates that the resulting on-disk file is consistent with `installation::types::InstallMethod` before the DB row is written.

## Batch Install

`batch.rs` powers two surfaces:

1. **Collections.** `commands::collections::batch_install_collection` walks a collection and installs each member into selected agents.
2. **Central → multiple platforms.** `commands::linker::batch_install_central_skills` is the toggle in `UnifiedSkillCard` and the Install dialog.

The batch path opens a single sqlx transaction and emits one `operation_logs` row per (skill, agent) pair so the Logs page surfaces granular failure points.

## Auto-centralize Invariant

`ensure_centralized` is idempotent and is invoked from every install entry point. It guarantees the rest of the pipeline sees a Central row, even when the user installs straight from a Discover result. Skipping it breaks symlink cleanup later, so install code always calls it instead of branching on `is_central`.

## Uninstall

Symbol installs are removed by `commands::linker::uninstall_skill_from_agent`:

1. Look up the install row by `(skill_id, agent_id)`.
2. Resolve `installed_path`; refuse to delete files outside the agent's `global_skills_dir` / `project_skills_dir`.
3. Remove the symlink or copy.
4. Delete the install row; emit an `operation_logs` row.

The path-bound check stops a malformed DB row from removing user data outside SkillPort's known surface.

Last reviewed: 2026-05-04
