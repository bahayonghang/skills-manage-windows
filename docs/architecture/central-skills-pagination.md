# Central Skills Pagination

`get_central_skills_page` keeps its existing IPC request and response shape,
but the backend now pages before relation hydration:

```text
CentralSkillsPageRequest
        |
        v
central_skills service
  trim/dedupe/typed normalization
  resolve shared-root agent state once
        |
        v
skills repository
  shared bound predicates
  one read snapshot: COUNT(*) + ordered SELECT/LIMIT/OFFSET
        |
        v
page-only enrichment (0..=500 skill IDs)
  installations + repository assignments + tags
        |
        v
CentralSkillsPage { items, total }
```

The SQL query uses literal ASCII-case-insensitive substring matching and
correlated `EXISTS` predicates for source, tag, and installation state. Stable
ordering ends with the binary name and skill ID, so page boundaries do not
move when primary keys tie.

Paginated timestamps are a persisted scanner snapshot:
`fs_created_at`/`fs_updated_at`, with `scanned_at` as the null fallback. The
same expressions drive SQL ordering and response display. This page path does
not read filesystem metadata; detail and legacy unpaged APIs retain their
best-effort filesystem behavior.

Dynamic enrichment reads are chunked at 500 IDs. With the current maximum page
size they use one query per relation type, while the defensive chunking keeps
the helpers within the SQLite bind budget if reused by a larger caller.
