# Update Center

Update Center is the single entry for refreshing remote state, applying upgrades, removing platform duplicates, and cleaning up stale local copies. It replaces three older dialogs — *Check for updates*, *Remote missing skills*, and *Repository sync* — with one screen that aggregates every pending change into five tabs.

## When to use it

- After importing a GitHub repository, to discover newly added or removed skills upstream.
- When you suspect a skill has changed upstream but the local card has no update badge yet.
- When the platform copy and central copy fall out of sync after manual edits.
- When you want a single review pass before pushing many decisions at once.

## Open the dialog

Click **Update Center** in the Central Skills toolbar at the top of `/central`. The legacy *Check for updates* button still works during the transition and both routes write to the same central library, but the new dialog covers every flow in one screen.

PlatformView's **Scan duplicates** button now also routes here, opening directly on the *Platform duplicates* tab. You no longer need to keep a separate dialog for that case.

## Refresh and apply are separate steps

Refresh is read-only. It fetches the latest GitHub snapshot for each repository in scope, compares against your local central library and platform installs, and writes the diff into an inventory table. Nothing on disk changes during a refresh. You can refresh as often as you want without side effects.

Apply runs the actual mutations against the central library and platform symlinks or copies. Selections from all five tabs are aggregated into one submit, then executed in a stable order so add / remove / update / duplicate cleanup never collide on the same skill id.

This split is intentional. Browsing the inventory should never trigger a write, and a write should never be hidden behind what looks like a "refresh" button.

## Scope

The dropdown next to the refresh button decides what gets scanned.

| Scope | What it does |
|-------|--------------|
| All | Every central skill plus every registered GitHub repository. |
| Current repository | Only the repositories backing the currently filtered Central view. |
| Current results | Only the skills currently visible after search and facet filters. |

Scope is sticky per session. The current selection is shown next to the refresh button so a narrow refresh cannot be mistaken for an empty inventory.

## The five tabs

Each tab is a bucket of pending changes. Counts on the tab headers reflect inventory length, not the current selection.

### Updatable

Skills whose remote content has changed since the last successful sync. Each row shows the affected source repository and the new last-updated timestamp. Check the rows you want to update; the apply step pulls the new SKILL.md tree and refreshes any linked platform copies that follow the central library.

### Added

Skills the remote repository now contains but the central library does not. Each row offers three per-item decisions:

- **Overwrite** — only available when a central skill already uses the same id; replaces the existing tree.
- **Rename** — store the new tree under a different id.
- **Skip** — leave the central library unchanged for this row.

When a row's id already collides with a central skill, the default decision is *Skip* so an accidental apply cannot blow away local edits.

### Removed

Skills the remote repository deleted while a local central copy remains. Each row offers two per-item decisions:

- **Keep** — detach the remote source link but retain the local files. Use this when you want to preserve work even if upstream gave up on the skill.
- **Delete** — remove the skill from the central library and from any linked platforms.

### Platform duplicates

The same skill id exists as both a plugin read-only copy (for example, `~/.claude/plugins/marketplaces/...`) and a manually installed writable copy on the same agent. Pick which writable paths to remove. Read-only plugin copies are listed for context but cannot be removed from here.

This tab is the same surface that PlatformView's *Scan duplicates* opens; the data is shared.

### Orphans

Reserved for stale symlinks pointing to deleted central directories. Detection is not yet implemented; the tab is a placeholder so its layout and decision affordances stay consistent across releases.

## Persisted inventory

Refresh results are persisted in the local SkillPort database, so closing the dialog and reopening it does not lose work. The persisted inventory survives application restarts and is keyed by scope, so a *Current results* refresh does not overwrite a previous *All* refresh in the unrelated buckets.

The **Clear inventory** footer button drops the persisted refresh results. Use it after a major reorganization to avoid stale entries. Clearing does not delete any skills or platform copies; it only resets the pending checklist.

## What's behind the scenes

- New Tauri commands: `refresh_skill_update_inventory`, `apply_skill_update_decisions`, `clear_skill_update_inventory`, `get_skill_update_inventory`, `scan_platform_duplicate_skills`.
- New DB table: `skill_repository_pending_additions`. New column: `skill_repositories.last_synced_at`.
- New backend enum `SkillUpdateStatus` replaces the previous status string constants.
- The legacy `check_central_skill_updates`, `check_central_repository_sync`, and `apply_central_repository_sync` commands still exist for backward compatibility and will be removed after a minor release.

## Migrating from the legacy "Check for updates"

Both entries remain functional. Keep using the legacy button if you depend on it; the new Update Center covers the same checks plus the *Added*, *Removed*, and *Platform duplicates* flows that used to live in separate dialogs. The legacy button will be retired once feedback on Update Center stabilizes.

## Where to go next

- Central library context and search syntax: [Central Skills](./central-skills).
- Where the per-platform writable copies live: [Platforms](./platforms).
- How GitHub repositories enter the central library in the first place: [GitHub Import](./github-import).

---

Last reviewed: 2026-05-22
