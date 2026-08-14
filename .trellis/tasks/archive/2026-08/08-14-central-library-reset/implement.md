# Implementation plan: unknown_source Central reset

## Guardrails

- Do not open or write `~/.skillsmanage/db.sqlite`, `~/.skillsmanage/skills/`, or live `targets/*/db.sqlite` in tests or manual `tauri:dev` verification against the repaired Local library.
- Prove Local and SSH with isolated fixtures / FakeRunner. Manual SSH verification, if any, uses the already-broken SSH target only, never Local.

## Stage 0: Red tests

- [x] Local `file_pool`: seed one GitHub-backed Central skill and one unknown-source Central skill; assert a new helper currently does not exist (or call site missing) by writing the intended tests first.
- [x] Fake SSH: unknown-source skill under remote Central root; membership skill retained.
- [x] Frontend: Unsupported tab has no reset control yet (failing test for the button + confirm).

Focused feedback:

```powershell
cd src-tauri
cargo test central_skills --locked
cargo test skill_update_inventory --locked
cd ..
pnpm exec vitest run src/test/components/central/updateCenter src/test/stores/centralSkillsStore.test.ts
```

## Stage 1: Service helper

- [x] `list_unknown_source_central_skill_ids(pool)` using the membership-absence predicate.
- [x] `preview_reset_unknown_source_skills_impl` / `reset_unknown_source_skills_impl` that freeze `TargetContext`, branch Local vs SSH/WSL onto existing delete preview/apply, then clear that pool's inventory after apply.
- [x] Typed errors stay in `CentralSkillsError`; no `Result<T, String>` in the service.
- [x] Tests: candidate filter, empty set, Local apply isolation, Fake SSH apply isolation, inventory cleared, GitHub-backed skill untouched, pool B unchanged.

Review gate: delete still goes through journaled batch delete; no raw `DELETE FROM skills`.

## Stage 2: IPC

- [x] Commands `preview_reset_unknown_source_skills` and `reset_unknown_source_skills` in `commands/skills.rs` using `resolve_target_context()` (never `active_target()` + later `active_db()`).
- [x] Register in `ipc_registry`, specta/command map, fixtures, i18n error map if a new code is introduced.
- [x] Operation log: target kind, counts, stable codes; redaction policy.
- [x] `pnpm docs:gen` / IPC coverage ratchet.

## Stage 3: Renderer

- [x] Store actions on `centralSkillsStore.installSlice` (preview + reset); Update Center dialog/Unsupported tab button.
- [x] Reuse `BatchDeleteCentralSkillsDialog` for confirm and copy-agent checkboxes.
- [x] `formatBackendError` + inline error + toast; disable confirm when preview count is 0.
- [x] After success: reload Central skills and Update Center inventory for the active target.
- [x] en/zh i18n for button, preview copy, success/empty states.
- [x] Vitest: button visibility, 0-count disabled, reject path, success refresh; mock via `mockIpcCommand`.

## Stage 4: Check

- [x] `python ./.trellis/scripts/task.py validate 08-14-central-library-reset`
- [x] Focused cargo + vitest above
- [x] `pnpm typecheck` and `pnpm lint`
- [x] `just ci`
- [x] Diff review: no real DB/skills paths, no startup rebuild, no CCR files

## Rollback

Revert the new commands/UI. Existing batch delete and Clear inventory remain. No migration.

## Follow-up before `task.py start`

Planning summary must be explicitly approved. Do not use the developer's repaired Local store as a verification target.
