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
| Node.js | Node 22 LTS, pinned by `.node-version` and `package.json` |
| pnpm | pnpm 10.12.3, pinned by `package.json` |
| Rust | Rust 1.97.0 with `rustfmt` and `clippy`, pinned by `rust-toolchain.toml` |
| Tauri prerequisites | Install the system dependencies listed in the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) guide |

### Clone and run

```bash
git clone <your-fork-url>
cd skills-manage
pnpm install
pnpm tauri dev
```

The Vite dev server runs on port `24200` during local development.

After installing dependencies, run `just doctor` to inspect the local toolchain
and Windows Tauri prerequisites. The command is read-only: it does not install
packages, switch Rust, modify PATH, or print credentials.

## Validation before opening a pull request

Run the complete local gate before you submit a PR:

```bash
just doctor
just check
just ci
just audit
```

`just check` is the quick static/generated-artifact lane for development
feedback. It does not replace the complete `just ci` and `just audit` gates.
`just ci` runs the platform-independent common lane (read-only version and
generated-artifact checks, frontend validation/build, documentation, and Rust
entrypoint/format/IPC contracts) in parallel with the current platform's
all-target Clippy and locked Rust tests. It fails on version drift without
changing tracked files; use `just sync-version` only when you intend to update
version metadata. `just audit` checks production pnpm
high/critical advisories and Cargo vulnerabilities against the exact,
time-bounded exceptions in `security/dependency-audit-exceptions.json`.

For pull requests targeting `dev` or `main`, GitHub runs common, Windows Rust,
Linux Rust, macOS Rust, and supply-chain lanes independently. The stable
`just-ci` required check is an aggregate only and fails unless every required
lane succeeds. These hosted lanes use the same command plan as local `just ci`;
routine pull requests do not build installers, and package smoke remains a
direct-manual or release-workflow concern. CI is triggered by pull requests to
`dev` or `main`, not ordinary pushes.

When a change affects Tauri commands or `src-tauri/src/db/schema/`, refresh and
commit the generated documentation explicitly:

```bash
pnpm docs:gen
pnpm docs:gen:check
pnpm docs:build
```

The last two commands never update tracked files. A drift failure must be fixed
with `pnpm docs:gen`, then reviewed and committed with its authoritative source.

- Keep production source files under `src/` and `src-tauri/src/` at or below 800 lines.
- `pnpm sizecheck` enforces the 800-line limit uniformly; no production file has a per-file allowlist bypass.
- Components must not call Tauri `invoke()` directly. Route IPC through stores or service-layer helpers so UI code stays testable and platform boundaries remain explicit.

If your change touches UI behavior, include screenshots or a short screen recording in the pull request.
If your change touches packaging or release automation, also run `pnpm tauri build` on Windows and confirm the expected bundle exists.

## Branch and merge model

Keep `dev` as the long-lived day-to-day development branch. Short-lived task
branches target `dev`, use squash merge, and are deleted automatically after
merge. A `dev` -> `main` promotion pull request must use a merge commit so the
ancestry remains explicit. After each promotion, refresh and verify the exact
promotion merge SHA, then fast-forward `dev` to that SHA before writing Trellis
bookkeeping or starting another task. Do not delete or retire `dev`.

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
