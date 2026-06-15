# Implementation Plan

## Preconditions

- Do not start implementation until this planning task is approved and `task.py start 06-13-skill-detail-repo-link-platform-installs` has been run.
- Before editing code, load `trellis-before-dev` for the frontend and capability files.

## Checklist

1. External URL helper
   - Add a narrow helper for opening `http(s)` URLs through Tauri shell in desktop mode and `window.open` in browser/test mode.
   - Add direct tests if the helper has validation branches.

2. Tauri capability
   - Add the minimal shell open permission to `src-tauri/capabilities/default.json`.
   - Validate the permission name through generated schema/typecheck/build feedback.

3. Repository open action
   - Replace the current `Open GitHub repo` anchor in `SkillDetailSidebar` with a button action.
   - Preserve the existing visual style, icon, label, and repository metadata display.
   - Add failure toast only if opener failures become user-visible; otherwise keep failure behavior consistent with existing non-blocking open actions.

4. Installed platform view model
   - Derive installed platform rows from `targetAgents`, `detail.installations`, and platform target helpers.
   - Keep shared-root or read-only states non-removable.
   - Avoid duplicating the existing install toggle row.

5. Confirmation dialog
   - Add local state in `SkillDetailView` for the pending platform uninstall.
   - Add a small confirmation dialog using existing `Dialog` primitives.
   - Confirm path calls the existing `uninstallSkill` action and refreshes counts/installations.

6. I18n
   - Add English and Chinese keys for installed platform section, delete aria-label/title, dialog title/body, cancel, confirm, success or error copy if needed.

7. Tests
   - Update `src/test/SkillDetailView.test.tsx` repository metadata test to assert the external opener is called.
   - Add tests that installed platform rows render from `detail.installations`.
   - Add tests that uninstalled platforms are not shown in the installed-platform section.
   - Add tests that clicking X opens confirmation without uninstalling.
   - Add tests for cancel and confirm paths.
   - Add a read-only or locked case if the implementation exposes such a row.

## Validation Commands

Run focused checks first:

```powershell
pnpm test -- src/test/SkillDetailView.test.tsx
pnpm typecheck
pnpm lint
```

Run the repository gate before completion:

```powershell
just ci
```

For desktop behavior, run a manual runtime check after implementation:

```powershell
just dev
```

Runtime checklist:

- Open a Central skill with repository metadata.
- Click "Open GitHub repo" and confirm the system browser opens the repository URL.
- Confirm installed platform rows show only installed platforms.
- Hover/focus an installed platform row, click X, cancel once, then confirm once.
- Confirm the platform disappears from the installed-platform list after uninstall.

## Risk Points

- Tauri shell capability misconfiguration can leave the UI looking correct but non-functional in the desktop app.
- Hover-only controls can be inaccessible if focus styles are omitted.
- Universal/platform target grouping can produce confusing labels if the row derivation ignores existing `platformTargetGroups` helpers.
- Tests that assert CSS hover visibility directly may be brittle; prefer asserting accessible controls and dialog behavior.

## Rollback Points

- External open helper and capability can be reverted independently if they cause desktop permission issues.
- Installed platform display can be removed without changing backend contracts because it derives from existing `SkillDetail.installations`.

## Review Gate Before Start

- Confirm the task remains single-scope: skill detail repository open plus single-platform installed target removal.
- Confirm no Central list or batch uninstall behavior is being changed.
- Confirm implementation will be added, not started, only after user approval.

## Implementation Status

Completed on 2026-06-13:

- Added the external URL helper, Tauri shell capability, and Skill Detail GitHub repository button path.
- Added the installed-platform list, hover/focus delete affordance, and confirmation dialog using the existing single-platform uninstall store action.
- Added English/Chinese i18n copy and focused React Testing Library coverage for repository open, installed-platform rendering, cancel, confirm, read-only, and shared-root cases.

Validation completed:

- `pnpm exec vitest run src/test/CentralSkillsView.github-import-preview.test.tsx`
- `pnpm exec vitest run src/test/SkillDetailView.test.tsx src/test/externalUrl.test.ts`
- `just ci`
- Local HTTP probe for the existing dev server at `http://127.0.0.1:24200/` returned the SkillPort index HTML.

Manual note:

- The in-app Browser backend was unavailable in this Codex session, so browser visual automation was not completed. The real desktop system-browser open behavior still depends on Tauri runtime execution; the code path and permission are covered by helper tests plus `just ci`.
