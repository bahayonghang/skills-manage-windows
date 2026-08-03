# Design: Registry-backed Marketplace Central install

## 1. 当前错误数据流

```text
marketplace row(name, download_url)
  -> GET cached download_url
  -> central_root / frontmatter.name
  -> write only SKILL.md
  -> set marketplace cache installed=true
```

这里同时混淆了 display identity、request authority、filesystem identity 和安装事实。

## 2. 目标数据流

```text
requested marketplace row id
  -> load enabled registry source
  -> resolve GitHub source + auth through github_import
  -> acquire one pinned snapshot/workspace
  -> rebuild candidates with shared parser
  -> map candidates with marketplace_skills_from_candidates
  -> exact unique id match
  -> GitHubSkillImportSelection(Overwrite)
  -> shared Local/SSH/WSL import use case
       lock -> recover -> stage/swap -> DB/provenance -> journal
  -> refresh/derive Marketplace installed state
```

## 3. Identity decisions

- Requested Marketplace row `id` remains the compatibility key.
- Re-run the existing candidate-to-marketplace mapping against the pinned snapshot, then exact-match the requested row id. Do not parse a display name into a path and do not reverse-parse `download_url`.
- Exactly zero matches means registry changed/stale cache; exactly more than one means ambiguous identity. Both fail before Central mutation and request a registry refresh.
- Candidate `skill_id`/source path from GitHub import decides import identity and target directory. Frontmatter `name` remains metadata only.
- Fix the sync mapper if duplicate candidate IDs can produce one DB id nondeterministically; duplicate display names and duplicate stable IDs need separate deterministic rejection tests.

## 4. Acquisition and byte identity

Extract/generalize the existing skills.sh snapshot helper instead of constructing a third client. The helper returns resolved repository identity, retained snapshot/candidate inventory and auth decision.

- Local: import the exact retained snapshot used for candidate matching.
- SSH/WSL: create/import a workspace pinned to the same resolved commit. No branch-tip re-resolution after validation.
- Network acquisition completes before Central lock; final apply rechecks target/DB state under the shared lock.
- `download_url` remains serialized for UI/backward cache compatibility but is never passed to `reqwest` by install.

## 5. Installed-state semantics

`marketplace_skills.is_installed` is a cache hint. The durable facts are Central `skills` plus repository membership/provenance.

After import success, update the requested cache row or re-enrich the query from live Central IDs/names. A cache update failure is logged as a redacted derived-state repair condition; it must not return a generic install failure after filesystem/DB commit. The next search/sync repairs the hint. Before import success, no true marker is written.

## 5.1 Approved journal persistence decision

The existing `central_update` Saga is generalized from "replace an existing Central skill" to "durably upsert Central content". A first import uses `OperationKind::CentralUpdate` with `UpdateManifest(had_target=false)` because the existing stage/swap/rollback/finalize and recovery state machine already models an absent target. This is an approved extension of the persisted operation-kind semantics; it does not add a migration or a parallel journal.

The shared final-apply helper must:

- acquire the target mutation guard and recover pending Central operations before apply;
- insert the prepared operation row, durably stage and swap the complete candidate directory;
- in one SQLite transaction upsert the skill, repository membership, commit/digest provenance, and transition the operation to `db_committed`;
- roll back or roll forward through the existing `UpdateManifest` recovery rules, including Local, SSH and WSL;
- preserve existing update callers and public pending-operation DTOs.

Do not route Marketplace through `update_skills_batch` if that would require a pre-existing `SkillUpdatePlan`. Extract or generalize the internal journaled content-upsert boundary so GitHub import and Central update share the Saga without coupling Marketplace to update inventory semantics.

## 6. Error and compatibility boundary

- Reuse `MarketplaceError::GithubImport`/typed variants; add semantic stale/ambiguous candidate variants if needed.
- Tauri command remains `install_marketplace_skill` with existing request/response unless implementation proves a result payload is needed. Any IPC change requires generated command docs and frontend parity updates.
- No URL, target path, preview token, PAT or body in public error details.

## 7. Files expected to move

- `services/marketplace/mod.rs`, `marketplace/tests.rs`, possibly a focused `marketplace/install.rs` if size requires.
- Reusable acquisition/import helpers under `services/github_import/` or the existing `marketplace/skills_sh.rs` boundary.
- Marketplace command only for dependency injection/error mapping if signatures change.
- `docs/architecture/marketplace-pipeline.md` and relevant backend spec if the shared install contract gains a durable scenario.

## 8. Rollback

First add tests and shared use case, then switch registry-backed install, then delete direct writer. The approved implementation reuses the existing `central_update` row and `UpdateManifest`; no persisted schema changes are required. After the security fix is selected for implementation, rollback must disable/fail closed on the registry-backed install rather than restore the name-derived direct writer; it must never delete a successfully imported user skill or provenance row.
