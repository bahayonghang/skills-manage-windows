# Renderer 权限最小化与 capability drift check - Design

## 1. Design Goal

主 WebView 不再拥有任意文件读写或已保存 secret 明文读取能力，同时保留现有 portability、Marketplace preview 安装、HTTP(S) 外链、更新与重启 UX。所有新增文件 IO、网络下载和 Central mutation 都进入已有 backend domain/service 边界。

## 2. Trust Boundary

```text
Renderer
  |-- dialog open/save -> selected local path only
  |-- typed store IPC -> portable_state / github_import commands
  |-- shell open -> HTTP(S)-validated URL only
  `-- no plugin-fs, no saved-secret plaintext command

Commands (IPC string errors only)
  |-- portable_state service -> bounded local file adapter
  |-- github_import service -> preview/import/staging/mutation lock
  `-- settings state -> configured/fingerprint only
```

The renderer remains responsible for UI state and dialog choice. It is not trusted to read/write a selected path, construct a Central target path, download install content, or retrieve stored credentials.

## 3. Capability And Plugin Contract

`src-tauri/capabilities/default.json` keeps only the plugin commands used by the main window:

- `dialog:allow-open` and `dialog:allow-save` for path selection;
- `shell:allow-open` for `src/lib/externalUrl.ts`, whose URL parser permits HTTP(S) only;
- updater check/install and process restart permissions;
- existing core/window permissions.

Remove every fs permission and `fs:scope`, remove the JavaScript `plugin-fs` dependency, remove the Rust `tauri-plugin-fs` dependency, and remove `tauri_plugin_fs::init()`. Keep `plugin-shell`; replacing `shell:default` must not break valid HTTP(S) external links.

## 4. Portable State File Adapter

Add a file adapter inside `services/portable_state` rather than in the command module.

### 4.1 Read path

`preview_skillport_state_import_file(path)` performs these steps through `run_blocking_fs_with`:

1. require a case-insensitive `.json` extension;
2. open the selected path read-only and inspect metadata from the opened handle;
3. require a regular file;
4. apply `ResourceBudget::default_skill().file_bytes` before allocation;
5. read at most `limit + 1` bytes and reject growth beyond the metadata snapshot;
6. decode UTF-8 and pass the same string into the existing manifest parser/preview flow.

The command returns `{ json, preview }` so the existing raw/pretty editor and later import flow continue to work without a renderer file read.

### 4.2 Write path

`save_skillport_state_export(path, json)` validates `.json`, parses the payload through the existing manifest parser, enforces the same byte budget, then writes with `tempfile::NamedTempFile` in the destination directory, flushes/syncs, and persists to the selected path. Failure leaves the original destination intact and the temporary file is cleaned up by RAII.

`PortableStateError` gains semantic variants for invalid extension, non-regular input, UTF-8, resource budget, IO context, and blocking task join. Commands remain the only `Result<T, String>` boundary.

## 5. Marketplace Preview Install

Do not add another downloader or Central writer.

`MarketplacePreviewSkill` becomes a discriminated frontend type:

- registry-backed entries keep the cached marketplace skill ID and call existing `install_marketplace_skill`;
- direct GitHub preview entries keep `sourcePath` and call a store action that performs a fresh `preview_github_repo_import`, confirms the selected source still exists, and immediately calls `import_github_repo_skills` with one overwrite selection and the returned workspace ID when available.

This removes page-level `fetch`, `mkdir`, and `writeTextFile` while reusing GitHub URL policy, archive/tree budgets, staging, Central mutation locking, target snapshot resolution, and DB persistence. Exact immutable commit binding remains owned by `07-24-github-preview-snapshot`; this child does not create a competing token format.

## 6. Secret UX And IPC Removal

Delete both reveal commands from command modules and `generate_handler!`, delete their service helpers/exports/tests, remove store actions and binding props, and remove both command names from the IPC allowlist/fixtures.

`SecretValueInput` becomes a write-only password input:

- configured-but-unchanged state renders a fixed mask/hint, never a real value;
- new input stays `type=password` and has no Eye control;
- overwrite, clear, configured/storage status, fingerprint display, and connection tests remain;
- reveal-specific loading, errors, i18n keys, and tests are removed or replaced with assertions that no reveal control/command exists.

SecretStore reads remain available only to backend operations that need the credential for authenticated requests or connection tests.

## 7. Required Drift Gate

Add `scripts/check-capability-drift.mjs` and a `capabilitycheck` package script, then insert it in the web chain of `scripts/run-ci.mjs`.

The checker:

1. parses `default.json` as JSON;
2. parses all `src/**/*.ts(x)` imports with the TypeScript compiler API;
3. parses `package.json` and `cargo metadata --no-deps --locked --format-version 1` for plugin dependencies;
4. checks the Rust entrypoint for registered plugin initializers;
5. reads a marker-delimited JSON contract embedded in `ipc-capability-inventory.md` and compares exact plugin/permission/dependency sets;
6. renders the inventory's marker-delimited human-readable table block from that same JSON with a deterministic shared renderer and requires an exact text match.

The JSON contract is the only hand-edited machine source of truth. The table renderer is shared by check and update modes so documentation does not acquire a second authority surface. Contract tests exercise both the valid repository state and negative fixtures for a missing permission, stale JSON entry, stale rendered table, unexpected plugin import, and stale plugin dependency/registration. The check is required immediately, not warning-only.

## 8. Compatibility And Rollback

- No database migration or persisted-secret rewrite occurs.
- Portability JSON format and paste/edit/import behavior remain compatible.
- Removing reveal is an intentional internal IPC breaking change approved by the user; no external protocol is advertised.
- Marketplace preview may perform a fresh backend preview immediately before import; this is an accepted temporary behavior until immutable snapshot binding is implemented in its dedicated child.
- Rollback is the local code commit revert. No persisted state rollback is needed.

## 9. Validation Mapping

| Risk | Evidence required |
|---|---|
| Renderer regains fs authority | capability drift negative fixtures; no `plugin-fs` import/dependency/initializer; capability JSON assertion |
| Portability reads unbounded files | metadata and `limit + 1` tests; non-file/extension/UTF-8 tests |
| Export corrupts an existing file | atomic replacement and injected/persist failure tests; temp cleanup assertion |
| Marketplace bypasses Central invariants | store routing tests plus existing GitHub import service tests |
| Stored secret reaches renderer | command-map/handler grep contract and settings UI/store tests |
| External links regress | `externalUrl` HTTP(S)/unsafe-scheme tests and Tauri build |
| Platform/package drift | `just ci`, Windows `pnpm tauri build`, and NSIS artifact existence |
