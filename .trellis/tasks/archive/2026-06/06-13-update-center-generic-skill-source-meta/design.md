# Design

## Boundaries

This task spans:

- Backend candidate discovery/import planning under `src-tauri/src/services/github_import/`.
- Update Center repository sync inventory under `src-tauri/src/commands/central_updates/` and `src-tauri/src/commands/skill_update_inventory/`.
- Update Center source metadata rendering under `src/components/central/updateCenter/SourceMeta.tsx`.
- Focused Rust and React tests.

It should not touch unrelated Central card rendering, skill detail work, or existing dirty files from the separate `skill-detail-repo-link-platform-installs` task.

## Backend Filtering

Use a shared helper for generic remote candidate rejection, for example:

- input: final normalized candidate skill id plus source path
- rule: reject/exclude when `skill_id == "skill"`
- reason: `skill` is a generic container name, not a stable Central skill id

The safest insertion point is after the final skill id is derived in `build_remote_skill_candidate`, because every discovery path already flows through this normalization. Implementation can either:

- return an invalid candidate with reason such as `generic_skill_id`, or
- return a valid candidate and filter it from inspected results before `remote_added` / import planning.

Recommended approach: make the shared discovery layer classify it as an invalid/non-importable candidate, then make Update Center ignore this reason as a quiet filtered candidate instead of surfacing it as a failed repository. This preserves a single source of truth while avoiding user-facing noise for expected filtering.

## Update Center Data Flow

Relevant flow:

1. `refresh_skill_update_inventory` prepares repository snapshots.
2. `collect_remote_added_skills` calls `inspect_repo_skill_candidates_from_snapshot_at_path`.
3. Valid candidates are converted to previews and persisted to pending additions.
4. `RemoteAddedTabPanel` displays those additions.
5. Applying decisions imports selected additions via GitHub import planning.
6. Force mirror imports additions from repository snapshots as a rescue path.

Filtering must prevent generic `skill` candidates from reaching steps 3, 5, and 6.

## UI Styling

Keep `SourceMeta` as the only rendering point for source metadata chips. Add a small keyed style map for row keys:

- `repository`: repository/source color treatment
- `path`: filesystem/path color treatment
- `url`: external URL color treatment
- `hash`: checksum/version color treatment
- `cache`: neutral or secondary treatment

The visual style should remain compact and compatible with the existing dark Update Center panel. Prefer existing Tailwind tokens and opacity variants over hard-coded one-off colors where possible. Long values should keep `break-all`, `min-w-0`, and title tooltips.

## Compatibility

- Root `SKILL.md` imports remain valid because the id is derived from the repository name, not `skill`.
- Legitimate explicit skills with specific names continue to import/update normally.
- Blocking exact `skill` is intentionally stricter than path-only filtering; it prevents any generic candidate from entering Central, regardless of where recursive discovery found it.
- Existing pending rows for `skill`, if already persisted, are not deleted automatically by this task unless the refreshed inventory clear path already removes stale additions for the repository. Existing Central rows are not deleted.

## Risks

- If a real published skill intentionally uses id `skill`, this blocks it. The tradeoff is acceptable for this product because Central skill IDs must be descriptive and generic `skill` rows are already causing pollution.
- If invalid candidates are surfaced as failed repositories, the UI may become noisier. Tests should pin the desired quiet-filter behavior.
- Styling assertions should avoid brittle full-class snapshots; prefer targeted data attributes or class-token checks on row kinds.
