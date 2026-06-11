# 优化中央技能标签机制 - Implementation Plan

## Preconditions

- 当前任务仍是 `planning` 状态。开始写代码前，先让用户确认 PRD/design/implement，并运行 `python ./.trellis/scripts/task.py start 06-11-optimize-central-skill-tags`。
- 写代码前加载 `trellis-before-dev`，并按涉及层读取 frontend/backend 具体 guideline。

## Checklist

1. Backend seed cleanup
   - Update `builtin_skill_tags()` so ordinary built-in taxonomy only includes `academic-research-writing`; keep `UNCATEGORIZED_TAG_ID` as system fallback.
   - Add an idempotent prune step for obsolete `is_builtin = 1` tags after current built-ins are seeded.
   - Clean related `skill_tag_links` and `skill_ai_tag_reviews` for pruned tag ids.
   - Update DB tests for fresh init and upgraded old built-in rows.

2. AI prompt reuse rules
   - Build AI candidate list from existing classifiable tags, excluding `uncategorized`.
   - Include tag id, name, and description in candidate lines.
   - Strengthen prompt wording: output only candidate ids, prefer existing custom tags, return empty when no clear match.
   - Add/adjust tests around prompt content, unknown tags, empty suggestions, and bulk tagging fixture responses.

3. Frontend visible tag filtering
   - Add a shared frontend helper for system tag ids, special filter ids, and visible skill tags.
   - Use visible tags in `CentralTopFilters` and manual categorize candidates.
   - Keep special smart views working through `uncategorized` / `updates` / `ai-review`.

4. Stale filter cleanup
   - Sanitize Central V2 `viewState.tags` against current tags + special filter ids.
   - Ensure SearchBar chips do not show deleted built-in ids as active filters.
   - Add focused Vitest coverage for stale tag id removal or ignore behavior.

5. Compatibility review
   - Confirm manual custom tags remain available and assignable.
   - Confirm `replace_skill_ai_tags()` still preserves manual tags.
   - Record portability old-export behavior as a known follow-up, not part of this implementation.

## Validation

Run targeted checks first:

```powershell
cd src-tauri; cargo test tags ai_tagging
pnpm test -- src/test/CentralTopFilters.test.tsx src/test/centralViewState.test.ts src/test/centralFilters.test.ts
pnpm typecheck
pnpm lint
```

Repository completion gate:

```powershell
just ci
```

## Rollback Points

- If built-in prune proves too destructive, revert only the prune step and keep prompt/frontend visibility changes.
- If stale saved-view cleanup causes URL sync churn, keep deleted ids ignored at match time and defer state mutation.
- If prompt changes reduce AI usefulness, keep candidate filtering but soften confidence wording before changing persistence logic.

