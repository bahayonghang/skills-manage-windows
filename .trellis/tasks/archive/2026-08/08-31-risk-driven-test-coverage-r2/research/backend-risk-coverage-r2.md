# Research: Remaining Rust/Tauri backend HIGH/MEDIUM risk coverage (round 2)

- **Query**: Fresh static risk-oriented assessment of remaining HIGH/MEDIUM backend test gaps after round 1. Prioritize core business uncovered logic, exception branches, SQLite transaction/rollback, path/capability/credential boundaries, and parameter validation. Do not re-recommend round-1 items unless still untested.
- **Scope**: internal
- **Date**: 2026-08-31

## Findings

### Method and round-1 baseline

This is a **static, risk-oriented assessment**, not a line/branch percentage report. Repository search found no `cargo llvm-cov`, Tarpaulin, or coverage threshold in the normal gate (`docs/agents/build-and-test.md`). Do not invent a coverage percentage from `#[test]` counts.

Round 1 (archived `.trellis/tasks/archive/2026-08/08-30-risk-driven-test-coverage/`) added backend tests that **now exist** and must not be duplicated:

| Round-1 item | Evidence it is now tested |
|---|---|
| Project install/uninstall FS+DB compensation | `src-tauri/src/services/projects/tests.rs` calls `install_skill_to_project_impl` / `uninstall_skill_from_project_impl` including rollback cases (e.g. around 817–875) |
| Target CRUD rollback / Local-empty-unknown ID | `src-tauri/src/targets/tests.rs` `invalid_delete_and_active_target_ids_make_zero_settings_writes` (141–178), `delete_target_rolls_back_settings_credential_and_pool_on_persistence_failure` (251–260), credential-failure restore (263–358), `set_active_target_persistence_failure_preserves_previous_state` (360–396) |
| Repository-sync prevalidation and skip-write rollback | `src-tauri/src/services/central_updates/repository_sync/tests.rs` invokes `apply_central_repository_sync_impl` (102–184) |

Round-1 **deferred** items and current status:

| Deferred item | Still untested? | Round-2 action |
|---|---|---|
| SSH/WSL connection-test `Ok(Result { ok: false })` success-payload redaction | Yes. `test_ssh_target_impl` / `test_wsl_target_impl` still copy `error.to_string()` into `message` (`src-tauri/src/targets/commands.rs:175-182`, `235-242`, `330-336`). No public-boundary leakage test. | **Do not write tests that freeze the current leak.** Requires a product/spec decision first (see Caveats). |
| Target ID vs remote cache path validator parity | Yes. `validate_target_ids` still only empty/duplicate/reserved (`src-tauri/src/targets/config.rs:316-331`). `sanitize_target_id` still requires ASCII alnum/`-`/`_` (`src-tauri/src/targets/commands.rs:714-724`). Cache-path test still only one valid ID (`src-tauri/src/targets/tests.rs:560-563`). | **Re-recommend as MEDIUM** (still untested). |
| AI settings concurrent flush latest-edit-wins | Frontend/product; not a remaining backend gap. Backend `set_ai_api_key_impl` already has secret-store + legacy-delete tests (`src-tauri/src/services/ai_provider/secret.rs:503-766`). | Do not test on the backend in this round. |

Permission risk in this app is **not** user/role RBAC. Remaining permission-shaped risk is Tauri capability, target scope, path boundary, and credential storage. `src-tauri/capabilities/default.json` is a static allowlist (`dialog`, `shell:allow-open`, updater, process restart) with no per-user ACL to unit-test.

### Risk-ranked remaining tests

#### 1. HIGH — Central store relocation copies/overwrites the filesystem, then rewrites SQLite without one transaction or compensation

**Production path**

- `apply_central_store_location_change_impl` copies every source skill directory into the new root (deleting an existing target skill first), then calls `update_central_root`, then rebuilds symlinks, then scans: `src-tauri/src/services/central_store_location/mod.rs:115-201`.
- `update_central_root` issues four independent `UPDATE`s (agents, scan_directories, skills `REPLACE` on `file_path`/`canonical_path`, skill_installations `REPLACE`) with **no** `BEGIN`/`commit`: `src-tauri/src/services/central_store_location/mod.rs:283-331`.
- Path rewrite uses SQL string `REPLACE(old_root, new_root)`, not component-aware path rewrite: `src-tauri/src/services/central_store_location/mod.rs:300-330`. A sibling path whose stored string shares the old-root prefix can be rewritten even though `validated_roots` only rejects same/nested source↔target (`205-227`).
- Overwrite is destructive: existing target skill dirs are removed then copied (`169-174`). A later DB failure cannot recover the overwritten target-only content from the copy step.

**Existing coverage**

- Preview counts copy/overwrite/target-only: `src-tauri/src/services/central_store_location/tests.rs:48-66`.
- Happy-path apply preserves old root, overwrites, imports target-only: `68-105`.
- Same/nested path rejection: `107-127`.
- Native installation path rewrite on success: `129-168`.
- No trigger-injected failure after FS copy, no assertion that agents/scan_directories/skills/installations stay aligned, no prefix-collision `REPLACE` case, no compensation of copied/overwritten trees.

**Why this is business-risk**

Relocating Central is a user-initiated mutation of the canonical skill root plus every Central path in SQLite. A mid-apply DB failure today can leave files at the new root while `agents.global_skills_dir` still points at the old root (or the reverse after a later `UPDATE` succeeds), and overwrite can destroy target-only skills that the product is required to retain (`docs/agents/security-and-shared-state.md` Central migration; AGENTS.md “retain target-only skills”). This is the same class of FS+DB divergence round 1 closed for project install, now on a larger mutation.

**Focused tests to add**

1. Seed source + target-only skills. Inject a trigger that rejects `UPDATE agents SET global_skills_dir`. Assert the call errors, source bytes are unchanged, target-only skill bytes are unchanged, and every Central path column still matches the pre-call snapshot.
2. Allow the agent-row update, reject a later `skills` `REPLACE` (or `scan_directories` write). Assert all four path-bearing tables still match the pre-call snapshot (one transaction / compensation, not “agent moved and skills did not”).
3. Table-test `REPLACE` prefix: Central skill under `…/store` plus a stored path `…/store-extra/...` that is `is_central=1`. After a successful relocate to `…/new`, `store-extra` must be untouched; only paths that are the old root or a child of it may change.
4. Successful retry after dropping the trigger: new root, retained target-only skill, source preserved.

If tests fail, keep the convergent invariant and fix ordering/transaction/compensation in production. Do not weaken assertions to document orphan Central trees.

**Focused command** (must discover nonzero tests)

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked services::central_store_location::tests
```

After adding named cases, also run a name filter such as `central_store_location_` and confirm the filter count is nonzero per `.trellis/spec/quality/test-suite-layout.md` (zero-test filter is not evidence).

#### 2. HIGH — SSH/WSL create/update still split credential storage, settings JSON, and remote-cache init; only delete was covered in round 1

**Production path**

- `create_ssh_target_impl` probes, saves the password, persists `ssh_targets_v1`, and on persist failure deletes the password; then opens `remote_db`: `src-tauri/src/targets/commands.rs:5-47`. Persist cleanup exists (`32-36`); a later `registry.remote_db` failure (`37`) does **not** roll back the persisted list or the credential.
- `update_ssh_target_impl` saves a new password **then** `save_remote_targets` **without** credential rollback if persist fails (`78-92`). Auth-method switch deletes the old password **after** persist (`93-98`). `drop_remote_pool` + `remote_db` run after persist (`99-100`).
- `create_wsl_target_impl` / `update_wsl_target_impl` persist WSL JSON then `remote_db` with no rollback of the list if cache init fails (`246-298`).
- `test_ssh_target_impl` (existing-target password store) and `update_ssh_target_password_impl` save the credential then persist settings (`139-148`, `207-209`) with the same split.

**Existing coverage**

- Round 1 covers **delete** and **set_active** persistence/credential rollback (`src-tauri/src/targets/tests.rs:141-396`).
- Request normalization, credential fallback, and `remote_cache_db_path("ssh-demo_1")` remain helper-level (`560-563`).
- A repository-wide symbol search of `src-tauri` found **no test call** to `create_ssh_target_impl`, `update_ssh_target_impl`, `create_wsl_target_impl`, `update_wsl_target_impl`, or `update_ssh_target_password_impl` (only IPC shells in `src-tauri/src/commands/targets.rs` and the impls themselves).

**Why this is business-risk**

The user-visible SSH target is the settings JSON **plus** the SecretStore password **plus** the per-target cache DB. Create already attempts credential cleanup on persist failure, but that branch is untested; update can leave a new password stored while the old host/user JSON remains (or the reverse after persist). A `remote_db` failure after a successful persist reports an error while the target is already listed. That is credential/target-scope integrity, not getter coverage.

**Test-harness note (not a product decision)**

`probe_ssh_target` / `probe_wsl_target` open a live transport (`src-tauri/src/targets/exec.rs:82-99`). Focused tests must inject a FakeRunner/probe seam (same pattern as `services/installation/tests.rs` `fake_ssh_transport`) or extract the post-probe persist+credential+cache helper and test that helper. Do not require a real SSH/WSL host. Do not skip the mutation because probe is currently un-injected.

**Focused tests to add**

1. Create password SSH: persist trigger rejects `ssh_targets_v1`; assert error, empty SSH list, no credential key, no cached remote pool. Retry after dropping the trigger succeeds and the password is stored once.
2. Create: persist succeeds, injected `remote_db` failure; assert either full rollback (list + credential + no pool) or a documented single retryable state — **if product wants “settings committed, cache lazy”, that is a spec decision before asserting the current split**. Default recommendation: treat create as one mutation outcome.
3. Update: change password; persist trigger rejects; assert the **old** password remains the only stored secret and JSON host/user/id are unchanged.
4. Update key←password: persist succeeds, credential delete fails; assert settings and credential converge (either still password-auth with secret, or key-auth with secret gone) — not JSON key-auth with a leftover password.
5. WSL create/update: persist-then-`remote_db` failure preserves or rolls back the WSL list the same way.

**Focused command**

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked targets::tests
```

Name new tests with a stable prefix (for example `create_ssh_target_`, `update_ssh_target_`) and confirm a name filter discovers nonzero tests. Do not treat the existing delete-only suite as evidence for create/update.

#### 3. HIGH — `ensure_centralized` copies into Central then upserts; on upsert failure the next call treats the orphan copy as already centralized and skips the DB write

**Production path**

- `ensure_centralized` returns `Ok` if `canonical_dir/SKILL.md` already exists (`src-tauri/src/services/installation/centralize.rs:21-28`), including the duplicated exists-check at `31-33`.
- Otherwise it copies `source_dir` → `canonical_dir` (`52-53`) and only then `db::upsert_skill` to set `is_central` / `canonical_path` / `file_path` (`55-63`).
- Local install always calls this before placement: `centralize_shared_root_local` and `prepare_target_local` (`src-tauri/src/services/installation/native.rs:20-48`); shared-root install records a native row only after centralize (`src-tauri/src/services/installation/install.rs:60-75`).
- There is no compensation if `upsert_skill` fails after the copy. A retry hits the early `SKILL.md` exists return and **never retries the upsert**.

**Existing coverage**

- `src-tauri/src/services/installation/tests.rs` is dense for install/uninstall happy paths, skip, occupied target, and FakeRunner remote script args (e.g. remote centralize+install `2710-2758` asserts `is_central` on **success**).
- Grep of `services/installation` found **no** `CREATE TRIGGER` / `RAISE(` and **no** call to `ensure_centralized` that injects upsert failure. `is_central: false` appears only as seed data for the remote happy path (`2692-2708`).

**Why this is business-risk**

This is the adopt-into-Central path required by AGENTS.md (`ensure_centralized`). A trigger-injected upsert failure after copy leaves Central files that are not `is_central` in SQLite. The next install/centralize returns success without repairing the row, so scanners and GitHub provenance can disagree with the filesystem. That is exception-branch integrity, not another happy-path install.

**Focused tests to add**

1. Seed a non-central skill whose `file_path` is outside Central. Call `ensure_centralized` with a trigger that rejects `skills` upsert. Assert the function errors, the canonical copy is absent **or** fully compensated, the source is byte-identical, and `is_central` remains false.
2. If today’s production leaves the copy in place, assert that a **retry after dropping the trigger still upserts** (`is_central` true, `canonical_path` set). That second assertion is what currently fails because of the early return at `centralize.rs:26-28`.
3. Drive the same failure through `prepare_target_local` / `install_skill` Local so the public install use case cannot report success with a non-central row.

If tests fail, fix compensation and/or the exists-short-circuit so retry repairs DB. Do not encode “orphan Central copy + non-central row” as the accepted contract.

**Focused command**

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked services::installation::tests
```

Add names containing `ensure_centralized` and confirm:

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked ensure_centralized
```

discovers nonzero tests.

#### 4. MEDIUM — Persisted target IDs still accept values that `remote_cache_db_path` rejects (round-1 deferred, still untested)

**Production path**

- Config load validates empty, duplicate, and reserved `local` only: `src-tauri/src/targets/config.rs:316-331` (SSH parse also uses this at `258-266`).
- Cache path construction requires ASCII alphanumeric, `-`, or `_`: `src-tauri/src/targets/commands.rs:706-724`.
- Registry opens the cache with that path (`src-tauri/src/targets/registry.rs` uses `remote_cache_db_path`).

**Existing coverage**

- Quarantine tests: duplicate and reserved IDs (`src-tauri/src/targets/config.rs:598-617` and surrounding quarantine suite). User asked **not** to re-test target quarantine happy/isolation paths.
- Cache path: one valid ID `ssh-demo_1` (`src-tauri/src/targets/tests.rs:560-563`).
- No matrix of `../escape`, `a/b`, `a\b`, whitespace, control characters, or validator↔cache parity.

**Why this is business-risk**

A persisted ID that config accepts can later fail cache-path construction or, worse, if sanitization were weakened, create directories outside `app_data/targets/<id>/`. The two boundaries must accept/reject the same ID matrix so list/select/open cannot drift. This is path-boundary / target-scope, not RBAC.

**Focused tests to add**

1. Table-test SSH and WSL persisted IDs: `../escape`, `a/b`, `a\\b`, embedded space/control, and valid `ssh-demo_1`.
2. Assert invalid IDs are quarantined (or otherwise rejected) **before** `remote_cache_db_path` / directory creation; assert no directory is created outside the target-cache root.
3. Assert `validate_target_ids` and `sanitize_target_id` accept/reject the same matrix.

**Focused command**

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked targets::config::tests
```

Also run `remote_cache_db_path` / `sanitize_target_id` cases under `targets::tests` so both modules stay in lockstep. Confirm nonzero discovery.

#### 5. MEDIUM — Local→remote sync apply has no FakeRunner mutation test; failure strings are raw `error.to_string()` on a successful IPC payload

**Production path**

- `apply_local_remote_sync_impl` connects, applies the repo snapshot, then each skill snapshot; per-item failures are pushed as `LocalRemoteSyncFailure { error: error.to_string() }` and the function still returns `Ok`: `src-tauri/src/services/local_remote_sync.rs:130-213`.
- `apply_snapshot` builds a tar.gz, then `run_command_with_stdin_bytes` with staging/backup dirs: `706-738`. The remote script restores backup on copy failure (`88-120`).
- Archive path safety is enforced when building (`is_safe_relative_archive_path`; unit tests `10-16`, `51-64`).
- IPC maps apply **envelope** failures to a reviewed unexpected diagnostic, but a partial `Ok` result includes `failed[].error` (`src-tauri/src/commands/local_remote_sync.rs:112-118`). Operation-log counts are capped to succeeded/skipped/failed counts (`102-110`); the DTO still carries raw error text to the renderer.

**Existing coverage**

- Only helper unit tests: slug, unsafe relative paths, snapshot excludes, archive reject, hash parse (`src-tauri/src/services/local_remote_sync/tests.rs:3-82`).
- **No** call to `preview_local_remote_sync_impl` or `apply_local_remote_sync_impl`. **No** FakeRunner assertion of the apply script/stdin/backup restore.

**Why this is business-risk**

This command writes the user’s local repo and Central skills onto a selected SSH/WSL target (target-scope + path boundary). Transport-seam tests required FakeRunner for remote install (`/.trellis/spec/backend/transport-seam.md`); sync apply is the same class of remote mutation and is uncovered. `error.to_string()` on a **success** IPC payload is the same leakage shape as connection-test `ok: false` (paths/commands/stderr). If product wants raw apply errors in the DTO, that is a spec decision; default recommendation is stable codes only, matching `.trellis/spec/backend/redaction-policy.md:36-37`.

**Focused tests to add**

1. `ConnectedSshTarget::for_tests_with_runner` + `apply_snapshot`: first FakeRunner response fails after staging; assert backup restore command/script contract and no leftover staging path in the recorded argv if the script encodes that.
2. Unsafe archive member cannot be applied (already unit-tested at build time; keep one apply-level assertion so apply cannot bypass `build_archive`).
3. Inject a high-entropy path/token in the runner error; serialize `LocalRemoteSyncApplyResult` (or the IPC-facing DTO); assert sentinels absent from `failed[].error` while a stable category remains.

Item 3 **requires the same redaction-spec decision** as connection-test success payloads if the current DTO is considered user-visible. If spec work is deferred, still add FakeRunner apply/backup tests (items 1–2) without asserting today’s raw `to_string()`.

**Focused command**

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked services::local_remote_sync
```

Today this filter already discovers the helper tests (nonzero). After adding apply tests, also run a name filter such as `apply_snapshot` / `local_remote_sync_apply` and confirm those names exist.

#### 6. MEDIUM — Collection JSON import creates the collection, then links skills one-by-one with no transaction

**Production path**

- `import_collection_impl` validates JSON/name, `create_collection`, then loops `get_skill_by_id` + `add_skill_to_collection` with no `BEGIN`: `src-tauri/src/commands/collections/export_import.rs:53-77`.
- `create_collection` is a single INSERT (`src-tauri/src/db/repos/collections_repo.rs:19-42`). `add_skill_to_collection` is a separate `INSERT OR IGNORE` (`95-110`).
- Delete **is** transactional (`81-91`) and already has trigger rollback at DB layer (`src-tauri/src/db/tests.rs` `transactional_collection_delete_restores_parent_and_child_on_trigger_failure`). Spec `.trellis/spec/backend/transactional-mutations.md:23-24` requires one transaction for collection **deletes**, not explicitly for import.

**Existing coverage**

- Happy create, skip unknown skills, invalid JSON, empty name, roundtrip: `src-tauri/src/commands/collections/tests.rs:378-475`.
- No trigger on a later `collection_skills` insert after the parent row exists.

**Why this is business-risk**

A mid-loop DB failure returns `Err` after a visible collection already exists with a partial membership list. The user can believe import failed and retry, creating a second collection. This is smaller than Central relocate / credentials, but it is still a public mutation without validation-before-write / one top-level transaction (`.trellis/spec/backend/transactional-mutations.md:11-18`).

**Focused tests to add**

1. JSON with two known skill IDs; trigger rejects the second `collection_skills` insert; assert error, **zero** new collections, **zero** new membership rows.
2. Retry after dropping the trigger imports exactly one collection with both skills.
3. Unknown skill IDs remain skipped (already tested) and must not be confused with trigger failure.

If product intentionally allows “collection created, links best-effort”, that is a spec decision before asserting current partial writes.

**Focused command**

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::collections::tests
```

Name new tests `import_collection_` and confirm a name filter discovers nonzero tests.

### Recommended execution order

1. Central store relocation FS+DB compensation (item 1).
2. SSH/WSL create/update credential+settings+cache (item 2).
3. `ensure_centralized` copy/upsert/retry (item 3).
4. Target ID ↔ `sanitize_target_id` parity (item 4).
5. Local-remote sync FakeRunner apply (item 5; redaction assertions only after spec if required).
6. Collection import transaction (item 6).

After each module filter passes with nonzero tests:

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked
just ci
```

Treat a filtered command that reports zero tests as missing evidence (`.trellis/spec/quality/test-suite-layout.md`).

## What NOT to test this round

Do not add tests whose only purpose is raising a count, or that re-cover already-dense surfaces:

- **Round-1 already shipped:** project install/uninstall compensation; target delete / Local / empty / unknown ID / active-target persistence; repository-sync prevalidation and skip-write rollback.
- **GitHub import** happy path, unsafe path, snapshot, redirect, budget, assignment rollback (`src-tauri/src/services/github_import/tests.rs` is already large, including PAT migration `3321+`).
- **IPC redaction contract** and runtime logger (`src-tauri/src/ipc_error/redaction_contract_tests.rs`, `src-tauri/src/logging/tests.rs`, `src-tauri/src/redaction.rs`).
- **Target config quarantine** isolation, duplicate/reserved, malicious metadata, recovery transaction (`src-tauri/src/targets/config.rs` tests) — except the **ID character-class vs cache-path** matrix in item 4.
- **Marketplace** snapshot replace/rollback/later-chunk/commit failure (`src-tauri/src/services/marketplace/tests.rs`).
- **Local archive** zip-slip/traversal/fingerprint/overwrite DB-failure restore (`src-tauri/src/services/local_archive_import/`).
- **Startup recovery** backup/partial-move rollback (`src-tauri/src/services/startup/mod.rs:319-509`).
- **FS+DB operation journal**, skill deletion cascade, unknown-source reset, update inventory replace, scanner keep-set, Skills CLI path containment, remote canonical containment, exclusive-job registry, settings-policy allowlist, AI/GitHub PAT secret-store migration, deep-link parser, portable-state file-adapter budgets, agents CRUD validation.
- **Getters**, Display boilerplate, third-party keyring/SQLx internals, `SecretStore` mock itself.
- **Tauri `capabilities/default.json` as a fake RBAC suite** — there is no user/role model; do not invent authorization tests.
- **Connection-test `Ok { ok: false, message: error.to_string() }`** until spec decides the public payload (see Caveats).
- **AI settings concurrent flush / latest-edit-wins** — frontend product decision; backend secret write is already tested.

## Files Found

| File Path | Description |
|---|---|
| `src-tauri/src/services/central_store_location/mod.rs` | Relocate Central: FS copy/overwrite then non-transactional path UPDATEs |
| `src-tauri/src/services/central_store_location/tests.rs` | Preview/happy-path/nested/native-path only |
| `src-tauri/src/targets/commands.rs` | SSH/WSL create/update/test/password; `sanitize_target_id`; connection-test `message: error.to_string()` |
| `src-tauri/src/targets/tests.rs` | Delete/active-target rollback (round 1); no create/update impl calls |
| `src-tauri/src/targets/config.rs` | Target JSON quarantine; `validate_target_ids` empty/duplicate/reserved only |
| `src-tauri/src/targets/exec.rs` | Live `probe_ssh_target` / `probe_wsl_target` |
| `src-tauri/src/services/installation/centralize.rs` | `ensure_centralized` copy-then-upsert and exists short-circuit |
| `src-tauri/src/services/installation/native.rs` | Local centralize/prepare call sites |
| `src-tauri/src/services/installation/tests.rs` | Dense install happy paths; no upsert-failure-after-copy |
| `src-tauri/src/services/local_remote_sync.rs` | Preview/apply orchestration and remote tar apply |
| `src-tauri/src/services/local_remote_sync/tests.rs` | Path/hash helpers only |
| `src-tauri/src/commands/local_remote_sync.rs` | IPC `Ok` partial result with `failed[].error` |
| `src-tauri/src/commands/collections/export_import.rs` | Collection import create-then-loop-link |
| `src-tauri/src/commands/collections/tests.rs` | Import happy path / skip unknown; no trigger rollback |
| `src-tauri/capabilities/default.json` | Static window capability allowlist (not RBAC) |
| `.trellis/spec/backend/transactional-mutations.md` | Validation-before-write and top-level transaction |
| `.trellis/spec/backend/redaction-policy.md` | IPC must not carry paths/commands/stderr/credentials |
| `.trellis/spec/backend/transport-seam.md` | FakeRunner required for remote mutation tests |
| `.trellis/spec/backend/settings-domain-boundary.md` | Target deletion rollback (already tested); ID character class not in quarantine matrix |
| `.trellis/spec/quality/test-suite-layout.md` | Zero-test filter is invalid evidence |

## External References

None. Assessment is repository-grounded.

## Related Specs

- `.trellis/spec/backend/transactional-mutations.md`
- `.trellis/spec/backend/path-policy.md`
- `.trellis/spec/backend/redaction-policy.md`
- `.trellis/spec/backend/transport-seam.md`
- `.trellis/spec/backend/settings-domain-boundary.md`
- `.trellis/spec/backend/test-support.md`
- `.trellis/spec/quality/test-suite-layout.md`
- `docs/agents/security-and-shared-state.md`

## Caveats / Not Found

- Tests were not executed in this research role (running them would write Cargo artifacts outside `{TASK_DIR}/research/`). Commands above are focused execution guidance for implement/check.
- **Product/spec required before tests:**
  1. SSH/WSL connection-test **success** payloads with `ok: false` still embed `error.to_string()` (`commands.rs:175-182`, `330-336`). Round 1 deferred this because `.trellis/spec/backend/redaction-policy.md` talks about IPC **errors**, not `Ok(Dto)`. Writing tests against today’s `message` would freeze leakage. Decide: map transport failures to stable codes on the DTO, or explicitly allow raw probe text on `ok: false`.
  2. Item 2 create: if `remote_db` after persist is intentionally best-effort, document that before asserting “settings committed, error returned”.
  3. Item 5 `failed[].error` on `Ok(LocalRemoteSyncApplyResult)` is the same success-payload class as (1).
  4. Item 6: if collection import is intentionally “create then best-effort links”, document that before requiring all-or-nothing.
- **Do not treat as remaining HIGH:** AI concurrent flush (frontend); GitHub PAT/AI key migration (already tested); marketplace/inventory/journal/startup/archive (already dense).
- No user/role RBAC exists to test. Capability files are static.
- `task.py current` reported no active task at research start; output was written to the path specified in the request: `.trellis/tasks/08-31-risk-driven-test-coverage-r2/research/`.
