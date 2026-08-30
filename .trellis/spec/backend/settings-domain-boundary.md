# Settings Domain Boundary and Target Quarantine Contract

## 1. Scope / Trigger

Apply this contract when adding a generic settings write, changing persisted SSH/WSL target configuration, or exposing target-recovery state over IPC. The generic settings commands are a renderer preference boundary, not an unrestricted wrapper around the `settings` table. Target, secret, migration, feature-gate, and recovery metadata remain owned by their domain services.

## 2. Signatures

```rust
async fn set_setting_impl(pool: &DbPool, key: &str, value: &str) -> Result<(), String>;
async fn set_settings_impl(
    pool: &DbPool,
    values: &HashMap<String, String>,
) -> Result<(), String>;

async fn load_target_config_snapshot(
    local_db: &DbPool,
) -> Result<TargetConfigSnapshot, TargetsError>;

#[tauri::command]
async fn get_target_config_quarantine_status(
    state: State<'_, AppState>,
) -> Result<TargetConfigQuarantineStatus, String>;
```

`db::set_setting` and `db::set_settings` stay policy-free repository primitives. Only domain code may use them for target, secret, migration, or quarantine keys.

## 3. Contracts

- Generic writes use the explicit allowlist in `commands/settings_policy.rs`: platform category visibility, Central update mode, the exact font key families, non-secret AI preferences, and the exact Skills CLI key `skills_cli.recent_sources` (`SettingCategory::SkillsCli`).
- A batch validates every key and value before making one transactional `db::set_settings` call.
- Settings operation logs contain only category set, key count, status, and `valueStored`; they never contain caller keys or values.
- The target snapshot reads `ssh_targets_v1`, `wsl_targets_v1`, `active_target_id_v1`, and `target_config_quarantine_v1` together. SSH and WSL validate independently and use all-or-nothing recovery per domain.
- Target deletion persists the SSH list, WSL list, and Local fallback in one settings transaction before removing the credential or cached remote pool. A credential-store failure restores the exact prior settings snapshot, including absent keys, restores any session password, retains the remote pool, and returns the credential error.
- `TargetConfigQuarantineStatus` is version 1 and contains only `domain`, RFC 3339 UTC `detectedAt`, stable `reasonCode`, `sourceBytes`, `sourceSha256`, and `activeTargetReset`.
- The frontend command map types `get_target_config_quarantine_status`; `targetStore.loadTargets()` loads targets and status independently so status failure does not discard a valid target list. UI text must not render the backend status-read error.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Unknown, target, secret, migration, feature, or quarantine key | `setting_key_forbidden`; no DB write; no caller key/value in error or log |
| Allowed key with invalid enum, range, JSON shape, URL, or control character | `setting_value_invalid`; no DB write |
| Any invalid member in a batch | Zero settings from that batch are written |
| SSH or WSL JSON/shape invalid | Clear only that domain to `[]`; keep the other domain |
| Empty, duplicate, or reserved `local` target ID | Quarantine the affected domain with a stable reason code |
| Active target absent after validation | Persist `active_target_id_v1=local` in the same recovery transaction |
| Recovery transaction fails | Preserve every original setting and return `TargetsError` |
| Target deletion settings write or credential cleanup fails | Preserve target lists, active target, credential availability, and cached remote pool; retry may complete after the injected failure is removed |
| Quarantine metadata is malformed or untrusted | Return the empty default status; never forward its free text |
| Explicit `target_by_id` misses | Return `TargetNotFound`; only missing active selection falls back to Local |

## 5. Good / Base / Bad Cases

- Good: corrupt SSH containing credential-like fields becomes `[]`, healthy WSL remains listed, active selection becomes Local, and the status exposes only bytes plus SHA-256.
- Base: healthy or absent target arrays load unchanged and `list_targets` contains Local plus all valid targets.
- Bad: widening the generic boundary to an `ai_*` prefix, logging an unknown key, salvaging individual rows from a corrupt domain, or persisting the original target JSON for recovery.

## 6. Tests Required

- Policy unit tests enumerate every live renderer key family and reject target, secret, migration, feature, quarantine, and unknown keys.
- Batch tests assert validation completes before persistence and verify zero writes after one invalid member.
- Target tests cover symmetric SSH/WSL isolation, healthy-domain preservation, duplicate/reserved IDs, legacy `credentialKey` and `protectedPassword`, Local fallback, repeat digest stability, malicious metadata, and transaction rollback.
- Target deletion tests inject each settings-write failure plus credential-store failure, compare the full persisted snapshot and credential/pool ownership, and prove successful retry.
- Registry tests assert missing active selection falls back to Local while explicit missing IDs remain `TargetNotFound`.
- Frontend tests assert target/status independent loading, persistent warning rendering, bilingual text, typed IPC coverage, and no raw error or target JSON in the DOM.
- Minimum closeout gate is focused Rust/Vitest coverage, `pnpm typecheck`, `pnpm lint`, `just ci`, task validation, and `git diff --check`.

## 7. Wrong vs Correct

```rust
// Wrong: lets renderer callers overwrite domain-owned state.
db::set_setting(pool, caller_key, caller_value).await?;

// Correct: validate the renderer preference boundary before the repository call.
validate_setting(caller_key, caller_value)?;
db::set_setting(pool, caller_key, caller_value).await?;
```

```rust
// Wrong: one parse failure disables every target and may leak parser text.
let ssh = serde_json::from_str(&ssh_raw)?;
let wsl = serde_json::from_str(&wsl_raw)?;

// Correct: validate each domain independently, store only stable evidence, and
// commit domain clearing, status, and Local fallback in one transaction.
let snapshot = load_target_config_snapshot(local_db).await?;
```
