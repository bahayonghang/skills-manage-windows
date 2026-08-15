# Security And Shared State

These rules apply when a change crosses the desktop app, the shared Central directory, persistence,
or the `skillport-cli` compatibility boundary.

## Central And Persistence

- Keep database schema, skill `uid` semantics, reference resolution, import/install services, and
  Central file mutations compatible across all entry points. A fix in only a Tauri command or only
  a CLI path is not sufficient.
- Put shared behavior in Rust services/repositories, use backward-compatible migrations, and do
  not regenerate persisted `uid` values.
- Reuse the existing Central mutation lock and installation/linker path, especially
  `ensure_centralized`. A Central migration retains target-only skills and does not delete the old
  directory.

## Credentials And Portable State

- Store GitHub PATs, AI API keys, SSH passwords, and private keys behind `SecretStore`.
- Prefer the operating-system credential store; on Windows the local fallback must be DPAPI
  protected. If protected persistence is unavailable, keep the secret in the current session only.
- Delete legacy plaintext only after protected storage write and read-back succeed.
- Keep logs, errors, telemetry, SQLite, and portable state exports redacted.

## Filesystem And Updater Boundaries

- Route heavy synchronous filesystem work in async Rust through the repository's blocking-FS
  wrapper; preserve progress/event emission on the async side.
- Windows updater `.sig` proves updater-key matching only; it does not replace Authenticode.
  Delivery order is Authenticode for EXE/NSIS/MSI, timestamp verification, signature generation
  from final NSIS, then metadata/checksum generation.
