# First Run

The first time you launch SkillPort, the app prepares its local data store and runs a full disk scan so the UI has something to show. This page walks through what happens behind the scenes and what you should see.

## What gets created

| Path | Purpose |
|------|---------|
| `~/.skillsmanage/` | Application data root |
| `~/.skillsmanage/db.sqlite` | SQLite database (WAL mode) |
| `~/.skillsmanage/skills/` | Central skill library — the single source of truth |
| `~/.skillsmanage/targets/<id>/db.sqlite` | Per-SSH-target cache (created on demand) |

The database is initialized automatically; existing installations keep their data because the directory name is preserved.

## Startup scan

```text
[App launch] ──┬── Read built-in agent registry
               ├── Iterate ~/.<platform>/skills/ for each enabled platform
               ├── Iterate ~/.skillsmanage/skills/ as Central
               ├── Read configured project scan directories
               ├── Parse SKILL.md frontmatter (name, description, etc.)
               ├── Detect symlink relationships via lstat
               └── Write rows into skills + skill_installations tables
```

The scan is idempotent and safe to repeat. You can re-run it from the top bar at any time.

## What you should see

- **Central Skills** view on the left navigation, with skills found under `~/.skillsmanage/skills/`.
- A list of detected platforms grouped under **Coding** and **Lobster**. Platforms that have a skills directory present become active rows; the rest stay greyed out until you opt in.
- A search box in the top bar with deferred queries — typing does not block scanning.

## If the scan returns nothing

- Confirm that at least one platform's skills directory exists (for example, `~/.claude/skills/`). The app does not create platform directories itself.
- Check Settings → Scan Directories. The Central path and built-in platform paths are present by default; custom project paths only appear if you added them previously.
- Re-run the scan from the top bar after creating a directory. The previous result is replaced atomically; partial states are not persisted.

## Where to go next

- Browse and install skills: [Central Skills](./central-skills).
- Manage per-platform installs: [Platforms](./platforms).
- Pull skills from the network: [Marketplace](./marketplace) or [GitHub Import](./github-import).

---

Last reviewed: 2026-05-04
