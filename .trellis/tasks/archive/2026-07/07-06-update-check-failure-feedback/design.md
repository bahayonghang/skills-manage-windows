# Design: update check failure feedback

## Root Cause

The primary Central "check updates" CTA now runs through `useCentralUpdateCheckModeController`, not the older `useCentralSkillsUpdateWorkflow` handlers. The controller awaits `useUpdateCenterStore.refresh(scope)` and opens Update Center only on success. On rejection it reaches `finally` without rendering or toasting the failure, so the user sees the same mode dialog with no explanation.

## UI Contract

- `UpdateCheckModeDialog` gets an optional `error` prop.
- When present, it renders an inline destructive alert above the footer so the failure is visible while the user decides whether to retry or cancel.
- The text uses the existing `central.updateCheckError` i18n key so the wording matches older check flows.
- `handleConfirm()` also calls `toast.error()` with the same translated message.

## State Flow

1. User opens the dialog and confirms a mode.
2. Controller clears any dialog-local error and sets `isSubmitting`.
3. Controller builds the scope and calls `refreshUpdateInventory(scope)`.
4. On success, Update Center opens and the dialog closes as it does today.
5. On failure, controller stores the translated error message, fires a toast, leaves the dialog open, and re-enables the submit button.

## Boundaries

- No new IPC calls are added; `useUpdateCenterStore.refresh()` remains the store action boundary.
- No backend changes are needed.
- The dialog remains presentational: it receives the already formatted error string and does not know about stores.

## Compatibility

- Existing success paths are unchanged.
- `syncDisabled` and saved mode preference fallback behavior remain unchanged.
- The inline error clears when the dialog closes or a new check attempt starts, avoiding stale failure copy.
