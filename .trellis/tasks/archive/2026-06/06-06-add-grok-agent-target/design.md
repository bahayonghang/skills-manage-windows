# Add Grok agent target design

## Architecture and Boundaries

This task adds a skills target, not a usage provider. The target surface spans:

- Backend seed data and built-in agent constants in `src-tauri/src/db/`.
- Project and global installation path resolution in existing installation/project helpers.
- Frontend platform visibility, target grouping, and icon rendering in `src/lib/` and `src/components/platform/`.
- Public documentation in `README.md` and `README_CN.md`.

No new database table or migration is expected. Existing `seed_builtin_agents()` performs `INSERT ... ON CONFLICT DO UPDATE` for built-in rows, so adding Grok to the built-in seed list should add it to existing databases on next startup.

## Data Contracts

Proposed `Agent` seed:

- `id`: `grok`
- `display_name`: `Grok`
- `category`: `coding`
- `global_skills_dir`: `<home>/.grok/skills`
- `project_skills_dir`: `.grok/skills`
- `icon_name`: `grok`
- `is_builtin`: `true`

Remote-home initialization should pass through the existing `builtin_agents_for_posix_home()` rewrite path. Since `.grok/skills` follows the same `<home>/.tool/skills` shape as non-universal agents, no custom remote branch should be required beyond test coverage.

## Compatibility Notes

This fork intentionally groups several global and project targets under Universal Agents. Upstream Grok is different: v1.23.0 documents a Grok-specific global and project directory. The approved design is to keep Grok independent so the implementation matches the upstream release and avoids silently writing Grok skills into `.agents/skills`.

Existing Grok usage ingestion under `src-tauri/src/services/usage/providers/grok.rs` is separate and should remain unchanged.

## UI Notes

Frontend platform sorting should show Grok near Codex in enabled-platform lists. Grok must not be added to Universal target member arrays.

`PlatformIcon.tsx` should map `grok` to a real icon source if available from `@lobehub/icons`; otherwise use a small local custom SVG branch. Avoid adding a large bitmap unless no lightweight icon path exists.

## Trade-offs

- Independent Grok target matches upstream and is easiest to reason about. It adds one more visible target in Platform UI.
- Universal grouping keeps Platform UI more compact but conflicts with upstream's documented `.grok/skills` project path and could make Grok skills invisible to Grok if Grok does not read `.agents/skills`.
- Approved trade-off: choose upstream compatibility over grouping compactness.
