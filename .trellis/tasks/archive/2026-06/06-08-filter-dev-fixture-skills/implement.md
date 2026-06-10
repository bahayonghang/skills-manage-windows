# Filter dev fixture skills from GitHub import discovery - implementation

## Checklist

1. Extend GitHub import skip-list segments.
   - File: `src-tauri/src/services/github_import/types.rs`
   - Add only `test`, `tests`, `fixture`, and `fixtures`.
   - Verify: existing discovery helper uses these segments through
     `has_skipped_discovery_segment`.

2. Add focused regression coverage.
   - File: `src-tauri/src/services/github_import/tests.rs`
   - Add a snapshot shaped like `everyinc/compound-engineering-plugin` with:
     - `plugins/compound-engineering/skills/ce-work/SKILL.md`
     - `tests/fixtures/custom-paths/custom-skills/custom-skill/SKILL.md`
     - `tests/fixtures/custom-paths/skills/default-skill/SKILL.md`
     - `tests/fixtures/sample-plugin/skills/skill-one/SKILL.md`
   - Assert that only the real plugin skill is discovered.
   - Add a small guard assertion that `sample` / `example` directories are not
     skipped merely because of their names.

3. Run focused Rust tests.
   - Command: `cd src-tauri; cargo test github_import`
   - Verify: new and existing GitHub import discovery tests pass.

4. Run repo gate before completion.
   - Command: `just ci`
   - Verify: frontend typecheck/lint and Rust clippy pass.

## Risky Files

- `src-tauri/src/services/github_import/types.rs`: the skip list affects both
  local GitHub import previews and remote SSH import previews.
- `src-tauri/src/services/github_import/tests.rs`: tests are broad; keep new
  test data focused and avoid changing existing fixtures unnecessarily.

## Review Gate Before Start

Before `task.py start`, confirm that the approved scope is:

- filter only `test`, `tests`, `fixture`, and `fixtures`;
- do not filter `sample`, `samples`, `example`, or `examples`;
- do not clean existing Central Skills rows as part of this implementation.
