# Central pagination performance and query-plan evidence

Date: 2026-08-03 (Windows, release profile)

## Fixture and method

- One in-memory initialized application database.
- 5,000 deterministic Central skill rows, seeded in one transaction.
- Every skill has two installations, two tags, and one repository assignment.
- Request: `updatedAt:desc`, `limit=25`, `offset=2500`.
- Three warm-up runs followed by 12 measured runs. Fixture setup and release
  compilation are outside the samples.
- The test asserts `total=5000`, 25 returned items, and SQL/reference ID
  equivalence. Wall-clock results are evidence for this machine, not a CI gate.

Command:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml --release benchmark_central_pagination_large_fixture -- --ignored --nocapture
```

## Before and after

| Path | p50 | p95 | Rows passed to enrichment | Data-read statements |
| --- | ---: | ---: | ---: | ---: |
| Before route switch | 163,868 us | 167,783 us | 5,000 | 6 |
| SQL page | 25,342 us | 31,442 us | 25 | 7 |
| Test-only reference after 500-ID guards | 188,286 us | 248,114 us | 5,000 | 33 |

The before result was captured by the same ignored benchmark immediately
before changing the production route. Its six statements were Central rows,
agents, installations, repository assignments, tags, and unknown-repository
metadata; all three relation queries received 5,000 IDs.

The SQL route uses agents, count, page rows, three page relation queries, and
unknown-repository metadata. Count and page share one short read transaction;
the table excludes transaction-control statements. The route performs one
additional data read but hydrates only 25 rows. The test-only reference is
listed only for reproducibility after the relation helpers gained 500-ID
chunking; it is not presented as the pre-change measurement.

Against the true pre-change capture, the final SQL route was about 6.5x faster
at p50 and 5.3x faster at p95 on this run.

## Structural evidence

Command:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml services::central_skills::pagination -- --nocapture
```

Result: 9 passed, 1 ignored. The 5k test's observer received exactly 25 rows.
An offset beyond the result set received zero rows. Each installation,
repository, and tag helper also returned complete data for 501 requested IDs
while using the shared 500-ID chunk size.

## Query plans

The focused test prints plans from the same builder used by production:

- Legacy full load: `SEARCH skills USING INDEX idx_skills_is_central`; filter,
  sort, pagination, and relation hydration happened after this complete read.
- Name and updated-time: `SEARCH s USING INDEX idx_skills_is_central`, then
  `USE TEMP B-TREE FOR ORDER BY` because `lower`/`coalesce` expressions do not
  match the existing name index.
- Source: Central index plus correlated subqueries using
  `idx_skill_repository_members_repository_skill_id` and the member/repository
  primary-key indexes.
- Tag: Central index plus correlated subqueries using the
  `skill_tag_links(skill_id, tag_id)` and tag primary-key covering indexes.
- Install: Central index plus the
  `skill_installations(skill_id, agent_id)` primary-key covering index.
- Contains: Central index plus a temporary sort. `instr(lower(...), ?)` scans
  the Central subset as expected for literal substring search.

No index or migration was added. The measured improvement comes from avoiding
full hydration. An expression index would add persistent schema cost while not
helping contains search, and the current plans already use relation indexes for
the correlated predicates.
