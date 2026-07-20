# Validation Evidence

## Test-First Baseline

- Initial `pnpm vitest run src/test/ciWorkflowContract.test.ts`: failed 3/3 as expected.
- Failures proved missing PR/push/manual triggers, missing `rustfmt`, and missing package event guards.

## Local Quality Gate

- Final focused CI contract: 3/3 passed.
- Final `just ci`: passed in 102.2 seconds.
- Frontend: 128 files passed; 1400 tests passed and 1 skipped; production Vite build succeeded.
- Rust: formatting passed; all-target locked Clippy passed; 874 library tests passed with 4 ignored; CLI and project E2E suites passed.
- Enabling formatting/all-target Clippy required a one-time mechanical baseline: rustfmt normalization plus 20 existing test lint fixes. No production behavior changed.

## Windows Bundle

- Command: `pnpm tauri build`.
- Result: passed in 339.9 seconds.
- Artifact: `src-tauri/target/release/bundle/nsis/SkillPort_0.10.14_x64-setup.exe`.
- Size: 15,207,626 bytes.
- SHA-256: `3FBD5AE361FF6A97A199A9EED68EF882A9877B578C45828E06EC2624332921AD`.

## GitHub Main Protection

- Precondition readback: `main` was unprotected; repository rulesets were empty.
- First request was rejected with HTTP 422 because the current API schema treats `contexts` and `checks` as mutually exclusive; GitHub made no change.
- Corrected request used `checks` only and succeeded.
- Final readback:
  - branch protected: `true`
  - strict required check: `just-ci`
  - required check app id: `15368`
  - enforce administrators: `true`
  - required approvals: `0`
  - conversation resolution: `true`
  - force pushes: `false`
  - deletions: `false`
  - linear history: `false`
  - signed commits: `false`
  - branch lock: `false`
  - repository rulesets: none

## Scope

- No implementation commit was pushed.
- No pull request, merge, tag, release, secret, or signing configuration was changed.
