# Repository CI Quality Gate

## 1. Scope / Trigger

This contract applies whenever changing test commands, `just ci`, GitHub Actions CI/release triggers or job names, Rust toolchain checks, contributor validation docs, or `main` branch protection.

The repository is Windows-first, but the merge gate covers the full frontend and Rust codebase. Cross-platform package builds are release/manual smoke evidence, not routine pull-request checks.

## 2. Signatures

```text
just ci
  -> node scripts/sync-version.mjs
  -> node scripts/run-ci.mjs

just audit
  -> node scripts/check-dependency-audit.mjs
  -> pnpm audit --prod --json + cargo audit --json

GitHub Actions required job/check:
  workflow: .github/workflows/ci.yml
  job id: ci
  job name / check context: just-ci
  check app id: 15368 (GitHub Actions)
  needs: source-validation + supply-chain
  runner: windows-2022

Routine prerequisites:
  source-validation: ubuntu-22.04 + macos-14, node scripts/run-ci.mjs
  supply-chain: ubuntu-22.04, pnpm audit:dependencies

Reusable CI:
  workflow_call.inputs.checkout_ref: required string
  checkout: frozen commit SHA
  jobs run: source-validation + supply-chain -> just-ci

Desktop release:
  trigger: push v* tag or workflow_dispatch(tag) from main
  context: tag + version + peeled commit SHA
  order: context -> reusable CI + required builds -> aggregate -> draft -> verify -> publish
```

The orchestrator owns two parallel, fail-fast chains:

```text
web: typecheck -> lint -> capabilitycheck -> sizecheck -> test -> build
rust: entrypointcheck -> ipc:codegen:check -> fmt --check -> clippy --all-targets --locked -> test --locked
```

`pnpm sizecheck` enforces the 800-line limit uniformly across production source
files. There are no frozen baseline exceptions or per-file allowlist bypasses.

## 3. Contracts

### Event contract

| GitHub event | Ubuntu/macOS source | Supply chain | Windows `just-ci` | Package smoke |
| --- | --- | --- | --- | --- |
| Pull request to `main` | Run | Run | Run after both pass | Skip |
| Push to `main` or `dev` | Run | Run | Run after both pass | Skip |
| `workflow_dispatch` | Run | Run | Run after both pass | Run |
| `workflow_call(checkout_ref)` | Run at ref | Run at ref | Run at ref after both | Skip |

The CI workflow has no `release.published` trigger. Direct manual dispatch is
the only CI event that runs package smoke jobs. The canonical
`release-desktop.yml` workflow exclusively owns the formal Windows x64, macOS
universal, Linux x64, and optional Linux arm64 release matrix.

### Release context and publication contract

- Validate the explicit `v<semver>` tag and peel it to a commit on `origin/main`
  before reading version files. Checkout that peeled commit, then require
  `package.json`, `tauri.conf.json`, Cargo metadata, and `Cargo.lock` to agree.
- Every reusable CI/build checkout uses the frozen SHA. Manual dispatch must use
  the `main` workflow definition; a selected branch is never a release context.
- Required platform builds finish before a draft is created. Optional Linux
  arm64 may be absent, but any arm64 output must be a complete DEB/RPM/AppImage
  group.
- Validate the exact artifact inventory, updater signature, `latest.json`, and
  deterministic `SHA256SUMS` before draft creation. Reset a reused draft, upload
  the validated set, verify API inventory, then fresh-download and recheck the
  manifest.
- `release.target_commitish` is not authoritative for an existing tag and may
  contain a branch such as `main`. Verify the remote tag's peeled commit instead,
  including immediately before publication.
- Workflow permissions default to `contents: read`; only the publish job receives
  `contents: write`. The sole public transition is the final `draft=false` API
  update after every prior check succeeds.

Package job guards must remain:

```yaml
if: ${{ github.event_name == 'workflow_dispatch' }}
```

### Supply-chain contract

- Every external Action in `.github/workflows/*.yml` is referenced by a full
  40-character commit SHA. Version comments are informational; Dependabot's
  weekly `github-actions` updates keep the commits reviewable and current.
- `pnpm audit --prod --json` blocks high/critical advisories. `cargo audit
  --json` blocks every vulnerability. Moderate/low npm advisories and Cargo
  informational warnings remain visible without expanding this gate's scope.
- Exceptions are exact `(ecosystem, advisory)` entries with non-empty owner and
  reason plus an ISO expiry date. Malformed, duplicate, expired, cross-ecosystem,
  or unused entries fail closed.
- Current exceptions expire on 2026-08-11: React Router's RSC-only advisory has
  no stable fixed release, and `tauri-plugin-sql 2.4.0` internally enables the
  SQLx/RSA closure even when the application uses only SQLite. Neither exception
  permits a package-wide or severity-wide ignore.

### Branch protection contract

- `main` requires an up-to-date `just-ci` check bound to app id `15368`.
- `just-ci` uses `needs` plus an `always()` result assertion so a failed source
  matrix or supply-chain job fails the existing required context; no new remote
  required-check name is needed.
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
| Source matrix or supply-chain prerequisite fails | Windows job still reports `just-ci` and its final prerequisite assertion fails |
| External Action uses a tag, branch, or short SHA | CI workflow contract fails |
| Audit exception is malformed, expired, duplicate, cross-ecosystem, or unused | `just audit` fails closed |
| New npm high/critical or Cargo vulnerability appears | `just audit` fails unless one exact current exception exists |
| Package job lacks event guard | Contract test fails |
| Reusable CI runs package jobs | Contract test fails; `workflow_call` owns only `just-ci` |
| Manual release runs from a non-`main` workflow ref | Context job fails before checkout/build |
| Tag is absent, invalid, outside `main`, or version metadata differs | Context job fails before CI/build |
| Required build or aggregate check fails | Publish job is not scheduled; no new release exists |
| Optional arm64 has a partial set | Artifact inventory fails before draft creation |
| Existing same-tag release is public | Publish job fails closed without overwriting it |
| Draft upload/API/fresh-download check fails | Release remains a private draft |
| Remote tag moves after context freeze | Draft creation or final publish check fails |
| Existing release reports `target_commitish: main` | Accept only after the remote tag itself peels to the frozen SHA |
| Signature, metadata, asset inventory, or checksum is invalid | Aggregate/publish fails before `draft=false` |
| Rust is not formatted | `cargo fmt --check` fails |
| Renderer capability, plugin wiring, or inventory drifts | `pnpm capabilitycheck` fails before size/test/build |
| A production source file exceeds 800 lines | `pnpm sizecheck` fails; no per-file baseline exemption is permitted |
| Rust/Serde IPC metadata drifts from the checked artifact | `pnpm ipc:codegen:check` fails before Rust fmt/Clippy/test |
| Test/bin target has a Clippy warning | all-target Clippy fails |
| Cargo lockfile would change | `--locked` command fails |
| Frontend unit tests pass but bundling is invalid | `pnpm build` fails |
| GitHub protection payload sends `contexts` and `checks` | HTTP 422; retry with `checks` only after confirming no remote change |
| Environment `GH_TOKEN` lacks administration permission | Do not persist or print another token; use the authenticated keyring identity for that process |

## 5. Good / Base / Bad Cases

- Good: a PR to `main` runs Ubuntu/macOS source validation and the supply-chain
  audit, then one stable Windows `just-ci` gate; package jobs skip and merge waits
  for the current head plus both prerequisite results.
- Base: a manual run executes the same prerequisites and `just-ci` plus all smoke
  packages for pre-release validation.
- Good: a release tag peels to `main`, reusable CI and all required builds pass,
  the draft survives fresh-download verification, and one final API call makes
  the complete release public.
- Base: optional Linux arm64 produces no artifact; the required x64 inventory is
  still valid. A retry reuses and resets the same-tag draft.
- Bad: trusting `release.target_commitish` as the tag commit. GitHub may return a
  branch name for an existing tag, so compare the peeled remote tag instead.
- Bad: creating or publishing a release before all required platform jobs and
  post-upload checks pass.
- Bad: expanding the whole release packaging matrix to every PR, which adds tens of minutes without improving routine feedback proportionally.
- Bad: adding a new local check without adding it to `scripts/run-ci.mjs`, or duplicating different commands directly in the workflow.
- Bad: adding non-required matrix jobs without propagating their result into the
  existing required `just-ci` context.
- Bad: suppressing an audit by package name, severity, or non-expiring ignore.

## 6. Tests Required

- `pnpm vitest run src/test/contracts/ciWorkflowContract.test.ts`
  - Parse YAML 1.2; assert event branches, reusable frozen checkout, stable job
    name, prerequisite result propagation, Ubuntu/macOS matrix, audit job, full
    Action SHAs, Dependabot schedule, Rust components, and manual-only package guards.
- `pnpm exec vitest run src/test/contracts/dependencyAuditContract.test.ts`
  - Assert unknown high/RUSTSEC findings block; exact current exceptions pass;
    malformed, duplicate, expired, cross-ecosystem, and unused exceptions fail.
- `just audit`
  - Run the live pnpm and Cargo advisory databases; inspect the raw audit outputs
    when the wrapper reports a new or malformed finding.
- `pnpm vitest run src/test/contracts/releaseWorkflowContract.test.ts src/test/scripts/release*.test.ts`
  - Assert frozen tag checkout, required-job DAG, optional artifact rules, draft
    ordering, least privilege, final tag recheck, sole public transition,
    metadata/checksum failures, and updater signature tamper cases.
- `cargo test --manifest-path src-tauri/Cargo.toml --bin release-signature-verifier --locked`
  - A runtime-compatible wrapped minisign fixture passes; changed installer,
    signature, and public key fail.
- `just ci`
  - Assert the complete frontend and Rust chains pass from their shared local/remote entrypoint, including capability and typed-IPC drift checks.
- `pnpm exec vitest run src/test/contracts/capabilityDrift.test.ts`
  - Assert missing permissions, stale JSON/table state, unexpected imports, and stale dependency/initializer sets all fail.
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

This starts validation after publication and cannot prevent an empty or partial
public release.

### Correct

```yaml
on:
  pull_request:
    branches: [main]
  push:
    branches: [main, dev]
  workflow_dispatch:
  workflow_call:
    inputs:
      checkout_ref:
        required: true
        type: string

jobs:
  source-validation:
    strategy:
      matrix:
        runner: [ubuntu-22.04, macos-14]
  supply-chain:
    name: supply-chain
  ci:
    name: just-ci
    needs: [source-validation, supply-chain]
    if: ${{ always() }}

# release-desktop.yml
jobs:
  quality-gate:
    uses: ./.github/workflows/ci.yml
    with:
      checkout_ref: ${{ needs.release-context.outputs.sha }}
  publish:
    needs: [release-context, aggregate]
```

Repository checks such as capability drift belong in the ordered `web` steps in
`scripts/run-ci.mjs`; workflows continue to invoke `just ci` instead of
duplicating the command. Formal release platform builds remain in
`release-desktop.yml`; CI manual smoke jobs do not become release dependencies.

## Scenario: Typed IPC Codegen And Parity Gate

### 1. Scope / Trigger

- 修改 Rust command 签名/Serde 字段、runtime registry、generated batch、IPC map 或 codegen 依赖时适用。

### 2. Signatures

```text
pnpm ipc:codegen       -> generate src/lib/ipc/generatedCommandMap.ts
pnpm ipc:codegen:check -> temporary generation + byte comparison + rename-drift probe
SkillUpdateState.status -> Rust/SQLx/Serde/Specta SkillUpdateStatus
cargo metadata bins     -> skillport, skillport-cli, release-signature-verifier
```

`ipc-codegen` feature 精确固定 `tauri-specta/specta = 2.0.0-rc.25` 与
`specta-typescript/specta-serde = 0.0.12`，正常 runtime/release 不启用该 feature。

### 3. Contracts

- runtime registry 184；generated registry 42 且为 runtime 子集。
- frontend 177 = typed 130 + allowlist 47；runtime - frontend 恰为冻结的 backend-only 7。
- 180 个 fallible command 使用 `IpcResult`；4 个 infallible command 保持原签名。
- generator 使用 structured Specta metadata 和 Serde phases；不得以 source regex 证明类型一致性。
- 对 codegen checker 按字节比较的受控生成文本，必须在 `.gitattributes` 中按路径固定 `text eol=lf`；不得依赖开发者本机的 `core.autocrlf` 设置维持 artifact parity。
- generated type 必须来自 Rust 字段的真实类型。持久化字段不得用 Specta-only override
  收窄：例如 SQLite `TEXT` 状态必须在 Rust 中使用实现 SQLx 文本编解码的 enum，Serde、
  Specta 与 SQLx 共享同一组 rename；既有合法值可回读，未知值 fail closed。
- application Cargo package 不得增加 feature-gated 独立 codegen bin。Tauri bundler 会从
  Cargo metadata 收集 package 的全部 bin，即使 `required-features` 未启用也可能继续查找
  未生成的 executable。codegen 复用 `skillport --ipc-codegen` tool mode；
  `entrypointcheck` 将 package bin 固定为上述三项。

### 4. Validation & Error Matrix

| Drift | Required failure |
| --- | --- |
| Rust arg/Serde field rename without artifact | byte check |
| generated name absent from runtime/caller | parity test |
| allowlist grows or overlaps typed map | count/overlap test |
| artifact contains runtime import/callable invoke/unknown | artifact contract test |
| raw string command rejection returns | fallibility boundary test |
| Specta output narrower than the Rust/SQLx persisted field | typecheck or persisted-value compatibility test |
| SQLite contains an unknown persisted enum value | typed row decode fails; do not widen generated output to `string` |
| application package gains an unexpected Cargo bin | `entrypointcheck` fails before release build |
| feature-gated codegen bin is absent during Tauri bundle | Windows bundle failure; consolidate it into `skillport` tool mode |

### 5. Good / Base / Bad Cases

- Good: regenerate once, check twice, commit Rust source and deterministic artifact together.
- Good: a persisted status enum derives SQLx/Serde/Specta from one Rust definition and all historical
  text values decode to the generated TypeScript union.
- Base: remaining 47 commands stay explicitly allowlisted.
- Bad: hand-edit generated output, use a Specta-only type override over `String`, float RC
  dependencies, parse names as proof of result types, or add a feature-gated bin to the application package.

### 6. Tests Required

- `pnpm ipc:codegen:check`.
- `pnpm entrypointcheck`; assert Cargo metadata exposes exactly the three approved bins.
- `pnpm exec vitest run src/test/contracts/ipcCommandCoverage.test.ts`.
- Persisted enum compatibility test: raw-insert every historical SQLite text value, decode the typed
  row, and assert an unknown value returns an error.
- `cargo clippy --all-targets --locked -- -D warnings` and `cargo test --locked`.
- Final `just ci`; Windows packaging changes also require `pnpm tauri build` and bundle inventory check.

### 7. Wrong vs Correct

```text
Wrong: edit generatedCommandMap.ts or add a Specta-only override until typecheck passes.
Correct: fix the authoritative Rust/SQLx/Serde type or the real caller, run pnpm ipc:codegen, then prove --check is clean.

Wrong: add src/bin/ipc-codegen.rs with required-features inside the Tauri application package.
Correct: dispatch --ipc-codegen from the existing skillport bin and keep entrypointcheck plus a real Windows bundle gate.
```
