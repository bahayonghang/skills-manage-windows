# Optimize skill detail metadata panel

## Goal

Make the skill detail right sidebar easier to scan by separating user-facing
source information from internal/local path diagnostics, while adding a clear
styled affordance for opening the local skill folder and a direct GitHub
repository link when repository metadata is available.

## User Value

- Users can understand where a skill came from without reading repeated path
  rows.
- Users can open the local skill folder and source GitHub repository directly
  from the detail panel.
- Technical path data remains available when it is useful for debugging, but it
  does not dominate the primary metadata view.

## Confirmed Facts

- `SkillDetailSidebar` currently renders `file_path`, `dir_path`, optional
  `canonical_path`, optional `source_root`, optional raw `source`, optional
  `repository.name`, optional `source_path`, and `scanned_at` as separate rows.
- `canonical_path` is not an arbitrary extra field. It is the Central canonical
  storage path returned by the backend and used by install/project flows.
- For normal Central skills, `canonical_path` is often visually equivalent to
  the directory containing `SKILL.md`, so showing it next to `file_path` and
  `directoryPath` creates noise.
- GitHub-imported skills store raw `source` as `github:<owner>/<repo>`, while
  repository metadata separately stores `owner`, `repo`, `branch`, `url`, and
  per-skill `source_path`.
- `repository` and `source_path` are not the same field: repository identifies
  the GitHub repo, and source path identifies the skill directory inside that
  repo.
- Existing frontend patterns already use external `<a href target="_blank"
  rel="noreferrer">` links for repository URLs in Settings.

## Requirements

- Keep the sidebar i18n-backed for all new or changed user-visible text.
- Preserve the backend `canonical_path`, `source`, `repository`, and
  `source_path` data contracts. This task should change the presentation unless
  a later implementation pass proves a frontend-only approach is insufficient.
- Replace the flat metadata list with clearer grouping:
  - Primary local location: show the directory/path needed for local action and
    the styled Open folder or copy-remote-path action.
  - Source provenance: for GitHub repositories, show a compact repository label
    and optional source subpath, plus an Open GitHub repo link.
  - Technical details: keep raw `file_path`, `canonical_path`, raw `source`,
    and scan timestamp available behind a collapsed details affordance by
    default.
- Suppress or merge duplicate-looking rows in the primary view:
  - Do not show `canonical_path` as a first-class row when it equals the local
    skill directory already shown.
  - Do not show raw `source` as a first-class row when GitHub repository
    metadata is available.
- Style the Open folder button so it reads as a deliberate utility action, not
  an unstyled incidental button.
- Add a direct GitHub repo link when `detail.repository` has either `url` or
  `owner` plus `repo`. Prefer `repository.url`, fallback to
  `https://github.com/<owner>/<repo>`.
- Keep remote target behavior intact: remote targets copy paths rather than
  attempting to open a remote file manager.

## Acceptance Criteria

- [ ] The skill detail sidebar no longer presents `file_path`,
      `directoryPath`, `canonical`, raw `source`, `repository`, and
      `sourcePath` as an undifferentiated stack for GitHub Central skills.
- [ ] For GitHub-backed skills, the sidebar shows a repository label and an
      Open GitHub repo link that opens the repo URL in a browser.
- [ ] The repository subpath remains visible when `source_path` is present.
- [ ] Raw `file_path`, `canonical_path`, raw `source`, and scan timestamp are
      hidden in a collapsed Technical details section by default and become
      visible when expanded.
- [ ] The local folder action is visually styled, keyboard accessible, and still
      uses the existing `open_in_file_manager` flow for local targets.
- [ ] Remote targets still show/copy the remote folder path instead of trying
      to open it locally.
- [ ] All changed copy is present in both `src/i18n/locales/en.json` and
      `src/i18n/locales/zh.json`.
- [ ] Frontend validation runs at minimum: `pnpm typecheck`, `pnpm lint`, and a
      focused Vitest test if existing tests cover this detail/sidebar surface.
- [ ] Final verification includes `just ci`.

## Out Of Scope

- Changing the database schema.
- Renaming backend fields or removing `canonical_path`.
- Adding GitHub source-path deep links unless explicitly requested after the
  repo-level link is planned.
- Redesigning the full skill detail drawer beyond the metadata sidebar block.

## Notes

- Screenshot diagnosis: the current right sidebar reads like a debug dump. The
  problem is not that every field is useless, but that local paths and GitHub
  source provenance are mixed at the same visual priority.
- Product decision: technical path rows should be collapsed by default. The main
  view should show only local directory, Open folder/copy path action, GitHub
  repository, and repository-internal source path.
