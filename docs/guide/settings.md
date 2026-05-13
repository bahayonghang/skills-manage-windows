# Settings

Settings is the single panel where every configuration lives. Each section is independent; changes are persisted immediately and apply to the current target (Local or the active SSH target, where applicable).

## Sections

| Section | Purpose |
|---------|---------|
| Scan Directories | Add or remove project paths Discover should scan. |
| Custom Platforms | Define new platforms (id, display name, skills directory, category). |
| Platform Visibility | Hide platforms you do not use; they still scan in the background. |
| Remote Targets | SSH targets used by [SSH Remote](./ssh-remote). |
| GitHub PAT | Personal access token for [Marketplace](./marketplace) and [GitHub Import](./github-import). |
| AI | Provider and key for [AI Explanation](./ai-explanation). |
| About | App version, database path, links to changelog and security docs. |

## Behaviour rules

- **Local-first**: every value lives in `~/.skillsmanage/db.sqlite` (or the matching per-target cache database under `~/.skillsmanage/targets/<id>/`).
- **Atomic updates**: setting changes commit immediately; no separate "save" button. Closing the dialog does not roll anything back.
- **No hidden migrations**: switching the active SSH target swaps the cache file rather than mutating the previous one.
- **Hidden paths**: secrets (GitHub PAT, AI keys, SSH passwords) appear masked after first save and can be cleared but not re-displayed in plain text.

## Recommended order for a fresh install

1. Add a **GitHub PAT** if you plan to use Marketplace or GitHub Import.
2. Configure an **AI** provider only if you want to generate explanations.
3. Add **Scan Directories** for any project directories you care about.
4. Optionally hide platforms you never use under **Platform Visibility**.
5. Add **Remote Targets** if you manage SSH-accessible machines.

You can revisit any of these any time. None of them is required for the basic Central → Platform install workflow.

## Where to go next

- Theme and language: [i18n and Themes](./i18n-and-themes).
- Resolve platform issues you might hit: [Troubleshooting](./troubleshooting).

---

Last reviewed: 2026-05-04
