# Central Skills

Central Skills is the canonical home for every skill SkillPort manages. Everything else — per-platform installs, collections, marketplace imports — feeds into or pulls from this single library.

## Why a Central library

Multiple AI coding tools want to read the same SKILL.md files. Without a single source of truth you end up with copies that drift out of sync. SkillPort takes the opposite stance:

- One canonical directory: `~/.skillsmanage/skills/`.
- Each per-platform install is either a symlink back to the canonical directory or a tracked copy.
- The Universal Agents path (`~/.agents/skills/`) is treated like any other platform target. It is *not* the source.

This means you can rebuild the entire fleet of platform installs from the central library at any time.

## Central Library V2 layout

Central Library V2 is the default Central Skills experience. It keeps the existing card, install, detail, AI explanation, and delete flows, then adds a denser information architecture for larger local libraries.

- **Three-part sidebar**: Smart Views, Repositories, and Tags are always visible as first-class filters.
- **Multi-facet filters**: combine repository, owner, source, tag, platform, and status filters instead of replacing one filter with another.
- **Structured search**: type filters such as `tag:writing repo:anthropics/* owner:anthropics has:update` directly into the search box.
- **URL-as-state**: query text, selected facets, sorting, grouping, and saved-view identity are encoded in the URL query string.
- **Saved Views**: persist useful combinations like "updates to review" or "anthropics writing skills" and reopen them from the sidebar or command palette.
- **Tag Groups**: keep large tag sets readable by grouping tags into first-level categories.
- **Group-by views**: switch between no grouping, repository, owner, tag, and update-status groupings.
- **Command palette**: press `Ctrl+K` to save the current view, create a tag group, switch grouping mode, or return to the classic layout.

If you need the older layout during the rollout, use the **Switch to classic layout** link near the V2 badge. Developers can also set `featureFlag.central.newLayout=off` in DevTools localStorage and dispatch `feature-flag-change`.

## Search syntax

Search keys are case-insensitive (`TAG:` and `tag:` are equivalent). Values keep their original casing.

| Syntax | Meaning | Example |
|--------|---------|---------|
| `tag:` / `-tag:` | Include or exclude a tag | `tag:writing -tag:wip` |
| `repo:` | Match `owner/name`, with `*` wildcards | `repo:anthropics/*` |
| `owner:` | Match repository owner | `owner:anthropics` |
| `source:` | Match repository source type | `source:github` or `source:local` |
| `has:` | Match derived status | `has:update`, `has:no-tag`, `has:ai-review` |
| `platform:` | Match a linked platform id | `platform:claude-code` |
| `created:` / `updated:` | Match rough dates or relative ages | `updated:<30d`, `created:>2026-01-01` |

Free text that is not parsed as a structured filter still matches the skill's searchable text.

## Saved views and tag groups

Saved Views and Tag Groups are stored locally in the SkillPort database. They are metadata over your Central library; they do not move or rewrite skill folders.

- Deleting a saved view deletes only the saved query, not the skills that matched it.
- Deleting a tag group leaves the member tags intact and moves them back to **Ungrouped**.
- Tags can be assigned or unassigned from groups directly from the V2 sidebar.
- Reorder IPC and store actions exist for future drag-and-drop UI, but the current UI relies on creation order and pinning.

## What you can do here

- **Browse** all central skills with virtualized rendering and deferred search.
- **Search and filter** with structured syntax, multi-select facets, saved views, and group-by modes.
- **Install / uninstall** to any active platform with one click using the platform icon row on each card.
- **Open the SKILL.md** detail view with rendered Markdown and raw source.
- **Generate AI explanations** of a skill's behaviour, cached per skill.
- **Group skills** into collections for batch install.
- **Delete** a central skill (with confirmation) and clean up its installations.

## Screenshots

The docs site reuses the same screenshot assets as the README.

![Central skills library view](/images/01.png)

![Skill detail and platform install status](/images/02.png)

![Collection and batch workflow](/images/03.png)

## Auto-centralization

When you install a skill that exists only on one platform (for example, a project-level Claude skill found via Discover) to another target, SkillPort first promotes it into the central library. The flow:

```text
[Skill exists only at ~/.<platform>/skills/<name>]
        │
        ├─ ensure_centralized: copy SKILL.md tree into
        │   ~/.skillsmanage/skills/<name>
        │
        ├─ DB updates: canonical_path, is_central
        │
        └─ Continue normal install: symlink or copy to
            other selected platforms
```

Auto-centralization is transparent. The skill appears in the Central view the next time the list refreshes.

## Symlink vs copy

| Mode | Behaviour | When to choose |
|------|-----------|---------------|
| Symlink | Per-skill symlink from platform dir → central. | Default. Fastest, single source of truth. |
| Copy | A duplicated tree under the platform dir. | When the platform or filesystem cannot follow symlinks (some Windows setups, restricted SSH targets). |

Switching modes triggers the install/uninstall pair under the hood; it does not corrupt the original central data.

## Where to go next

- See per-platform views: [Platforms](./platforms).
- Group skills for batch operations: [Collections](./collections).
- Pull skills from outside: [Marketplace](./marketplace) and [GitHub Import](./github-import).

---

Last reviewed: 2026-05-11
