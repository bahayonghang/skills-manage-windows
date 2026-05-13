# GitHub Import

GitHub Import is a wizard for pulling skills from any specific GitHub repository. Unlike Marketplace, which keeps a curated list, this flow is for one-off imports — anything from your own repo to a colleague's prototype.

## Wizard stages

```text
[Wizard] ──┬── Step 1: paste repo URL or owner/repo
           ├── Step 2: pick branch and source path
           │            (root, skills/, or any sub-dir)
           ├── Step 3: preview discovered SKILL.md files
           │            (name, description, target central path)
           └── Step 4: confirm import → write to
                       ~/.skillsmanage/skills/
```

The preview is read-only — nothing is written until you confirm.

## Authentication

- **GitHub PAT (recommended)**: stored locally in the settings table. Adds 5,000 requests/hour and works on private repos.
- **Anonymous fallback**: used when no PAT is configured. Limited to 60 requests/hour and public repos. The wizard automatically retries on rate-limit errors.

The PAT is never sent anywhere except `api.github.com`. Add or remove it in Settings → GitHub PAT.

## What gets imported

For each SKILL.md the wizard finds, it copies the *whole skill directory* — every file alongside SKILL.md (scripts, references, assets) goes into central storage. Hidden files like `.git` are skipped.

If a skill of the same name already exists, you can:

- **Skip** — keep the existing version.
- **Replace** — overwrite with the new tree.
- **Rename** — store the new one under a different name.

## After import

Imported skills appear in Central with a source label that points back to the GitHub URL. Updates surface in the same Updates panel used by Marketplace skills.

## Common pitfalls

- **No SKILL.md found**: the wizard recurses one level under the chosen source path. If a repo nests skills deeper, point the wizard at the subdirectory directly.
- **403 from API**: the repo is private and the PAT is missing or lacks `repo` scope. Add a PAT with the right scope.
- **Frontmatter parse errors**: the wizard skips invalid SKILL.md entries and shows them in the preview as warnings; fix the source repo and re-run.

## Where to go next

- Track updates from imported sources: [Marketplace](./marketplace) → Updates panel.
- Inspect a skill's content quickly: [AI Explanation](./ai-explanation).

---

Last reviewed: 2026-05-04
