# Final Cross-Child Acceptance

Date: 2026-08-01

## Baseline

- Parent: `08-01-engineering-delivery-workflow-optimization`
- Final promotion SHA: `d68387bffdd2f9e0b9f05d978ed925976913ef42`
- `origin/dev` and `origin/main`: `d68387bffdd2f9e0b9f05d978ed925976913ef42`
- `gh-pages`: absent from remote refs and local remote-tracking refs
- Archived children: docs, CI, developer/PR, desktop release assurance

## Delivery Chain

| Slice | PR / commit | Hosted evidence |
| --- | --- | --- |
| Desktop implementation | PR #33, squash `f4dadb798acf0bdd22f82818379144de9eefe7eb` | `30693986360` passed required lanes |
| Desktop rehearsal prerequisite repair | PR #36, squash `6c01884f1a779753b748f7649c7d3349dce2af38` | `30697970762` passed; promotion PR #37 checks `30698241307` |
| macOS asset staging compatibility fix | PR #38, squash `82c6c550e96d7d2d83b27f8ce97d8180072c9f94` | exact-head checks `30700333767` passed |
| Final promotion | PR #39, merge `d68387bffdd2f9e0b9f05d978ed925976913ef42` | exact-head checks `30700638444` passed; `dev` fast-forwarded without force |

## Rehearsal Evidence

The first rehearsal, run `30698509277` at old SHA `fe8d869baafe53e3209d0454b7143d125e80e4f0`, failed only in macOS asset preparation because the runner's Bash 3.2 has no `mapfile`; Linux arm64 also exposed a missing `xdg-mime` dependency. Windows smoke passed. The workflow fix added Bash 3.2-compatible `nullglob` arrays and the Linux `xdg-utils` dependency. The original failure is retained as regression evidence.

Final rehearsal run `30700955460` was manually dispatched from canonical `main` with:

- `mode=rehearsal`
- `rehearsal_ref=d68387bffdd2f9e0b9f05d978ed925976913ef42`
- `run_updater_staging_smoke=false`

All quality, common, Rust, supply-chain, and `just-ci` lanes passed. The following real platform-sensitive work passed:

- Windows MSI smoke package
- Linux x86_64 and arm64 smoke packages
- macOS universal smoke package
- Windows x64 bundle
- Linux x86_64 and arm64 bundles
- macOS universal app and DMG bundle, including `Prepare macOS assets`
- Windows install, launch, and uninstall smoke
- `Verify release artifacts` checksum, `latest.json`, signature inventory, and validation artifact

The downloaded `release-validation.json` recorded SHA `d68387b`, version `0.10.14`, 15 assets (`latest.json`, `SHA256SUMS`, Windows NSIS/MSI/ZIP and `.sig`, Linux x86_64/arm64 DEB/RPM/AppImage, and macOS universal DMG/TAR.GZ/ZIP), `authenticode=not-configured`, and `updaterSignature=valid`. The downloaded `windows-signing.json` recorded `NotSigned` for `skillport.exe`, NSIS, and MSI, with no signer or timestamp, matching the unsigned rehearsal contract.

The `Publish verified draft` and `Deferred updater staging smoke` jobs were skipped. No GitHub Release was created, no tag was moved, no Azure signing variables or secrets were written, and no public release was performed.

## Local Final Gates

- `just ci`: passed; 1607 frontend tests passed, 1 skipped; Rust 1031 passed, 6 ignored.
- `just audit`: passed; 2 blocking advisories and 2 approved exceptions.
- Focused CI, release, and docs workflow contract tests: passed (18 tests across 3 files).
- `pnpm docs:gen:check`, `pnpm docs:build`, `pnpm version:check`, `pnpm typecheck`, `pnpm lint`, `git diff --check`: passed.
- Windows `pnpm tauri build`: passed; NSIS and MSI bundles were present locally, with Authenticode `NotSigned` and updater signing state kept separate.

## Deferred Boundaries

Azure Artifact Signing, `desktop-release` production environment configuration, updater staging feed smoke, public GitHub Release, tag movement, and release credentials remain intentionally deferred. The workflow fails closed for publish without those separately authorized settings; this does not invalidate the successful non-public rehearsal.
