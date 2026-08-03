# Implementation Plan: Marketplace Central install contract

## Step 1 - Red tests

- [x] Add malicious frontmatter-name table proving current target path escape.
- [x] Add a multi-file registry candidate fixture proving current install loses peers and Central DB/provenance.
- [x] Add failure matrix for candidate stale/ambiguous, lock, FS, DB and installed marker timing.
- [x] Add structural assertion that the target module contains no direct registry URL downloader/writer after migration.

Gate: focused Marketplace tests fail for the intended reasons without touching real HOME/remote targets.

## Step 2 - Shared acquisition identity

- [x] Extract a reusable pinned source/snapshot/candidate acquisition helper from existing GitHub import/skills.sh code.
- [x] Map the fresh candidates through the current Marketplace identity function and require one exact requested id.
- [x] Reject missing/ambiguous candidates before any Central lock or mutation.
- [x] Keep `download_url` out of request construction.

Gate: candidate identity, branch pinning, duplicate and redaction unit tests pass.

## Step 3 - Local apply

- [x] Generalize the existing `central_update` Saga into an internal journaled Central content-upsert boundary; first import uses `UpdateManifest(had_target=false)` without a schema migration.
- [x] Route Local through snapshot partial import with explicit overwrite selection and the generalized journaled boundary.
- [x] Assert complete directory, skill row, canonical path and per-skill provenance.
- [x] Verify Central lock/journal/recovery are the existing top-level boundaries, with no nested lock.

Gate: Local success and injected FS/DB rollback tests pass.

## Step 4 - SSH/WSL parity

- [x] Route remote targets through the pinned GitHub import workspace use case and the same generalized `central_update` journal boundary.
- [x] Reuse transport/path helpers; do not construct shell source from names.
- [x] Cover Fake SSH/WSL command protocol, complete-directory parity, target mutation contention, pending recovery and failure rollback.

Gate: remote target tests prove name-independent paths and no direct write path.

## Step 5 - Derived installed state and cleanup

- [x] Set/recompute installed only after durable import success.
- [x] Add marker failure/repair behavior without misreporting a committed import.
- [x] Delete `central_skill_dir_for_name` and the direct downloader/writer.
- [x] Update Marketplace architecture docs and any durable backend spec scenario.

## Step 6 - Validation

- [x] `cargo test --manifest-path src-tauri/Cargo.toml marketplace --locked`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml github_import --locked`
- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml --locked`
- [x] If command/schema changed: `pnpm docs:gen` and commit both generated docs; run `pnpm docs:gen:check`.
- [x] `just ci`
- [x] Inspect final diff for product-code scope and direct writer/URL authority absence.

## Rollback points

- Acquisition helper/tests can revert independently before routing changes.
- Local and remote routing should land as separate green commits if implementation size warrants.
- Once safe routing ships, never restore direct name-derived writes as a fallback; failures remain fail closed.
