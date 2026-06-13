# Skill Detail Metadata Panel Design

## Boundaries

- Frontend owner: `src/components/skill/SkillDetailSidebar.tsx`.
- Shared row/button styling owner:
  `src/components/skill/SkillDetailViewShared.tsx` if a reusable metadata
  helper is needed.
- Store boundary: keep `src/stores/skillDetailStore.ts` as the only detail
  caller. Components must not call Tauri `invoke()` directly.
- Backend contract: no schema or command changes are expected because
  `SkillDetail.repository.url`, `owner`, `repo`, and `source_path` already
  carry the needed data.

## Data Interpretation

- `file_path`: absolute path to `SKILL.md`.
- `dir_path`: absolute local directory containing the skill.
- `canonical_path`: Central canonical storage directory. It is operationally
  meaningful, especially for installs and project links, but often duplicates
  `dir_path` visually in Central detail views.
- `source`: legacy/raw source marker such as `github:owner/repo` or link type.
- `repository`: structured source repository metadata.
- `source_path`: repository-internal skill directory path.

## Recommended Presentation

1. Local location block
   - Show a concise local directory value.
   - Place the Open folder or copy remote path action immediately under it.
   - Style the action as a full-width utility button with icon, stable height,
     left-aligned label, and hover/focus states consistent with existing
     inspector controls.

2. GitHub source block
   - Render when `detail.repository` is non-unknown and has URL information.
   - Display `owner/repo` when available, otherwise repository name.
   - Include a small external-link action: Open GitHub repo.
   - Show `source_path` as a subpath row inside the same block.

3. Technical details block
   - Keep `file_path`, `canonical_path`, raw `source`, and `scanned_at`.
   - Hide these behind a collapsed details disclosure by default to reduce
     sidebar noise.

## Tradeoffs

- Hiding technical rows improves normal scanability, but debugging requires one
  extra click. This is the accepted tradeoff for this task.
- A repo-level link is lower risk than a source-path deep link because branch
  and path URL semantics are not needed for the user's stated request.

## Compatibility

- Remote targets keep copy-path behavior.
- Unknown/local repositories should not show a GitHub link.
- Existing i18n keys should be reused where possible, with new keys added only
  for new labels.
