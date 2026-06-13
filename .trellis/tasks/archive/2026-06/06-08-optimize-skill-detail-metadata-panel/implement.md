# Implementation Plan

## Checklist

1. Read the relevant detail sidebar and shared UI helpers.
   - Verify: confirm no component-level direct `invoke()` calls are introduced.

2. Add small frontend helpers if needed.
   - Build a repository display label from existing `repositoryDisplayName`.
   - Build a GitHub repo URL from `repository.url` or `owner/repo`.
   - Verify: helper behavior covers URL, owner/repo fallback, and unknown repo.

3. Restructure `SkillDetailSidebar` metadata rendering.
   - Group local location, GitHub source, and technical details.
   - Put raw `file_path`, `canonical_path`, raw `source`, and scan timestamp in
     a collapsed Technical details disclosure by default.
   - Suppress duplicate primary rows as defined in the PRD.
   - Verify: GitHub Central skill no longer shows all source rows as one flat
     list.

4. Style the local folder action.
   - Use the existing `Button`, lucide icon, and inspector visual language.
   - Keep remote copy behavior and local open behavior unchanged.
   - Verify: button has accessible label, stable height, and no text overflow.

5. Add i18n keys.
   - Update English and Chinese locale files for any new labels.
   - Verify: no hard-coded user-facing strings remain in the component.

6. Validate.
   - Run `pnpm typecheck`.
   - Run `pnpm lint`.
   - Run focused Vitest if an existing sidebar/detail test is available.
   - Run `just ci` before completion.

## Risk Points

- `SkillDetailSidebar.tsx` is already sizeable. Keep changes localized and avoid
  broad component extraction unless the grouping becomes hard to read.
- `canonical_path` must remain in data, even if not first-class in the default
  UI.
- External links should not replace the existing local file-manager action.
