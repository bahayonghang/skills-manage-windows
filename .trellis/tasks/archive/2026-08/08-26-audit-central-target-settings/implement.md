# Central/Target/Settings 覆盖实施计划

## Steps

1. 从 core policy 生成本 child command 清单，逐项标注 owner/delegation/lifecycle/status matrix，并把 owned
   command迁移到Runtime child提供的命名IPC boundary。
2. 迁移 target/settings/log-admin/startup 的短 operation，并锁 secret/path adversarial tests。
3. 迁移 scan/sync 与 Central install/delete/update/recovery 长 operation，消除内外层重复 logger。
4. 为缺少 stable code 的已知 domain variants补 reviewed mapper，不解析 Display。
5. 补 clear/export/rebuild/exit 的安全 admin audit与 best-effort fallback。
6. 运行 focused domain suites并对 Operation action snapshot查重。

## Validation

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::targets
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::settings
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::scanner
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::skill_update_inventory
cargo test --manifest-path src-tauri/Cargo.toml --locked services::central_updates
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::logs
cargo test --manifest-path src-tauri/Cargo.toml --locked commands::startup
cargo fmt --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
git diff --check
```

## Rollback

Revert per command family while retaining core adapter. Never roll back journal/target mutation state or delete logs; a
logging regression must not trigger business-data recovery.
