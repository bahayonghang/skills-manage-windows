# Discover

Discover scans your local disks for project-level skill libraries — directories that contain `SKILL.md` files but live outside the global platform paths. It is the primary way to surface skills shipped with a repository or stored in a team folder.

## What gets scanned

- All directories registered in **Settings → Scan Directories**.
- Common project-level skill paths inside those directories, including `.claude/skills/`, `.cursor/skills/`, `.agents/skills/`, `.factory/skills/`, and equivalents for other platforms.
- On macOS, `/Applications` is also examined for app-bundled skill directories.

The scan never modifies the source files. It only reads `SKILL.md` and records what it finds.

## Layout of the Discover view

The page is split:

- **Left panel** — list of detected projects, grouped by root directory. A counter shows how many skills each project exposes.
- **Right panel** — details of the selected project: its discovered skills, their inferred platform, and quick actions.

## Importing a discovered skill

When you import a project skill, SkillPort:

1. Promotes it into the Central library if it is not already centralized (`ensure_centralized`).
2. Creates an installation record so the original platform still sees it.
3. Optionally re-installs to other platforms via the standard install dialog.

The original project file is left in place; only a copy reaches the central library.

## Refreshing

Discover is on-demand. Use the refresh button on the page after:

- Adding or removing a scan directory in Settings.
- Cloning a new project that contains skills.
- Editing skills outside the app (the file watcher does not pick up arbitrary external paths).

## When to use Discover vs Marketplace

| Use case | Preferred entry point |
|----------|----------------------|
| Skills shipped with a local project | Discover |
| Skills published by a vendor or community | [Marketplace](./marketplace) |
| Specific GitHub repo you want to mirror | [GitHub Import](./github-import) |

## Where to go next

- Configure scan directories: [Settings](./settings).
- Promote project skills to the central library: [Central Skills](./central-skills).

---

Last reviewed: 2026-05-04
