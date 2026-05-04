# SSH Remote

SSH Remote lets SkillPort manage skills on a remote Linux or macOS host without leaving the desktop. The UI still runs locally; the backend opens an SSH session to the chosen target and operates on the *remote user's* skill directories.

## What you get

- One-click switching between Local and any registered SSH target.
- Remote Central library at `~/.skillsmanage/skills/` and Universal Agents at `~/.agents/skills/` on that host.
- Per-target local cache database under `~/.skillsmanage/targets/<target_id>/db.sqlite`. Each target keeps its own scan results and metadata.
- Add, test, delete, and switch targets from Settings → Remote Targets.

## Authentication

| Method | Storage |
|--------|---------|
| Private key | Path stored locally; key contents never copied into SkillPort. |
| Password | Stored in the system credential store (Keychain / Credential Manager / libsecret), not in SQLite. |

The desktop UI never displays raw credentials after they are saved.

## What is supported in this version

- Scanning the remote user's Central and platform skill directories.
- Installing skills with **copy** mode (symlink mode is disabled for SSH targets in this version).
- Browsing skill detail and AI explanation against the remote skill content.

## What is not supported in this version

- Symlink-based install on remote targets.
- Remote Discover (project-level) scanning.
- File-manager actions are replaced by **copy remote path** because the path lives on the remote host, not the local machine.

## Switching back to Local

Use the target switcher in the top bar. Switching back restores the Local cache database and stops sending commands over SSH. Remote mode never modifies local skills, and Local mode never reaches the remote host.

## Common issues

- **Connection refused**: confirm the SSH port and that the remote `sshd` accepts the chosen authentication method.
- **HOME detection failed**: ensure the remote user has a writable home directory; SkillPort reads `$HOME` after login.
- **Permission denied on install**: the target user lacks write access to the platform directory; install with copy mode and confirm directory permissions.

## Where to go next

- Audit what runs in the desktop: see [Settings → Remote Targets](./settings).
- Combine with collections for one-click batch installs across machines: [Collections](./collections).

---

Last reviewed: 2026-05-04
