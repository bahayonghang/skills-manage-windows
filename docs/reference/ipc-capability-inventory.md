# IPC and Capability Inventory

> Scope: S-02 first tightening for the main Tauri window. This inventory is based on current frontend imports and `src-tauri/capabilities/default.json`.

## Baseline risk

- `src-tauri/tauri.conf.json` previously set `app.security.csp` to `null`.
- `src-tauri/capabilities/default.json` previously granted broad plugin defaults, including `fs:default`, `shell:default`, `sql:default`, `dialog:default`, and `updater:default`.
- The backend still registers `tauri_plugin_sql`, `tauri_plugin_fs`, `tauri_plugin_dialog`, `tauri_plugin_shell`, `tauri_plugin_process`, and `tauri_plugin_updater` in `src-tauri/src/lib.rs`; renderer access is now constrained by this capability file.

## Frontend plugin usage

| Plugin | Current frontend usage | Required permissions after S-02 | Notes |
|---|---|---|---|
| `@tauri-apps/plugin-dialog` | Directory pickers in `CentralStoreLocationDialog.tsx` and `ProjectPathPicker.tsx`; JSON import/export open/save in `CentralStatePortabilityDialog.tsx` | `dialog:allow-open`, `dialog:allow-save` | `dialog:default` was replaced with command-specific permissions. |
| `@tauri-apps/plugin-fs` | JSON export/import read/write in `CentralStatePortabilityDialog.tsx`; legacy Marketplace preview install writes `~/.skillsmanage/skills/<skill>/SKILL.md` in `MarketplaceView.tsx` | `fs:allow-mkdir`, `fs:allow-read-text-file`, `fs:allow-write-text-file`, plus explicit `fs:scope` | The scope keeps current Windows home/common-user-folder flows working while removing the broader default permission set. |
| `@tauri-apps/plugin-updater` | `check()` and `downloadAndInstall(...)` in `appUpdateStore.ts` | `updater:allow-check`, `updater:allow-download-and-install` | `updater:default` was replaced with the specific commands the app calls. |
| `@tauri-apps/plugin-process` | `relaunch()` after installing an update in `appUpdateStore.ts` | `process:allow-restart` | Existing specific permission kept. |
| `@tauri-apps/plugin-shell` | No current frontend imports found | none | `shell:default` removed. File manager opening goes through guarded backend commands. |
| `@tauri-apps/plugin-sql` | No current frontend imports found | none | `sql:default` removed. Database access stays behind backend commands. |

## CSP baseline

The first production CSP baseline is:

```text
default-src 'self';
script-src 'self';
style-src 'self' 'unsafe-inline';
img-src 'self' asset: https: data: blob:;
font-src 'self' data:;
connect-src 'self' ipc: http://ipc.localhost http://localhost:* ws://localhost:* https:;
object-src 'none';
base-uri 'self';
frame-ancestors 'none'
```

Rationale:

- `style-src 'unsafe-inline'` is retained for the current Tailwind/runtime style path and should be revisited only after a rendered smoke test.
- `img-src` allows local app assets, Tauri `asset:` URLs, remote skill/repository images, and data/blob previews.
- `connect-src` remains broad for HTTPS in this first round because the renderer still downloads Marketplace preview files with `fetch(...)` and provider/GitHub endpoints can be user-configured or mirrored. Backend GitHub/AI requests are not controlled by renderer CSP, but the renderer-side Marketplace path still needs HTTPS.
- Tauri IPC/dev support is kept with `ipc:`, `http://ipc.localhost`, `http://localhost:*`, and `ws://localhost:*`.

## Current capability after first tightening

The main window now keeps only:

- `core:default`
- `core:window:allow-show`
- `dialog:allow-open`
- `dialog:allow-save`
- `fs:allow-mkdir`
- `fs:allow-read-text-file`
- `fs:allow-write-text-file`
- explicit `fs:scope` for home/common user export and legacy Marketplace preview install paths
- `updater:allow-check`
- `updater:allow-download-and-install`
- `process:allow-restart`

The capability no longer grants:

- `fs:default`
- `shell:default`
- `sql:default`
- `dialog:default`
- `updater:default`

## Follow-up hardening

1. Move Marketplace preview installation out of direct `plugin-fs` calls and into a backend command that validates skill IDs, normalizes target paths, and reuses the Central skill install/import path.
2. Move state import/export file reads and writes behind backend commands that accept dialog-selected paths, enforce `.json`, apply size caps, and report clear user-facing errors.
3. Replace broad `connect-src https:` with an allowlist once all renderer-side downloads are routed through backend commands or a typed registry of allowed sources.
4. Add a command/capability drift check so future plugin imports fail CI unless this inventory and `default.json` are updated together.
