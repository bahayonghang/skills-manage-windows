# Design

## Concepts

Use three separate terms consistently:

- **Target**: SkillPort execution/cache boundary, one of Local, SSH, or WSL. This is selected by the app-level target switcher and backed by `ActiveTarget`.
- **Central store**: the target's Central Skills root and database/cache records.
- **Agent/platform**: a skills directory inside a target, such as Codex, Cursor, Claude Code, or the Universal virtual group.

This task is target-aware Central state portability. It is not platform-install topology portability.

## Current Data Flow

Export:

```text
CentralStatePortabilityDialog
  -> centralSkillsStore.exportSkillportState()
  -> invoke("export_skillport_state")
  -> commands/portable_state.rs
  -> export_skillport_state_impl(state.db)
  -> services/portable_state/export.rs
  -> JSON manifest
```

Preview:

```text
Dialog import JSON
  -> invoke("preview_skillport_state_import")
  -> parse_manifest()
  -> build_remote_catalog(state.db, local secrets, manifest)
  -> preview_skillport_state_import_impl(state.db, manifest, catalog)
```

Import:

```text
Dialog resolutions
  -> invoke("import_skillport_state")
  -> parse_manifest()
  -> import_skillport_state_impl(state.db, local secrets, manifest, resolutions)
  -> github_import::import_github_repo_skills_partially_with_auth(...)
```

The problem is that all three paths use `state.db`, so they are local/root scoped. Command logs also use `local_target_context()`.

## Target-aware Flow

Command layer should become the target router:

```text
active_target = state.active_target()
active_pool = state.active_db()
target_context = target_context_from_active_target(active_target)
```

Export:

```text
export_skillport_state
  -> active_target + active_db
  -> export_skillport_state_impl(active_db, origin target metadata)
```

Preview:

```text
preview_skillport_state_import
  -> active_target + active_db
  -> build_remote_catalog(active_db, local secrets, manifest)
  -> preview_skillport_state_import_impl(active_db, manifest, catalog)
```

Import:

```text
import_skillport_state
  -> active_target + active_db
  -> import_skillport_state_for_target(...)
     Local: existing local implementation
     SSH/WSL: remote-aware implementation using existing GitHub import remote path
```

The local secret store remains local for GitHub PAT lookup. Target cache DBs should not store PATs.

## Backend Shape

### Manifest Types

Add optional origin target metadata without breaking v1:

```rust
pub struct ExportedFrom {
    pub app: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ExportedTarget>,
}

pub struct ExportedTarget {
    pub id: String,
    pub kind: String,
    pub label: String,
}
```

Keep `EXPORT_VERSION = 1` unless implementation proves a breaking schema change is required. Existing JSON with only `{ "app": "SkillPort" }` should deserialize.

### Service Signatures

Prefer small parameter objects over a wide argument list:

```rust
pub(crate) struct PortableStateTargetContext {
    pub id: String,
    pub kind: String,
    pub label: String,
}
```

Export can accept `Option<&PortableStateTargetContext>` to populate metadata. Preview does not need target context beyond the chosen DB.

Import needs target behavior:

- Keep the existing local import path for `ActiveTarget::Local`.
- Add a target-aware wrapper in the command or service layer that delegates:
  - Local: `import_skillport_state_impl(active_db, ...)`
  - Remote: a new remote variant that mirrors local grouping/source/tag behavior but calls `github_import::import_github_repo_skills_remote_with_auth(...)`.

Avoid duplicating conflict/resolution grouping logic. Reuse `ensure_github_sources`, `build_import_groups`, and `restore_skill_tags` where possible.

### Remote Import

Existing remote GitHub import requires:

- active target
- active DB
- repo URL
- selections
- optional preview workspace id
- app handle
- auth

Portable state import groups already contain `repo_url` and selections, so the remote variant can call:

```rust
github_import::import_github_repo_skills_remote_with_auth(
    pool,
    active_target,
    &group.repo_url,
    group.selections,
    None,
    app,
    auth.as_deref(),
)
```

Then restore tags into `active_db` exactly as local import does.

### Operation Logging

Replace `local_target_context()` with `target_context_from_active_target(&active_target)` in export, preview, and import. Details payloads can retain existing counts plus optional target fields only if useful.

## Frontend Shape

`useTargetStore` already exposes `activeTarget`. Pass target context through Central view bindings into `CentralSkillDialogs` and `CentralStatePortabilityDialog`, or let the dialog read the target store directly if that matches existing local patterns.

Dialog changes:

- Show the active target label/kind near the description.
- Use separate wording for export and import when possible:
  - Export tab: target is the source.
  - Import tab: target is the destination.
- Keep the existing export/preview/import buttons and JSON text areas.
- Optional warning if `manifest.exportedFrom.target` exists and differs from the current active target.

## Compatibility

- Old v1 JSON: accepted, with no origin target warning.
- New v1 JSON with optional `exportedFrom.target`: accepted by new app; older app may ignore unknown fields if serde structs allow it on their side, but do not rely on old app behavior.
- Active target changed while dialog is open: existing store resets on target switch. The dialog should either close/reset or update its displayed target and require a fresh export/preview.

## Risk Points

- Remote import must not accidentally call the local partial GitHub import path, or it will write files to local Central while updating remote DB inconsistently.
- `build_remote_catalog` currently inspects GitHub repositories from the local machine. That is acceptable because it only checks GitHub source availability, not target filesystem state.
- Target cache DB can be stale if the remote target was changed outside SkillPort. This is an existing target-cache issue; import/preview should behave consistently with other Central views and refresh flows.
- Progress cancellation is shared across export, preview, and import. Preserve the existing cancel checks when adding remote branching.

## Rollback

The implementation should be revertible by restoring:

- portable state command target routing
- portable state manifest optional metadata
- remote import wrapper
- dialog target-label changes
- related tests/i18n

No database migration should be needed.
