---
name: claude-scan-backend-worker
description: Implements Rust/backend scan, storage, and query changes for Claude multi-source platform behavior.
---

# Claude Scan Backend Worker

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the WORK PROCEDURE.

## When to Use This Skill

Use for features that change:
- Claude root discovery and scan traversal
- source-aware observation storage/query behavior
- Tauri command payloads for platform/detail surfaces
- conflict/read-only metadata emitted by the backend
- rescan cleanup behavior for Claude rows

## Required Skills

None.

## Work Procedure

1. Read `mission.md`, mission `AGENTS.md`, `.factory/library/architecture.md`, `.factory/library/claude-platform-scan.md`, and `.factory/library/user-testing.md` before changing code.
2. Treat canonical/manageable skill state and observed Claude scan state as separate concerns. Do not make marketplace rows behave like installs.
3. Write failing Rust tests first for the exact feature scope. Prefer temp directories and explicit fixture trees for:
   - `~/.claude/skills`
   - `~/.claude/plugins/marketplaces/<name>`
   - duplicate logical skills across both roots
   - rescan stale-row cleanup
4. Implement the backend change with the smallest stable row identity that keeps duplicate Claude rows distinct from list through detail.
5. If a payload shape changes for frontend consumers, update only the backend contract for this feature; do not make speculative frontend edits, but you **must** run the mission frontend-targeted validators to catch caller compatibility breaks.
6. Run verification commands from `.factory/services.yaml` relevant to your scope:
   - `test-rust-targeted`
   - `clippy`
   - if any IPC/query payload consumed by the frontend changed: `test-frontend-targeted`, `typecheck`, and `lint`
   - any focused single-test commands needed during iteration
7. If your change affects live Tauri scan behavior, perform at least one isolated-home sanity check with `HOME=/tmp/skills-manage-test-fixtures/claude-multi-source` and record the observed outcome.
8. In the handoff, explicitly call out whether marketplace rows remain outside install/centralize/link semantics.

## Example Handoff

```json
{
  "salientSummary": "Added Claude multi-root scan discovery plus source-aware observation rows, keeping marketplace rows read-only and separate from canonical install semantics. Targeted Rust suites and clippy passed, and an isolated-home Tauri sanity check showed both Claude roots feeding one platform.",
  "whatWasImplemented": "Implemented Claude root fan-out for `~/.claude/skills` plus `~/.claude/plugins/marketplaces/*`, introduced source-aware row identity for platform/detail queries, and preserved existing canonical install semantics so marketplace rows do not populate linked/install state.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      {
        "command": "cargo test --manifest-path src-tauri/Cargo.toml scanner::tests::",
        "exitCode": 0,
        "observation": "Scanner tests passed including duplicate-source and multi-marketplace fixtures."
      },
      {
        "command": "cargo test --manifest-path src-tauri/Cargo.toml skills::tests::",
        "exitCode": 0,
        "observation": "Platform/detail query tests passed with source-aware payload assertions."
      },
      {
        "command": "cargo test --manifest-path src-tauri/Cargo.toml linker::tests::",
        "exitCode": 0,
        "observation": "Linker semantics remained unchanged for canonical installs."
      },
      {
        "command": "cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings",
        "exitCode": 0,
        "observation": "No warnings."
      },
      {
        "command": "pnpm exec vitest run src/test/platformStore.test.ts src/test/PlatformView.test.tsx src/test/SkillDetailView.test.tsx src/test/skillDetailStore.test.ts && pnpm typecheck && pnpm lint",
        "exitCode": 0,
        "observation": "Frontend-targeted tests, TypeScript typecheck, and lint all passed after the backend payload change."
      }
    ],
    "interactiveChecks": [
      {
        "action": "Ran a Tauri sanity check with HOME=/tmp/skills-manage-test-fixtures/claude-multi-source",
        "observed": "One Claude platform surfaced both user and marketplace source rows without mutating plugin fixture directories."
      }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "src-tauri/src/commands/scanner.rs",
        "cases": [
          {
            "name": "claude_scan_collects_user_and_marketplace_roots",
            "verifies": "Claude scan discovers both user and marketplace roots in one pass."
          },
          {
            "name": "claude_scan_keeps_duplicate_source_rows_distinct",
            "verifies": "Duplicate logical skills from different Claude roots remain separate observation rows."
          }
        ]
      },
      {
        "file": "src-tauri/src/commands/skills.rs",
        "cases": [
          {
            "name": "get_skills_by_agent_returns_source_metadata_for_claude_rows",
            "verifies": "Platform/detail payloads carry source kind, rooted path, and read-only/conflict metadata."
          }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- You cannot keep duplicate Claude rows distinct without changing mission-level data-model assumptions.
- The cleanest backend solution would require broad install/canonical semantics changes outside this mission.
- Real Claude scan behavior depends on an unresolved path/source decision beyond `marketplaces/*`.
