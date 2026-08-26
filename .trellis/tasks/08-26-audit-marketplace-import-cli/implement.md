# Marketplace/Import/CLI 覆盖实施计划

## Steps

1. Generate owned policy slice; mark operation/runtime-only/excluded/delegated commands and migrate owned commands to the
   Runtime child's named IPC boundary.
2. Migrate registry/credential/probe short operations with privacy fixtures.
3. Migrate Marketplace/GitHub/local archive/portable-state/Skills CLI long jobs to lifecycle audit.
4. Remove nested duplicate recorders and raw Display details; preserve stable domain codes.
5. Add cross-domain adversarial absence, batch/job/cancel and no-operation-on-successful-preview tests.

## Validation

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::marketplace
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::github_import
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::local_archive_import
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::portable_state
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::skills_cli
cargo test --manifest-path src-tauri/Cargo.toml --locked services::github_import
cargo fmt --all -- --check
git diff --check
```

## Rollback

Revert one command family behind the core adapter. Do not delete imported skills, portable state files, registries or user
logs; remote/provider validation remains `UNVERIFIED` unless separately exercised.
