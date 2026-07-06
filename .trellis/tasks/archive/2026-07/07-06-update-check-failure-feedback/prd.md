# Improve update check failure feedback

## Goal

When the user starts a Central skill update check from the mode selection dialog and the refresh fails, the failure must be visible immediately instead of leaving the user in a quiet modal with no actionable feedback.

## Confirmed Facts

- The screenshot shows `UpdateCheckModeDialog` open after starting an incremental/removal check, with no failure message visible.
- `useCentralUpdateCheckModeController.handleConfirm()` calls `useUpdateCenterStore.refresh(scope)` and only resets `isSubmitting` in `finally`; it does not surface rejected errors.
- `useUpdateCenterStore.refresh()` stores `error` and rethrows, but the mode dialog does not read that store error and the Update Center dialog is not opened on failure.
- Older Central update workflows already show `toast.error(t("central.updateCheckError", { error }))` on check failures.
- User-visible copy must go through i18n in both `zh.json` and `en.json`.

## Requirements

- Failed checks from the update-check mode dialog must show an error while the dialog remains open.
- The user must also receive a toast-level failure notification consistent with existing Central update failure behavior.
- Starting a new check attempt must clear the stale dialog-local error.
- Successful checks must keep the current behavior: refresh inventory, open Update Center on the preferred tab, and close the mode dialog.
- The change must stay in the frontend controller/component path; components must not call IPC directly.

## Acceptance Criteria

- [x] If `refreshUpdateInventory(scope)` rejects, the mode dialog displays the translated failure message including the backend error text.
- [x] The same failure triggers `toast.error` with the existing Central update-check error wording.
- [x] The failed dialog remains open and the submit button returns to enabled state after the failed attempt.
- [x] Retrying after a failure clears the old inline error before the new attempt runs.
- [x] Existing successful regular and sync check behavior remains covered by tests.
- [x] `pnpm test -- UpdateCheckModeDialog CentralSkillsView.updates-and-search` passes.
- [x] `pnpm typecheck && pnpm lint` passes, and final project gate runs `just ci`.

## Out Of Scope

- Backend retry, GitHub/network diagnostics, or error classification changes.
- Changing the Update Center inventory model.
- Changing the destructive-action confirmation flow for imports, removals, or update application.
