# Harden main protection and test workflow

## Goal

Protect the public repository's `main` branch from unreviewed or unverified changes, and make the repository's local and GitHub-hosted quality gates comprehensive, consistent, and maintainable.

## Background

- Repository: `bahayonghang/skills-manage-windows`; GitHub reports `main` as the default branch.
- GitHub reports `main` as unprotected and returns no repository rulesets.
- The only direct collaborator is the owner `bahayonghang`; the repository has no `CODEOWNERS` file.
- `.github/workflows/ci.yml` runs only for `release.published`, so pull requests and ordinary pushes currently receive no CI gate.
- The latest successful release produced a `just-ci` check from GitHub Actions app id `15368`.
- `scripts/run-ci.mjs` already runs frontend type checking, linting, size contracts, and Vitest in parallel with Rust entrypoint contracts, Clippy, and unit tests.
- Rust formatting, locked dependency resolution, all-target Clippy, and the frontend production build are not part of that gate.
- Repository documentation says `just ci` gates pull requests, which is not true with the current workflow trigger.
- Project policy treats Windows as first-class, requires `just ci` before completion, and requires a real Windows Tauri bundle check for packaging or release workflow changes.

## Requirements

### R1. Pull Request And Push CI

- Run the stable `just-ci` check for pull requests targeting `main`, pushes to `main` and `dev`, manual dispatches, and published releases.
- Keep expensive cross-platform smoke packaging limited to published releases and manual dispatches.
- Preserve concurrency cancellation so obsolete runs do not waste runner time.

### R2. Complete Local Quality Gate

- Keep `just ci` and the GitHub Actions `just-ci` job on the same `scripts/run-ci.mjs` entrypoint.
- The web chain must cover TypeScript type checking, ESLint, source-size contracts, Vitest, and the production frontend build.
- The Rust chain must cover entrypoint contracts, `cargo fmt --check`, Clippy across all targets with warnings denied, and tests.
- Cargo validation must honor the committed lockfile.
- Existing fail-fast behavior between the parallel web and Rust chains must remain intact.

### R3. CI Contract Regression Coverage

- Add a focused automated contract test that parses the workflow as YAML and verifies triggers, the stable `just-ci` name, the shared local entrypoint, Rust toolchain components, and package-job event guards.
- Use a direct development dependency on a YAML parser instead of ad hoc string parsing.

### R4. Documentation Consistency

- Update contributor guidance, root English/Chinese README validation summaries, project agent guidance, and English/Chinese `just` reference docs to describe the real gate.
- State which events run the required quality gate and which events run smoke packaging.

### R5. Main Branch Protection

- Require pull requests for `main` while requiring zero approving reviews, so the sole owner can merge.
- Require the strict/up-to-date `just-ci` status check and bind it to GitHub Actions app id `15368`.
- Apply the required checks to administrators.
- Require conversation resolution.
- Disallow force pushes and branch deletion.
- Do not require signed commits, linear history, code-owner review, deployments, or a merge queue because the repository does not currently support those policies.

### R6. Scope And Safety

- Do not change unrelated product behavior or publish a release.
- Do not push or merge repository commits without separate authorization.
- Preserve unrelated user work and use the authenticated keyring GitHub identity for administration because the environment `GH_TOKEN` lacks repository-administration access.

## Acceptance Criteria

- [x] AC1 (R1): A parsed workflow contract proves `pull_request` targets `main`, `push` targets `main` and `dev`, and `workflow_dispatch` plus `release.published` remain available.
- [x] AC2 (R1): `just-ci` runs without event guards, while every smoke-package job is guarded to release/manual events.
- [x] AC3 (R2): `just ci` passes with typecheck, lint, size contracts, Vitest, frontend build, entrypoint contracts, Rust formatting, all-target Clippy, and locked Rust tests.
- [x] AC4 (R3): The focused CI contract Vitest passes and fails meaningfully when a required trigger, job name, entrypoint, toolchain component, or package guard is missing.
- [x] AC5 (R4): English and Chinese project documentation accurately describe the implemented local and remote gates.
- [x] AC6 (R5): GitHub reads `main` back as protected with strict `just-ci`, app id `15368`, administrator enforcement, pull requests with zero approvals, conversation resolution, and force-push/deletion disabled.
- [x] AC7 (R5): The remote policy does not enable signed commits, linear history, code-owner reviews, deployments, locking, or a merge queue.
- [x] AC8 (R6): A Windows Tauri build succeeds and an installer/bundle artifact is confirmed after the workflow change.
- [x] AC9 (R6): The final diff contains only task artifacts, CI/test tooling, workflow configuration, directly related documentation, and mechanical formatting/lint baseline changes required by the new gate.

## Out Of Scope

- Product feature changes unrelated to testability or CI correctness.
- Code coverage thresholds, browser end-to-end automation, fuzzing, or paid third-party testing services.
- Publishing a release, pushing the implementation branch, opening a pull request, or merging `dev` into `main`.
- Rewriting the existing release workflows or changing signing secrets.
