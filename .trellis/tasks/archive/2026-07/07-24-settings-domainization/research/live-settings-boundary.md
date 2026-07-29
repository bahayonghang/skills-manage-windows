# Live Settings / Target Boundary Evidence

Date: 2026-07-27
Branch: `dev`

## Generic settings IPC

- `src-tauri/src/commands/settings.rs:17-27` only treats GitHub PAT, AI API key, and provider-scoped AI API keys as protected.
- `set_setting_impl` and `set_settings_impl` at lines 117-142 accept every other non-empty key.
- `src-tauri/src/db/repos/settings_repo.rs:81-96` already gives batch writes one SQLite transaction; validation must finish before calling it.
- Current operation logs at `commands/settings.rs:378-439` persist caller-provided keys. Values are not logged, but the task requirement is category-only metadata.

## Live renderer writers

- `src/stores/platformStore.ts:396-399`: `platform_category_visibility`.
- `src/stores/settingsStore.ts:232-235`: `central_update_check_mode_v1`.
- `src/lib/displayFont.ts:117-166,698`: exact legacy/themed display/body font keys plus `font_scale_v1`.
- `src/stores/settingsStore.aiSlice.ts:67-90,180-198,334,430`: global AI preferences and provider-scoped region/model/url/protocol preferences. API keys use dedicated commands.
- Scan directories and target CRUD use dedicated commands and do not need generic setter access.

## Target persistence and recovery gap

- `src-tauri/src/targets/model.rs:2-4`: SSH, WSL, and active target are ordinary settings keys.
- `src-tauri/src/targets/commands.rs:371-395`: dedicated active-target writes validate existence.
- `src-tauri/src/targets/commands.rs:418-453`: SSH/WSL loaders directly deserialize raw JSON; errors become `ParseRemoteTargets` / `ParseWslTargets`.
- `src-tauri/src/targets/registry.rs:332-364`: either domain parse error aborts the complete list.
- `src/components/layout/AppShell.tsx:72-81`: startup catches and discards `loadTargets` rejection.
- `src/stores/targetStore.ts:90-102`: store records the error, but there is no persisted quarantine status or Settings warning.

## Credential boundary

- `RemoteTargetConfig.password` is `serde(skip)`, while `protected_password` remains serialized for legacy Windows DPAPI fallback.
- Arbitrary/corrupt JSON may contain unknown plaintext secret-like fields because serde does not provide a safe redacted recovery artifact.
- Per `.trellis/spec/backend/redaction-policy.md`, controlled recovery stores must not leak secrets to logs, IPC, exports, or ordinary state. The safe persistent evidence is therefore irreversible metadata: domain, timestamp, byte length, SHA-256, and a stable reason code.

## Error / UI contracts

- `src/lib/backendError.ts:3-31` recognizes lowercase coded errors shaped as `code: message`.
- `src/pages/settingsPageSections.tsx` routes the connections page to `RemoteTargetsSettingsSection`.
- `src/components/settings/RemoteTargetsSettingsSection.tsx:145-160` already owns the top-of-section status area; the quarantine warning belongs there rather than in a separate card or global toast.

## Audit mapping

- `skills-manage-windows-extreme-review-2026-07-24.md:533-553` describes the generic target-key bypass.
- The audit acceptance at lines 873-880 requires generic target/migration/security writes to return `SETTING_KEY_FORBIDDEN`; current frontend parser requires the lowercase wire code `setting_key_forbidden`.
