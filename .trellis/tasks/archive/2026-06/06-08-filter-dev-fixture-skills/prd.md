# Filter dev fixture skills from GitHub import discovery

## Goal

Prevent GitHub repository import discovery from treating repository development
fixtures as real Central Skills candidates.

The immediate user-visible issue is that importing or checking
`everyinc/compound-engineering-plugin` shows fixture skills such as
`custom-skill`, `default-skill`, `skill-one`, and `disabled-skill` in Central
Skills. These rows are not published plugin skills; they are test fixture
manifests from the source repository.

## User Value

Users can import real skills from GitHub repositories without reviewing,
installing, updating, or deleting skills that came from a repository's test or
development fixtures.

## Confirmed Facts

- The screenshot shows `custom-skill`, `default-skill`, and `skill-one` mixed
  into the Central Skills grid while filtering by repository
  `everyinc/compound-engineering-plugin`.
- External repository evidence, collected from a shallow Git tree inspection of
  `https://github.com/everyinc/compound-engineering-plugin`, shows the real
  plugin skills under `plugins/compound-engineering/skills/.../SKILL.md`.
- The same external repository tree shows the red-box examples under
  development fixture paths:
  - `tests/fixtures/custom-paths/custom-skills/custom-skill/SKILL.md`
  - `tests/fixtures/custom-paths/skills/default-skill/SKILL.md`
  - `tests/fixtures/sample-plugin/skills/disabled-skill/SKILL.md`
  - `tests/fixtures/sample-plugin/skills/skill-one/SKILL.md`
- Local code evidence: GitHub import discovery is implemented in
  `src-tauri/src/services/github_import/source.rs`.
- Local code evidence: discovery first checks direct and priority skill roots,
  then uses bounded recursive fallback when no priority manifests are found.
- Local code evidence: skipped discovery segments are centralized in
  `SKIP_DISCOVERY_DIRS` in `src-tauri/src/services/github_import/types.rs`.
- Local code evidence: the current skip list covers generated/build directories
  such as `.git`, `node_modules`, `dist`, `build`, `target`, `outputs`, and
  `__pycache__`, but not `tests` or `fixtures`.
- Local test evidence: existing GitHub import tests already cover recursive
  fallback and skip-list behavior in `src-tauri/src/services/github_import/tests.rs`.
- User scope decision: only unambiguous test fixture directory segments should
  be added now: `test`, `tests`, `fixture`, and `fixtures`.

## Requirements

- Filter unwanted development fixture manifests during GitHub import discovery,
  before candidates are built or previewed.
- Keep the filtering rule shared by local archive preview and remote SSH
  workspace preview, since both call `discover_skill_manifests_from_paths`.
- Preserve valid discovery for supported layouts:
  - repository root `SKILL.md`
  - common skill roots such as `skills/`, `.agents/skills/`, `.claude/skills/`,
    and `.codex/skills/`
  - explicitly supplied repository subpaths
  - bounded recursive fallback for repositories without standard roots
- Add regression tests that prove `tests/fixtures/.../SKILL.md` candidates are
  skipped and real plugin skill paths remain discoverable.
- Do not add broader `sample`, `samples`, `example`, or `examples` filtering in
  this task.
- Keep the change in the parsing/discovery layer rather than only hiding cards
  in the Central Skills UI.
- Do not delete existing Central Skills rows from the user's database unless
  cleanup is explicitly approved as part of this task.

## Acceptance Criteria

- [ ] A repository snapshot containing
      `tests/fixtures/custom-paths/custom-skills/custom-skill/SKILL.md` does not
      produce a `custom-skill` candidate.
- [ ] A repository snapshot containing
      `tests/fixtures/custom-paths/skills/default-skill/SKILL.md` does not
      produce a `default-skill` candidate.
- [ ] A repository snapshot containing
      `tests/fixtures/sample-plugin/skills/skill-one/SKILL.md` does not produce
      a `skill-one` candidate.
- [ ] A repository snapshot containing real plugin skills under
      `plugins/compound-engineering/skills/.../SKILL.md` still produces those
      real candidates through the existing fallback path.
- [ ] Directories named `sample`, `samples`, `example`, and `examples` are not
      newly filtered by this task.
- [ ] Existing tests for root, priority-root, agent-specific, duplicate-name,
      recursive fallback, and source-subpath discovery still pass.
- [ ] `cargo test github_import` passes.
- [ ] Final verification includes `just ci`, unless planning is not yet approved
      for implementation.

## Out of Scope

- UI-only hiding of already-imported fixture cards.
- Automatic deletion of existing `custom-skill`, `default-skill`, `skill-one`,
  or `disabled-skill` rows from the user's Central Skills database.
- Rewriting the full GitHub import flow or replacing recursive fallback.
- Adding user-configurable import ignore rules.

## Notes

- Complex enough to add `design.md` and `implement.md` before `task.py start`.
