# Implementation Plan

## Ordered Checklist

1. Remediate the dependency baseline before adding exceptions.
   - Reclassify/update `shadcn`, update `react-router-dom` and
     `@lobehub/icons`, then refresh `pnpm-lock.yaml` with pnpm.
   - Disable application-controlled SQLx/plugin defaults while retaining
     SQLite/runtime/derive; document the plugin's remaining internal default closure.
   - Apply only the researched precise Cargo updates for `plist`,
     `quinn-proto`, and `rustls-webpki`.
   - Run both live audits and record the remaining advisory IDs.
2. Add the dependency audit policy.
   - Implement the cross-platform Node runner plus pure normalization/policy
     functions.
   - Add the checked exception manifest with only still-live advisories.
   - Add fixture tests for unknown, exact, expired, malformed/duplicate,
     cross-ecosystem, unused, and Rust cases.
   - Add a package script and a local `just audit` recipe.
3. Harden workflows.
   - Pin all researched external Action refs across CI, release, and docs.
   - Add weekly github-actions Dependabot configuration.
   - Add Ubuntu/macOS source-validation and the blocking supply-chain job while
     preserving `just-ci`, reusable checkout, and manual package guards.
4. Extend contract tests.
   - Assert matrix runners, absence of soft-failure guards, audit command,
     stable Windows check, all-workflow SHA pins, and Dependabot shape.
   - Keep the release frozen-SHA and publication-order assertions green.
5. Synchronize `CONTRIBUTING.md` and
   `.trellis/spec/quality/ci-quality-gate.md` with the observable commands,
   job topology, exception policy, and hosted-runner evidence boundary.
6. Inspect the final diff and confirm no unrelated Trellis runtime, sibling task,
   audit report, or user-owned dirty file entered the change set.

## Validation

Run the smallest checks first, then the repository gate:

```powershell
pnpm exec vitest run src/test/contracts/dependencyAuditContract.test.ts src/test/contracts/ciWorkflowContract.test.ts src/test/contracts/releaseWorkflowContract.test.ts
pnpm audit:dependencies
pnpm typecheck
pnpm lint
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cd ..
just ci
```

Also re-run `pnpm audit --prod --json` and `cargo audit --json` directly to
confirm the wrapper did not hide raw findings. `task.py validate` verifies the
task artifacts before activation; it is not a substitute for these checks.

## Risky Files And Rollback Points

- `package.json`, `pnpm-lock.yaml`, `src-tauri/Cargo.toml`, and
  `src-tauri/Cargo.lock`: keep dependency edits focused; revert this group if
  compatibility checks fail instead of broad-updating unrelated packages.
- `.github/workflows/*.yml`: preserve job names, trigger semantics, release
  checkout refs, permissions, and package guards; contract tests are the first
  rollback signal.
- `security/dependency-audit-exceptions.json`: never extend expiry or add an
  advisory before proving the dependency cannot be fixed in the current scope.

## Pre-start Review Gate

- `prd.md`, `design.md`, `implement.md`, and research agree on scope.
- Both context JSONL files contain real entries.
- `task.py validate` passes.
- The latest final planning summary is explicitly approved before
  `task.py start 07-24-ci-supply-chain`.
