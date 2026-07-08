# Add confirmed one-click platform leftover cleanup

## Goal

Add a confirmed one-click cleanup action for Update Center platform leftovers so users can remove all stale platform-side copies without manually checking every path.

## Requirements

- The action is specific to `Platform leftovers` / `平台残留`.
- The action must ask for confirmation before deleting anything.
- The action must delete all leftover paths from the currently loaded Update Center inventory, even if the user has not manually selected them.
- The action must not apply unrelated update, addition, remote-missing, platform duplicate, or orphan decisions.
- The action must reuse the existing Update Center apply path so backend deletion, inventory reload, error state, and Tauri IPC boundaries stay consistent.
- User-visible copy must go through i18n in both English and Chinese.

## Acceptance Criteria

- [x] Update Center exposes a one-click cleanup control when platform leftovers exist.
- [x] Clicking the control shows a destructive confirmation with the number of leftover paths.
- [x] Cancelling the confirmation does not call apply.
- [x] Confirming sends only `removeDeletedPlatformCopies` decisions for the current inventory's leftover paths.
- [x] Success and partial failure are reported with localized toasts.
- [x] Existing manual selection and `Apply selected` behavior continues to work.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
- Confirmed user choice: use a confirmation prompt instead of immediate deletion.
