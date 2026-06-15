# Design

## Boundaries

Primary files:

- `src/components/skill/SkillDetailSidebar.tsx` owns the right sidebar presentation.
- `src/components/skill/SkillDetailView.tsx` owns detail-level actions, store calls, dialogs, and refresh orchestration.
- `src/stores/skillDetailStore.ts` already owns `installSkill`, `uninstallSkill`, and `openInFileManager`.
- `src/i18n/locales/en.json` and `src/i18n/locales/zh.json` own all new copy.
- `src-tauri/capabilities/default.json` owns desktop permission for shell open.
- `src/test/SkillDetailView.test.tsx` should cover the user-facing behavior.

Avoid touching:

- `UnifiedSkillCard` and Central list card footer behavior, unless a shared helper must stay type-compatible.
- Rust uninstall implementation, unless existing frontend calls cannot represent the needed row identity.

## Repository Open Flow

Current flow:

`SkillDetail.repository` -> `buildGitHubRepositoryUrl()` -> `<a target="_blank">`.

Target flow:

`SkillDetail.repository` -> resolved URL -> button click -> external URL opener -> Tauri shell `open(url)` in desktop, browser fallback outside Tauri.

Recommended implementation shape:

- Add a narrow helper such as `src/lib/externalUrl.ts`:
  - validates that the URL protocol is `http:` or `https:`;
  - calls `open(url)` from `@tauri-apps/plugin-shell` when `isTauriRuntime()` is true;
  - calls `window.open(url, "_blank", "noopener,noreferrer")` outside Tauri.
- Add `"shell:default"` to `src-tauri/capabilities/default.json`, or the narrow permission that compilation/schema validation proves is sufficient. The generated schema documents `shell:default` as allowing `http(s)://`, `tel:`, and `mailto:` open behavior and including `allow-open`.
- Convert the current anchor to a real button with `ExternalLink`, localized label, and the same visual styling.

Trade-off:

- A helper is slightly more code than inline `open(url)`, but it keeps the component testable and preserves browser/test mode without calling a Tauri plugin directly in every caller.

## Installed Platform Display

Current data:

- `SkillDetailView` already has `detail.installations`.
- `SkillDetailSidebar` already receives `installationMap`, target agent groups, `sharedRootAgentIds`, `installingAgentId`, and `onToggleInstall`.
- `SkillInstallation` exposes `agent_id`, `installed_path`, `link_type`, and optional `installed_at`.

Target UI:

- Add a compact installed-platform section near the existing install status area.
- The section shows only platforms with a current installation for this skill.
- Each row/chip shows:
  - platform icon;
  - platform display name;
  - optional install type/path hint in title or secondary text if space allows;
  - a hidden-until-hover/focus X button for removable installs.
- If no platforms are installed, show the existing no-platform/empty state only where useful and avoid duplicating noise.

Platform mapping:

- For each rendered platform target, use `getPlatformTargetMemberIds(agent)` to decide whether any member has a matching installation.
- Use `getPlatformTargetInstallAgentIds(agent)` to identify the uninstall target where the existing toggle code already does so.
- Do not treat `sharedRootAgentIds` as independently removable installs unless a real `detail.installations` row exists and the backend supports the uninstall.

## Confirmation Flow

Recommended flow:

1. User hovers or focuses an installed platform row.
2. X button appears.
3. Clicking X stores `{ agentId, displayName }` in local `SkillDetailView` state and opens a `Dialog`.
4. Cancel closes the dialog with no store action.
5. Confirm calls the existing `uninstallSkill(skillId, agentId, rowId?)` store action.
6. On success, refresh `refreshCounts()` and `refreshInstallations(skillId)`, matching the existing `handleToggle` post-mutation behavior.
7. On failure, show the existing localized `detail.uninstallError` toast with `formatBackendError`.

Use the same row-aware uninstall guard as existing code only if the detail context requires it:

- `claude-code` user-source rows can require `detail.row_id`;
- Central skill platform installs should normally use `uninstallSkill(skillId, agentId)` with no row id.

## Data Flow

Open repository:

`SkillDetail.repository` -> URL resolver -> `openExternalUrl(url)` -> Tauri shell or browser fallback.

Remove installed platform:

`detail.installations` -> installed platform view model -> confirmation dialog -> `skillDetailStore.uninstallSkill` -> backend `uninstall_skill_from_agent` -> reload detail -> refresh counts/installations -> sidebar rerenders.

## Accessibility

- The repository open control is a button with the existing text label and `ExternalLink` icon.
- The hover-only delete control must also appear on focus for keyboard users.
- Icon-only X buttons need localized `aria-label` and `title`.
- The confirmation dialog needs localized title, description, cancel, and confirm text.

## Compatibility

- The feature uses existing `SkillDetail.installations`; no backend DTO change is planned.
- Existing install/uninstall toggles remain available.
- Read-only plugin copies stay display-only.
- Remote targets should not try to open local file managers; repository URLs are local browser actions and remain allowed if the URL is `http(s)`.

## Risks

- `SkillDetailSidebar.tsx` is already a dense component. Keep any new view-model derivation small or extract a local helper if the file grows hard to scan.
- `src/pages/CentralSkillsView.tsx` has a known 800-line size budget history; this task should not add logic there.
- If shell capability is missed, unit tests can pass while the desktop button still does nothing. Capability validation and a runtime check are required.
