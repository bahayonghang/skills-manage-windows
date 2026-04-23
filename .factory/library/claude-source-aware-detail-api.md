## Claude source-aware platform/detail API

Backend contract added for the `claude-source-aware-platform-detail-api` feature:

- `get_skills_by_agent("claude-code")` now returns row-level metadata per Claude observation:
  - `row_id`
  - `source_kind` (`user` or `marketplace`)
  - `source_root`
  - `is_read_only`
  - `conflict_group`
  - `conflict_count`
- Non-Claude rows also expose `row_id`, using the logical `skill_id`; the Claude-specific metadata fields stay empty/defaulted.

- `get_skill_detail` now accepts optional `agent_id` and `row_id` arguments.
  - When called with `agent_id: "claude-code"` and a Claude `row_id`, detail resolves that exact observation row instead of collapsing to the logical `skill_id`.
  - If `agent_id: "claude-code"` is provided without `row_id`, unique Claude rows still resolve, but duplicate rows return an error requiring `row_id`.

- `SkillDetail` now includes:
  - `row_id`
  - `dir_path`
  - `source_kind`
  - `source_root`
  - `is_read_only`
  - `conflict_group`
  - `conflict_count`

- Marketplace Claude detail rows are observational only:
  - `installations` is empty
  - `collections` is empty
  - `canonical_path` is `null`

Frontend follow-up: for duplicate Claude rows, call `get_skill_detail({ skillId, agentId: "claude-code", rowId })`, then load content from `read_file_by_path(detail.file_path)` so the selected row’s content/path stays source-aware.

Update from `claude-platform-ux` scrutiny rerun:

- The AI explanation surface is now row-aware for duplicate Claude rows.
  - `SkillDetailView` derives the explanation request key from the selected row identity (`detail.row_id ?? rowId ?? skillId`) before loading cached explanations or starting generate/refresh requests.
  - The existing store/backend explanation cache and stream paths continue to key by the passed `skill_id`, so using Claude `row_id` cleanly separates `user` and `marketplace` explanation state without changing non-Claude behavior.
  - Result: duplicate Claude source rows now keep explanation text scoped to the selected row instead of leaking cached explanations across sources.
