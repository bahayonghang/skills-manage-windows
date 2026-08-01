# Desktop release process

SkillPort desktop releases are built, validated, and atomically published by the
canonical `Release Desktop` workflow at `.github/workflows/release-desktop.yml`.

## Canonical workflow

- Trigger: push an existing `v<semver>` tag for publish, or manually select `rehearsal` with an exact 40-character `rehearsal_ref` on `origin/main`.
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
6. Use manual `rehearsal` first. It validates the frozen SHA and retains a 14-day Actions artifact, but does not create or modify a GitHub Release. `publish` remains bound to an existing `v<semver>` tag.
7. Authenticode and the Tauri updater `.sig` are separate checks. Windows files are Authenticode-signed first, then the final NSIS bytes are signed and verified with the updater key. An updater `.sig` never proves Windows Authenticode.
8. Merge the release commit to `main`.
9. Create `v<version>` at that `main` commit and push the tag. For a retry,
   manually dispatch `Release Desktop` with the same existing tag.
10. Wait for frozen-context validation, reusable CI, every required platform
   build. The workflow verifies the complete artifact inventory, updater
   signature, metadata, and `SHA256SUMS` before creating or reusing a draft.
11. Confirm the atomically published release contains:
   - `latest.json`
   - `skillport_<version>_windows_x64_nsis.exe`
   - `skillport_<version>_windows_x64_nsis.exe.sig`
   - Windows MSI / ZIP assets
   - macOS and Linux install assets when those jobs pass
12. Fetch
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
- The release workflow injects the real updater public key but keeps automatic
  updater artifacts disabled. It Authenticode-signs EXE/NSIS/MSI first, then
  signs the final NSIS bytes and runs updater cryptographic preflight.
- Rehearsal may report `authenticode=not-configured`; publish fails closed unless
  Azure Artifact Signing produces timestamped valid Authenticode for EXE, NSIS,
  and MSI. Publish alone creates provenance attestations and verifies them after
  fresh download.
- The real previous-to-candidate updater smoke remains deferred until a staging
  feed is approved; follow [the staging runbook](updater-staging-runbook.md).
- Every build and the reusable CI gate use the same peeled tag SHA. Draft
  creation happens only after all required predecessors succeed, and the sole
  public transition is the final `draft=false` API update.
- Built-in app updates are Windows x64 only until release metadata includes
  macOS and Linux platform entries.
- The `/releases/latest/download/latest.json` endpoint assumes the latest
  GitHub release is a desktop release that includes `latest.json`.

Last reviewed: 2026-07-27
