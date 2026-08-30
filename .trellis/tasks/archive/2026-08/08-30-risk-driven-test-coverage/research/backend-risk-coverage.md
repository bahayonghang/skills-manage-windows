# Research: Rust/Tauri backend risk-driven test coverage

- Query: Analyze current Rust/Tauri backend test coverage and identify the highest-risk missing tests for core business logic, exceptional branches, boundary conditions, error handling, SQLite transaction/rollback, authorization/permission/capability checks, and parameter/path validation.
- Scope: internal
- Date: 2026-08-30

## Findings

### Coverage baseline and method

This is a **static, risk-oriented coverage assessment**, not a line/branch percentage report. The repository completion gate runs locked Rust tests (`docs/agents/build-and-test.md:15-21`), but repository search found no `cargo llvm-cov`, Tarpaulin, or Rust coverage threshold wired into the normal gate. A fresh read-only source count found 1,516 `#[test]` / `#[tokio::test]` attributes under `src-tauri/src` and `src-tauri/tests`; that count establishes breadth only and must not be presented as executed-test or branch-coverage evidence.

Existing tests are already strong in the expensive domains, so these should not be duplicated merely to raise a number:

- GitHub import has happy-path, unsafe-path, snapshot-integrity, redirect, budget, rollback, and failure-isolation coverage; for example, DB-assignment rollback is exercised by `src-tauri/src/services/github_import/tests.rs:2342-2468`.
- Repository detach rollback and retry are already covered around `src-tauri/src/db/tests.rs:2763-2799`, matching the top-level transaction contract in `.trellis/spec/backend/transactional-mutations.md:11-18`.
- Target-config quarantine has symmetric recovery and trigger-injected transaction rollback at `src-tauri/src/targets/config.rs:351-516`.
- Project install/uninstall has meaningful happy paths and basic validation at `src-tauri/src/services/projects/tests.rs:451-494`, `534-648`, and `651-699`, but not the cross-resource failure cases below.

### Risk-ranked missing tests

#### 1. HIGH — Project install/uninstall can leave filesystem and SQLite state divergent

**Production path**

- `install_skill_to_project_impl` creates the project skills directory, materializes a copy/symlink, and only then writes `project_skill_installations`: `src-tauri/src/services/projects/crud.rs:368-417`.
- `uninstall_skill_from_project_impl` removes the on-disk target before deleting the installation row: `src-tauri/src/services/projects/crud.rs:429-470` and the following DB-delete tail.

**Existing coverage**

- Copy install proves both filesystem and row on success: `src-tauri/src/services/projects/tests.rs:451-494`.
- Uninstall proves both are removed on success: `src-tauri/src/services/projects/tests.rs:651-682`.
- There is no trigger-injected DB failure after the filesystem mutation, and no assertion that an error converges to either the complete old state or complete new state.

**Focused tests to add**

1. Install a copy while a SQLite trigger rejects `project_skill_installations` insert/update; assert the call errors, the target directory is absent, the canonical source remains byte-identical, and no installation row exists.
2. Install first, then use a trigger to reject the installation-row delete; assert an uninstall error does not leave a live row pointing at a missing path (restore the removed copy/link or otherwise preserve a convergent state).
3. Run both copy and symlink variants where the platform supports symlinks; do not silently return early on the branch being tested.

**Focused command**

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked services::projects::tests
```

These tests are expected to expose a product defect in the present ordering. Keep the invariant and fix compensation/ordering if they fail; weakening the assertions to document orphan state would not protect the business contract.

#### 2. HIGH — Target deletion mutates credential storage and three settings without one rollback boundary

**Production path**

- `delete_target_impl` removes an SSH credential first, then writes SSH targets, WSL targets, and possibly the active target as separate operations: `src-tauri/src/targets/commands.rs:337-369`.
- SSH create/update also save credentials before target persistence and initialize a remote DB after persistence: `src-tauri/src/targets/commands.rs:19-35` and `76-98`.
- Existing target tests cover request normalization, credential fallback, and context snapshots, not the top-level CRUD functions: representative coverage is `src-tauri/src/targets/tests.rs:128-287` and `802-1016`. A repository-wide symbol search found no test call to `create_ssh_target_impl`, `update_ssh_target_impl`, `delete_target_impl`, or `set_active_target_impl`.

**Focused tests to add**

1. Seed SSH + WSL targets, make the SSH target active, and use `MemoryCredentialBackend`; inject a trigger that rejects the WSL-settings write. Deleting the SSH target must return an error while preserving the SSH list, WSL list, active target, credential, and remote-pool ownership.
2. Inject rejection of the active-target write and assert the same all-or-nothing state.
3. Cover Local deletion and unknown ID as zero-write boundaries, then a successful retry after removing the trigger.

**Focused command**

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked targets::tests
```

This is higher value than testing command wrappers: it protects the user-visible target definition, the selected target, and its secret as one mutation outcome.

#### 3. HIGH — Repository-sync decisions are written incrementally before later parameters are validated

**Production path**

- `apply_central_repository_sync_impl` applies keep/delete decisions before validating skip/unskip/addition paths, then persists skip/unskip requests one at a time: `src-tauri/src/services/central_updates/repository_sync.rs:299-359`.
- Addition selections are also normalized and persisted inside nested loops after earlier decisions may already have committed: `src-tauri/src/services/central_updates/repository_sync.rs:361-449`.
- `normalize_repo_path` rejects unsafe relative paths at `src-tauri/src/services/github_import/source.rs:603-611`, so a later invalid `../escape` decision is a deterministic way to test partial writes.

**Existing coverage**

- The helper that detects remote additions and persisted skips is covered at `src-tauri/src/services/central_updates/core/tests.rs:142-264`.
- Keeping one remote-missing skill and rejecting an invalid keep state are covered at `src-tauri/src/services/central_updates/core/tests.rs:338-454`.
- A repository-wide symbol search found no test invocation of the top-level `apply_central_repository_sync_impl`; therefore no test combines heterogeneous decisions or proves validation-before-write.

**Focused tests to add**

1. Submit two `skip_additions`: first valid, second `../escape`; assert the typed path error and zero new skip rows.
2. Combine a valid keep decision with a later invalid skip path; assert repository membership and update state are unchanged, proving all caller input is validated before authoritative writes as required by `.trellis/spec/backend/transactional-mutations.md:15-18`.
3. Inject a trigger failure on the second skip write; assert the first write rolls back and a retry succeeds after trigger removal.

**Focused command**

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked services::central_updates::repository_sync
```

If cross-category atomicity is intentionally not required, that is a missing business contract decision; at minimum, each skip/unskip batch should still be prevalidated and transactional.

#### 4. HIGH — SSH/WSL connection-test success payloads have no negative leakage test

**Production path**

- SSH transport deliberately retains raw stderr in `TargetsError::RemoteCommandFailed`: `src-tauri/src/targets/exec.rs:262-282`.
- `test_ssh_target_impl` and password-update probing copy `error.to_string()` into the successful IPC result's `message`: `src-tauri/src/targets/commands.rs:135-180` and `201-240`.
- The lower-level test explicitly asserts raw path-bearing stderr survives: `src-tauri/src/targets/tests.rs:1127-1137`.
- The redaction contract says paths, commands, stdout/stderr, and credentials must not enter IPC error surfaces: `.trellis/spec/backend/redaction-policy.md:36-37`, with raw dynamic error text called out as a bad boundary at line 75.

**Focused tests to add**

1. Add an injectable probe runner (or a pure reviewed-message mapper used by both SSH and WSL connection tests) that returns a high-entropy token, password, hostname, absolute path, and command in stderr.
2. Serialize `SshTargetTestResult` / `WslTargetTestResult`; assert every sentinel is absent while a stable actionable category remains.
3. Assert the operation log and IPC error still share the same correlation ID without reintroducing raw details.

**Focused command**

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked targets::tests::connection_test
```

Do not delete the low-level diagnostic test; the missing test is at the public boundary where detailed transport errors must be reduced.

#### 5. MEDIUM — Persisted target IDs accept values that the cache-path boundary later rejects

**Production path**

- SSH/WSL config parsing validates only empty, duplicate, and reserved `local` IDs: `src-tauri/src/targets/config.rs:198-242` and `256-269`.
- Remote cache creation later requires the ID to contain only ASCII alphanumeric, `-`, or `_`: `src-tauri/src/targets/commands.rs:683-700`.

**Existing coverage**

- Quarantine tests cover duplicate and reserved IDs, but not slash, backslash, whitespace-containing, control-character, or traversal-like IDs: `src-tauri/src/targets/config.rs:538-560`.
- The cache-path test covers only one valid ID: `src-tauri/src/targets/tests.rs:289-293`.

**Focused tests to add**

1. Table-test SSH and WSL persisted IDs such as `../escape`, `a/b`, `a\\b`, embedded whitespace/control characters, and valid `ssh-demo_1`.
2. Assert invalid domains are quarantined before list/select/open can reach cache-path construction; assert no directory outside the target-cache root is created.
3. Assert the validator and `remote_cache_db_path` accept/reject the same ID matrix so the two boundaries cannot drift.

**Focused command**

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked targets::config::tests
```

### Recommended execution order

1. Project FS/DB compensation tests.
2. Target mutation rollback tests.
3. Repository-sync prevalidation/transaction tests.
4. Public connection-test redaction tests.
5. Target-ID validator parity tests.

After each module's focused command passes, run the backend suite and repository completion gate:

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked
just ci
```

Treat a filtered command that reports zero tests as missing evidence, per `.trellis/spec/quality/test-suite-layout.md:107`.

## Files Found

- `src-tauri/src/services/projects/crud.rs` — project CRUD and project-local skill install/uninstall filesystem plus DB ordering.
- `src-tauri/src/services/projects/tests.rs` — project happy paths and basic validation; no DB failure injection after FS mutation.
- `src-tauri/src/targets/commands.rs` — target CRUD, secret handling, active-target updates, probe result mapping, and target-cache ID validation.
- `src-tauri/src/targets/config.rs` — persisted SSH/WSL target schema validation and quarantine transaction.
- `src-tauri/src/targets/tests.rs` — target request/credential/transport/context coverage; no top-level CRUD rollback tests.
- `src-tauri/src/targets/exec.rs` — raw SSH process diagnostics and connection probing.
- `src-tauri/src/services/central_updates/repository_sync.rs` — repository-sync check/apply orchestration and incremental decision persistence.
- `src-tauri/src/services/central_updates/core/tests.rs` — helper-level remote-added/keep tests, not top-level apply atomicity.
- `src-tauri/src/services/github_import/source.rs` — authoritative safe repository-relative path validator.
- `.trellis/spec/backend/transactional-mutations.md` — validation-before-write and top-level transaction requirements.
- `.trellis/spec/backend/settings-domain-boundary.md` — target quarantine and renderer settings boundary.
- `.trellis/spec/backend/redaction-policy.md` — public IPC/log diagnostic privacy boundary.
- `.trellis/spec/quality/test-suite-layout.md` — focused-test discovery and zero-test evidence rule.
- `docs/agents/build-and-test.md` — backend and repository-level test gates.

## External References

None. This assessment is repository-grounded; no external API or version claim was needed.

## Related Specs

- `.trellis/spec/backend/transactional-mutations.md`
- `.trellis/spec/backend/settings-domain-boundary.md`
- `.trellis/spec/backend/redaction-policy.md`
- `.trellis/spec/backend/test-support.md`
- `.trellis/spec/quality/test-suite-layout.md`
- `.trellis/spec/quality/ci-quality-gate.md`

## Caveats / Not Found

- No quantitative Rust line/branch coverage artifact or enforced coverage threshold was found. Do not invent a percentage from source test counts.
- Tests were not executed in this research role; Trellis restricted writes to this task's `research/` directory, while compiling/running Rust tests would write build artifacts outside that boundary. Commands above are focused execution guidance for the implementation/check phases.
- This repository is a local desktop application and exposes no user/role RBAC model in the inspected backend. Static Tauri command capabilities are a separate frontend/quality contract surface; this report therefore did not fabricate business authorization tests where no authorization domain exists.
- Findings 1-4 are likely to reveal product defects, not merely increase coverage. If a new regression fails, preserve the safety invariant and route the necessary production fix through the implementation task instead of asserting the unsafe current behavior.
