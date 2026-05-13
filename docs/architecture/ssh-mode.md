# SSH Mode

`src-tauri/src/targets/` lets SkillPort drive a remote skills store the same way it drives the local one. Commands always go through the registry instead of branching on a `remote` flag.

## Targets

A target row encodes how SkillPort reaches a skills store:

| Field | Notes |
| --- | --- |
| `id` | UUID, stable across renames |
| `kind` | `local` or `ssh` |
| `host`, `port`, `user` | SSH only |
| `private_key_path` | Optional; falls back to ssh-agent |
| `password_ref` | Pointer into encrypted credential store |
| `remote_root` | Default `~/.skillsmanage` on the host |

`local` is the implicit fallback when no SSH target is active.

## Modules

```text
targets/
├── model.rs        persisted target rows
├── registry.rs     active target + cached remote DbPool
├── exec.rs         exec_local / exec_ssh; uniform stdin / stdout / stderr
├── cred.rs         encrypt + persist passwords
├── askpass.rs      SSH_ASKPASS helper used to feed passwords
├── commands.rs     IPC commands re-exported as commands::targets::*
└── tests.rs        round-trip tests across local + mocked SSH
```

## Active Target Resolution

```text
[any command] ──► AppState::active_target()
                          │
                          ├─ Local → returns local DbPool + local exec
                          └─ SSH   → opens / reuses sqlite pool over SSH
                                       remote ~/.skillsmanage/db.sqlite
```

The registry caches connections so back-to-back commands do not pay the SSH handshake cost. Cache invalidation happens when the user switches the active target via `set_active_target`.

## Exec Contract

All shell-level work (e.g., `ssh-keyscan` validation, remote `mkdir`, install fallbacks) goes through `targets::exec::run_command`:

- Local mode: `std::process::Command`.
- SSH mode: `ssh user@host -- 'cmd'` with the password supplied via `SSH_ASKPASS` (`askpass.rs`).

Services never call `Command::new("ssh")` directly. That keeps the SSH plumbing testable and lets us swap the transport later without rewriting business logic.

## Remote Installation

`services::installation::remote.rs` is the SSH-aware equivalent of `native.rs`:

1. Resolve the remote `global_skills_dir` for the agent.
2. Use `exec` to `mkdir -p` and `ln -s` (or `cp -r` when symlinks fail).
3. Update the remote SQLite pool's `skill_installations` row.
4. Mirror an entry in the local `operation_logs` so the UI shows the action.

## Failure Handling

- Connection failures surface as `Result::Err(String)` and show up in the Logs page tagged with `target_kind = 'ssh'`.
- Long-running operations emit progress events via the registry; the UI listens and updates the SSH banner.
- A transient failure does not flip the active target back to local; the user must explicitly switch.

## Test Strategy

`targets/tests.rs` runs both legs:

- Local exec uses a temp directory and asserts file system effects directly.
- SSH exec runs against a mock binary substituted on `PATH`; assertions cover argument shape rather than network behavior.

Last reviewed: 2026-05-04
