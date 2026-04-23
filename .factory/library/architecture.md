# Architecture

How the system works at a high level, with emphasis on the Claude multi-source scan mission.

## System Shape

The app has three layers:

1. **React/Tauri UI** in `src/`
   - Pages and drawers render platform lists and skill details.
   - Zustand stores call Tauri commands via `invoke()`.
2. **Rust command layer** in `src-tauri/src/commands/`
   - Scans agent roots, reads skill metadata/content, and computes platform/detail payloads.
3. **SQLite state** in `src-tauri/src/db.rs`
   - Stores canonical skill records, installation relationships, settings, and cached marketplace/discover state.

## Existing Skill Model

There are two different concepts that must stay separate.

### 1. Canonical/manageable skill state
This is the normal app-managed world:
- central skills under `~/.agents/skills`
- install relationships to agents/platforms
- centralization, linking, uninstalling, and collection management

This model drives:
- `skills`
- `skill_installations`
- linker/install semantics
- collection membership and detail actions

### 2. Observed platform scan state
This is what the app sees when it scans an agent/platform directory.

For most agents today, one platform maps cleanly to one root. The Claude mission changes only the **scan observation model** for `claude-code`: one logical platform must roll up multiple Claude-owned physical roots while preserving per-row source identity.

## Claude Multi-Source Goal

`claude-code` remains a single platform in the sidebar and platform routing.

Within that one platform, the scan/query layer must surface rows from:
- `~/.claude/skills`
- `~/.claude/plugins/marketplaces/*`

The user must see both sources together, but the system must not treat marketplace rows as normal manageable installs.

## Key Invariants For This Mission

### Claude stays one platform
- No second Claude platform entry
- No Discover-style separate surface for plugin marketplace rows

### Source identity is first-class
Every Claude row must carry enough data for the UI to render:
- source origin (`user` vs `marketplace`)
- source root/path
- read-only/display-only status
- conflict state when the same logical skill appears in both Claude roots
- stable row identity that keeps duplicate logical skills distinct in list and detail flows

### Marketplace rows are observational only
Marketplace-source Claude rows:
- are visible on Claude list/detail surfaces
- are read-only/display-only
- do **not** participate in centralize/install/uninstall/link semantics
- do **not** pollute `linked_agents` or canonical install state

### User-source Claude rows keep current behavior
Normal Claude rows from `~/.claude/skills` must continue to behave like ordinary platform rows even when a matching marketplace row exists beside them.

## Current Choke Points Workers Must Respect

These are the places where the current code shape will fight the mission if treated like a light payload tweak.

### Single-root Claude assumption
Today Claude is modeled with one built-in root (`~/.claude/skills`), and the scan path walks one `global_skills_dir` per agent. Marketplace roots are not part of that current agent/root model.

### Shared identity assumptions in the UI
Current Claude list/detail flow assumes one row per `skill.id`:
- `PlatformView` keys rows by `skill.id`
- detail open/close bookkeeping is keyed by `skill.id`
- `SkillDetailView` loads by `skillId` only

If the same logical skill appears in both Claude roots, workers must ensure the selected row can still be uniquely identified from list to detail.

### Shared identity assumptions in the DB/install model
Current install state is keyed around canonical skill identity (`skill_id`, `agent_id`). That is correct for manageable installs but not sufficient for representing two observed Claude rows with the same logical skill under one platform.

Workers should assume this mission needs a **source-aware observation identity** for Claude rows rather than stretching canonical install state to fit the new behavior.

## Recommended Internal Boundary

The safest boundary is:
- keep canonical/manageable skill tables and semantics intact
- add a **source-aware Claude observation/query layer** for platform rendering

In other words, extend Claude scan/query modeling rather than redefining what a managed install means.

## Minimum Claude Row Data Needed End-to-End

Whatever storage/query shape is chosen, workers should preserve at least these fields through backend and frontend boundaries for Claude rows:
- stable row identity
- logical skill identity (`skill_id` and/or name as currently used)
- source kind (`user` or `marketplace`)
- source root
- concrete file path
- read-only/display-only flag
- conflict grouping signal

Without these, the list and detail surfaces cannot keep duplicate-source rows distinct.

## Runtime Flow

### Startup / rescan
1. `platformStore.initialize()` or `refreshCounts()` triggers `scan_all_skills`
2. Rust scanner resolves Claude roots and scans both user and marketplace trees
3. Query layer returns source-aware Claude rows
4. Sidebar/platform counts refresh from the new scan result
5. Claude platform page renders all rows under one platform

### Claude platform list
1. `PlatformView` loads Claude rows for `claude-code`
2. `全部` is the default source-tab view and shows both source types together
3. The active source tab filters the source-aware Claude rows before any text search runs
4. Search filters only the rows that remain visible inside the active source tab, not a deduplicated logical projection
5. UI renders source badge + read-only state + row-specific actions within the filtered row set

### Claude detail
1. User opens a specific row from the Claude platform list
2. Detail payload must represent that exact row/source, not just a generic skill identity
3. User-source rows expose normal actions
4. Marketplace rows expose source/path information but remain read-only

### Rescan behavior
After Claude roots change, a single rescan must:
- remove disappeared rows
- keep surviving rows attached to the correct source identity
- avoid stale ghosts and cross-source swaps
- update the live Claude page without requiring navigation away/back

## Areas Explicitly Out Of Scope

- Discover scanning behavior
- Plugin source mutation or auto-centralization
- Treating marketplace rows as a separate platform
- Extending scan support to Claude `cache` roots in this mission

## Worker Guidance

When implementing, always ask:
- Is this data about a **manageable canonical skill**, or about an **observed Claude scan row**?
- Could this change accidentally make marketplace rows act like normal installs?
- Does the list/detail UI still have enough identity to keep duplicate rows distinct?
- If two Claude rows share the same logical skill identity, what uniquely selects the intended row end-to-end?
