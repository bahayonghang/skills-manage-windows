# Implementation plan

## Checklist

1. **Repo batch read**
   - In `src-tauri/src/db/repos/projects_repo.rs` add `list_linked_projects_for_skills(pool, skill_ids) -> HashMap<String, Vec<LinkedProjectRow>>`.
   - SQL: `project_skill_installations` JOIN `projects`，`skill_id IN (...)`，按 `project_id` 去重，排序 pinned / basename / id。
   - Chunk `IN` at 500. Empty input → empty map, no query.
   - Do not put this SQL in `central_skills` or `installation`.

2. **DTO + enrichment**
   - Add `LinkedProject { project_id, path }` and `SkillWithLinks.linked_projects` with `serde(default)`.
   - Fill it in `skills_with_links_from_rows` (`central_skills/query.rs`) for both unpaged and page paths.
   - Keep CLI struct compatible with `serde(default)`.
   - Extend `src/types/index.ts` `SkillWithLinks`. Fixtures / store mocks default `[]`.

3. **Persist after Central project install**
   - Add `services/projects` helper `record_project_skill_from_central_install(...)`.
   - Local: `add_project_impl` + upsert psi (`source_origin=central`).
   - Remote: insert/reuse project by normalized POSIX path, no local `is_dir`.
   - Call it from the batch/project install success **and** skip-already-exists branches (`installation/batch.rs` and any direct `install_central_skill_to_project` callers that return Installed/Skipped).
   - On DB failure after a new FS write: compensate like `install_skill_to_project_impl` (`crud.rs:500-525`). Skip + no FS change: do not delete existing target.

4. **Card UI**
   - Central-only prop `linkedProjects` on `UnifiedSkillCard` types → `toModel` → chips row above footer.
   - `buildCentralSkillCardProps` maps `skill.linked_projects`.
   - List and grid both receive it (do not leave it only on `renderGridCard`).
   - `getPathBasename` + `formatPathForDisplay`. i18n en/zh for aria-label.
   - Update `unifiedSkillCardVariants.test.tsx` mutex examples.

5. **Tests**
   - Rust: Central project install writes project+psi; skip existing target still records; `get_central_skills` / page enrichment returns deduped `linked_projects`; empty page skips IN.
   - Frontend: card chips (basename, dedupe, empty, list+grid); install-to-project store refresh includes `linked_projects`.
   - Prefer colocated files: `src-tauri/src/services/projects/tests.rs`, `central_skills/tests.rs` / `pagination_tests.rs`, `src/test/components/skill/`, `src/test/stores/centralSkillsStore.test.ts`, `src/test/pages/CentralSkillsView.*.test.tsx` only if the page assertion is necessary.

6. **Docs / codegen**
   - No schema migration. Run `pnpm docs:gen` only if generated architecture docs include `SkillWithLinks` fields. Include generated files if they change.

## Validation

While iterating:

```text
pnpm exec vitest run src/test/components/skill/unifiedSkillCardVariants.test.tsx src/test/stores/centralSkillsStore.test.ts src/test/lib/path.test.ts
cd src-tauri
cargo test --locked projects:: central_skills:: installation::
```

Before claiming done:

```text
just ci
```

If IPC/DTO docs changed: `pnpm docs:gen` then `pnpm docs:gen:check` must be clean as part of `just ci`.

## Risky files / rollback

| File | Risk |
|---|---|
| `installation/batch.rs` / `installation/project.rs` | Double FS install if mistakenly delegated to `install_skill_to_project_impl` |
| `projects/crud.rs` `add_project_impl` | Remote caller using local `is_dir` will fail or register a wrong path |
| `UnifiedSkillCard.tsx` | List vs grid prop wiring; compact hiding would hide the new chips |
| `SkillWithLinks` fixtures | Missing `linked_projects` will break typecheck until defaulted |

Rollback: revert the helper call first if persistence misfires; the card can ship behind empty `linked_projects` without breaking global agent icons.

## Follow-up before `task.py start`

- `implement.jsonl` / `check.jsonl` have real spec + research entries (not only `_example`).
- User explicitly approves this planning summary.
