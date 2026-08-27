# Skills CLI Global Contract

## 1. Scope / Trigger

Apply this contract when adding, changing, or calling the official Skills CLI
global (`-g`) surface: `skills_cli_*` IPC, `services/skills_cli`, leftover
CLI-lock exclusion, leftover apply mutation locking, or platform
`install_origin` annotation.

This is **not** SkillPort Central (`~/.skillsmanage/skills/`) and **not** the
`skillport-cli` binary (`shared-local-cli.md`). The npm package is pinned
(`SKILLS_CLI_NPM_SPEC = "skills@1.5.23"`). Commands freeze a TargetContext,
query `ensure_capability_for_target`, then build `SkillsCliTransport` only when
the capability is open. This seam task opens **Doctor** on Remote; other
capabilities stay `skills_cli.local_target_only` until a later child opens them.
`RevealFolder` is permanently unsupported on Remote (no host file manager).

## 2. Signatures

```rust
// commands/skills_cli.rs — each command resolve_target_context() first.
pub async fn skills_cli_doctor() -> IpcResult<SkillsCliDoctorReport>;
pub async fn skills_cli_list_global() -> IpcResult<SkillsCliGlobalSnapshot>;
pub async fn skills_cli_install_targets() -> IpcResult<Vec<SkillsCliInstallTarget>>;
pub async fn skills_cli_preview_source(source: String) -> IpcResult<SkillsCliSourcePreview>;
pub async fn skills_cli_add_global(
    job_id: String,
    source: String,
    skill_names: Vec<String>,
    skillport_agent_ids: Vec<String>,
) -> IpcResult<SkillsCliAddResult>;
pub async fn skills_cli_remove_global(
    job_id: String,
    skill_name: String,
) -> IpcResult<SkillsCliRemoveResult>;
pub async fn skills_cli_preview_remove_global(
    skill_name: String,
) -> IpcResult<SkillsCliRemovePlan>;
pub async fn skills_cli_read_skill_md(skill_name: String) -> IpcResult<SkillsCliSkillDoc>;
pub async fn skills_cli_reveal_skill_folder(skill_name: String) -> IpcResult<()>;
pub async fn skills_cli_link_platform(
    job_id: String,
    skill_name: String,
    skillport_agent_id: String,
) -> IpcResult<SkillsCliPlacement>;
pub async fn skills_cli_unlink_platform(
    job_id: String,
    skill_name: String,
    skillport_agent_id: String,
) -> IpcResult<SkillsCliPlacement>;
pub async fn skills_cli_export_inventory(path: String, json: String) -> IpcResult<()>;
pub async fn cancel_skills_cli_job(job_id: String) -> IpcResult<bool>;
pub async fn skills_cli_check_updates(job_id: String) -> IpcResult<SkillsCliUpdateInventory>;
pub async fn skills_cli_update_inventory() -> IpcResult<SkillsCliUpdateInventory>;
pub async fn skills_cli_verify_update_baseline(
    job_id: String,
    skill_names: Vec<String>,
) -> IpcResult<SkillsCliUpdateInventory>;
pub async fn skills_cli_apply_updates(
    request: SkillsCliApplyUpdateRequest,
) -> IpcResult<SkillsCliApplyResult>;
pub async fn skills_cli_retry_update_recovery(
    job_id: String,
    operation_id: String,
) -> IpcResult<SkillsCliApplyRecoveryResult>;
```

Frontend: only `src/stores/skillsCliStore.ts` may `invoke` these commands.
Renderer job IDs follow `job-correlation-cancellation.md`. Reveal does not accept a path.
Export writer owns atomic persist; the renderer never receives filesystem write authority.
`batchProgress` is the duplicate-submit lock for serial store loops (cleanup, apply-by-repo, per-platform unlink). Do not fold that lock into `skillsCliOperationBusy`: serial `applyUpdates` clears the exclusive job between groups and would throw busy mid-batch.
Cleanup candidates are skills whose placements are all `unavailable`. Group `stale` iff any placement `reasonCode` is `canonical_missing` (default selected); other unavailable reasonCodes are `platformUnavailable` (default unchecked, real uninstall). Confirm still uses `skills_cli_preview_remove_global` + existing `removeGlobalBatch`. The "all placements unavailable" predicate lives in one shared helper used by both the Unavailable badge and the cleanup set.

## 3. Contracts

- **Capability matrix**: query `SkillsCliTransport::ensure_capability_for_target`
  after `resolve_target_context()` and **before** `for_target()`. Unsupported
  Remote capabilities return `skills_cli.local_target_only` before handshake,
  spawn, lock reads, leftover CLI protection, or origin annotation, and are
  zero-write. Do not use this machine's lock to protect remote leftover.

| Capability | Local | Remote (this seam) | Opens in |
| --- | --- | --- | --- |
| doctor | supported | supported | seam |
| list / install_targets / read / export | supported | `local_target_only` | inventory |
| reveal | supported | permanently unsupported | — |
| link / unlink / preview_remove / remove / leftover | supported | `local_target_only` | mutate |
| preview_source / add / update family | supported | `local_target_only` | install-update |
| cancel_job | supported | `local_target_only` | stays gated until a child opens the matching mutation |
- **Ownership**: a path is CLI-owned only when `.skill-lock.json` (version 3)
  contains the sanitized name. Do not treat `~/.agents/skills/` as wholly
  owned. Lock path: `$XDG_STATE_HOME/skills/.skill-lock.json`, else
  `home / UNIVERSAL_AGENTS_DIR_NAME / .skill-lock.json` (no `.agents` literal).
- **Launcher**: program is `node.exe` / `node`. `argv[1]` is npm `npx-cli.js`.
  Doctor only resolves the node program and runs `node --version`; `npx-cli.js`
  resolution belongs to spawn paths (add/preview). Never `Command::new("npx.cmd")`
  or `cmd /c` string concat. Prefix:
  `--yes --package=skills@1.5.23 -- skills`. Add/remove then add skills-layer
  `-g -y` plus at least one `-a` and `-s`. Never default `--all` or `--agent '*'`.
- **Doctor gate (UI)**: a failed doctor (`node_missing`, `timeout`, `cancelled`,
  or `internal.unexpected`) fail-closes Install only. list / remove / link /
  unlink / export / detail stay available regardless of doctor result.
  `cli_unavailable` is spawn-path only (add/preview) and must not lock the page.
  This supersedes archived `08-25-skills-cli-inventory-frontend` R5, which
  disabled uninstall when `cli_unavailable` appeared.
- **Inventory read**: `skills_cli_list_global` reads lock v3 + filesystem + the
  same mapped∩detected platform set as `install_targets`. It must not spawn
  the CLI. Membership is lock names only. `path` / `installKind` prefer
  `universal_skills_dir/<name>` when that directory exists; otherwise a copy
  directory `{agent.global_skills_dir}/<name>` on a mapped detected agent
  (`canonical` | `copy` | `missing`). Authoritative platform state is
  `placements` (`managed_link` | `direct_copy` | `missing` | `conflict` |
  `unavailable`). Do not add parallel `agentIds` / `linkTargets` arrays.
  Compatibility `agents` is derived only from `managed_link` and `direct_copy`
  display names. Platform `agents` include copy hits even
  when `classify_local_path_origin` is `Other`. Missing lock or empty lock
  returns an empty `skills` array with `canonicalRoot` and `lockPath` still
  set — not an error. List IO maps to `internal.unexpected`, never
  `skills_cli.cli_unavailable`.
- **Directory links**: Skills CLI managed links are Windows junctions (reparse
  API, no `cmd.exe`/`mklink`/symlink privilege/copy fallback) or Unix directory
  symlinks. Never auto-convert `direct_copy` into a junction/symlink. Never
  delete an ordinary directory or call `remove_dir_all` on a platform path.
- **Remove recovery**: `skills_cli_remove_global` does not spawn `skills remove`
  and never uses unverified `--force`/`--keep-links`. Domain-local manifests live
  under `skills_cli_remove_recovery_dir()`. Phases: prepared → staged →
  metadata_committed → cleanup. Lock fingerprint CAS. Conflict is zero-write.
  Direct copies are byte-preserved and never entered in the mutation path.
- **Settings**: exact generic key `skills_cli.recent_sources` with
  `SettingCategory::SkillsCli`. Array, 0–8 items, 16 KiB serialized, 2048-byte
  item, no control chars, exact-trim, BTreeSet dedupe, Skills CLI source
  validation without URL credentials/query/fragment.
- **Process**: reuse `ProcessRequest` + Job Object. preview = Standard;
  add = BulkTransfer; list/read/preview-remove do not spawn. stderr cap 1 MiB.
  stdout/stderr/URLs stay out of `IpcError.message` and unredacted
  operation-log details.
- **FS mutex vs job family**: exclusive job `skills_cli` is cancel/progress
  only (`exclusive-job-lifecycle.md`). Filesystem writes take
  `acquire_target_mutation_guard` (`central-mutation-lock.md`). Order: lease →
  guard → under-guard ownership/placement recheck → FS/lock mutation → drop
  guard → drop lease. Link/unlink/remove follow this order.
- **Upstream updates**: GitHub SHA/snapshot pinning for *detection* reuses
  SecretStore / `github_client` at the command boundary. Product argv never
  includes `--force`, `--keep-links`, or an unverified full-SHA `skills add`
  source. Pinned full-SHA `skills add`/`update` and direct-copy refresh are
  fail-closed (`verified_unsupported` / `unverified`). Apply refreshes owned
  canonical files from a pinned GitHub snapshot over HTTP, then journals
  `prepared → cli_started → db_committed` (or `recovery_required`). Order:
  `skills_cli` lease → network prepare → Local mutation guard → recheck →
  journal. Never delete ordinary directories; never auto-convert `direct_copy`;
  conflict is zero-write. `skills_cli_update_inventory` is a cache read and
  must not fail global inventory. Progress event: `skills-cli://update-progress`.
- **Leftover**: Local scan sets `cli_lock_protect=true` and excludes lock-owned
  canonicals, resolved links, **and** `{mapped_detected_agent.global_skills_dir}/<name>`
  when the lock contains `name`. Unlocked copies under the Universal root stay
  eligible. Do not exclude the whole Universal root. Remote leftover must not
  use this machine's lock. Local leftover apply holds the Local guard for the
  whole delete loop; remote leftover apply holds that target's guard.
- **Origin**: Local `get_skills_by_agent` annotates `install_origin` via
  `classify_local_path_origin`. Renderer never reads the lock.
  `link_type === "symlink"` is not automatically Central
  (`platform-origin-classification.md`).
- **Selection**: candidate platforms = detected ∩ mapped. Default selected =
  those that are enabled. Empty skill or platform lists refuse add. Every seed
  builtin id is mapped or explicitly unsupported.

## 4. Validation & Error Matrix

| Condition | Code | retryable |
| --- | --- | --- |
| ActiveTarget is SSH/WSL and the capability is not yet opened (or is permanently unsupported, e.g. reveal) | `skills_cli.local_target_only` | false |
| Node missing or `< 22.20.0` | `skills_cli.node_missing` | false |
| npx JS cannot be resolved, or the CLI process cannot spawn | `skills_cli.cli_unavailable` | false |
| CLI non-zero on add | `skills_cli.cli_failed` | false |
| Source fails whitelist (`&\|^%!<>"'`, spaces, `-c`) | `skills_cli.source_invalid` | false |
| `--list` stdout has no parseable skill names | `skills_cli.preview_unparsed` | false |
| Zero skills or zero platforms | `skills_cli.selection_empty` | false |
| Selected SkillPort id has no `--agent` mapping | `skills_cli.agent_unmapped` | false |
| Target mutation lock or same-family job busy | `skills_cli.busy` | true |
| Process deadline exceeded | `skills_cli.timeout` | false |
| Exclusive-job cancel | `skills_cli.cancelled` | false |
| lock/FS IO, output cap, listing parse failure | `internal.unexpected` | false |
| Lock does not own the name | `skills_cli.skill_not_owned` | false |
| Canonical missing / not a directory | `skills_cli.canonical_missing` | false |
| SKILL.md missing / too large / invalid UTF-8 | `skills_cli.skill_doc_missing` / `skills_cli.skill_doc_too_large` / `skills_cli.skill_doc_invalid_utf8` | false |
| Direct copy cannot be linked or unlinked | `skills_cli.direct_copy_not_toggleable` | false |
| Placement conflict / unavailable | `skills_cli.placement_conflict` / `skills_cli.placement_unavailable` | false |
| Export schema / persist | `skills_cli.export_invalid` / `skills_cli.export_failed` | false |
| Reveal spawn failure | `skills_cli.reveal_failed` | false |
| Remove recovery required | `skills_cli.recovery_required` | true |
| Update cache/request stale | `skills_cli.update_stale` | true |
| No exact installed baseline | `skills_cli.update_baseline_required` | false |
| Source/path not a supported GitHub update | `skills_cli.update_unsupported` | false |
| GitHub primary/secondary/429 limit | `skills_cli.update_rate_limited` | true |
| Repository check failed after settle | `skills_cli.update_check_failed` | true |
| Canonical files differ from installed baseline | `skills_cli.update_local_modified` | false |
| direct_copy or conflict placement | `skills_cli.update_topology_conflict` | false |
| Interrupted journaled apply | `skills_cli.update_recovery_required` | true |
| Post-apply digest/lock mismatch | `skills_cli.update_integrity` | false |
| v7 update tables unavailable | `skills_cli.update_migration` | false |

Same-family exclusive-job busy at the registry is `job.skills_cli_busy`; the
command layer remaps it to `skills_cli.busy` so the UI sees one envelope.

## 5. Good / Base / Bad Cases

- Good: add acquires the `skills_cli` lease, then the Local mutation guard,
  spawns `node` + `npx-cli.js --yes --package=skills@1.5.23 -- skills add … -g -y -a … -s …`,
  and leftover local apply / Central install wait Busy until the guard drops.
- Base: preview is a lock+FS read that may spawn; doctor only probes node
  (Local PATH or one Remote `run_script`); list is a lock+FS read: no exclusive
  job, no mutation lock, no CLI spawn. Remote doctor round-trips are constant
  in platform count and never run `skills --help`.
- Bad: `Command::new("npx.cmd")`; leftover scan skipping every path under
  `~/.agents/skills/`; using the local lock while scanning SSH leftover;
  mixing `active_db()` + `active_target()` when annotating origin;
  `match ActiveTarget` in Skills CLI business logic outside `transport.rs`.

## 6. Tests Required

- Argv table: `--yes`, PIN spec, `-g -y -a -s`; assert no `--all`, `*`, `npx.cmd`.
- Mapping closure: every seed builtin id is mapped or unsupported.
- Doctor: missing node / too old; missing npx JS only affects spawn paths.
  Remote doctor: constant remote command count for 1 vs 6 platforms; Node
  missing and too-old both `skills_cli.node_missing`; no `skills --help`.
- Capability matrix: Remote + unopened capability → `skills_cli.local_target_only`
  and zero writes; Remote + Doctor → Ok; Local + any capability → Ok.
  `cli_lock_protect=false` does not exclude remote leftover.
- Leftover: lock canonical/link excluded; lock-named mapped agent copy
  excluded; unlocked sibling copy still listed; remote scan ignores local lock
  (lands in `remote-mutate`; this seam only keeps the lock-isolation rule).
- Inventory: copy-only (no canonical) still listed with `installKind=copy`;
  lock name with no directories listed as `missing`; unknown `sourceType` →
  `sourceTypeBucket=unknown`; `placements` five-state table; compatible `agents`
  derived only from managed_link + direct_copy.
- Lock parse: camelCase and snake_case optional fields; empty/missing → `None`.
- Bounded SKILL.md: exact 1 MiB, growth, UTF-8, missing, escape, non-directory,
  reveal spawn failure. Shared `limit + 1` opened-handle reader.
- Link/unlink: Missing↔ManagedLink only; ordinary directory / conflict zero-write;
  cancel before guard; busy; partial-create cleanup; operation-log redaction.
- Safe remove: preview has no paths/argv; conflict zero-write; copy byte
  preservation; prepared/fingerprint recovery; never spawn `skills remove`.
- Export: v1 envelope exact keys; old target preserved; temp cleanup.
- Settings: `skills_cli.recent_sources` single/batch zero-write, audit redaction,
  restart roundtrip.
- Origin: CLI junction/symlink vs Central symlink vs copy.
- Cancel: fake runner observes cancel flag → `skills_cli.cancelled`.
- Timeout and stdout cap via `ProcessPolicy::for_tests`.
- Source `&|^%!` rejected; stderr absent from `IpcError.message`.
- Contention: held Local guard → leftover apply and `acquire_target_mutation_guard` Busy/Timeout.
- Vitest: list, default platform checks, changed add payload, uninstall confirm
  stays open on failure, non-Local sidebar hidden, doctor error. No public network.
- Update statuses: assert `local_modified` and `unsupported` from classify/check
  (not only the nine-label UI list); empty cache yields `not_checked`; pending
  update rows survive file-DB close/reopen. Parent AC9 stays fail until these exist.
- Apply journal: inject `ApplyFault::{Backups, CliStarted, CliSucceeded, DbCommitted}`
  and assert lock + placement after success. Parent AC10 stays fail until these exist.

## 7. Wrong vs Correct

```rust
// Wrong: batch file as program; skills-layer -y stolen by npx.
Command::new("npx.cmd").args(["-y", "skills@1.5.23", "add", source]);

// Correct: node + npx JS; npx flags before `--`; skills flags after.
ProcessRequest::new(Command::new(&launcher.program), policy)
    .args(launcher.npx_argv_prefix()) // npx-cli.js --yes --package=skills@1.5.23 -- skills
    .args(["add", source, "-s", name, "-g", "-a", agent, "-y"])
```

```rust
// Wrong: exclusive job family as filesystem mutex (other families still run).
let _lease = state.skills_cli_jobs.acquire(&job_id)?;
spawn_add().await; // leftover apply / install_skill can write the same path

// Correct: lease then Local target mutation guard covering the whole child.
let _lease = state.skills_cli_jobs.acquire(&job_id)?;
let _guard = acquire_target_mutation_guard(&ActiveTarget::Local, "Skills CLI global install", timeout).await?;
spawn_add().await;
```

```rust
// Wrong: doctor resolves NodeLauncher (npx-cli.js) then probes `skills --help`.
doctor_with_launcher(runner, &resolve_launcher()?).await?; // CliUnavailable if npx JS missing
build_probe_argv(&launcher); // extra spawn + network

// Correct: doctor only resolves node and runs `node --version`.
doctor_with_program(runner, &resolve_node_program()?).await?;
// Map Start-phase spawn failure to NodeMissing so the header never shows cli_unavailable.
```

```tsx
// Wrong: any doctor/runtime error locks uninstall, link, unlink, export, detail.
const runtimeBlocked = runtimeError !== null;

// Correct: Install is fail-closed; other surfaces stay usable.
// cli_unavailable is spawn-path only (add/preview). Add non-zero is skills_cli.cli_failed.
```

```rust
// Wrong: one Local-only gate, then match ActiveTarget in list/link/remove.
domain::ensure_local_target(target)?;
match target { ActiveTarget::Local => local_home(), ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => remote_home() }

// Correct: gate with the capability matrix before connect; paths/fs come from the transport.
SkillsCliTransport::ensure_capability_for_target(target, SkillsCliCapability::Doctor)?;
let tx = SkillsCliTransport::for_target(target).await?;
// Business logic reads tx.paths() / tx.fs(). Only transport.rs matches ActiveTarget.
```
