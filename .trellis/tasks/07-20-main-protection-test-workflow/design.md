# Design: main protection and test workflow

## Overview

Use one stable required check, `just-ci`, as the merge gate. The local `just ci` recipe and GitHub Actions job continue to call the same Node orchestrator, avoiding two independent definitions of the quality standard. Release/manual events retain cross-platform smoke packaging, while PR and ordinary push events receive only the faster quality gate.

The branch policy uses GitHub's classic branch-protection endpoint because the repository has no existing ruleset and the requested target is one branch. The policy binds the required check to GitHub Actions app id `15368`, preventing another integration from satisfying a same-named status.

## Boundaries

### Repository Changes

- `.github/workflows/ci.yml`: events, Rust formatter setup, and event guards on package jobs.
- `scripts/run-ci.mjs`: complete web and Rust gate definitions.
- `src/test/ciWorkflowContract.test.ts`: parsed YAML contract regression test.
- `package.json` and `pnpm-lock.yaml`: direct YAML parser development dependency.
- `justfile`, `AGENTS.md`, `CONTRIBUTING.md`, `README.md`, `README_CN.md`, and bilingual `docs/*/cli-just.md`: accurate test-flow documentation.

### Remote Change

- `PUT /repos/bahayonghang/skills-manage-windows/branches/main/protection` after local validation.
- No commits are pushed, no PR is created, and no release is changed.

## CI Event Model

| Event | `just-ci` | Windows/Linux/macOS smoke packages |
| --- | --- | --- |
| Pull request to `main` | Required | No |
| Push to `main` | Yes | No |
| Push to `dev` | Yes | No |
| Manual dispatch | Yes | Yes |
| Release published | Yes | Yes |

The existing workflow-level concurrency group remains the cancellation boundary. Job names stay stable, especially `just-ci`, because branch protection addresses checks by context.

## Local Gate Model

`scripts/run-ci.mjs` retains two parallel, fail-fast chains:

```text
web: typecheck -> lint -> sizecheck -> vitest -> production build
rust: entrypoint contract -> rustfmt check -> all-target locked clippy -> locked tests
```

The production build catches Vite/Rollup integration failures that type checking and unit tests do not. `cargo fmt --check` adds a deterministic formatting gate; `--all-targets` expands Clippy to tests and binary targets; `--locked` rejects lockfile drift.

## Workflow Contract Test

The test reads `.github/workflows/ci.yml` and parses it with the YAML 1.2 `yaml` package. It asserts semantic values rather than indentation or textual formatting:

- exact trigger branches and retained manual/release events;
- `jobs.ci.name === "just-ci"`;
- the job invokes `node scripts/run-ci.mjs`;
- Rust setup includes both `clippy` and `rustfmt`;
- each package job has the release/manual event guard.

The test also imports/reads the CI orchestrator contract only where necessary to verify the newly required commands. It does not duplicate all implementation details.

## Branch Protection Policy

The update payload sets:

- strict required status check `just-ci`, app id `15368`;
- administrator enforcement enabled;
- pull-request review policy present with `required_approving_review_count: 0`;
- stale/code-owner/last-push approval requirements disabled;
- conversation resolution enabled;
- force pushes and deletion disabled;
- linear history, branch lock, and fork syncing disabled;
- no push restrictions for the personal repository.

The current absence of protection is captured before the write. After the update, the full protection object is read back and compared to the intended policy. Recovery, if the policy proves unusable, is the explicit branch-protection delete endpoint; it is documented but not invoked unless separately authorized or needed to reverse a failed in-scope update.

## Compatibility And Trade-offs

- Zero approvals is intentionally less strict than a team repository, but it avoids making a one-collaborator repository impossible to merge while still prohibiting direct unverified changes.
- Requiring strict/up-to-date checks may cause an additional CI run after the base branch changes. This is the intended safety/cost trade-off.
- Cross-platform packages remain outside PR gating because recent release CI takes tens of minutes; release/manual coverage preserves packaging evidence without imposing that cost on every pull request.
- Signed commits and linear history remain off because current history contains unsigned commits and merge commits. Enabling either now would be a separate migration.

## Rollout Order

1. Implement and validate the local test/CI contract without changing GitHub protection.
2. Run the focused test, full `just ci`, workflow syntax/contract checks, and Windows Tauri build.
3. Write branch protection using the keyring identity with repository-administration access.
4. Read the policy back from GitHub and verify each required/disabled field.
5. Inspect the final local diff and report that implementation commits were not pushed.
