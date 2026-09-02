# Research: Tauri Windows bundle-phase evidence (R1/R2)

- **Query**: Hard R1/R2 gate — with Node 26, pnpm 10.34.5, and lockfile `@tauri-apps/cli`, record actual `--help` capabilities and a production-credential-free rehearsal that either proves compile/bundle segmentation plus bundler input path+digest identity, or fails closed.
- **Scope**: mixed (local pinned CLI + lockfile + one disposable Windows rehearsal; version-matched upstream `bundle.rs` only to explain an observed log line)
- **Date**: 2026-09-02
- **Gate result**: **FAIL**
- **Workflow edits authorized**: **NO** — `.github/workflows/release-desktop.yml` must stay unmodified after this pass.

Authenticode trust, NSIS/MSI inner `skillport.exe` signature status, and production publish remain **UNVERIFIED**. This file does not claim them.

## Findings

### Toolchain and lockfile

| Check | Result |
|---|---|
| `node --version` | `v26.7.0` (satisfies 26.x) |
| `pnpm --version` | `10.34.5` |
| `package.json` `@tauri-apps/cli` | specifier `^2.11.4` |
| `pnpm-lock.yaml` importer | `specifier: ^2.11.4` → `version: 2.11.4` (devDependencies) |
| `pnpm-lock.yaml` package | `@tauri-apps/cli@2.11.4` integrity `sha512-R8xGtMpwyetawSqm9kYOuMmEqkhUbvcUy8n0aNXIxollKBLESUu5f4Fx+64hgASYm1H+jSWq6jCW6zqTnH6hqQ==` |
| host native optional | `@tauri-apps/cli-win32-x64-msvc@2.11.4` integrity `sha512-+vDiqBIU5dMISg/wNvX3sF+ZHfgJGJ5T0AcO+EHNXV9GGAG+P5fzodlDXD3QdKCRgZxMoCm5PPvj3BqLNjBthw==` |
| `pnpm exec tauri --version` | `tauri-cli 2.11.4` |
| Host `rustc -vV` (rehearsal machine) | `rustc 1.98.0 (88d9e12ae 2026-08-18)`, host `x86_64-pc-windows-msvc` |

Installed JS wrapper: `node_modules/@tauri-apps/cli/package.json` `"version": "2.11.4"`. Native binary: `node_modules/.pnpm/@tauri-apps+cli-win32-x64-msvc@2.11.4/.../cli.win32-x64-msvc.node`.

### Files Found

| File Path | Description |
|---|---|
| `package.json` | CLI specifier `^2.11.4` |
| `pnpm-lock.yaml` | Resolved `@tauri-apps/cli@2.11.4` and win32-x64 native optional |
| `src-tauri/tauri.conf.json` | `mainBinaryName: skillport`; `beforeBuildCommand: pnpm build`; `beforeBundleCommand: pnpm build:cli`; `bundle.createUpdaterArtifacts: false` |
| `package.json` script `build:cli` | `cargo build --manifest-path src-tauri/Cargo.toml --release --bin skillport-cli --locked` (does not target `skillport.exe`) |
| `.github/workflows/release-desktop.yml` | Current Windows job still runs a single `pnpm tauri build --target x86_64-pc-windows-msvc --bundles nsis,msi ...` (read-only this pass) |
| `node_modules/@tauri-apps/cli/config.schema.json` | Installed schema `$id` `https://schema.tauri.app/config/2.11.3`; `mainBinaryName` text says `tauri build` renames the cargo binary and `tauri bundle` targets it |

### Printed CLI capabilities (verbatim from this install)

`pnpm exec tauri --help` commands (complete command list as printed):

```
Commands:
  init         Initialize a Tauri project in an existing directory
  dev          Run your app in development mode
  build        Build your app in release mode and generate bundles and installers
  bundle       Generate bundles and installers for your app (already built by `tauri build`)
  android      Android commands
  migrate      Migrate from v1 to v2
  info         Show a concise list of information about the environment, Rust, Node.js and their versions as well as a few relevant project configurations
  add          Add a tauri plugin to the project
  remove       Remove a tauri plugin from the project
  plugin       Manage or create Tauri plugins
  icon         Generate various icons for all major platforms
  signer       Generate signing keys for Tauri updater or sign files
  completions  Generate Tauri CLI shell completions for Bash, Zsh, PowerShell or Fish
  permission   Manage or create permissions for your app or plugin
  capability   Manage or create capabilities for your app
  inspect      Inspect values used by Tauri
  help         Print this message or the help of the given subcommand(s)
```

`pnpm exec tauri build --help` — flags actually printed (no others used or assumed):

| Flag | Printed meaning |
|---|---|
| `-r, --runner <RUNNER>` | Binary to build the application, defaults to `cargo` |
| `-v, --verbose...` | Verbose logging |
| `-d, --debug` | Builds with the debug flag |
| `-t, --target <TARGET>` | Target triple |
| `-f, --features [<FEATURES>...]` | Cargo features |
| `-b, --bundles [<BUNDLES>...]` | `msi` or `nsis` |
| `--no-bundle` | Skip the bundling step even if `bundle > active` is `true` |
| `-c, --config <CONFIG>` | Merge extra JSON/JSON5/TOML config |
| `--ci` | Skip prompting (`CI=`) |
| `--skip-stapling` | macOS notarization staple skip |
| `--ignore-version-mismatches` | Do not error on Tauri package version mismatch |
| `--no-sign` | Skip code signing when bundling the app |
| `[ARGS]...` | Arguments passed to the runner; `--` marks the start |

`build` long description as printed: it runs `build.beforeBuildCommand`, then compiles, and also runs `build.beforeBundleCommand` before generating bundles.

`pnpm exec tauri bundle --help` — flags actually printed:

| Flag | Printed meaning |
|---|---|
| `-d, --debug` | Builds with the debug flag |
| `-v, --verbose...` | Verbose logging |
| `-b, --bundles [<BUNDLES>...]` | `msi` or `nsis` |
| `-c, --config <CONFIG>` | Merge extra config |
| `-f, --features [<FEATURES>...]` | Must match features passed to `tauri build` if any |
| `-t, --target <TARGET>` | Target triple |
| `--ci` | Skip prompting |
| `--skip-stapling` | macOS staple skip |
| `--no-sign` | Skip code signing during the build or bundling process |

Printed `bundle` about text: **Generate bundles and installers for your app (already built by `tauri build`)**. It runs `build.beforeBundleCommand`.

**Not printed by this CLI (therefore not used):**

- `--dry-run` (absent on `tauri`, `tauri build`, and `tauri bundle`)
- any `--binary-path` / `--exe` / input-digest / `--skip-patch` / compile-only named stage other than `--no-bundle`
- `tauri bundle` has **no** `[ARGS]...` cargo-passthrough in its help (unlike `tauri build`)

`pnpm exec tauri inspect --help` only offers `wix-upgrade-code`. It does not print binary-path inspection.

`pnpm exec tauri signer sign --help` signs a file with **updater** minisign keys (`TAURI_SIGNING_PRIVATE_KEY` / path / password). That is not Authenticode. Rehearsal did not set those variables and did not run `signer sign`.

### Code Patterns

Installed schema (`node_modules/@tauri-apps/cli/config.schema.json`) describes `mainBinaryName` as renaming cargo output during `tauri build` and targeting that name from `tauri bundle`. That is path-shape documentation, not digest identity.

Version-matched upstream `crates/tauri-bundler/src/bundle.rs` at tag `@tauri-apps/cli-v2.11.4` (same 2.11.4 as `pnpm exec tauri --version`) matches the rehearsal log `Patching ... with bundle type information: nsis`:

- `bundle_project` copies the main exe, then for each package type calls `patch_binary` (in-place equal-length replace of `__TAURI_BUNDLE_TYPE_VAR_UNK` → `__TAURI_BUNDLE_TYPE_VAR_NSS` or `_MSI`), optionally Windows-signs **after** that patch when `windows().can_sign()`, then invokes the NSIS/MSI bundler, then **restores** the copied original bytes.
- Comments in that file state that patching a signed binary without updating the PE checksum can break signature verification, and that (re)signing should happen after every `patch_binary()`.
- Help text does not mention this patch or expose a skip.

This explains why a predecessor Authenticode digest cannot be shown to be the digest NSIS `File` copies, even when the **path** is the same file.

### Rehearsal (no production credentials)

Environment: no `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `TAURI_SIGNING_PRIVATE_KEY_PATH`, `TAURI_UPDATER_PUBLIC_KEY`, or Azure client/tenant/secret in the process environment. `--no-sign` and `--ci` were passed because they were printed. No updater private key was generated.

Profile: `-d` (printed) so the compile reused `src-tauri/target/debug` incremental artifacts. This is a debug-profile identity rehearsal, not a release-profile or Authenticode rehearsal. `bundle.rs` patch/restore is not gated on debug vs release in the observed log or the version-matched source.

Commands actually executed (repo root):

```powershell
pnpm exec tauri build --no-bundle --no-sign --ci -d -vv
# then overlay-append a unique ASCII marker onto skillport.exe (not a Tauri flag)
pnpm exec tauri bundle --no-sign --ci -d --bundles nsis -vv
```

`--target` was not passed; the CLI still reported `TAURI_ENV_TARGET_TRIPLE=x86_64-pc-windows-msvc`. A custom `CARGO_TARGET_DIR` was not required: `tauri bundle` help has no cargo `--target-dir` passthrough, so both steps used the default `src-tauri/target`.

#### Independent app exe (compile without installer)

`tauri build --no-bundle` exited 0. Relevant log lines:

```
Warn [tauri_cli::build] --no-sign flag detected: Signing will be skipped.
...
Finished `dev` profile [unoptimized] target(s) in 1m 27s
Built [tauri_cli::build] application at: D:\Documents\Code\Agents\skills-manage-windows\src-tauri\target\debug\skillport.exe
```

No NSIS/MSI output was produced by this step.

| Field | Value |
|---|---|
| Path | `D:\Documents\Code\Agents\skills-manage-windows\src-tauri\target\debug\skillport.exe` |
| Size after compile | `55714816` bytes |
| SHA-256 after compile | `29D780039809390F9BBB0FE042A483CD754EF13CE7CFD4AF06FFD92E8A540C9C` |

#### Distinguishable digest (local overlay, not Authenticode)

ASCII overlay appended (not a signing certificate): `SKILLPORT-R1-REHEARSAL-MARKER-126b2506c8df4b04b0344dbb1e5ca136` (62 bytes).

| Field | Value |
|---|---|
| Size after overlay | `55714878` bytes |
| SHA-256 after overlay | `5E5FD6EBF06481BD31C3F8DB9C992765DC0D5851B01EA7157614C54AD14E4994` |
| Digests equal to compile output? | **No** (distinguished) |

This overlay is **not** Authenticode. It only makes the predecessor bytes unique.

#### Bundler consumption

`tauri bundle --no-sign --ci -d --bundles nsis -vv` exited 0.

`beforeBundleCommand` ran `pnpm build:cli` → `cargo build ... --release --bin skillport-cli --locked` (`Finished release profile` in ~4m30s). That is a **different** binary (`skillport-cli`), not a rebuild of debug `skillport.exe`.

Then the bundler logged:

```
Info [tauri_bundler::bundle] Patching D:\Documents\Code\Agents\skills-manage-windows\src-tauri\target\debug\skillport.exe with bundle type information: nsis
```

Generated `installer.nsi`:

```
!define MAINBINARYSRCPATH "D:\Documents\Code\Agents\skills-manage-windows\src-tauri\target\debug\skillport.exe"
...
File "${MAINBINARYSRCPATH}"
```

makensis `-V4` recorded:

```
File: "skillport.exe" 55714878 bytes
```

That size equals the **post-overlay** size (compile size + 62), not the pre-overlay size. Path identity and size identity of the distinguished file are therefore observed.

Installer output:

| Field | Value |
|---|---|
| Path | `D:\Documents\Code\Agents\skills-manage-windows\src-tauri\target\debug\bundle\nsis\SkillPort_1.0.2_x64-setup.exe` |
| Size | `13680675` bytes |
| SHA-256 | `582CB51BCA9EA03B54B1F07646398CC9BFDA4240359C689FE28DCB876DF4B08B` |
| Compression (makensis) | `lzma (compress whole)` |
| Plaintext overlay marker inside setup exe | **False** (expected under solid LZMA; not used as inner-exe proof) |

After bundle returned, on-disk `skillport.exe`:

| Field | Value |
|---|---|
| Size | `55714878` |
| SHA-256 | `5E5FD6EBF06481BD31C3F8DB9C992765DC0D5851B01EA7157614C54AD14E4994` |
| Matches post-overlay digest? | **True** |
| Overlay still at PE tail? | **Yes** |

So the bundler **did not leave** a rebuilt debug `skillport.exe` on disk. Combined with the patch log and version-matched `bundle.rs`, the on-disk file was patched, packaged, then **restored** to the pre-patch (post-overlay) bytes.

#### What this does *not* prove

- The SHA-256 of the bytes NSIS `File` copied is **not** equal to the predecessor overlay digest: `patch_binary` runs in-process after the copy-for-restore and before `File`. Help prints no flag to skip that patch or to pin an input digest.
- Inner `skillport.exe` was **not** extracted from the LZMA NSIS payload. Inner-exe digest and Authenticode of that payload stay **UNVERIFIED**.
- MSI was **not** rehearsed. Dual `nsis,msi` would patch the same path twice with different tokens in the version-matched source; that was not executed here.
- `--no-sign` on `bundle` produced no extra "signing skipped" line in the bundle log (unlike `tauri build`). Signing certs/`can_sign` were not configured. Internal Authenticode remains **UNVERIFIED**.
- Release-profile `target/release/skillport.exe` was not produced. Segmentation was shown on `-d` only.

### Related Specs

- `.trellis/tasks/09-02-windows-release-signing/prd.md` — R1 CLI research gate; R2 fail closed if segmentation + bundler input identity are not uniquely proven; R8 UNVERIFIED boundary for real certs / inner signatures / publish.
- `.trellis/tasks/09-02-windows-release-signing/design.md` — research artifact is the hard gate before any workflow edit; do not invent Tauri flags; absence of dry-run must be recorded.
- `.trellis/spec/` — not consulted beyond task docs; this pass writes research only.

## Gate decision

**FAIL** (R1 AC2 digest identity not uniquely proven; R2 fail closed).

Proven:

1. Pinned toolchain and lockfile CLI 2.11.4 `--help` / `--version`.
2. Compile can be separated from packaging via printed `--no-bundle` plus printed `tauri bundle`.
3. Bundler **path** for NSIS is the compile output exe (`MAINBINARYSRCPATH`).
4. Bundler saw the distinguished **size** (`55714878`).
5. On-disk exe digest after `tauri bundle` matched the distinguished digest (restore).

Missing proof (do not fill with guessed flags):

1. **No printed dry-run.**
2. **No printed flag** to skip in-process `patch_binary` or to make the bundler consume a caller-frozen SHA-256 unchanged.
3. **Packaged-bytes digest** of the NSIS payload exe was not obtained and, given the observed patch, cannot be claimed equal to the predecessor SHA-256 `5E5F…4994`.
4. Therefore AC2 (“bundler reads the same path **and** the same digest as the Authenticode predecessor output”) is **not** satisfied.

**Workflow edits are not authorized.** Desired R3 order `compile → Authenticode app exe → bundle` is a contract goal, not something this rehearsal uniquely proved the installed CLI can execute as two digest-identical steps.

## Caveats / Not Found

- `--dry-run`: not printed; not run.
- Release-mode (`target/release`) rehearsal: not run.
- `--target x86_64-pc-windows-msvc` as used by `.github/workflows/release-desktop.yml`: not passed; host triple already matched.
- `--bundles msi` / WiX: not run.
- Azure Artifact Signing, Authenticode, timestamp, NSIS/MSI **inner** exe signature, updater `.sig`, `latest.json` publish: **UNVERIFIED** (not attempted; no production secrets).
- Large disposable outputs (`SkillPort_1.0.2_x64-setup.exe` and generated `target/debug/nsis/`) are deleted after this capture; hashes above are the retained evidence.
- Overlay bytes on the local debug `skillport.exe` are truncated back to the compile size after hashing so the working tree binary is not left mutated.
