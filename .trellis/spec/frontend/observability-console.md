# Observability Console Presentation And Navigation Contract

## Scope

Apply this contract to Operation Log rows, Runtime Log parsing/filtering, cross-layer navigation, detail UI, copying and
user-visible diagnostics under `/logs`.

## Contracts

- Operation rows are projected through `logDiagnostics.ts` before rendering. The projection accepts reviewed IPC codes,
  fixed phase/status/target/source values, UUID correlation IDs, counts and booleans only. Raw `summary`, `errorSummary`,
  target labels/IDs, subjects, failure identifiers and arbitrary structured values are not presentation inputs.
- Runtime rows expose `backend`, `frontend` or `legacy` as visible text plus an icon. Exact operation-ID and event-source
  filters live in `runtimeLogStore`; Operation and Runtime modes can jump to the other layer without losing the shared ID.
- A correlation ID is visible, filterable and copyable only when it is a valid UUID. Legacy/invalid IDs show a fixed
  fallback, offer no cross-layer jump and never reach the clipboard.
- Operation detail uses the shared accessible `Dialog` primitive as a centered, compact, viewport-safe window. Header and
  controls stay fixed; `DialogBody` is the single vertical scroller. Closing by button, backdrop or Escape restores focus
  to the triggering row/action control.
- Reading order is status/action, localized status summary, reviewed reason and next action, diagnostic keys, metadata,
  bounded failure items, then collapsed safe structured details. Users must not need raw JSON to understand the failure.
- User-entered operation-ID filters use a draft value and execute only on explicit Search. Narrow layouts remain usable;
  full filter density is reserved for wide desktop breakpoints.
- All text and copy-failure feedback use `src/i18n/`. Never interpolate `String(error)` or a stored raw diagnostic value.

## Required tests

- Centering, viewport bounds, one scroll owner, Escape and focus restoration.
- Valid UUID copy/jump plus legacy and adversarial ID clipboard rejection.
- Operation-to-Runtime and Runtime-to-Operation navigation with exact filters.
- Started/interrupted/source states use icon plus localized text, not color alone.
- Adversarial host, path, token, raw error and identifier seeds are absent from the DOM and clipboard.
- English/Chinese locale parity and narrow-layout class contracts.
