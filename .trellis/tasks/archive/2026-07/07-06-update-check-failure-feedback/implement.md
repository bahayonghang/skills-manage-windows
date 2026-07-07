# Implement: update check failure feedback

## Checklist

- [x] Load frontend pre-dev guidance before editing.
- [x] Add an optional inline error state to `UpdateCheckModeDialog`.
- [x] In `useCentralUpdateCheckModeController`, catch refresh failures, translate them with `central.updateCheckError`, toast them, and keep the dialog open.
- [x] Clear stale dialog-local errors on open/close and before each retry.
- [x] Add component coverage for the dialog inline error.
- [x] Add CentralSkillsView coverage proving a rejected Update Center refresh shows toast + inline error, keeps the dialog open, and re-enables submit.
- [x] Run focused tests: `pnpm test -- UpdateCheckModeDialog CentralSkillsView.updates-and-search`.
- [x] Run frontend checks: `pnpm typecheck && pnpm lint`.
- [x] Run final gate: `just ci`.
- [x] Record the reusable async error-feedback convention in `.trellis/spec/frontend/async-error-feedback.md`.

## Risk Points

- Avoid swallowing successful refresh results or changing `openUpdateCenter()` arguments.
- Avoid relying on direct Tauri mocks; the view tests use the mocked Update Center store seam.
- Keep all new visible copy in i18n resources.
