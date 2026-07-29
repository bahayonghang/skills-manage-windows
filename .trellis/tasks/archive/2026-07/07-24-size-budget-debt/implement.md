# 07-24-size-budget-debt Implementation Plan

## Preconditions

- Start only after the user approves this planning summary.
- Preserve the existing unrelated `.trellis` runtime/config/script changes, other task directories, `.gitattributes`, and audit report.
- Do not push or create a remote pull request.

## Execution

1. Record the five pre-change line counts and run the smallest relevant tests to establish the baseline.
2. Extract Central update state/source helpers to `services/central_updates/core/state.rs`; keep `core.rs` as the stable orchestration surface and re-export helpers needed by existing sibling modules and tests.
3. Move the existing collections `#[cfg(test)]` body to `commands/collections/tests.rs` and replace it with the module declaration; retain all production command code and command attributes.
4. Move builtin-agent construction and path helpers to `db/seed/agents.rs`, add explicit re-exports from `seed.rs`, and verify all `crate::db::*` consumers compile without path changes.
5. Extract the CentralSkillsView action binding and scroll-preserving selection wrapper to a sibling hook. Keep route-level state ownership, `CentralSkillsShell`, `CommandPalette`, `CentralStoreLocationDialog`, and update-check dialog composition in `CentralSkillsView.tsx`.
6. Extract UnifiedSkillCard public type declarations and leaf UI helpers to sibling modules. Re-export type names from the original module; retain `SkillCardModel`, `toModel`, and the sole `UnifiedSkillCard` render entry there.
7. Once the five original files are below 600 lines, remove the checker allowlist code and exception report. Update the CI quality spec to record that the 800-line production check has no frozen exceptions.
8. Run targeted Rust and frontend tests after each logical extraction, then run all required gates. Inspect the direct-count output and final diff before commit.

## Validation

```powershell
cd src-tauri; cargo fmt --all -- --check
cd src-tauri; cargo test --locked central_updates
cd src-tauri; cargo test --locked collections
cd src-tauri; cargo test --locked db::tests
pnpm typecheck
pnpm lint
pnpm test -- UnifiedSkillCard CentralSkillsView
pnpm sizecheck
$files = @('src-tauri/src/services/central_updates/core.rs','src-tauri/src/commands/collections.rs','src-tauri/src/db/seed.rs','src/pages/CentralSkillsView.tsx','src/components/skill/UnifiedSkillCard.tsx'); $files | ForEach-Object { [pscustomobject]@{ Path = $_; Lines = (Get-Content $_).Count } } | Format-Table -AutoSize
just ci
```

The final direct-count table must show every listed file below 600 lines. `just ci` is the final quality gate; no packaging build is required because this task does not modify packaging or release files.

## Closeout

1. Run `trellis-check` after code and full validation; address findings before proceeding.
2. Update the quality spec through `trellis-update-spec` if the no-exception size policy is not already captured by the implementation change.
3. Make one scoped Chinese emoji local commit for the completed child, then archive `07-24-size-budget-debt` and add a journal entry referencing the real work commit. Do not push.
4. Re-open the parent evidence: confirm all 16 children are archived, P3-01 has the recorded direct-count proof, and the latest `just ci` passed. Archive and journal the parent only after that integration review is complete.

