# Filter generic skill updates and style source metadata

## Goal

Keep generic remote candidates named exactly `skill` out of Update Center decisions so they cannot pollute the Central skill library, and make Update Center source metadata easier to scan by giving repository, path, URL, and hash fields distinct visual treatments.

User value:

- Update Center no longer presents or applies low-quality generic `skill` candidates such as `agent_reach/skill`.
- Central skill IDs remain meaningful and do not get overwritten by generic directory names from recursive discovery.
- Users can visually distinguish repository, source path, URL, cache, and hash metadata in dense Update Center rows.

## Assumptions

- "A skill just called `skill`" means the normalized candidate skill id is exactly `skill`, regardless of repository path.
- The filtering should be enforced in backend candidate discovery / import planning, not only hidden in React.
- The task should not automatically delete an already-existing Central skill named `skill`; destructive cleanup is a separate explicit user action.
- The visual change should reuse the existing `SourceMeta` component so Updatable, Added, and Removed rows remain consistent.

## Confirmed Facts

- `src/components/central/updateCenter/SourceMeta.tsx` renders repository, path, URL, cache, and hash rows with the same neutral chip classes.
- `UpdatableTabPanel`, `RemoteAddedTabPanel`, and `RemoteMissingTabPanel` all reuse `SourceMeta`, so one component-level style change covers the visible Update Center source metadata rows shown in the screenshot.
- `src/test/UpdateCenterSourceMeta.test.tsx` already verifies that repository, path, URL, cache, and hash values render for Update Center rows.
- Remote additions are collected by `src-tauri/src/commands/central_updates/repository_sync.rs::collect_remote_added_skills`, which calls `github_import::inspect_repo_skill_candidates_from_snapshot_at_path`.
- Candidate IDs are derived in `src-tauri/src/services/github_import/source.rs::build_remote_skill_candidate`; non-root candidates use the directory leaf (`manifest.skill_directory_name`) passed through `sanitize_skill_id`.
- `src-tauri/src/services/github_import/tests.rs` currently has `recursive_fallback_skips_large_generated_directories`, which accepts `packages/example/skill/SKILL.md` as source path `packages/example/skill` with `skill_id == "skill"`. This existing test captures the behavior that should change.

## Requirements

- Exclude any remote candidate whose final normalized skill id is exactly `skill` from Update Center additions and update/import decisions.
- The exclusion must happen before pending additions are persisted, before selected additions can be imported, and before force mirror can import remote additions.
- Existing valid skill discovery under `skills/`, `.agents/skills/`, `.codex/skills/`, curated/system roots, and known content-skill layouts must keep working.
- The exclusion must not break root repository `SKILL.md` imports, because root candidates derive their id from the repository name rather than the directory leaf.
- Failed/invalid reporting should be clear enough for logs/tests, but the generic candidate should not appear as a user-selectable failed repository if that would make Update Center noisier.
- `SourceMeta` must give repository, path, URL, and hash rows distinct color/style treatments. Cache can remain neutral or receive its own secondary treatment.
- Metadata values must remain readable in narrow rows: long URLs and hashes should still wrap or break without overlapping.
- All changed user-visible text, if any, must go through `src/i18n/locales/en.json` and `src/i18n/locales/zh.json`.

## Acceptance Criteria

- [ ] A snapshot containing `agent_reach/skill/SKILL.md` or `packages/example/skill/SKILL.md` does not produce a user-selectable remote addition with `skillId == "skill"`.
- [ ] Refreshing Update Center in repository sync mode does not persist a pending addition for a generic `skill` candidate.
- [ ] Applying update decisions cannot import a filtered generic `skill` candidate, even if a stale or crafted selection references that source path.
- [ ] Force mirror repository import does not create a Central skill with id `skill` from a generic directory candidate.
- [ ] Existing non-generic candidates, including `skills/planning-with-files-zh`, `.agents/skills/universal-skill`, and `plugins/compound-engineering/skills/ce-work`, remain discoverable.
- [ ] `SourceMeta` renders repository, path, URL, cache, and hash rows without layout regression.
- [ ] Repository, path, URL, and hash rows have distinguishable visual treatments in the Update Center.
- [ ] Focused Rust tests cover candidate filtering and inventory/pending-addition behavior.
- [ ] Focused React tests cover the differentiated source metadata rendering.
- [ ] `pnpm typecheck`, `pnpm lint`, relevant Vitest tests, relevant Cargo tests, and final `just ci` pass before completion.

## Out of Scope

- Automatically deleting existing Central skills named `skill`.
- Reworking the full remote repository discovery heuristic beyond the exact generic `skill` exclusion.
- Changing Update Center tab structure, selection math, or force update/mirror UX beyond the filtering needed to prevent generic imports.
- Adding new source metadata fields.

## Open Questions

None blocking. Recommended implementation direction is to enforce an exact `skill_id == "skill"` candidate filter in the shared GitHub import/discovery layer, then keep UI changes limited to `SourceMeta` styling and tests.
