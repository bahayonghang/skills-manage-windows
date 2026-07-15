# Shared Local CLI Contract

## 1. Scope / Trigger

Apply this contract when changing `skillport-cli`, `cli_api`, CLI-visible service signatures, or Tauri packaging in a package with multiple Rust binaries. The MVP manages only the Local target.

## 2. Signatures

```rust
pub struct CliContext { /* DbPool + SecretStore + ActiveTarget::Local */ }

pub fn parse_install_source(value: &str) -> Result<InstallSource, CliApiError>;
pub async fn sync_skills(refs, all, agents, method, dry_run) -> Result<SyncOutput, CliApiError>;
```

Development and installation entry points:

```powershell
npm run cli -- <args>
cargo install --path src-tauri --bin skillport-cli --locked --force
```

## 3. Contracts

- `src/bin/skillport-cli.rs` owns Clap, human/JSON rendering, and process exit codes only.
- `cli_api` calls existing DB repositories and service use cases; it never calls `commands::*` or performs skill file placement directly.
- Skill refs resolve only against Central rows in order: exact uid, exact slug/id, unique case-sensitive name.
- Source classification is deterministic: github.com URL or `owner/repo@skill`; filesystem existence is not a classifier.
- JSON uses `schemaVersion: 1`, locale-neutral error codes, stdout for success, and stderr for errors/diagnostics.
- CLI mutations use the shared Central lock through existing services and write existing redacted operation logs.

## 4. Validation & Error Matrix

| Condition | Code / exit |
| --- | --- |
| Invalid source, method, or sync scope | `input.invalid` / 2 |
| Missing, ambiguous, or duplicate skill | skill code / 3 |
| Shared mutation lock busy/timeout | `mutation.busy` / 4 |
| Mixed batch result | success payload / 5 |
| Internal service or DB error | `internal.error` / 1 |
| Success | payload / 0 |

## 5. Good / Base / Bad Cases

- Good: `install --sync` imports through GitHub/skills.sh services, then passes imported ids to the installation batch service.
- Base: `skills sync <ref> --dry-run` resolves Central skills and agents but writes no DB or filesystem state.
- Bad: copying Tauri command loops or invoking `commands::marketplace` from the binary.

## 6. Tests Required

- Parser table for shorthand, GitHub URL, unsupported URL, and local path rejection.
- Stable uid/slug/name list-show tests plus non-Central name-shadow coverage.
- JSON envelope and exit-code tests in the binary target.
- Temporary HOME binary smoke test with an empty Local DB.
- `pnpm entrypointcheck`, `just ci`, locked release CLI build, and Windows `pnpm tauri build`.
- Verify both `target/release/skillport.exe` and `target/release/skillport-cli.exe` exist and run their expected entry points.

## 7. Wrong vs Correct

```toml
# Wrong: multiple Cargo bins without an explicit default application target.
[package]
name = "skillport"

# Correct: Cargo and Tauri both resolve the desktop application as the main bin.
[package]
name = "skillport"
default-run = "skillport"
```

`mainBinaryName` only overrides the selected app binary's output filename. It does not
select a Cargo target and must not replace the `package.default-run` contract.
