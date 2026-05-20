# Projects

Projects gives you a manual, per-project view over project-level skill libraries — the SKILL.md folders that live inside a code repo at `.claude/skills/`, `.kiro/skills/`, `.agents/skills/`, and friends.

Unlike the old "Discover" workflow that scanned every disk on launch, Projects is **opt-in**: you add the roots you care about, then SkillPort scans just those roots whenever you ask.

## Adding a project

1. Open **Projects** in the sidebar.
2. Click **Add project**, pick the folder, confirm.
3. SkillPort persists the project, returns immediately, and runs a background scan that fills in the skill count.

The same path is idempotent — clicking **Add project** on a folder you already tracked simply re-selects it.

## What gets scanned

For each project root, SkillPort walks the per-platform skill directories of **enabled agents only**:

- `.claude/skills/` for Claude Code
- `.kiro/skills/`, `.codex/skills/`, `.opencode/skills/`, and the shared `.agents/skills/` workspace path used by Universal-compatible agents including Antigravity. Legacy Gemini CLI rows remain compatible, but Antigravity project skills are represented through `.agents/skills/`.

Disabled agents are skipped, and the `central` agent is always skipped (a project is never a global library by itself).

Each discovered SKILL.md becomes a `project_skill_installations` row keyed by `(project, skill_id, agent)`. Symlinks are tagged as `symlink`, real directories as `copy`.

## Layout

The page is split:

- **Left panel** — list of tracked projects, with search and a pin toggle. Hovering a row reveals quick actions for pin / rename / remove.
- **Right panel** — details of the selected project: its installed skills, the agent each one belongs to, and the install method (symlink badge in green, copy badge in amber). Each card has a one-click uninstall.

## Installing a skill from Central

The right panel's **Install from Central** button opens a dialog that:

1. Lists central skills with search.
2. Lists enabled agents that declare a project-skill directory. Universal workspace targets are grouped together; selecting Universal installs through a representative member into `.agents/skills/`.
3. Offers symlink (default) or copy.

Confirming the dialog runs `install_skill_to_project`, materialises the SKILL.md folder under the project's per-agent path, and writes the psi row.

## Uninstalling

Click the trash button on a skill card. SkillPort deletes the on-disk directory (or symlink) and removes the psi row. The central skill itself is untouched.

## Managing projects

- **Pin** — keeps a project at the top of the list, regardless of last-scanned order.
- **Rename** — display-only override; the underlying path is never changed.
- **Remove** — opens a dialog with a checkbox for **Uninstall all skills first**. Unchecked (default) leaves disk untouched; checked walks every psi row and clears the on-disk install before deleting the project.

## When to use Projects vs Marketplace

| Use case | Preferred entry point |
|----------|----------------------|
| Skills shipped with a local repo | Projects |
| Skills published by a vendor or community | [Marketplace](./marketplace) |
| Specific GitHub repo you want to mirror | [GitHub Import](./github-import) |

## Migration from Discover

The legacy Discover page was a full-disk crawl that depended on hard-coded scan roots; arbitrary user paths could not be added and the depth limit hid deeply nested projects. It was replaced by Projects in 0.10.x.

Visiting `/discover` now redirects to `/projects` and shows a one-time banner. Old discovered records are dropped on first migration; re-add the projects you care about.

## Where to go next

- Promote project skills to the central library: [Central Skills](./central-skills).
- Configure which agents are enabled: [Platforms](./platforms).

---

Last reviewed: 2026-05-14
