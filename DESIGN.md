# SkillPort Design Contract

## Repository Sync Preview Dialog

This dialog is the legacy repository-sync preview surface. It stays separate from the newer Update Center inventory flow until a dedicated migration is planned.

### Layout

- Use a desktop-sized modal: approximately `92-96vw`, max `96rem`, height around `88vh`.
- Keep the header and footer fixed inside the modal; only the middle content area scrolls.
- Put a lightweight summary area under the title before the tab content.
- The content must be organized into four tabs:
  1. Pending additions
  2. Skipped additions
  3. Remote removals
  4. Failed repositories
- Default tab priority is: pending additions, skipped additions, remote removals, failed repositories.

### Destructive action hierarchy

- Import/rename/skip decisions are the primary row action group for remote additions.
- Deleting an old local Central skill is a separate destructive secondary action. Do not place it as a fifth peer in the primary action row.
- Show the destructive action only for conflict rows where the existing Central skill id is known. If delete preview data is unavailable, show a disabled action with the reason.
- Selecting the destructive action means "delete the old Central skill only". It must not implicitly import, unskip, or otherwise process the remote skill.

### Delete behavior

- Pending addition + delete old skill: submit only `deleteRequests` for the existing Central skill. Do not submit an addition or a skip request for that remote item.
- Skipped addition + delete old skill: submit only `deleteRequests` for the existing Central skill. Keep the remembered skip state; do not submit `unskipAdditions`.
- Remote removal rows continue to support keep/delete decisions with optional copy-install removal.
- The IPC and persistence contract stays unchanged: reuse `apply_central_repository_sync`, `preview_delete_central_skills`, and `deleteRequests`.

### Copy and link preview

- Inline delete preview must show the Central path, automatically removed platform links, optional copy installs, and preview failure state.
- Optional copy installs are opt-in checkboxes; automatic link removals are informational.

### Rename validation

- Rename IDs must be validated before Apply.
- Empty or non-canonical skill ids are shown inline and disable Apply.
- Canonical skill ids are lower-case ASCII kebab identifiers: `a-z`, `0-9`, separated by single dashes.

### Copywriting

- User-visible strings must be present in both `src/i18n/locales/zh.json` and `src/i18n/locales/en.json`.
- Destructive copy must say "old skill" / "旧 skill" to avoid implying the remote skill is imported or overwritten.
