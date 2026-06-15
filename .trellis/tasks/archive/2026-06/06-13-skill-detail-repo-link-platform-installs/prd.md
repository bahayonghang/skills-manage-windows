# Optimize skill detail repository link and platform installs

## Goal

Fix the Skill Detail sidebar so repository metadata actions work in the desktop app and installed platform targets are visible and removable from the detail page.

User value:

- Users can click "Open GitHub repo" from the metadata card and get the repository in the system browser.
- Users can see which platforms currently have this skill installed without scanning all install toggle icons.
- Users can remove this skill from one installed platform from the detail sidebar after an explicit confirmation.

## Assumptions

- The requested screen is the shared `SkillDetailView` / `SkillDetailSidebar` used by the Central skill page and drawer.
- "Installed platforms" means platform installs represented by `SkillDetail.installations`, not every configured target.
- The new installed-platform display should be added near the existing sidebar metadata/install status area and should not replace the existing install-to-platform toggle row.
- Shared-root or read-only cases must not present an independent delete action when the backend cannot remove that platform independently.

## Confirmed Facts

- `src/components/skill/SkillDetailSidebar.tsx` builds the GitHub repository URL from `detail.repository.url` or `owner/repo` and renders "Open GitHub repo" as a plain `<a target="_blank">`.
- `src-tauri/src/lib.rs` initializes `tauri_plugin_shell`, and `package.json` includes `@tauri-apps/plugin-shell`.
- `src-tauri/capabilities/default.json` does not currently include `shell:default` or `shell:allow-open`, so a frontend `@tauri-apps/plugin-shell.open()` call would need the capability updated.
- `SkillDetail.installations` already includes `agent_id`, `installed_path`, `link_type`, and `installed_at`.
- `src/components/skill/SkillDetailView.tsx` already builds an `installationMap` from `detail.installations`.
- `src/stores/skillDetailStore.ts` already exposes `uninstallSkill(skillId, agentId, rowId?)` and reloads detail after uninstall.
- Existing tests in `src/test/SkillDetailView.test.tsx` already cover repository metadata rendering and can be extended to cover the open action and installed-platform removal flow.

Root cause sentence for issue 1:

I believe the root cause is that `SkillDetailSidebar` renders "Open GitHub repo" as a browser-style external anchor instead of calling Tauri's shell open API, and the app capability currently lacks shell open permission, so the desktop WebView has no reliable allowed path to open the URL externally.

## Requirements

- Replace the plain repository external link behavior with a desktop-safe external open action.
- Keep a browser-mode fallback so tests and browser previews do not break outside Tauri.
- Add any required Tauri shell capability for opening `http(s)` links.
- Add an installed-platform section that lists only currently installed platform targets for the skill.
- Each removable installed platform entry must expose a delete affordance that appears on hover/focus and uses an explicit confirmation dialog before uninstalling.
- The confirmation dialog must identify the skill and platform, state that only the platform install is removed, and leave the Central skill intact.
- Confirmed deletion must call the existing single-platform uninstall path and refresh the detail/count state the same way existing install toggles do.
- Read-only details and non-removable shared-root installs must not show a destructive delete action.
- All new user-visible text must be localized in `src/i18n/locales/en.json` and `src/i18n/locales/zh.json`.

## Acceptance Criteria

- [ ] Clicking "Open GitHub repo" in the desktop app calls the shell open path for the resolved GitHub URL.
- [ ] The Tauri capability file grants the minimum shell open permission needed for `http(s)` repository links.
- [ ] Browser/test mode still has a safe fallback for opening or mocking external repository URLs.
- [ ] The detail sidebar displays installed platform names derived from `detail.installations`.
- [ ] Uninstalled platforms are not listed in the installed-platform section.
- [ ] Hovering or focusing a removable installed platform reveals an X/delete button.
- [ ] Clicking the X opens a confirmation dialog and does not uninstall until the user confirms.
- [ ] Confirming uninstall calls the existing `uninstallSkill(skillId, agentId, rowId?)` path and refreshes detail/count data.
- [ ] Cancelling the dialog leaves installations unchanged.
- [ ] Read-only skill details do not show platform deletion controls.
- [ ] Shared-root locked platform state is either omitted from the removable list or shown as non-removable with no X.
- [ ] Focused React Testing Library coverage proves repository open, installed-platform rendering, confirmation, cancel, and confirm behavior.
- [ ] `pnpm typecheck`, `pnpm lint`, relevant Vitest tests, and `just ci` pass before completion.

## Out of Scope

- Changing repository import, sync, or update semantics.
- Reworking Central list cards or `UnifiedSkillCard` platform toggle behavior.
- Adding batch uninstall behavior; this task is single skill detail only.
- Changing backend uninstall semantics unless an existing command bug is found during implementation.

## Open Questions

None blocking. Recommended implementation direction is to add the installed-platform management UI without replacing the existing install status toggle row.
