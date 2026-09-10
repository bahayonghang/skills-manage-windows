# CLAUDE.md

@AGENTS.md

This file is a thin Claude Code entry. Grok also loads it, so it must not become a second
project contract. Durable rules live in `AGENTS.md`. Architecture inventories and command
counts belong in `code_map.md` and generated architecture docs, not here.

## Navigation

- [`code_map.md`](code_map.md) — repository search anchors
- [`docs/agents/build-and-test.md`](docs/agents/build-and-test.md) — commands and gates
- [`docs/agents/git-and-release.md`](docs/agents/git-and-release.md) — branches, PRs, release
- [`docs/agents/harness-guide.md`](docs/agents/harness-guide.md) — five-tool matrix, evidence layers, task handoff
- [`docs/agents/security-and-shared-state.md`](docs/agents/security-and-shared-state.md) — Central, credentials, updater
- [`.trellis/workflow.md`](.trellis/workflow.md) — Trellis phases, tasks, and skill routing
- [`.trellis/spec/`](.trellis/spec/) — package- and layer-scoped coding guidelines

Read the relevant `.trellis/spec/<layer>/index.md` before editing backend, frontend, or quality
code.

## Agent skills

Issues for this repo live in GitHub Issues and should be managed with the `gh` CLI; external PRs
are not a triage surface. See `docs/agents/issue-tracker.md`.

This repo uses the default five-label triage vocabulary. See `docs/agents/triage-labels.md`.

This repo uses a single-context domain-doc layout. See `docs/agents/domain.md`.
