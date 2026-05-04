# Marketplace

The Marketplace view aggregates remote skill catalogs so you can discover and install skills without leaving the desktop. It speaks GitHub directly, so any public repository that follows the SKILL.md convention can be a source.

## Three tabs

| Tab | Content | Source |
|-----|---------|--------|
| Recommended | A curated set of vetted skills, grouped by tag. | Bundled with the app (`src/data/officialSources.ts`). |
| Official sources | A directory of well-known publishers and their repositories. | Bundled list, refreshed via app updates. |
| My sources | Your own remote sources (GitHub repos you have added). | Stored locally in the SQLite settings table. |

The Recommended and Official tabs do not require a network connection until you explicitly install a skill.

## How a sync works

When you sync a source, SkillPort:

1. Authenticates the request with your GitHub PAT if you provided one (Settings → GitHub PAT).
2. Walks the repository's root and `skills/` directory.
3. Parses each `SKILL.md` frontmatter to extract `name` and `description`.
4. Caches results in the `marketplace_skills` table for fast browsing.
5. Falls back to anonymous, retried requests if the PAT is missing or rate-limited.

The cache is per-source; clearing one does not affect the others.

## Installing from the Marketplace

Pick a skill from any tab, then press **Install**. SkillPort:

1. Downloads the SKILL.md tree to `~/.skillsmanage/skills/<name>/`.
2. Records the source so future updates know where to fetch from.
3. Optionally lets you continue with the standard install dialog to fan it out to platforms.

## Updates

The Central view shows an update badge when a Marketplace-sourced skill has a newer commit upstream. Use the Updates panel to review changes and apply them. The original directory is replaced atomically; the previous version is overwritten in place.

## Adding a custom source

In **My sources**, paste a GitHub URL. The source is stored locally and shows up in subsequent searches. Remove a source any time to drop its cached entries.

## Where to go next

- Pull a single one-off repository: [GitHub Import](./github-import).
- Make a skill self-explanatory before installing: [AI Explanation](./ai-explanation).

---

Last reviewed: 2026-05-04
