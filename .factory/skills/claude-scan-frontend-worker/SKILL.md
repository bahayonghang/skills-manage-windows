---
name: claude-scan-frontend-worker
description: Implements source-aware Claude platform/detail UI and store behavior for marketplace read-only rows.
---

# Claude Scan Frontend Worker

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the WORK PROCEDURE.

## When to Use This Skill

Use for features that change:
- TypeScript types for source-aware Claude rows
- Zustand platform/detail stores consuming new backend payloads
- `PlatformView`, skill cards, drawers, and detail affordances
- read-only labeling and conflict presentation
- list/detail continuity after rescan or duplicate-row switching

## Required Skills

None.

## Work Procedure

1. Read `mission.md`, mission `AGENTS.md`, `.factory/library/architecture.md`, `.factory/library/claude-platform-scan.md`, `.factory/library/user-testing.md`, and any newly added mission library notes that document backend/frontend contracts for the feature you are repairing (for example source-aware detail API notes from prior backend work or scrutiny rounds) before changing code.
2. Write failing Vitest tests first for the exact UI/state contract you are changing. Use duplicate-source fixtures, not distinct-name shortcuts.
3. Keep one Claude platform surface. Do not introduce a second Claude platform entry or Discover-style escape hatch.
4. Make source origin explicit in the UI (`user` vs `marketplace`). Do not reuse install/link indicators as the source marker.
5. Marketplace rows must stay visibly read-only on both list and detail surfaces, and normal Claude user rows must remain manageable beside them.
6. When a feature claims the detail surface is source-aware, verify the full detail experience for duplicate Claude rows: metadata, markdown content, management affordances, and the AI Explanation tab / cached explanation state must all follow the selected row.
7. If duplicate rows exist for one logical skill, preserve row-specific identity through:
   - list rendering
   - detail open/close
   - focus restoration
   - rescan refreshes
8. For native Tauri validation, use the mission guidance plus `.factory/library/user-testing.md` as the canonical recipe for isolated HOME launches, screenshots, and known automation limits. Do not rediscover the native validation approach ad hoc unless the documented path fails.
9. Run verification commands from `.factory/services.yaml` relevant to your scope:
   - `test-frontend-targeted`
   - `typecheck`
   - `lint`
10. Perform a native Tauri check with the isolated mission HOME when your feature changes user-visible list/detail behavior. Browser-only verification is insufficient for final sign-off.
11. In the handoff, explicitly state whether marketplace rows are visibly read-only and whether duplicate-source switching updates the full detail surface, including any explanation-tab behavior touched by the feature.

## Example Handoff

```json
{
  "salientSummary": "Updated Claude platform and detail surfaces to render user vs marketplace source rows under one platform, keep marketplace rows visibly read-only, and preserve row-specific identity when switching duplicate rows. Targeted Vitest, typecheck, lint, and isolated-home Tauri checks passed.",
  "whatWasImplemented": "Extended frontend row types with source/read-only/conflict metadata, updated PlatformView list rendering and search to keep duplicate-source rows visible, and changed SkillDetailView so marketplace rows show rooted source paths plus blocked management actions while user rows retain normal controls.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      {
        "command": "pnpm exec vitest run src/test/platformStore.test.ts src/test/PlatformView.test.tsx src/test/SkillDetailView.test.tsx src/test/skillDetailStore.test.ts",
        "exitCode": 0,
        "observation": "Targeted store and UI tests passed, including duplicate-source list/detail cases."
      },
      {
        "command": "pnpm typecheck",
        "exitCode": 0,
        "observation": "No type errors."
      },
      {
        "command": "pnpm lint",
        "exitCode": 0,
        "observation": "No lint errors."
      }
    ],
    "interactiveChecks": [
      {
        "action": "Opened the Claude platform in a Tauri session with the isolated mission HOME",
        "observed": "User and marketplace rows appeared together under one Claude platform with visible source badges."
      },
      {
        "action": "Opened duplicate user and marketplace rows one after the other",
        "observed": "Detail metadata, content, and action affordances switched with the selected row; marketplace rows remained read-only."
      }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "src/test/PlatformView.test.tsx",
        "cases": [
          {
            "name": "renders duplicate Claude rows from user and marketplace sources",
            "verifies": "One Claude platform can show both sources simultaneously with origin markers."
          },
          {
            "name": "search keeps both duplicate-source rows visible",
            "verifies": "Filtering does not collapse or hide duplicate-source results."
          }
        ]
      },
      {
        "file": "src/test/SkillDetailView.test.tsx",
        "cases": [
          {
            "name": "marketplace rows show read-only detail state",
            "verifies": "Marketplace rows render source-rooted path plus blocked management affordances."
          },
          {
            "name": "switching duplicate rows updates the whole detail surface",
            "verifies": "Metadata, content, and management state follow the selected source row."
          }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- Backend payloads still cannot uniquely identify duplicate Claude rows from list through detail.
- The desired read-only UX conflicts with existing shared card/detail abstractions in a way that requires mission-level design direction.
- A required native Tauri validation path cannot be exercised with isolated fixtures or the real-home manual spot-check plan.
