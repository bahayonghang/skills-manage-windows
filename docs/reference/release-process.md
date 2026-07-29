# Desktop release process

SkillPort desktop releases are built, validated, and atomically published by the
canonical `Release Desktop` workflow at `.github/workflows/release-desktop.yml`.

## Canonical workflow

- Trigger: push an existing `v<semver>` tag, or manually select an existing tag.
- Quality gate: reusable `just-ci` runs against the tag's peeled commit SHA.
- Release body source: `scripts/prepare-release-body.mjs`.
- Updater metadata source: `scripts/generate-latest-json.mjs`.
- Required Windows updater secrets:
  - `TAURI_UPDATER_PUBLIC_KEY`
  - `TAURI_SIGNING_PRIVATE_KEY`
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

Do not add another release workflow for the same desktop assets. If the release
flow changes, update `release-desktop.yml` and these scripts together so Windows
signing and `latest.json` stay in sync.

## Release checklist

1. Bump `package.json`.
2. Run `node scripts/sync-version.mjs`.
3. Verify the version fields in:
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/Cargo.lock`
4. Add release notes at `release-notes/<version>.md`, or a series fallback at
   `release-notes/<major>.<minor>.md`.
5. Run the local gates:
   - `pnpm typecheck`
   - `pnpm lint`
   - `pnpm test`
   - `pnpm sizecheck`
   - `cargo test --manifest-path src-tauri/Cargo.toml`
   - `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`
6. After the Windows release-only config and `release-assets/` exist, run:
   - `pnpm release:preflight -- --version <version> --tag v<version> --config release-updater-config.json --asset-dir release-assets`
   - This validates the updater configuration and `latest.json`, then
     cryptographically verifies the NSIS bytes with the updater public key.
7. Merge the release commit to `main`.
8. Create `v<version>` at that `main` commit and push the tag. For a retry,
   manually dispatch `Release Desktop` with the same existing tag.
9. Wait for frozen-context validation, reusable CI, and every required platform
   build. The workflow verifies the complete artifact inventory, updater
   signature, metadata, and `SHA256SUMS` before creating or reusing a draft.
10. Confirm the atomically published release contains:
   - `latest.json`
   - `skillport_<version>_windows_x64_nsis.exe`
   - `skillport_<version>_windows_x64_nsis.exe.sig`
   - Windows MSI / ZIP assets
   - macOS and Linux install assets when those jobs pass
11. Fetch
    `https://github.com/bahayonghang/skills-manage-windows/releases/latest/download/latest.json`
    and confirm it has the expected version, Windows URL, and signature.

If upload or post-upload verification fails, the release remains a private
draft. Fix the cause and rerun with the same tag; the workflow resets stale
draft assets before upload. Do not manually publish that draft before the fresh
download checksum verification succeeds. A public release with the same tag is
rejected rather than overwritten.

## Updater invariants

- The Tauri config in `src-tauri/tauri.conf.json` intentionally keeps
  `bundle.createUpdaterArtifacts` disabled and stores a placeholder updater
  public key for local builds.
- The release workflow must inject the real updater public key and enable
  updater artifacts for Windows builds, then pass cryptographic preflight.
- Every build and the reusable CI gate use the same peeled tag SHA. Draft
  creation happens only after all required predecessors succeed, and the sole
  public transition is the final `draft=false` API update.
- Built-in app updates are Windows x64 only until release metadata includes
  macOS and Linux platform entries.
- The `/releases/latest/download/latest.json` endpoint assumes the latest
  GitHub release is a desktop release that includes `latest.json`.

Last reviewed: 2026-07-27
