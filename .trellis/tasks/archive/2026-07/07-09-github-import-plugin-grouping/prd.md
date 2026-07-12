# Add plugin manifest grouping to GitHub import

## Goal

Make the GitHub repo import wizard understand Claude plugin manifest grouping so repositories such as `mattpocock/skills` preview skills in the same conceptual groups as `npx skills`, while preserving the current SKILL.md discovery behavior for repositories without those manifests.

## User Value

When a repository publishes skill grouping metadata, the import preview should present that structure instead of a long flat list. This makes large skill repositories easier to scan and lets users distinguish promoted plugin skills from additional valid skills.

## Confirmed Facts

- `npx skills add mattpocock/skills -g` parses `mattpocock/skills` as GitHub shorthand, clones the repo for this owner, discovers valid `SKILL.md` files, then groups skills using `.claude-plugin/plugin.json` when present.
- In `mattpocock/skills`, `.claude-plugin/plugin.json` names the group `mattpocock-skills` and lists 21 promoted skill directories.
- The current GitHub import preview already downloads a repository snapshot and discovers `SKILL.md` candidates from that snapshot.
- The current preview DTO exposes `sourcePath`, `rootDirectory`, `skillName`, and related fields, but no plugin/group field.
- The current preview UI renders `preview.skills` as one flat list.
- Verified 2026-07-09 against upstream `vercel-labs/skills` (commit #259): manifest-declared skill paths are searched at their declared depth; `marketplace.json` skill paths resolve relative to the plugin directory (`pluginRoot` + `source`), not the repo root; upstream accepts bare relative `source` values without a `./` prefix.
- Marketplace GitHub source sync (`services/marketplace/mod.rs::fetch_github_skills`) and skills.sh install (`services/marketplace/skills_sh.rs`) reuse the same snapshot discovery functions, so discovery changes propagate to those flows.
- The local preview/import path (`build_repo_skill_candidates_from_snapshot_at_path`) and the remote workspace candidate builder hard-fail on any invalid candidate; only the partial-import path tolerates per-skill failures.
- Product decision: manifest grouping is preview-only for this task. It must not be written to the Central database or imported skill metadata.

## Requirements

- If a repository snapshot includes `.claude-plugin/plugin.json`, parse its local `skills` entries and assign the manifest `name` to matching discovered skills.
- If a repository snapshot includes `.claude-plugin/marketplace.json`, parse local plugin entries and assign each plugin `name` to matching discovered skills.
- Manifest-declared local skill paths also act as explicit discovery hints. A configured path is previewed when it yields a valid skill candidate (its `SKILL.md` exists in the snapshot and passes candidate validation), even when the legacy priority/fallback discovery would not otherwise include it.
- Manifest hints are strictly additive. They must never suppress the legacy recursive fallback: a repository whose unlisted skills are today discovered by recursive fallback keeps discovering them when a manifest is present.
- Manifest skill paths must be normalized safely. Local relative paths are accepted with or without a `./` prefix (matching upstream `npx skills`); traversal, absolute paths, and remote/object manifest sources are ignored.
- Manifest parsing must be best-effort at both levels: (a) missing, invalid, or partially invalid manifest JSON must not prevent normal GitHub preview/import discovery; (b) a syntactically valid manifest entry whose path has no `SKILL.md`, or whose `SKILL.md` fails candidate validation, is silently dropped and must not fail preview, import, marketplace source sync, or skills.sh install.
- Shared-pipeline flows (marketplace GitHub source sync, skills.sh install) may additionally discover manifest-declared skills, but must not regress: every skill they discover today remains discovered, and invalid manifest hints never fail those flows.
- Any discovered valid skill not matched by manifest grouping remains importable and is displayed in an `Other` group only when at least one manifest group exists.
- Repositories without manifest grouping keep the current flat preview behavior.
- Grouping is preview metadata only. Import selection, conflict handling, file writing semantics, Central records, and imported skill metadata remain unchanged.
- User-visible UI labels introduced by this task must go through i18n.

## Acceptance Criteria

- [ ] `mattpocock/skills`-style `plugin.json` produces grouped preview metadata for the 21 manifest-listed skills and leaves unlisted valid skills ungrouped.
- [ ] Manifest-declared deep skill paths are discovered even if a repository also contains other priority-root skills that would otherwise prevent recursive fallback.
- [ ] Anti-suppression: with no priority-root skills, a manifest listing only a subset of deep skills still previews the unlisted deep skills via recursive fallback (the `mattpocock/skills` shape).
- [ ] A manifest entry pointing at a directory with no `SKILL.md`, or at a `SKILL.md` with invalid frontmatter, is dropped without failing preview or import.
- [ ] A preview with at least one grouped skill renders grouped sections, with ungrouped skills under `Other`.
- [ ] A preview with no grouped skills renders like the current flat list.
- [ ] `marketplace.json` local plugin entries can provide group names; remote/object sources are skipped.
- [ ] `marketplace.json` skill paths resolve relative to the plugin directory (`pluginRoot` + `source`), and bare relative `source` values without a `./` prefix are accepted.
- [ ] Unsafe or non-local manifest skill paths are ignored without failing preview.
- [ ] Existing GitHub import selection and duplicate-resolution behavior keeps working.
- [ ] Marketplace GitHub source sync keeps working on repositories with manifests, including repositories whose manifest contains broken entries.
- [ ] Imported skill summaries and persisted Central skill data do not include plugin group metadata.
- [ ] Targeted backend and frontend tests cover grouped, ungrouped, and invalid-manifest cases.
- [ ] `just ci` passes before the task is reported complete.

## Out of Scope

- Persisting plugin group metadata into the central skills database after import.
- Adding `pluginName` to `ImportedGitHubSkillSummary`, central sync state, or skill detail metadata.
- Changing installation target selection, duplicate resolution, or import write behavior.
- Replacing root-directory/category metadata; `rootDirectory` remains available as before.
- Fetching manifests from remote plugin sources declared in `marketplace.json`.

## Open Questions

- None currently blocking planning.
