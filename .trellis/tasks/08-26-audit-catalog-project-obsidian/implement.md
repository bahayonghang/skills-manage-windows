# Catalog/Project/Obsidian 覆盖实施计划

## Steps

1. Generate the owned policy slice, document operation/runtime-only/delegated decisions, and migrate owned commands to
   the Runtime child's named IPC boundary.
2. Migrate metadata/collection/view/group/agent short mutations to terminal-only audit.
3. Migrate AI jobs, collection batch install/import and project/Obsidian file operations to lifecycle audit.
4. Add reviewed domain error mappings and safe typed result details; remove raw error logging.
5. Add adversarial privacy, duplicate-row and unchanged-order/transaction regression tests.

## Validation

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::central_metadata
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::collections
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::saved_views
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::tag_groups
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::agents
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::projects
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::obsidian
cargo fmt --all -- --check
git diff --check
```

## Rollback

Revert one owned domain at a time behind the core compatibility adapter. Do not revert metadata/project/vault mutations or
delete generated user operation history.
