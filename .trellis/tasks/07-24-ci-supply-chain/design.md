# Technical Design

## Boundaries

The task changes CI configuration, dependency metadata, one cross-platform Node
policy script, contract tests, contributor docs, and the CI quality spec. It does
not alter application behavior, release signing, updater metadata, or remote
repository settings.

## Workflow Topology

`ci.yml` retains the current `ci` job as `just-ci` on `windows-2022`.

Two blocking jobs are added without event guards:

1. `source-validation` uses a matrix of `ubuntu-22.04` and `macos-14`, installs
   frozen pnpm/Rust dependencies, and runs `node scripts/run-ci.mjs`. Ubuntu
   installs the existing Tauri GTK/WebKit/libsoup build dependencies. It never
   runs `pnpm tauri build` and therefore does not replace package smoke.
2. `supply-chain` runs on `ubuntu-22.04`, installs frozen JS dependencies and
   pinned `cargo-audit 0.22.2`, then runs the repository audit script.

The workflow continues to support `workflow_call(checkout_ref)`, so a release
validates all three source platforms and the dependency baseline at the frozen
release commit. Existing package jobs remain guarded by direct manual dispatch.

Every external Action use in all three workflows is replaced by the researched
40-character commit. A source-level contract scans every workflow file and
exempts only local paths beginning with `./`.

## Dependency Audit Contract

Add `scripts/check-dependency-audit.mjs` and a package script such as
`audit:dependencies`. The script owns process execution and policy evaluation:

```text
pnpm audit --prod --json ----> normalize GHSA + severity --+
                                                          +--> policy --> exit
cargo audit --json ----------> normalize RUSTSEC vuln ----+
                                    ^
security/dependency-audit-exceptions.json
```

The command must capture JSON even when the child audit command exits non-zero;
parse errors or missing expected fields are policy failures. JS high/critical
and every Rust vulnerability form the blocking set. Lower JS severities and
Rust warnings are printed as non-blocking observations.

The policy module exports pure parsing/evaluation functions so Vitest can feed
fixtures without network access or a local `cargo-audit` binary.

## Exception Schema

The checked manifest is a JSON array. Every entry has exactly these semantic
fields:

```json
{
  "ecosystem": "npm",
  "advisory": "GHSA-xxxx-xxxx-xxxx",
  "owner": "lyh",
  "reason": "Upstream stable line has no fix; affected RSC mode is unused.",
  "expires": "2026-08-11"
}
```

`ecosystem` is `npm` or `cargo`; Cargo IDs must match `RUSTSEC-YYYY-NNNN`.
Dates are compared as UTC calendar dates. Duplicate keys, empty ownership or
reason, invalid/future-ambiguous dates, expiry before the current date, and
entries that match no current blocking advisory all fail. An exception matches
only the exact `(ecosystem, advisory)` pair.

## Baseline Remediation

- Move `shadcn` from production to development dependencies and update it within
  the current major.
- Update `react-router-dom` to the current compatible stable fix and
  `@lobehub/icons` to the current compatible release; re-run the live audit
  before creating any exception.
- Set direct SQLx and `tauri-plugin-sql` declarations to
  `default-features = false`, retaining only SQLite/runtime/derive features at
  the application boundary. `tauri-plugin-sql 2.4.0` still depends internally
  on SQLx defaults, so its unpatched RSA advisory requires a separate exact,
  expiring exception rather than a broad ignore.
- Use precise Cargo updates for `plist`, `quinn-proto`, and `rustls-webpki`.
  Do not accept the 165-package broad update shown by the dry run.

Only the still-live React Router RSC advisory and the `tauri-plugin-sql`-owned
RSA advisory may enter the initial manifest, each with expiry no later than
2026-08-11. If either post-update finding disappears, its now-unused exception
must fail and be removed.

## Compatibility And Rollback

- Contract tests preserve the stable Windows check, manual package guards,
  release frozen-SHA checkout, and publication ordering.
- Rust compile/test proves the reduced SQLx feature set still supplies
  `FromRow`, SQLite, and runtime APIs.
- Dependency metadata/lockfile changes form one rollback point; audit policy and
  workflow changes form a second. Reverting either does not migrate user data.
- A hosted-runner failure after push should be fixed in the matrix job; it must
  not be hidden with `continue-on-error` or by restoring movable Action tags.
