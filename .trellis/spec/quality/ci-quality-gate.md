# Repository CI Quality Gate

## 1. Scope / Trigger

This contract applies whenever changing test commands, `just ci`, GitHub Actions CI triggers or job names, Rust toolchain checks, contributor validation docs, or `main` branch protection.

The repository is Windows-first, but the merge gate covers the full frontend and Rust codebase. Cross-platform package builds are release/manual smoke evidence, not routine pull-request checks.

## 2. Signatures

```text
just ci
  -> node scripts/sync-version.mjs
  -> node scripts/run-ci.mjs

GitHub Actions required job/check:
  workflow: .github/workflows/ci.yml
  job id: ci
  job name / check context: just-ci
  check app id: 15368 (GitHub Actions)
```

The orchestrator owns two parallel, fail-fast chains:

```text
web: typecheck -> lint -> sizecheck -> test -> build
rust: entrypointcheck -> fmt --check -> clippy --all-targets --locked -> test --locked
```

## 3. Contracts

### Event contract

| GitHub event | `just-ci` | Package smoke jobs |
| --- | --- | --- |
| Pull request to `main` | Run | Skip |
| Push to `main` or `dev` | Run | Skip |
| `workflow_dispatch` | Run | Run |
| `release.published` | Run | Run |

Package job guards must remain:

```yaml
if: ${{ github.event_name == 'release' || github.event_name == 'workflow_dispatch' }}
```

### Branch protection contract

- `main` requires an up-to-date `just-ci` check bound to app id `15368`.
- Administrators are subject to the check.
- A pull request is required, with zero approvals for the single-maintainer repository.
- Conversations must be resolved; force pushes and deletion are disabled.
- Signed commits, linear history, branch locking, code-owner reviews, and merge queues are not enabled without a separate migration.

For GitHub REST updates, `required_status_checks.checks` and legacy `contexts` are alternative schemas. Use `checks` alone when binding `app_id`; sending both returns HTTP 422.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| PR/push trigger removed | `ciWorkflowContract.test.ts` fails |
| `just-ci` renamed or guarded | Contract test fails; do not update branch protection casually |
| Package job lacks event guard | Contract test fails |
| Rust is not formatted | `cargo fmt --check` fails |
| Test/bin target has a Clippy warning | all-target Clippy fails |
| Cargo lockfile would change | `--locked` command fails |
| Frontend unit tests pass but bundling is invalid | `pnpm build` fails |
| GitHub protection payload sends `contexts` and `checks` | HTTP 422; retry with `checks` only after confirming no remote change |
| Environment `GH_TOKEN` lacks administration permission | Do not persist or print another token; use the authenticated keyring identity for that process |

## 5. Good / Base / Bad Cases

- Good: a PR to `main` runs one stable `just-ci` gate; package jobs skip; merge waits for the current head to pass.
- Base: a manual run executes `just-ci` plus all smoke packages for pre-release validation.
- Bad: expanding the whole release packaging matrix to every PR, which adds tens of minutes without improving routine feedback proportionally.
- Bad: adding a new local check without adding it to `scripts/run-ci.mjs`, or duplicating different commands directly in the workflow.

## 6. Tests Required

- `pnpm vitest run src/test/contracts/ciWorkflowContract.test.ts`
  - Parse YAML 1.2; assert event branches, stable job name, shared entrypoint, Rust components, and every package guard.
- `just ci`
  - Assert the complete frontend and Rust chains pass from their shared local/remote entrypoint.
- For CI, packaging, or release workflow changes on Windows: `pnpm tauri build` and confirm the expected NSIS/MSI bundle path.
- After changing protection: read the GitHub protection object and branch endpoint back; do not treat a successful PUT alone as evidence.

## 7. Wrong vs Correct

### Wrong

```yaml
on:
  release:
    types: [published]

jobs:
  ci:
    name: renamed-check
```

This leaves pull requests untested and breaks the required status context.

### Correct

```yaml
on:
  pull_request:
    branches: [main]
  push:
    branches: [main, dev]
  workflow_dispatch:
  release:
    types: [published]

jobs:
  ci:
    name: just-ci
```
