# Target-aware Central state import/export

## Goal

Make Central state import/export operate against the currently selected SkillPort target, not implicitly against the local/root installation only.

The current dialog exports and imports GitHub sources plus Central Skills manifest data from the root/local database. That is misleading once the app can switch to SSH and WSL targets: a user viewing an SSH or WSL target expects the portable state action to describe and affect that target's Central store/cache, not silently fall back to local.

## User Value

- Users can move or back up Central state for Local, SSH, and WSL targets with the same mental model as the rest of Central Skills.
- The dialog makes the target boundary explicit before the user saves or imports JSON.
- Import previews and results match the target that is active in the app.

## Confirmed Facts

- Portable state manifest `version=1` currently contains `githubSources`, `centralSkills`, and `unrestorableSkills`; it does not encode target identity.
- `export_skillport_state`, `preview_skillport_state_import`, and `import_skillport_state` currently pass `state.db` into portable state services and record operation logs with `local_target_context()`.
- The portable state backend restores only GitHub-backed Central skills. Local/unknown-source Central skills are exported as `unrestorableSkills` with `source_unknown`.
- Existing remote support already has the needed target primitives:
  - `AppState.active_target()` returns `ActiveTarget::Local`, `ActiveTarget::Ssh`, or `ActiveTarget::Wsl`.
  - `AppState.active_db()` / `TargetRegistry.active_db()` isolate SSH/WSL target cache DBs and initialize remote-home agent paths.
  - GitHub import commands already branch on `ActiveTarget` and call remote GitHub import for SSH/WSL.
  - Central update commands already use `CentralFs::from_active_target()` for local/remote file-system behavior.
- The frontend Central store already resets on target changes and portable progress is tracked through `portabilityJob`.
- The visible dialog currently says "Import / export Central state" and "Move GitHub sources and the Central Skills manifest between SkillPort installations", which does not expose the active target boundary.
- Existing tests cover v1 manifest parsing/export/preview/import grouping, but there is no coverage for target-aware portable state commands.

## Requirements

- Export uses the active target's Central DB/cache:
  - Local target behavior remains backward compatible.
  - SSH/WSL target export reads that target's cached Central skills, GitHub source registries, tags, and unrestorable skills.
- Import preview uses the active target's Central DB/cache:
  - conflicts, existing sources, and existing skills are classified against the active target only.
  - GitHub remote catalog checks still use the app-level GitHub PAT from the local secret store.
- Import applies to the active target:
  - Local target continues using local GitHub import/write behavior.
  - SSH/WSL targets use the existing remote GitHub import path so files are written on the remote target and the remote cache DB is updated.
  - Added GitHub source registries and restored tags are written to the active target DB/cache.
- Operation logs use `target_context_from_active_target()` rather than always recording Local.
- The dialog and export JSON summary make the active target boundary visible enough that users know what they are exporting/importing.
- Existing v1 JSON files remain importable.
- New manifest metadata, if added, must be optional or versioned in a way that does not break v1 imports.
- All user-visible text goes through English and Chinese i18n resources.
- Keep the change scoped to portable state target-awareness. Do not redesign repository sync, platform install/uninstall, target management, or Central store location movement in this task.

## Recommended Product Decisions

- Keep manifest `version=1` import-compatible and add optional `exportedFrom.target` metadata instead of making target identity part of restore matching.
  - Reason: a portable backup should be restorable to a different target; hard-binding JSON to one target would block legitimate migration.
  - Trade-off: the manifest is advisory about origin target, while the active target remains the execution boundary.
- Make the UI label say the active target name/kind near the dialog title or description.
  - Example meaning: "Exporting Central state from Local" or "Importing into Ubuntu WSL".
  - Reason: target selection already lives in the app shell; portable actions should mirror it, not add an independent target picker.
- Do not restore platform/agent installation links in the first implementation.
  - Reason: the current manifest describes Central source state, not per-agent install topology. Restoring links would require link/copy policy, symlink support, and remote-platform validation.

## Acceptance Criteria

- [ ] Local export/import behavior and existing v1 tests continue to pass.
- [ ] When the active target is SSH or WSL, export reads from the target cache DB rather than the local DB.
- [ ] When the active target is SSH or WSL, import preview detects conflicts/sources against that target cache DB rather than the local DB.
- [ ] When the active target is SSH or WSL, import writes skills through the remote GitHub import path and records target DB rows for imported skills, repositories, registries, and tags.
- [ ] Operation log rows for export, preview, and import carry the active target context.
- [ ] The dialog shows which target is being exported from/imported into.
- [ ] Exported JSON includes optional origin-target metadata without rejecting older JSON that lacks it.
- [ ] English and Chinese locale files are updated for new visible strings.
- [ ] Rust tests cover active DB / active target routing for portable state commands or equivalent service-level target branching.
- [ ] Frontend tests cover the target label/description in the portability dialog or the binding that feeds it.
- [ ] Final implementation verification includes `cargo test` for portable state/GitHub import-adjacent tests, `pnpm typecheck`, `pnpm lint`, and `just ci`.

## Out of Scope

- Restoring per-agent/platform installation links.
- Moving Central store locations.
- Syncing local Central state into a remote target outside JSON import/export.
- Changing the global target switcher UX.
- Supporting non-GitHub source restoration.

## Open Questions

- Should import show a warning when the JSON was exported from a different target kind/name than the current active target?
  - Recommended answer: yes, as a non-blocking warning. This preserves portability while making accidental cross-target imports visible.
