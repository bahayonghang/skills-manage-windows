# GitHub Import Preview Contract

## Scenario: Plugin Manifest Grouping

### 1. Scope / Trigger

Update this spec when GitHub repository import preview changes candidate discovery,
preview DTO fields, manifest interpretation, or import persistence behavior.

This flow spans repository snapshot or remote workspace reading, backend candidate
construction, command serialization, frontend preview rendering, and final import
selection. Keep grouping metadata display-only unless a task explicitly changes
the persistence contract.

### 2. Signatures

Backend preview candidate fields:

```rust
pub(crate) struct RemoteSkillCandidate {
    pub(crate) plugin_name: Option<String>,
    // existing fields omitted
}

pub struct GitHubSkillPreview {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
    // existing fields omitted
}
```

Frontend preview DTO field:

```ts
export interface GitHubSkillPreview {
  pluginName?: string | null;
}
```

Do not add plugin grouping fields to `GitHubSkillImportSelection`,
`ImportedGitHubSkillSummary`, `GitHubRepoImportResult`, central database rows, or
source/update metadata.

### 3. Contracts

- `pluginName` is optional preview metadata only. It may be omitted, `null`, or a
  non-empty manifest plugin name.
- `.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json` are read
  relative to the effective source root, not always the repository root.
- `plugin.json` skill paths resolve against the effective source root.
- `marketplace.json` skill paths resolve against
  `effective_root / pluginRoot? / source`, and `source` must be a local string.
- Local manifest paths may be bare relative values or `./`-prefixed values.
  Traversal, absolute paths, backslashes, URLs, and object/remote sources are
  ignored.
- Manifest skill paths are additive discovery hints. They append after existing
  direct, priority-root, and recursive discovery and must not suppress recursive
  fallback for unlisted skills.
- Hint-derived candidates that are missing, over budget, invalid UTF-8, missing
  frontmatter, or otherwise invalid are dropped silently. Heuristic-discovered
  invalid candidates keep the existing hard-fail behavior.
- Remote workspace manifest reads use the same path rules and are bounded by the
  default skill `ResourceBudget`; unreadable or over-budget manifests are treated
  as absent.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Missing or invalid manifest JSON | Continue legacy discovery with no grouping |
| Manifest path has traversal, absolute path, backslash, URL, or remote object source | Ignore that path or entry |
| Manifest hint has no `SKILL.md` | Drop hint, keep preview/import working |
| Manifest hint has invalid `SKILL.md` | Drop hint, keep preview/import working |
| Same invalid skill is discovered by legacy heuristics | Preserve existing invalid-candidate failure |
| At least one preview skill has `pluginName` | Frontend renders grouped sections and puts ungrouped skills under localized `Other` |
| No preview skill has `pluginName` | Frontend renders the existing flat preview list |
| User imports grouped preview | Import selection payload stays flat and keyed by `sourcePath` |

### 5. Good / Base / Bad Cases

- Good: `plugin.json` lists two deep skills, recursive fallback also finds an
  unlisted deep skill; listed skills have `pluginName`, the unlisted skill stays
  importable with no grouping metadata.
- Base: repository has no plugin manifests; preview output and list rendering
  stay flat.
- Bad: `marketplace.json` points at a remote source object or an invalid local
  path; the entry is skipped without failing marketplace sync, skills.sh
  resolution, preview, or import.

### 6. Tests Required

- Backend candidate tests for `plugin.json` grouping, `marketplace.json`
  `pluginRoot + source + skill` resolution, additive deep hints, recursive
  anti-suppression, and unsafe or broken hints.
- Backend serialization or import tests proving `pluginName` appears only on
  preview DTOs and not on imported summaries or result metadata.
- Marketplace shared-pipeline tests proving GitHub source sync still accepts
  repositories with manifests and broken manifest entries.
- Frontend wizard tests for grouped rendering, flat fallback, grouped selection
  controls, and a flat import payload with no `pluginName`.
- Full gate: `just ci`.

### 7. Wrong vs Correct

#### Wrong

```rust
// Persisting display grouping into import result metadata changes the contract.
ImportedGitHubSkillSummary {
    plugin_name: candidate.plugin_name.clone(),
    // ...
}
```

#### Correct

```rust
// Thread grouping only into preview DTOs.
GitHubSkillPreview {
    plugin_name: candidate.plugin_name.clone(),
    // ...
}
```

