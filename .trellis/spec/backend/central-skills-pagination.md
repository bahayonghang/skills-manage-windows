# SQL-Backed Central Skill Pagination

## 1. Scope / Trigger

This contract applies to `get_central_skills_page`, its service normalization,
repository query, relation enrichment, and timestamp display semantics. The
unpaged Central list and skill detail APIs remain separate compatibility paths.

## 2. Request Contract

- Normalize `query` with trim plus ASCII lowercase. Empty query is absent.
- Normalize source and tag filters by trimming, removing empty/`all` values,
  and deduplicating while preserving order.
- Source and tag filters each allow at most 100 normalized unique values.
  Exceeding the limit returns `CentralSkillsError::PageFilterValuesExceeded`.
- Offset defaults to 0 and clamps negative values to 0. Limit defaults to 100
  and clamps to `1..=500`.
- Install aliases are `linked|installed` and
  `unlinked|not_installed|notInstalled`; unknown values mean all.
- Sort fields are `name`, `createdAt|created_at`, and
  `updatedAt|updated_at`, with `asc|desc`. Any malformed or unknown sort falls
  back to `name:asc`.

## 3. Repository Query Contract

- SQLite owns filter, count, order, limit, and offset. Count and page queries
  share one short read transaction and predicate builder, and bind every user
  value.
- Text search uses `instr(lower(column), normalized_query)` across name,
  description, and id. `%`, `_`, and backslash are literal characters.
- Source values have OR semantics. `unassigned` matches a missing assignment,
  an unknown repository, or a member that cannot resolve to a known repository.
- Tag values have OR semantics. `uncategorized` matches no valid tags or only
  the system `uncategorized` tag.
- Linked means a direct installation exists or at least one agent shares the
  Central root. With a shared-root agent, every Central skill is linked.
- Order is stable. Name uses `lower(name), name, id`; time uses the persisted
  time key, then `lower(name), name, id`. Descending reverses the full tuple.

## 4. Timestamp And Enrichment Contract

- Paginated created/updated timestamps use only persisted
  `fs_created_at`/`fs_updated_at`, falling back to `scanned_at`. Page sorting and
  displayed values therefore share the same snapshot authority and never stat
  the filesystem.
- The service resolves agents once, fetches `(page rows, total)` from the
  repository, and enriches only those page rows.
- Empty pages do not issue dynamic relation `IN` queries. Installation,
  repository, and tag batch readers chunk dynamic `IN` lists at 500 IDs.
- The page command/DTO name and frontend response shape remain unchanged.

## 5. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| More than 100 normalized source or tag values | Return `CentralSkillsError::PageFilterValuesExceeded`; issue no page query |
| Unknown install-state or malformed sort | Preserve compatibility by falling back to all installs or `name:asc` |
| Negative offset or out-of-range limit | Clamp offset to 0 and limit to `1..=500` before SQL construction |
| Offset is beyond the filtered result set | Return an empty page with the full filtered `total`; issue no relation `IN` query |
| A shared-root agent exists | Treat every Central skill as linked; an unlinked filter returns no rows |
| Persisted filesystem timestamps are null | Sort and display with `scanned_at`; do not stat the filesystem |
| Count or page query fails | Roll back the read transaction and return the original `sqlx::Error` |

## 6. Good / Base / Bad Cases

- Good: a 5,000-row Central library returns page 101 with 25 stable IDs, a
  total of 5,000, and relation enrichment input limited to those 25 IDs.
- Base: an empty or default request returns the first name-sorted page with the
  existing DTO shape and compatibility defaults.
- Good: mixed repository/tag filters retain OR semantics, while different
  filter families combine with AND semantics and shared-root installs remain
  visible as linked.
- Bad: loading or enriching the full Central library before `skip/take`, using
  `LIKE` for literal substring search, or deriving page timestamps from live
  filesystem metadata.

## 7. Tests Required

- SQL/reference equivalence across query, source, tag, install, sort, offset,
  and limit combinations.
- Literal `%`, `_`, and backslash queries; source/tag special values; install
  aliases and shared-root behavior; legacy null timestamps; stable id ties.
- Typed failure for 101 unique source or tag values and success for duplicates.
- A deterministic 5k fixture proving page size 25 passes exactly 25 rows to
  enrichment, plus 501-ID relation reads proving bounded chunking.
- `EXPLAIN QUERY PLAN` evidence for name/time/source/tag/install/contains and a
  release benchmark with warm-up and p50/p95. Wall-clock values are evidence,
  not CI thresholds.

## 8. Wrong vs Correct

```rust
// Wrong: full hydration and filesystem fallback happen before pagination.
let mut items = get_central_skills_impl(pool).await?;
items.retain(matches_request);
let page = items.into_iter().skip(offset).take(limit).collect();

// Correct: SQLite pages one read snapshot, then only page rows are enriched.
let (rows, total) = db::get_central_skills_page(pool, &filter).await?;
let items = skills_with_links_from_rows(pool, rows, TimestampMode::Persisted).await?;
```
