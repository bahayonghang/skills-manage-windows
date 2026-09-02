# Parent-session independent verification

Scanner identity: UNVERIFIED.

| Command | Exit |
| --- | ---: |
| `pnpm exec vitest run` GlobalSearchDialog + CollectionsListView + collectionStore + centralSkillsStore + i18nLocales | 0 (128 tests) |
| `pnpm typecheck` | 0 |
| `pnpm lint` | 0 |
| `just ci` | 0 |
| `git diff --check` | (run at commit) |

## Dispatch record

| Role | Agent | Verdict |
| --- | --- | --- |
| trellis-implement | 260da10d-da93-43c0-a633-15c72c57cf9d | implemented routing/loaded/latest-wins |
| trellis-check | 078149f7-ec26-44f3-b18f-1339d36a29d2 | PASS |

## Owned findings

| id | status | evidence |
| --- | --- | --- |
| FE-CORR-001 | fixed | search navigates to `/collections` + collectionContext |
| FE-CORR-002 | fixed | monotonic detailRequestId latest-wins tests |
| FE-CORR-003 | fixed | hasLoaded + gated loaders; error ≠ empty |

## UNVERIFIED

- AC21 Windows WebView2 keyboard/focus
- AC22 real Tauri data load (jsdom is not a substitute)
