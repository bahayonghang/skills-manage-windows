# Backend Layer Boundaries

## 1. Scope / Trigger

Apply this contract when adding a service HTTP client, importing
`crate::commands` from `src-tauri/src/services/**`, calling a repository
function through `crate::db::<fn>`, or changing `src-tauri/src/db/mod.rs`
visibility.

## 2. Signatures

```rust
// src-tauri/src/http_identity.rs
pub(crate) const APP_USER_AGENT: &str =
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

// Production services call repositories through the owner module:
crate::db::repos::skills_repo::upsert_skill(...)
crate::db::repos::fs_db_operations_repo::insert_fs_db_operation(...)

// Shared pool/row types may still use the facade:
use crate::db::{DbPool, Skill};
```

Dependency direction is `commands → services → db::repos`. `http_identity` has
no Tauri, command, database, or service imports.

## 3. Contracts

- Production `src-tauri/src/services/**` must not reference `crate::commands`
  (module docs that mention command shells are allowed; executable code is not).
- HTTP clients that send SkillPort's identity use `crate::http_identity::APP_USER_AGENT`.
  Do not recreate the constant in `commands` or a service module.
- Canonical SQL owners remain `src-tauri/src/db/repos/*_repo.rs`. Do not copy
  SQL into services. Do not add a second DB facade, repository framework, or DI.
- `central_updates`, `skills_cli`, and `installation` production code must call
  repository *functions* via `crate::db::repos::<owner>_repo`. `DbPool` and shared
  row/domain types may stay on `crate::db`.
- `db/mod.rs` keeps compatibility `pub use repos::*` for unmigrated callers.
  Historical wide `crate::db::<repo_fn>` hits outside the three trees are debt:
  they must not grow. The ratchet count is the function-call scan (388 at the
  2026-09-03 remasurement), not the audit's file-count 87.
- Import-path migrations must not change journal phases, Central mutation lock,
  `ensure_centralized`, persisted `uid`, target-only skills, or public DTO/CLI
  results.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Production service imports `crate::commands` | `rustBoundaryContract` fails |
| `APP_USER_AGENT` defined in `commands/mod.rs` | services cannot compile without a commands import; forbidden |
| Wide `crate::db::<repo_fn>` in the three trees | contract fails |
| Historical wide-fn count > 388 | contract fails; do not raise the baseline to hide new debt |
| New SQL in a service file | reject; put it in the owning `*_repo.rs` |
| Nested transaction or dropped mutation lock during an import-only change | reject; restore the original call |

## 5. Good / Base / Bad Cases

- Good: `ai_provider` sets `.user_agent(crate::http_identity::APP_USER_AGENT)`.
- Base: an unmigrated service still calls `crate::db::list_skills`; the historical
  ratchet holds as long as the hit count does not grow.
- Bad: wrap `skills_repo` in a new generic repository trait, or restore
  `commands::APP_USER_AGENT` so services can keep compiling.

## 6. Tests Required

- `pnpm exec vitest run src/test/contracts/rustBoundaryContract.test.ts`
  - scan roots/exclusions are fixed
  - services→commands production hits are `[]`
  - three-tree wide repo-fn hits are `[]`
  - historical wide-fn count `<= 388`
- Directed Rust: `cargo test --manifest-path src-tauri/Cargo.toml --locked`
  with filters `services::central_updates`, `services::skills_cli`, and
  `services::installation`. In PowerShell do not join filters with `|` (it is
  a pipeline). `as const` allowlist arrays used with `.includes(string)` must
  be read as `readonly string[]` so `pnpm typecheck` passes.
- Real GitHub user-agent bytes, live SSH/WSL, and Windows installer stay
  `UNVERIFIED` until captured on those surfaces.

## 7. Wrong vs Correct

```rust
// Wrong
.user_agent(crate::commands::APP_USER_AGENT)
crate::db::update_skills_batch(pool, rows).await?;

// Correct
.user_agent(crate::http_identity::APP_USER_AGENT)
crate::db::repos::skills_repo::update_skills_batch(pool, rows).await?;
```
