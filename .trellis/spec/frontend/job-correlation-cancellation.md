# Renderer Job Correlation And Cancellation Contract

## 1. Scope / Trigger

Apply this contract to renderer stores that start a long-running IPC job, merge backend progress events, expose cancellation, or can be reached from multiple UI entry points. It prevents an older event or promise from overwriting a newer Central update or portability job.

## 2. Signatures

```ts
interface CentralSkillUpdateJob {
  jobId: string | null;
  phase: "checking" | "updating" | null;
  status: CentralSkillUpdateJobStatus;
}

interface SkillportStatePortabilityJob {
  jobId: string | null;
  phase: SkillportStatePortabilityPhase;
  status: SkillportStatePortabilityJobStatus;
}

interface CentralSkillUpdateProgressPayload { jobId: string; /* ... */ }
interface SkillportStatePortabilityProgressPayload { jobId: string; /* ... */ }
```

All affected entries in `src/lib/ipc/commandMap.ts` type `jobId: string` for their start and cancel arguments.

## 3. Contracts

- Generate one job ID immediately before start with `crypto.randomUUID()` and a non-empty local fallback; write it into running state before `invoke`.
- Pass that captured ID to the start command. Cancellation reads the current store ID and sends exactly `{ jobId }`; a null ID is a no-op.
- `mergeUpdateProgress` and `mergePortabilityProgress` return the current object unchanged when `payload.jobId !== current.jobId`.
- Every post-`await` success, failure, or terminal state write is conditional on the captured ID still matching current state.
- A same-store start while its family is running/cancelling rejects with the stable busy envelope instead of replacing active state. The backend registry remains authoritative across stores and UI entry points.
- Update Center apply generates and passes a Central update job ID even though that view does not expose cancel.
- Stores retain raw backend errors for diagnostics. Visible Central workflow, Update Center, Skill detail, and portability surfaces render them through `formatBackendError`; coded envelopes must not be shown directly.

## 4. Validation & Error Matrix

| Condition | Renderer behavior |
| --- | --- |
| Progress `jobId` matches current state | Merge counters, item state, phase, and status |
| Progress `jobId` is stale or unknown | Return current state unchanged |
| Older invoke settles after a successor | Ignore its success/error state write |
| Same-store duplicate start | Reject with family busy envelope; preserve active job |
| Cancel with current non-null ID | Set cancelling only for that ID and invoke cancel with it |
| Cancel with null ID | No invoke, no state change |
| Coded backend error reaches visible UI | Localize with `formatBackendError` in current locale |

## 5. Good / Base / Bad Cases

- Good: B starts after A; a delayed terminal event and rejected promise from A leave B untouched.
- Base: a matching event advances only the active job and its item counters.
- Bad: a listener merges every event by family name, allowing A's completion to mark B completed.

## 6. Tests Required

- Store tests prove stale update and portability events preserve object identity/state, while matching events merge.
- Assert every start invoke and both cancel invokes carry the current job ID.
- Assert duplicate starts do not issue a second invoke or replace active state.
- Assert Update Center apply generates `jobId` and visible English/Chinese errors do not leak `code:summary` envelopes.
- Run focused Vitest, `pnpm typecheck`, `pnpm lint`, IPC coverage, and `just ci`.

## 7. Wrong vs Correct

```ts
// Wrong: any event from the family mutates current state.
set((state) => ({ updateJob: merge(state.updateJob, event.payload) }));

// Correct: the merge boundary rejects stale correlation IDs.
if (payload.jobId !== current.jobId) return current;
return mergeMatchingProgress(current, payload);
```
