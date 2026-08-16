# Skill Usage Analytics Contract

## 1. Scope / Trigger

Read this spec before changing `services/usage`, usage DB tables, provider refresh,
the unused-skills report, or the usage Tauri commands. It protects three invariants:
provider logs remain immutable facts, skill identity is never guessed, and calendar
buckets follow the machine running SkillPort rather than UTC.

## 2. Signatures

```text
skill_calls(target_id, skill, timestamp_ms, project, session_id, source)

skill_usage_metadata(
  target_id, skill, match_status,
  resolved_skill_id?, static_token_estimate?, static_byte_count?, scanned_at_ms
)
PRIMARY KEY (target_id, skill)

usage_get_overview(topSkillsLimit?, source?) -> UsageOverview
usage_get_recent(limit?, source?) -> RecentSkillCall[]
usage_get_skill_detail(skill, source?) -> SkillUsageDetail
usage_resolve_skill_id(skillName) -> string?
usage_get_unused_skills(source?, thresholdDays?) -> UnusedSkillsReport

UnusedAgentInstall {
  agentId, linkType, installedPath, hasPendingRecovery
}
UnusedPlatformInstall {
  agentId, rowId?, skillId, linkType, sourceKind?, isReadOnly,
  installedPath, hasPendingRecovery
}
```

`replace_calls_for_target` receives calls, provider outcomes, metadata, and the
scan timestamp; it replaces all four target-scoped cache families in one SQL
transaction.

## 3. Contracts

- `skill_calls` stores only facts proven by logs. Never add a resolved skill id,
  file path, static Token estimate, or current Central-library interpretation.
- Resolve metadata against `skills.is_central = 1`: normalized exact id first,
  then a unique normalized name. Multiple normalized matches are `ambiguous`;
  no match is `unmatched`. Only `matched` rows have `resolved_skill_id`.
- Read matched `SKILL.md` files through the active `Scope::fs_backend()` batch
  API. Apply `ResourceBudget::default_skill()` after reading. A missing,
  unreadable, non-UTF-8, or oversized file leaves both static metrics `NULL` but
  does not fail the usage scan.
- Static Token is a Skill.md content-size estimate: CJK characters count 1:1;
  other non-whitespace characters count at about 3.8:1, rounded up. It is never
  task Token consumption.
- SQL returns raw `timestamp_ms`. `SystemLocalDayResolver` converts every event
  independently with `Utc -> Local`; do not capture one fixed offset because DST
  may change inside the 16-week window. Invalid timestamps are skipped.
- Fixed horizons: overview KPI/ranking = all recorded history; heatmap = 112
  local calendar days arranged as 16 weeks; recent = latest 20; skill-card badges
  remain the existing 30-day query.
- `source` is optional but, when present, must filter overview, recent, detail
  summary, project distribution, and detail heatmap consistently.
- The unused report reads usage facts from the always-local usage pool and inventory
  facts from the already-resolved active-target skills pool. `target_id` scopes calls,
  metadata, and pending recovery; the report must not re-resolve ambient target state.
- Central entries expose one `UnusedAgentInstall` per `skill_installations` row.
  Platform entries use `agent_skill_observations` as authority and expose one
  `UnusedPlatformInstall` per observation, including the exact `rowId` required by
  row-aware unlink. Read-only/plugin observations remain visible but are not writable.
- Platform observations with the same normalized skill name share usage statistics.
  Preserve every observation in `installs`: one agent can have both a writable user
  copy and a read-only plugin copy, and collapsing either row loses action authority.
- `hasPendingRecovery` is derived from pending FS/DB operations for the same target and
  skill id. The renderer uses it only to disable the action; the installation service
  repeats the authoritative guard before mutation.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| duplicate normalized Central names | `ambiguous`, no id, no file read |
| no Central candidate | `unmatched`, no id, no file read |
| matched file missing/oversized | keep `matched`; static metrics are `NULL` |
| provider collect fails | provider unavailable; other provider facts persist |
| metadata insert violates CHECK | entire target replacement rolls back |
| remote target unreachable | do not replace cache; command returns last complete cache when present |
| timestamp cannot be parsed | omit it from heatmap; never create a 1970 bucket |
| platform observation is read-only or plugin-owned | include it with row metadata; renderer disables unlink |
| same agent has user and plugin observations for one normalized name | return both `installs`; do not duplicate or discard facts in the repository layer |
| pending recovery exists for the target/skill | set `hasPendingRecovery=true`; mutation still fails closed in installation service |

## 5. Good / Base / Bad Cases

- Good: `" REVIEW "` uniquely matches Central id `review`, reads its current
  target-aware Skill.md, and records matched metadata with estimates.
- Base: an old database has no metadata rows; overview remains usable with
  `unmatched` and unavailable static metrics until the next successful scan.
- Good: Codex has user and plugin observations for `review`; one platform report entry
  contains both rows, retaining their distinct `rowId`, source, and read-only state.
- Bad: two Central rows normalize to `review`; selecting the first by sort order
  would create a false navigation target and is forbidden.
- Bad: use `skill_installations` alone for platform entries; loose native directories
  and plugin ownership/read-only facts would disappear.

## 6. Tests Required

- Schema/repo: table creation, target isolation, CHECK constraints, and rollback
  preserving old calls + metadata.
- Enrichment: exact id, unique name, ambiguous name, unmatched, trim/case, ASCII,
  CJK, mixed content, empty content, missing file, and over-budget file.
- Aggregation: Asia/Shanghai crossing a UTC boundary, an injectable resolver that
  changes offsets between events, invalid timestamps, and exactly 112 cells.
- Detail: source filtering and `COUNT(DISTINCT session_id)` per project.
- Command/IPC: Rust camelCase serialization and matching TypeScript command map.
- Unused report: target/source isolation, Central per-agent link metadata, platform
  `rowId`/source/read-only fields, pending-recovery flags, and same-agent user/plugin
  observations preserved in one normalized entry.

## 7. Wrong vs Correct

```rust
// Wrong: UTC SQL grouping and arbitrary Central navigation.
strftime("%Y-%m-%d", timestamp_ms / 1000, "unixepoch");
SELECT id FROM skills WHERE LOWER(name) = LOWER(?) LIMIT 1;

// Correct: raw facts -> per-event local resolver; unique metadata only.
let timestamps = db::list_timestamps_since(...).await?;
let heatmap = heatmap_grid_16w_from_timestamps(
    &timestamps,
    now_ms,
    &SystemLocalDayResolver,
);
```
