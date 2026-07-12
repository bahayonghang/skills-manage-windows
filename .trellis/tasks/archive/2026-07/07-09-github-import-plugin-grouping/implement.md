# Implementation Plan

## Checklist

1. Backend data model
   - Add `plugin_name: Option<String>` to `RemoteSkillCandidate`.
   - Add `plugin_name: Option<String>` to `GitHubSkillPreview`.
   - Thread the field through `build_preview_skills`.

2. Backend manifest grouping
   - Implement best-effort parsing for `.claude-plugin/plugin.json` (skill paths relative to the effective source root).
   - Implement best-effort parsing for `.claude-plugin/marketplace.json`, resolving skill paths against `plugin_dir = effective_root / pluginRoot? / source`; accept bare relative values without a `./` prefix; skip object/remote sources.
   - Parse manifests relative to the effective source root, not always the repo root.
   - Normalize local manifest paths to discovered `source_path` keys via the existing `join_repo_path`/`normalize_repo_path` helpers.
   - Append manifest hints after heuristic and recursive discovery; the recursive-fallback emptiness check must ignore hints (evaluate on heuristic results only).
   - Only accept a hint when `<path>/SKILL.md` exists in the file set; drop hint-derived candidates that fail validation instead of recording invalid candidates (the `build_*` paths hard-fail on any invalid candidate).
   - Explicit hints bypass `SKIP_DISCOVERY_DIRS`.
   - Attach grouping in snapshot preview/import candidate construction.
   - Cover remote workspace candidate construction by reading manifest files from the remote checkout (budget-bounded with `reject_file_read_size`, treated as absent on read failure) and feeding the same helper used by snapshot construction.
   - Serialize the new preview field with `#[serde(default, skip_serializing_if = "Option::is_none")]`.

3. Backend tests
   - Add a `plugin.json` fixture where listed and unlisted skills both exist.
   - Anti-suppression: fixture with no priority-root skills where the manifest lists only a subset of deep skills; assert unlisted deep skills are still discovered via recursive fallback (extend the `compound_plugin_like_snapshot` family).
   - Add a `marketplace.json` fixture asserting plugin-dir-relative skill resolution (`pluginRoot` + bare `source` without `./`).
   - Add a mixed-layout fixture where a manifest-declared deep skill is found even though legacy priority discovery would otherwise stop before recursive fallback.
   - Broken hints: a manifest entry with no `SKILL.md`, and one whose `SKILL.md` has invalid frontmatter, are dropped while `build_repo_skill_candidates_from_snapshot_at_path` still succeeds.
   - Add invalid path cases: traversal, absolute path, and remote/object source.
   - Assert `GitHubRepoImportResult` / imported summaries do not carry `pluginName`.

4. Frontend types and rendering
   - Add optional `pluginName` to `GitHubSkillPreview`.
   - Derive grouped preview sections in the preview component or nearby view model.
   - Render flat list when there are no groups.
   - Render grouped sections plus localized `Other` when groups exist.
   - Add i18n keys for the fallback group label if needed.
   - Keep import selection payload unchanged.

5. Frontend tests
   - Update preview fixtures as needed.
   - Add tests for grouped preview and flat fallback.
   - Verify selection/rename/conflict controls still use `sourcePath` and work inside groups.
   - Verify confirm/import payloads do not include `pluginName`.

6. Validation
   - Run targeted Rust tests for `github_import`.
   - Run targeted frontend GitHub import preview tests.
   - Run `just ci`.

## Likely Files

- `src-tauri/src/services/github_import/source.rs`
- `src-tauri/src/services/github_import/types.rs`
- `src-tauri/src/services/github_import/import.rs`
- `src-tauri/src/services/github_import/tests.rs`
- `src/types/index.ts`
- `src/components/marketplace/GitHubRepoImportWizardPreview.tsx`
- `src/test/*github*import*`
- `src/i18n/locales/en.json`
- `src/i18n/locales/zh.json`
- Watch only (shared pipeline consumers, no intended edits): `src-tauri/src/services/marketplace/mod.rs`, `src-tauri/src/services/marketplace/skills_sh.rs`

## Review Gates

- Confirm manifest grouping annotates but does not filter discovery.
- Confirm ungrouped skills remain visible.
- Confirm the recursive-fallback trigger ignores manifest hints (no suppression of unlisted deep skills).
- Confirm hint-derived invalid candidates are dropped silently, never hard-failing preview/import.
- Confirm marketplace source sync and skills.sh flows stay non-regressive.
- Confirm no component directly invokes Tauri.
- Confirm UI text additions are localized.

## Validation Commands

```powershell
cd src-tauri; cargo test github_import
cd src-tauri; cargo test marketplace
pnpm test -- GitHubRepoImportWizard MarketplaceView.github-preview
just ci
```
