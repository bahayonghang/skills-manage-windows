# User Testing

Testing surface, required tools, and validation constraints for the Claude multi-source scan mission.

## Validation Surface

### Primary surface: native Tauri app
- Run via `pnpm tauri dev`
- Validate the `Claude Code` platform page and the skill detail surface
- This is the authoritative user-facing surface because scan behavior depends on Rust/Tauri state

### Secondary automated surface: targeted tests
- Rust targeted tests for scanner/query/linker behavior
- Vitest targeted tests for `PlatformView`, `SkillDetailView`, `platformStore`, and `skillDetailStore`
- For the source-tab follow-up, extend `PlatformView`-focused tests to prove `全部 / 用户来源 / 市场来源` switching plus tab-scoped search composition
- `pnpm typecheck` and `pnpm lint`

## Fixture Strategy

### Automated validation
Use an isolated HOME rooted at:
- `/tmp/skills-manage-test-fixtures/claude-multi-source`

Populate fixture inputs under:
- `/tmp/skills-manage-test-fixtures/claude-multi-source/.claude/skills`
- `/tmp/skills-manage-test-fixtures/claude-multi-source/.claude/plugins/marketplaces/<publisher-or-plugin>`

Automated validation should prove:
- one Claude platform can roll up both roots
- duplicate source rows remain distinct
- marketplace rows are read-only
- rescans update the live page without stale row corruption

### Final manual validation
After automated work is complete, run a real-home Tauri check using the user's actual:
- `~/.claude/skills`
- `~/.claude/plugins/marketplaces/*`

That manual pass should confirm the same behaviors against real Claude data without mutating plugin directories.

## Validation Commands

### Frontend targeted
- `pnpm exec vitest run src/test/platformStore.test.ts src/test/PlatformView.test.tsx src/test/SkillDetailView.test.tsx src/test/skillDetailStore.test.ts`
- `pnpm typecheck`
- `pnpm lint`

### Rust targeted
- `cargo test --manifest-path src-tauri/Cargo.toml scanner::tests::`
- `cargo test --manifest-path src-tauri/Cargo.toml skills::tests::`
- `cargo test --manifest-path src-tauri/Cargo.toml linker::tests::`

## Validation Concurrency

Machine profile from dry run:
- 12 CPU cores
- 48 GiB RAM
- significant unrelated background Node/process load already present

Mission limit:
- **Maximum 2 validation lanes total**
- At most **1 frontend lane + 1 Rust lane**
- Serialize heavy checks and native Tauri validation

## Required Evidence

For contract assertions on the native Tauri surface, capture:
- sidebar/platform screenshots
- list screenshots with source/read-only markers
- source-tab screenshots for `全部`, `用户来源`, and `市场来源`
- detail screenshots with rooted path and action affordances
- before/after screenshots for rescan flows
- brief notes for any disabled/blocked management action checks

## Important Constraints
- Browser-only Vite runs are not sufficient proof for this mission
- Marketplace rows must remain observational; validators must not treat absence of mutation as optional
- Use repeated-skill fixtures when validating duplicate-source behavior; distinct-name fixtures are insufficient
- For backend/frontend compatibility fixes that unblock scrutiny before the full Claude UX lands, a native launch/count smoke check is acceptable for that fix feature; the full duplicate-row click-through remains owned by later Claude UX features and final user-testing validation.
- For command-palette or global-search flows, prefer validating the underlying state transition with targeted tests first; native Tauri evidence may use launch/screenshot confirmation plus documented macOS accessibility limits when the command-palette surface is not reliably scriptable.
- For the Claude source-tab follow-up, validators must prove the active tab hides opposite-source rows before search is applied, and that search does not re-surface rows excluded by the selected tab.

## Native macOS Tauri Recipe

- Launch the real app with `HOME=/tmp/skills-manage-test-fixtures/claude-multi-source pnpm tauri dev`.
- Wait for `curl -sf http://localhost:24200` before attempting native inspection.
- If port `24200` is already occupied by an existing repo dev server, reuse it with the `.factory/services.yaml` command `manual-validation-isolated-reuse-devurl-24200` instead of starting a second Vite instance on the same port.
- Prefer macOS `screencapture` against the real app window for screenshots; prior runs found Quartz window capture can skew this Tauri webview.
- For interaction and inspection, prefer PyObjC `ApplicationServices` / `AXUIElement` helpers over plain `System Events`; prior Claude and platform validations found AXUIElement markedly more reliable for sidebar/detail clicks.
- If the native window renders but does not expose actionable AX children, capture screenshots plus any OCR/SQLite evidence and record the assertion as blocked instead of guessing.

## Flow Validator Guidance: manual-tauri

- Surface: the native `skills-manage` Tauri window only. Do not treat the plain browser preview as authoritative proof for this milestone.
- Isolation: use `HOME=/tmp/skills-manage-test-fixtures/claude-multi-source` and keep validation scoped to that fixture unless a final real-home spot check is explicitly called for.
- Concurrency: run only one native manual-tauri validator at a time. The shared app window, AX state, and fixture HOME are not safe for parallel mutation.
- Allowed fixture mutation: user-source fixture paths under `/tmp/skills-manage-test-fixtures/claude-multi-source/.claude/skills` may be changed for rescan assertions and then restored if needed. Never modify any real-home Claude marketplace directory.
- Off-limits: do not broaden coverage into Discover or Claude cache roots, and do not create any management action for marketplace rows.
- Evidence: capture screenshots for list/detail/rescan states and pair them with concise notes about source badges, rooted paths, read-only treatment, and whether management controls are absent or still enabled on the user-source sibling row.
