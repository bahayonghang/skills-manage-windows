# Rust checks — 2026-09-06

All Cargo commands were run serially.

| Check | Command | Exit | Result |
| --- | --- | ---: | --- |
| Format | `rtk cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | 0 | no diff |
| Clippy | `rtk cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings` | 0 | no issues |
| IPC generated map | `rtk cargo run --manifest-path src-tauri/Cargo.toml --features ipc-codegen --bin skillport --locked -- --ipc-codegen --check` | 0 | generated map checked |
| Locked test suite | `rtk cargo test --manifest-path src-tauri/Cargo.toml --locked` | 0 | 1553 passed, 7 ignored across 7 suites; 107.22 s |

These are current Windows-host results. They do not prove Linux/macOS, Windows installer, provider, SSH/WSL, updater-signing, or production behavior.
