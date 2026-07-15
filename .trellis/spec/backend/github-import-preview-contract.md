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

## Scenario: Repository-Level Singular Skill Directory

### 1. Scope / Trigger

Update this scenario when candidate identity or filtering changes for a
repository-level `skill/SKILL.md`, or when the GitHub import wizard changes how
it classifies backend error strings for authentication guidance.

### 2. Candidate Contract

- A valid `skill/SKILL.md` directly under the repository root is a
  repository-level skill container. It uses the same normalized repository ID
  as a root `SKILL.md`.
- The candidate keeps `sourcePath = "skill"`, `rootDirectory = "/"`, and
  `skillDirectoryName = "skill"`. Import copying and source/update metadata
  remain scoped to the `skill/` subtree.
- Repository-root and explicit `tree/<branch>/skill` inputs produce the same
  candidate identity.
- Deeper generic paths such as `agent_reach/skill/SKILL.md` and
  `packages/example/skill/SKILL.md` keep the existing generic-candidate filter.
- Named skill directories, root `SKILL.md`, manifest grouping, DTOs, import
  selections, and persisted metadata keep their existing semantics.

### 3. Authentication Guidance Contract

The frontend may show PAT guidance only when the backend message carries an
explicit authentication signal: rate limiting, `Personal Access Token`, a
standalone `PAT`, `GitHub denied access`, `requires authentication`, or a
configured GitHub token.

Bare `github`, bare `settings`, or embedded character sequences such as the
`pat` inside `subpaths` are not authentication signals.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Repository contains valid top-level `skill/SKILL.md` | Preview one candidate using the normalized repository ID |
| Same repository is addressed through `tree/<branch>/skill` | Return the same candidate identity and source path |
| Repository contains only deep `.../skill/SKILL.md` wrappers | Preserve generic-candidate filtering |
| `NoImportableSkills` text contains `subpaths` | Render the error without PAT guidance |
| URL validation text contains `github.com` | Render the error without PAT guidance |
| Backend reports rate limiting or unauthenticated access denial | Render the generic PAT settings guidance |
| Backend reports a configured token access denial | Render the configured-token guidance |

### 5. Tests Required

- Backend candidate regression using a `kill-ai-slop`-shaped snapshot with
  `skill/SKILL.md` plus unrelated repository files.
- Repository-root and explicit-subpath parity assertion.
- Existing deep generic-wrapper filtering and crafted-selection tests.
- Real wizard component tests for `subpaths`, URL validation, rate limiting,
  and configured-token denial messages.
- Full gate: `just ci`.

## Scenario: Root Skill Repository Content Boundary

### 1. Scope / Trigger

Apply this scenario when import or Central update code maps repository snapshot
files into a selected skill source path. A root `SKILL.md` uses `sourcePath = "."`
and represents one complete repository-backed skill package.

### 2. Shared Path Contract

```rust
pub(crate) fn repo_file_relative_to_source(
    repo_path: &str,
    source_path: &str,
) -> Option<String>;
```

- For `sourcePath = "."`, every snapshot file remains in scope and keeps its
  repository-relative path. Descendants such as `references/guide.md` must not
  be filtered because their path contains `/`.
- For a nested source such as `skills/agent-browser`, only files below that
  exact directory remain in scope and the source prefix is removed.
- Import staging/progress and Central update hashing/writes must call the same
  mapping helper. Do not maintain parallel root-path branches.
- Candidate identity, repository assignment and update metadata continue to
  persist `sourcePath = "."`; no schema or DTO change is required.
- GitHub archive resource budgets and existing safe-relative-path checks remain
  the security boundary for whole-repository root packages.

### 3. Update And Repair Contract

- Root descendants participate in remote hashes. Adding, changing or deleting
  only a descendant file must change update state.
- A previously truncated root package whose top-level files still match becomes
  `update_available` after a fresh comparison.
- Normal and force update reuse existing atomic staging/backup/swap behavior to
  install the complete root tree and remove stale local files.
- Copy installations refresh from the repaired Central directory through the
  existing batched copy path.
- SSH/WSL direct GitHub import keeps its existing recursive `cp -a` behavior;
  local snapshot import and Central updates must produce the same file scope.

### 4. Validation Matrix

| Condition | Required behavior |
| --- | --- |
| Root snapshot has `SKILL.md` plus `assets/`, `references/`, or `scripts/` descendants | Import every file with its original relative path |
| Existing root package lacks descendants | Fresh inventory reports an update and update restores descendants |
| Root upstream deletes a file | Atomic replacement removes the stale local file |
| Source is `skills/agent-browser` | Import only that subtree, never repository root or sibling directories |
| Update write fails | Restore the previous directory and leak no staging/backup directory |

### 5. Good / Base / Bad Cases

- Good: a root package contains `SKILL.md`, `references/guide.md`, and
  `scripts/run.py`; import and update preserve all three paths.
- Base: a nested `skills/agent-browser/SKILL.md` package has no descendants;
  only that file is imported and repository siblings remain excluded.
- Bad: a root collector treats any path containing `/` as out of scope, so
  local files and remote hashes silently omit every descendant directory.

### 6. Tests Required

- Pure root-vs-nested source-path mapping table.
- Root import collector and end-to-end import with nested resources.
- Root snapshot hash parity with a complete local directory.
- Inventory regression for a top-level-equal but descendant-incomplete package.
- Force update regression covering Central repair, stale-file removal,
  repository assignment preservation and managed-copy refresh.
- Existing nested import, `skill/` container, plugin grouping, generic filtering,
  remote import and Central batching tests.
- Full gate: `just ci`.

### 7. Wrong vs Correct

#### Wrong

```rust
if source_path == "." && repo_path.contains('/') {
    return None;
}
```

#### Correct

```rust
let relative_path = repo_file_relative_to_source(repo_path, source_path)?;
```
