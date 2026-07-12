# Design

## Architecture

Extend the existing GitHub import preview pipeline instead of adding a second discovery path.

Current flow:

`repo URL -> resolve_repo_source -> download_repo_snapshot -> discover_skill_manifests -> build_remote_skill_candidate -> build_preview_skills -> GitHubRepoImportWizardPreview`

New flow:

`repo snapshot -> parse plugin manifests from the effective source root -> discover skills with manifest-declared path hints plus existing heuristics -> attach optional plugin_name by source_path -> preview DTO -> grouped UI when present`

## Backend Contract

Add optional grouping metadata to the preview-only candidate types:

- `RemoteSkillCandidate.plugin_name: Option<String>`
- `GitHubSkillPreview.plugin_name: Option<String>` serialized as `pluginName`, annotated `#[serde(default, skip_serializing_if = "Option::is_none")]` following the existing `preview_workspace_id` pattern so payloads and fixtures without the field keep deserializing.

The value is the raw manifest plugin name, for example `mattpocock-skills`. Formatting for display belongs in the frontend.

Do not add this field to:

- `GitHubSkillImportSelection`
- `ImportedGitHubSkillSummary`
- `GitHubRepoImportResult`
- central skill database rows or source metadata
- update/sync state

## Manifest Parsing

Add a small parser in `src-tauri/src/services/github_import/source.rs` or a focused sibling module if the code grows.

The parser works against the effective source root:

- No URL subpath: repo root.
- URL subpath: that subpath acts as the source root, matching `npx skills` behavior where manifest discovery starts from the requested search path.

It reads these files relative to that effective root:

- `.claude-plugin/plugin.json`
- `.claude-plugin/marketplace.json`

Supported `plugin.json` shape:

```json
{
  "name": "mattpocock-skills",
  "skills": ["./skills/engineering/ask-matt"]
}
```

Supported `marketplace.json` shape:

```json
{
  "metadata": { "pluginRoot": "./plugins" },
  "plugins": [
    {
      "name": "docs",
      "source": "./docs-plugin",
      "skills": ["./skills/write-docs"]
    }
  ]
}
```

Path resolution (mirrors `vercel-labs/skills` #259):

- `plugin.json`: each skill path resolves against the effective source root.
- `marketplace.json`: `plugin_dir = normalize(effective_root / pluginRoot? / source)`; each skill path resolves against `plugin_dir`, not the repo root.
- `pluginRoot`, string `source`, and skill paths accept local relative values with or without a `./` prefix (upstream's own examples use bare `source` values). Normalization goes through the existing `join_repo_path`/`normalize_repo_path` helpers, which already reject traversal, absolute paths, and backslash separators.
- Object/remote `source` entries are skipped.

Rules:

- A manifest skill path maps to the discovered skill `source_path`, not to `SKILL.md`.
- Invalid JSON or invalid entries are ignored; preview stays best-effort.
- Best-effort extends to the candidate level: a hint only enters discovery when `<path>/SKILL.md` exists in the file set, and a hint-derived candidate that fails validation (UTF-8, frontmatter, skill id) is dropped silently — never recorded as an invalid candidate. This matters because `build_repo_skill_candidates_from_snapshot_at_path` and the remote workspace builder hard-fail the whole preview/import on any invalid candidate; hard-fail semantics stay reserved for heuristic discoveries.
- Manifest paths are annotations plus discovery hints, not filters. Unconfigured valid skills discovered by the existing heuristics and recursive fallback stay visible.
- Explicit hints bypass `SKIP_DISCOVERY_DIRS` (explicit manifest intent beats the heuristic skip list), matching upstream behavior.

The parser should return a structure similar to:

```rust
struct PluginManifestDiscovery {
    explicit_skill_paths: Vec<String>, // manifest declaration order, deduplicated
    plugin_by_source_path: HashMap<String, String>,
}
```

`explicit_skill_paths` feeds discovery so configured deep paths can be found deterministically. `plugin_by_source_path` feeds preview grouping.

## Discovery Ordering

`discover_skill_manifests_from_paths` keeps its current stages untouched and appends hints last:

1. Direct root manifest plus `PRIORITY_SKILL_ROOTS` immediate children — unchanged.
2. Recursive fallback — its trigger stays "heuristic results are empty". Hint availability must not enter this check; otherwise repositories that rely on recursive fallback (the `mattpocock/skills` shape) would lose their unlisted skills.
3. Manifest hints — appended after stages 1–2 in manifest declaration order, deduplicated through the existing `seen_source_paths` set.

Because hints are appended last, repositories without manifests keep byte-identical candidate lists, and the `seen_names` first-wins duplicate rule keeps preferring heuristic discoveries over hint-only ones.

## Local and Remote Parity

Local preview/import uses `GitHubRepoSnapshot`, so manifest JSON contents are already available.

SSH preview/import builds candidates from an extracted remote workspace. To avoid local/remote drift, it must use the same manifest interpretation rules:

- collect all remote `SKILL.md` paths as today
- read `.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json` from the effective source root when present
- bound each manifest read with `ResourceBudget::default_skill().reject_file_read_size`, matching the existing remote `SKILL.md` reads; a failed or over-budget read is treated as an absent manifest
- feed the same explicit path and grouping maps into candidate discovery

Best-effort behavior still applies: unreadable or invalid remote manifests are ignored, but normal skill discovery continues.

## Shared Pipeline Blast Radius

`fetch_repo_skill_candidates_from_source` and `build_repo_skill_candidates_from_snapshot_at_path` are also used by marketplace GitHub source sync (`services/marketplace/mod.rs::fetch_github_skills`, whose doc comment pins "stay in sync" with the import flow) and skills.sh install (`services/marketplace/skills_sh.rs`). Hint discovery therefore also surfaces manifest-declared skills in those flows — intended, since it keeps Marketplace listings and import previews consistent. The drop-invalid-hints rule above is what guarantees a broken manifest entry cannot fail marketplace source sync or skills.sh resolution; marketplace tests run as part of validation.

## Frontend Contract

Extend `GitHubSkillPreview` in `src/types/index.ts` with:

```ts
pluginName?: string | null;
```

In `GitHubRepoImportWizardPreview.tsx`, derive grouped display data:

- If no skill has `pluginName`, render the existing flat list.
- If any skill has `pluginName`, group by formatted plugin name and place ungrouped skills in the localized `Other` section.
- Preserve selection state keys by `sourcePath`; grouping is display-only.
- Preserve preview list order within each group as returned by the backend; do not sort skills in a way that changes current selection defaults.
- Auto-selection stays derived from the flat `preview.skills` order (`selectedPreviewSkill` defaults to `preview.skills[0]` in the view model); grouped rendering may display a different item first, which is acceptable.

## Compatibility

This is additive JSON. Existing frontend tests and browser fixtures that omit `pluginName` should keep working if the field is optional.

No database migration is required because grouping is not persisted.

Existing import calls should naturally drop `pluginName` because selections still contain only `sourcePath`, `resolution`, and optional `renamedSkillId`.

## Risks

- A manifest may declare paths with platform separators or traversal. Mitigation: the existing repo-path normalization helpers already reject traversal, absolute paths, and backslash separators.
- Manifest hints could suppress recursive fallback and hide unlisted skills. Mitigation: the fallback trigger is evaluated on heuristic results only; hints are appended after heuristic and fallback discovery (see Discovery Ordering).
- A hint pointing at a broken skill could hard-fail preview, import, or marketplace sync, because the `build_*` paths error on any invalid candidate. Mitigation: hint-derived candidates that fail validation are dropped silently and never recorded as invalid candidates.
- Local and SSH preview paths could diverge. Mitigation: remote workspace import reuses the same grouping helper, with budget-bounded remote manifest reads treated as absent on failure.
- Discovery changes propagate to marketplace source sync and skills.sh install through the shared pipeline. Mitigation: hints are additive-only, invalid hints are dropped, and marketplace tests run in validation.

## Rollback

The change is additive. Rollback removes the optional field, manifest parser, and grouped rendering while leaving existing flat preview discovery intact.
