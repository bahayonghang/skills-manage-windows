# Current project-install visibility (Central Skills)

Date: 2026-09-10
Source: local repository inspection against the two UI screenshots
(Central Skills list + Install-to-project dialog).

## Problem restated

After a Central skill is installed into a project directory, the Central
Skills list still only exposes **global platform destinations**. The user
wants those **project destinations** visible next to / under the skill, as a
folder icon plus the path's last segment (not the full path).

## Two install-to-project pipelines

| Pipeline | Entry | Persistence | Visible afterwards |
|---|---|---|---|
| Projects page | `install_skill_to_project` → `projects::install_skill_to_project_impl` | Writes `project_skill_installations` (`crud.rs:485-500`) | Projects page + skill **detail** sidebar (`list_projects_using_skill`) |
| Central / Marketplace / Platform Install dialog | `batch_install_central_skills` with `project_path` → `install_central_skill_to_project` | Filesystem copy/symlink only (`installation/project.rs:345-412`, `installation/batch.rs:104-124`) | **Not** in `linked_agents`, **not** in `list_projects_using_skill` unless the path was previously registered as a Project **and** installed via the Projects pipeline |

This is the core gap behind screenshot 2 (`D:\Documents\Code\Rust\Exp\PromptHub`):
that dialog is the Central Install pipeline. Closing it does not update the
Central card, and does not guarantee a `projects` row.

## What Central Skills already shows

### Main app sidebar (`src/components/layout/Sidebar.tsx`)

- Top-level `Projects` nav goes to `/projects` (catalog + per-project skill
  management).
- After the divider, **CODING** lists globally detected agents grouped by
  `getPlatformTargetGroups` (Universal, Claude Code, Grok, …).
- Registered projects are **not** listed under CODING.

### Central skill cards

- Grid cards pass `platformIcons.linkedAgents = skill.linked_agents`
  (`CentralSkillListContent.tsx:142-149`) and render platform toggle icons in
  `UnifiedSkillCardFooter`.
- `linked_agents` is built only from `skill_installations` plus shared-root
  agents (`central_skills/query.rs:223-234`). Project installs are a different
  table.
- Search forces list view; list cards currently omit `platformIcons` /
  footer, so the screenshot's search result does not show agent icons.
- `SkillCardMeta` already has a `projectBadge` with `Folder` + label
  (`SkillCardMeta.tsx:100-105`), but Central cards do not pass it; Obsidian
  vault rows do.

### Skill detail

- Global installs: `detail.installations` from `skill_installations`.
- Project installs: separate `list_projects_using_skill` query joining
  `project_skill_installations` × `projects` (`crud.rs:331-385`).
- Detail UI already uses `FolderOpen` + `projectName` + agent + link type
  (`SkillDetailSidebar.tsx:651-695`). Full path is tooltip-only via
  `formatPathForDisplay`.
- Display name for registered projects is `projects.name`, which
  `add_project_impl` seeds from the path basename (`project_name_from_path`,
  `crud.rs:130-137`). Rename can diverge from basename.

## Existing helpers (reuse, do not duplicate)

- Path last segment: `getPathBasename` in `src/lib/path.ts:103-111`
  (tested in `src/test/lib/path.test.ts:75-78`).
- Project display name: `Project.name` / `ProjectUsingSkill.projectName`.
- Reverse lookup IPC: `list_projects_using_skill` already exists; it is
  detail-only today (`skillDetailStore.loadProjectsUsingSkill`).

## Cross-layer implication

If the list surface is the Central card (recommended), `SkillWithLinks` must
carry a compact project-destination list. That is a DTO enrichment on the
Central list/page path, not a renderer-only heuristic over `linked_agents`.

If Central's project-path install remains filesystem-only, screenshot-2
installs will never appear unless we also **register the project** (idempotent
`add_project_impl`) and write `project_skill_installations` (reuse Projects
install persistence, do not invent a third table).

## Decision (2026-09-10)

Placement is **Central skill cards** (user chose A), not the CODING sidebar.
Display remains folder icon + path basename. Central project-path installs
must be persisted into `projects` + `project_skill_installations` or the
card cannot show screenshot-2 results.

## Likely out of scope unless requested

- Changing CODING sidebar to list every registered project (duplicates
  `/projects`).
- Making project chips toggle-install like platform icons.
- Showing the full project path in the list (explicitly rejected).
- Remote-target path display beyond existing `formatPathForDisplay` / basename
  rules.
