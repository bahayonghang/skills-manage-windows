# Central Skills

Central Skills is the canonical home for every skill SkillPort manages. Everything else — per-platform installs, collections, marketplace imports — feeds into or pulls from this single library.

## Why a Central library

Multiple AI coding tools want to read the same SKILL.md files. Without a single source of truth you end up with copies that drift out of sync. SkillPort takes the opposite stance:

- One canonical directory: `~/.skillsmanage/skills/`.
- Each per-platform install is either a symlink back to the canonical directory or a tracked copy.
- The Universal Agents path (`~/.agents/skills/`) is treated like any other platform target. It is *not* the source.

This means you can rebuild the entire fleet of platform installs from the central library at any time.

## What you can do here

- **Browse** all central skills with virtualized rendering and deferred search.
- **Install / uninstall** to any active platform with one click using the platform icon row on each card.
- **Open the SKILL.md** detail view with rendered Markdown and raw source.
- **Generate AI explanations** of a skill's behaviour, cached per skill.
- **Group skills** into collections for batch install.
- **Delete** a central skill (with confirmation) and clean up its installations.

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

Last reviewed: 2026-05-04
