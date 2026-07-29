# Live Research: Release Pipeline Gate

## Repository State

- Branch is `dev` at `75950b3d`; HEAD is not in `origin/main`, so planning and local tests must not treat the current branch as a releasable SHA.
- Latest local release tag is `v0.10.14` at `61c544a4`. The current task performs no tag creation, push, draft creation, or publication.
- `.github/workflows/release-desktop.yml:3-5` still starts from `release.published`. Its three build jobs have no reusable-CI dependency, and `publish` at lines 315-344 uploads with `softprops/action-gh-release` after the release is already public.
- `.github/workflows/ci.yml:3-10` still includes `release.published`; package smoke guards at lines 69, 113, and 185 are coupled to `release` or manual dispatch.

## Current Release Contracts

- Windows assets are named `skillport_<version>_windows_x64_nsis.exe`, its `.sig`, `skillport_<version>_windows_x64.msi`, `skillport_<version>_windows_x64.zip`, and `latest.json`.
- macOS universal assets are DMG, ZIP, and TAR.GZ. Linux emits DEB, RPM, and AppImage for x64 and optional arm64.
- `scripts/release-preflight.mjs:69-119` sorts matching NSIS files and selects the last one, then checks only pubkey placeholder state, signature presence/text equality, version, URL, and platform keys. It does not reject duplicate/unexpected assets or verify installer bytes cryptographically.
- `scripts/generate-latest-json.mjs:8-9` and workflow asset preparation still fall back to `GITHUB_REF_NAME`; manual dispatch therefore needs a frozen explicit tag/version/SHA context.
- `src/test/contracts/ciWorkflowContract.test.ts:30-64` freezes the current release event and package guards. It must be updated losslessly while preserving the `just-ci` job name.
- English release docs still say to publish a GitHub Release first, while Chinese docs already claim tag push. README validation text still describes published-release CI/smoke behavior. Both languages need one state-machine contract.

## Signature Verifier Feasibility

- `src-tauri/Cargo.lock` contains `minisign-verify 0.2.5` and `base64 0.22.1` through `tauri-plugin-updater 2.10.1`; neither is a direct dependency of the `skillport` package today.
- The locked updater implementation base64-decodes the configured public key and release signature into text, calls `PublicKey::decode` and `Signature::decode`, then calls `public_key.verify(data, &signature, true)`.
- The local `minisign-verify 0.2.5` source exposes those APIs. A release-only binary can therefore mirror runtime verification without introducing another crypto implementation or a new version family.
- Adding a third Rust binary must retain `default-run = "skillport"`, the existing desktop/CLI entrypoints, and Tauri's `mainBinaryName`. `pnpm entrypointcheck`, all-target Clippy/tests, locked verifier tests, and Windows bundle verification remain required.

## Implementation Boundaries

- Reusable CI owns only `just-ci`; release workflow owns the formal platform matrix and signing secrets. Manual CI smoke packaging remains a separate operator check.
- Release jobs checkout the peeled tag SHA. Release context rejects tags outside `origin/main` history and version mismatch before any signing build.
- Required builds finish before draft creation. A reused same-tag draft is reset and remains draft on every failure before the sole final `draft=false` transition.
- Local contract/unit tests mock or model GitHub release state. This task does not claim a real remote draft/publish rehearsal without separate authorization.
