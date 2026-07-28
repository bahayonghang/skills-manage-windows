# Skill Usage 状态与交互契约

## 1. Scope / Trigger

修改 `/usage`、`usageStore`、usage fixtures、排序/热力图/详情组件时阅读。
目标是让所有可见面板始终来自同一 target/source，并让未匹配技能仍可查看统计。

## 2. Signatures

```ts
type UsageSkillMatchStatus = "matched" | "ambiguous" | "unmatched";
type UsageMatchFilter = "all" | "installed" | "unlinked";

type SkillUsageSummary = {
  skill: string;
  count: number;
  projects: number;
  sessions: number;
  lastUsedMs: number;
  matchStatus: UsageSkillMatchStatus;
  resolvedSkillId: string | null;
  staticTokenEstimate: number | null;
  staticByteCount: number | null;
};

invoke("usage_get_skill_detail", { skill, source });
```

`usageStore` owns `selectedSource`, `selectedSkill`, `usedCachedData`,
`refreshError`, `loading`, `refreshing`, and `detailLoading`.

## 3. Contracts

- Components never call `invoke`; use typed `@/lib/ipc` only inside the store.
- Source selection requests overview + recent with `Promise.all` and commits both
  in one `set`. Keep the previous source/data visible until the new pair succeeds.
- Page, refresh, and detail requests each have a sequence. A response commits only
  when sequence, active target id, source, and selected skill still match.
- `overview === null` is the first-load authority even before the bootstrap effect
  sets `refreshing`. Render the final-layout scanning skeleton and withhold numeric
  KPI output until an overview exists or a page-level error is available.
- Target changes invalidate all three request sequences and immediately reset
  `overview`, `recent`, `providers`, `loading`, detail state, and source selection
  before starting a forced refresh. Never leave an old source request loading or
  render panels from the previous target during the rescan.
- A filtered refresh must not briefly publish the unfiltered refresh payload.
  Update provider/scope/freshness first, retain the filtered page, then refetch it.
- Ranking install-state filtering is view-local: `installed` keeps only `matched`,
  while `unlinked` keeps `ambiguous` plus `unmatched`. It must not enter the store,
  change backend requests, or filter the recent-calls feed.
- Match status remains readable without color. Pair translated status text with
  semantic `statusTone` dots for matched/success and ambiguous/warning; unmatched
  uses the neutral muted token.
- Selecting any ranking/recent row opens inline statistics. Only rows with a
  returned `resolvedSkillId` render a separate open-skill action; never invoke a
  resolver on click.
- Project paths display only their basename. Prompt text, assistant responses,
  tool arguments, credentials, and full paths are not rendered.
- Fixed ranges are visible in UI: all history, last 16 weeks, and latest 20.
  Provider health stays in a secondary disclosure.
- Browser fixtures run the real store and include matched, ambiguous, unmatched,
  missing static metrics, filtered sources, and non-empty heatmap data.

## 4. Validation & Error Matrix

| Condition | Required UI state |
| --- | --- |
| first load | stable final-layout skeleton |
| bootstrap effect has not started yet | skeleton, never a one-frame zero KPI strip |
| target changes while source load is pending | clear target-scoped panels, reset `loading`, force refresh |
| source request fails | keep previous selected source and page; show recoverable error |
| refresh fails with cache | keep page; set `usedCachedData`; show freshness warning |
| refresh fails without cache | page-level error with retry action |
| target/source changes during detail load | discard stale detail result |
| ambiguous/unmatched row | inline detail works; no open-skill button |
| installed/unlinked ranking filter | filter only ranking rows; show filtered/total count and a distinct empty state |
| static estimate `null` | explicit unavailable state, never numeric zero |

## 5. Good / Base / Bad Cases

- Good: a rapid Claude -> Codex switch finishes out of order; only Codex overview
  and recent commit, and old Claude detail is discarded.
- Good: a target switch during a pending source request immediately shows the
  scanning skeleton; the stale request cannot keep `loading` true or republish data.
- Base: an unmatched historical call still opens its project/activity detail.
- Base: selecting `unlinked` shows ambiguous and unmatched ranking rows while
  recent calls remain unchanged.
- Bad: publish `selectedSource = Codex` while the visible overview is still
  Claude, or resolve a Central id lazily when the row is clicked.

## 6. Tests Required

- Store: rapid A->B source completion, target change, filtered refresh, cached
  failure, detail source args, stale detail, atomic overview/recent commit, and
  target-change clearing of page data plus `loading`.
- Components: three match states, explicit sort controls, row keyboard behavior,
  separate open action, basename-only projects, detail close/focus return,
  pre-refresh skeleton, install-state filters/counts, and filtered empty state.
- Heatmap: 112 cells, quantile top level, numeric aria labels, roving arrow focus,
  month labels, legend, and empty state.
- Browser: 1440x900 and 1280x720 dark, 1024x768 light, narrow desktop, Chinese
  and English, with page `scrollWidth === clientWidth` and no console errors.

## 7. Wrong vs Correct

```tsx
// Wrong: component makes a second request and guesses navigation on click.
const id = await invoke("usage_resolve_skill_id", { skillName: row.skill });

// Correct: select statistics; use backend-provided identity for explicit action.
<button onClick={() => loadDetail(row.skill)}>{row.skill}</button>
{row.resolvedSkillId && <OpenSkillButton id={row.resolvedSkillId} />}
```
