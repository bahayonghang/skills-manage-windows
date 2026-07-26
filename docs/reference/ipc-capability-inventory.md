# IPC and Capability Inventory

The main WebView has no direct filesystem permission and no saved-secret plaintext command. User-selected portability paths cross typed IPC into the portable-state service, and Marketplace preview installs reuse the Marketplace or GitHub import service boundary.

External links remain available through `@tauri-apps/plugin-shell`, but `src/lib/externalUrl.ts` accepts only parsed HTTP(S) URLs and the main capability grants only `shell:allow-open`.

The JSON block below is the machine-owned capability contract. Edit it when an intentional plugin or permission change lands, then run `pnpm capabilitycheck -- --update` to regenerate the human-readable table.

<!-- capability-contract:start -->
```json
{
  "capabilityPermissions": [
    "core:default",
    "core:window:allow-show",
    "dialog:allow-open",
    "dialog:allow-save",
    "process:allow-restart",
    "shell:allow-open",
    "updater:allow-check",
    "updater:allow-download-and-install"
  ],
  "frontendPluginImports": [
    "@tauri-apps/plugin-dialog",
    "@tauri-apps/plugin-process",
    "@tauri-apps/plugin-shell",
    "@tauri-apps/plugin-updater"
  ],
  "frontendPluginDependencies": [
    "@tauri-apps/plugin-dialog",
    "@tauri-apps/plugin-process",
    "@tauri-apps/plugin-shell",
    "@tauri-apps/plugin-sql",
    "@tauri-apps/plugin-updater"
  ],
  "rustPluginDependencies": [
    "tauri-plugin-deep-link",
    "tauri-plugin-dialog",
    "tauri-plugin-process",
    "tauri-plugin-shell",
    "tauri-plugin-single-instance",
    "tauri-plugin-sql",
    "tauri-plugin-updater"
  ],
  "rustPluginInitializers": [
    "tauri-plugin-deep-link",
    "tauri-plugin-dialog",
    "tauri-plugin-process",
    "tauri-plugin-shell",
    "tauri-plugin-single-instance",
    "tauri-plugin-sql",
    "tauri-plugin-updater"
  ]
}
```
<!-- capability-contract:end -->

<!-- capability-table:start -->
| Surface | Exact values |
|---|---|
| Main-window permissions | `core:default`<br>`core:window:allow-show`<br>`dialog:allow-open`<br>`dialog:allow-save`<br>`process:allow-restart`<br>`shell:allow-open`<br>`updater:allow-check`<br>`updater:allow-download-and-install` |
| Frontend plugin imports | `@tauri-apps/plugin-dialog`<br>`@tauri-apps/plugin-process`<br>`@tauri-apps/plugin-shell`<br>`@tauri-apps/plugin-updater` |
| Frontend plugin dependencies | `@tauri-apps/plugin-dialog`<br>`@tauri-apps/plugin-process`<br>`@tauri-apps/plugin-shell`<br>`@tauri-apps/plugin-sql`<br>`@tauri-apps/plugin-updater` |
| Rust plugin dependencies | `tauri-plugin-deep-link`<br>`tauri-plugin-dialog`<br>`tauri-plugin-process`<br>`tauri-plugin-shell`<br>`tauri-plugin-single-instance`<br>`tauri-plugin-sql`<br>`tauri-plugin-updater` |
| Rust plugin initializers | `tauri-plugin-deep-link`<br>`tauri-plugin-dialog`<br>`tauri-plugin-process`<br>`tauri-plugin-shell`<br>`tauri-plugin-single-instance`<br>`tauri-plugin-sql`<br>`tauri-plugin-updater` |
<!-- capability-table:end -->

`@tauri-apps/plugin-sql` remains installed for compatibility, but no renderer module imports it and the main-window capability grants no SQL command. Deep-link and single-instance plugins are backend lifecycle integrations and do not require renderer permissions.
