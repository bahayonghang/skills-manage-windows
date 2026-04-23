# Environment

Environment variables, external dependencies, and setup notes for the current mission.

## Required Tools
- Node.js / pnpm
- Rust / Cargo
- Tauri CLI from project devDependencies

## Key Paths
- Project root: `/Users/happypeet/Documents/GitHubMe/skills-manage`
- Mission fixture home: `/tmp/skills-manage-test-fixtures/claude-multi-source`
- Real Claude user root: `~/.claude/skills`
- Real Claude marketplace roots: `~/.claude/plugins/marketplaces/*`
- Central skills root: `~/.agents/skills`

## Mission Boundaries
- Do not mutate anything under `~/.claude/plugins/marketplaces/*`
- Do not expand scope into Discover scanning or Claude cache roots
- Treat real Claude directories as read-only validation inputs
- Prefer isolated fixture homes for automated validation; use the real `~/.claude/...` tree only for final manual spot-checks

## Ports
- Avoid `5000`, `7000`, and `3010`
- Default Tauri dev URL uses `24200`
- If `24200` is occupied, prefer `24202-24210`

## Validation Notes
- Full `pnpm test` is not the mission baseline because the repo already carries unrelated frontend failures outside this mission scope.
- Use targeted Vitest commands from `.factory/services.yaml` for worker and validator runs.
