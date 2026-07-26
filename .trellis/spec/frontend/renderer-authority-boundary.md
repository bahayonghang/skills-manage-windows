# Renderer Authority Boundary

## 1. Scope / Trigger

Apply this contract when a frontend feature adds a Tauri plugin import, changes
`src-tauri/capabilities/default.json`, handles a user-selected local file, installs
a Marketplace preview, or reads/writes a saved credential.

## 2. Signatures

```rust
#[tauri::command]
pub async fn preview_skillport_state_import_file(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<SkillportStateImportFilePreview, String>;

#[tauri::command]
pub async fn save_skillport_state_export(
    path: String,
    json: String,
) -> Result<(), String>;
```

```ts
installGitHubPreviewSkill(repoUrl: string, sourcePath: string): Promise<GitHubRepoImportResult>;

interface SecretValueInputProps {
  value: string;
  configured: boolean;
  onChange(value: string): void;
}
```

## 3. Contracts

- The main WebView has no filesystem permission or `@tauri-apps/plugin-fs`
  dependency. File dialogs return paths only; backend services own all reads and
  writes.
- Portable-state reads require a regular `.json` file, enforce the shared file
  budget before and during allocation, reject growth after the opened-handle
  metadata snapshot, and decode UTF-8 before parsing. Writes validate the
  manifest and budget before using a same-directory temporary file plus atomic
  persist.
- Direct GitHub preview installation submits only `repoUrl` and `sourcePath` to
  a store action. That action performs a fresh backend preview and imports the
  matching candidate through the existing GitHub staging, target snapshot,
  Central mutation lock, and database persistence path. The renderer never
  downloads or writes the skill.
- Saved PATs and AI API keys are write-only from the renderer. Production code
  has no `reveal_github_pat` or `reveal_ai_api_key` command, store action, or UI
  control. Configured/fingerprint status, overwrite, clear, and connection tests
  remain supported.
- `docs/reference/ipc-capability-inventory.md` contains the exact marker JSON
  contract. `pnpm capabilitycheck` compares it with capability permissions,
  frontend imports/dependencies, Rust dependencies/initializers, and the
  deterministically rendered human table. It is a required `just ci` step.

## 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Selected import is not `.json` or not a regular file | Backend returns a semantic portable-state error; renderer performs no read |
| Declared or streamed bytes exceed the shared budget | Reject before parsing; do not allocate past cap plus one |
| File grows after metadata snapshot | Reject as changed or over-budget; do not preview partial content |
| Export payload is invalid or atomic persist fails | Preserve the existing destination and clean the temporary file |
| GitHub candidate disappeared before install | Discard the fresh preview workspace and perform no import |
| Saved secret is configured | Render a fixed mask with an empty password input; never return plaintext |
| Capability/import/dependency/inventory set drifts | `pnpm capabilitycheck` and therefore `just ci` fail |

## 5. Good / Base / Bad Cases

- Good: renderer selects `state.json`; backend reads it with the shared budget,
  returns `{ json, preview }`, and the existing editor/import flow continues.
- Base: an HTTP(S) external link still uses `shell:allow-open` after
  `externalUrl.ts` rejects every other scheme.
- Bad: a component imports `plugin-fs`, a page fetches `downloadUrl` and writes
  Central directly, or an IPC command returns stored secret plaintext.

## 6. Tests Required

- Rust file-adapter tests: extension, missing/non-file, metadata cap, growth
  within/over cap, UTF-8, invalid manifest, atomic overwrite, persist failure,
  and temporary cleanup.
- Frontend tests: path-only portability IPC, direct GitHub and registry install
  routing, disappeared candidate cleanup, fixed-mask secret UI, and absence of
  reveal commands in production entrypoints.
- Drift tests: valid repository plus missing permission, stale JSON, stale
  rendered table, unexpected import, and stale dependency/initializer fixtures.
- Full gate: `just ci`; on Windows capability/plugin changes also run
  `pnpm tauri build` and verify a newly generated NSIS installer.

## 7. Wrong vs Correct

```ts
// Wrong: renderer chooses network authority and writes Central directly.
const content = await fetch(skill.downloadUrl).then((response) => response.text());
await writeTextFile(`.skillsmanage/skills/${skill.name}/SKILL.md`, content);

// Correct: renderer submits structured identity to the existing backend flow.
await installGitHubPreviewSkill(skill.repoUrl, skill.sourcePath);
```
