# Implementation Plan: Marketplace Central install contract

## Step 1 - Red tests

- [ ] Add malicious frontmatter-name table proving current target path escape.
- [ ] Add a multi-file registry candidate fixture proving current install loses peers and Central DB/provenance.
- [ ] Add failure matrix for candidate stale/ambiguous, lock, FS, DB and installed marker timing.
- [ ] Add structural assertion that the target module contains no direct registry URL downloader/writer after migration.

Gate: focused Marketplace tests fail for the intended reasons without touching real HOME/remote targets.

## Step 2 - Shared acquisition identity

- [ ] Extract a reusable pinned source/snapshot/candidate acquisition helper from existing GitHub import/skills.sh code.
- [ ] Map the fresh candidates through the current Marketplace identity function and require one exact requested id.
- [ ] Reject missing/ambiguous candidates before any Central lock or mutation.
- [ ] Keep `download_url` out of request construction.

Gate: candidate identity, branch pinning, duplicate and redaction unit tests pass.

## Step 3 - Local apply

- [ ] Route Local through snapshot partial import with explicit overwrite selection.
- [ ] Assert complete directory, skill row, canonical path and per-skill provenance.
- [ ] Verify Central lock/journal/recovery are the existing top-level boundaries, with no nested lock.

Gate: Local success and injected FS/DB rollback tests pass.

## Step 4 - SSH/WSL parity

- [ ] Route remote targets through the pinned GitHub import workspace/use case.
- [ ] Reuse transport/path helpers; do not construct shell source from names.
- [ ] Cover Fake SSH/WSL command protocol and complete-directory parity.

Gate: remote target tests prove name-independent paths and no direct write path.

## Step 5 - Derived installed state and cleanup

- [ ] Set/recompute installed only after durable import success.
- [ ] Add marker failure/repair behavior without misreporting a committed import.
- [ ] Delete `central_skill_dir_for_name` and the direct downloader/writer.
- [ ] Update Marketplace architecture docs and any durable backend spec scenario.

## Step 6 - Validation

- [ ] `cargo test --manifest-path src-tauri/Cargo.toml marketplace --locked`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml github_import --locked`
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --locked`
- [ ] If command/schema changed: `pnpm docs:gen` and commit both generated docs; run `pnpm docs:gen:check`.
- [ ] `just ci`
- [ ] Inspect final diff for product-code scope and direct writer/URL authority absence.

## Rollback points

- Acquisition helper/tests can revert independently before routing changes.
- Local and remote routing should land as separate green commits if implementation size warrants.
- Once safe routing ships, never restore direct name-derived writes as a fallback; failures remain fail closed.
