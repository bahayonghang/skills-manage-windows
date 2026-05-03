# Contributing to skills-manage

Thanks for your interest in improving `skills-manage`.

## Before you start

- Use English for documentation, comments, commit messages, and pull request descriptions whenever practical.
- Keep changes focused. Small, reviewable pull requests are easier to merge and maintain.
- For large features, behavior changes, or new dependencies, open an issue first so the direction can be discussed before implementation work starts.

## Development setup

### Prerequisites

| Tool | Notes |
|------|-------|
| Node.js | Current LTS release |
| pnpm | Package manager used by this repository |
| Rust | Stable toolchain |
| Tauri prerequisites | Install the system dependencies listed in the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) guide |

### Clone and run

```bash
git clone <your-fork-url>
cd skills-manage
pnpm install
pnpm tauri dev
```

The Vite dev server runs on port `24200` during local development.

## Validation before opening a pull request

Run the relevant checks locally before you submit a PR:

```bash
pnpm test
pnpm sizecheck
pnpm typecheck
pnpm lint
cd src-tauri && cargo test
cd src-tauri && cargo clippy -- -D warnings
```

- Keep production source files under `src/` and `src-tauri/src/` at or below 800 lines whenever practical.
- The repository currently carries a small frozen allowlist of oversized production files; `pnpm sizecheck` fails if a new file crosses 800 lines or if an allowlisted file grows further.
- Components must not call Tauri `invoke()` directly. Route IPC through stores or service-layer helpers so UI code stays testable and platform boundaries remain explicit.

If your change touches UI behavior, include screenshots or a short screen recording in the pull request.

## Pull request guidelines

- Describe the user-facing problem and the approach you took to solve it.
- Mention any tradeoffs, follow-up work, or known limitations.
- Add or update tests when behavior changes.
- Do not mix unrelated refactors with feature work or bug fixes.
- Never include real credentials, tokens, or private keys in code, screenshots, issues, or pull requests.

## Reporting bugs

When filing a bug report, include:

- Your operating system and version
- The app version or commit you tested
- Clear reproduction steps
- Expected behavior and actual behavior
- Relevant logs or screenshots

For security vulnerabilities, do not open a public issue. Follow [SECURITY.md](SECURITY.md) instead.

## License

By contributing to this repository, you agree that your contributions will be licensed under the Apache License 2.0. See [LICENSE](LICENSE).
