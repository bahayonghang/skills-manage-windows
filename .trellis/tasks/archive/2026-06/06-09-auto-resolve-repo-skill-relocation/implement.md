# Implementation Plan

## Checklist

1. Add backend relocation reconciliation near `refresh_skill_update_inventory`
   remote-added collection.
   - Verify with Rust unit tests for safe path move, changed content, and
     ambiguity.
2. Add or reuse a repository-member source-path update helper if existing DB
   APIs cannot express the update clearly.
   - Verify the helper has focused DB test coverage when added.
3. Adjust frontend decision aggregation only if backend output requires a type
   or display change.
   - Verify existing Vitest coverage still passes.
4. Run focused checks, then full repo gate.

## Validation Commands

```powershell
pnpm exec vitest src/test/updateCenterDecisionAggregation.test.ts
cd src-tauri; cargo test commands::skill_update_inventory
just ci
```

## Risk Points

- Do not auto-delete Central skills during refresh.
- Do not import remote content during refresh unless it is an existing update
  flow for the same Central skill.
- Do not hide ambiguous Added/Removed evidence from the UI.
- Preserve pending-addition skip-memory behavior.
