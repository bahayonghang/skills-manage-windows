# GitHub Import Preview Contract

## Scenario: Structured Markdown Fetch Boundary

### 1. Scope / Trigger

Apply this contract when GitHub preview Markdown loading, the shared GitHub HTTP
client, raw/API endpoint construction, or remote preview workspace reuse changes.
The renderer may choose a previewed repository and repository-relative skill
path, but it must never choose the request scheme, authority, port, IP address,
redirect target, or authentication destination.

### 2. Signatures

```rust
#[tauri::command]
pub async fn fetch_github_skill_markdown(
    state: State<'_, AppState>,
    repo: GitHubRepoRef,
    source_path: String,
    preview_workspace_id: Option<String>,
) -> Result<String, String>;

pub(crate) async fn fetch_skill_markdown(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    source_path: &str,
    auth_token: Option<&str>,
) -> Result<String, GithubImportError>;
```

```ts
fetchGitHubSkillMarkdown(repo: GitHubRepoRef, sourcePath: string): Promise<void>;
```

### 3. Contracts

- The IPC payload contains `repo`, `sourcePath`, and `previewWorkspaceId`; it
  never accepts `downloadUrl` as request authority.
- Local Markdown reads validate `owner`, `repo`, `branch`, and `sourcePath`, then
  construct `<sourcePath>/SKILL.md` under the fixed `GITHUB_MIRROR_ENDPOINTS`.
  `normalizedUrl` is display/reference data and is not used for HTTP routing.
- Remote Markdown reads do not issue local HTTP requests. They require the
  submitted `repo` to equal the repository stored in the preview workspace
  before reading the requested relative path.
- Every production API/raw request is HTTPS, uses the endpoint's exact host and
  base-path prefix, has no userinfo or fragment, and uses the standard HTTPS
  port. Bearer auth is sent only to the direct GitHub endpoint.
- The shared client has a 5-second connect timeout, a 30-second total timeout,
  and `redirect::Policy::none()`. A 3xx response cannot select a second URL.
- Raw response bodies are checked against `content_length` when present and are
  then accumulated with checked arithmetic through `bytes_stream()`. The budget
  is checked before each chunk is appended.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Invalid owner, repo, or branch component | Return `InvalidRepoComponent`; issue no request |
| Unsafe or empty repository-relative path | Return `UnsupportedRepoPath`; issue no request |
| URL falls outside a built-in endpoint's scheme/host/port/path policy | Return `InvalidUrl`; issue no request |
| Remote workspace repo differs from submitted repo | Return `PreviewWorkspaceMismatch`; read no file |
| Remote workspace expired or target changed | Preserve the typed preview workspace error |
| 3xx points to another host or private address | Do not follow; classify/fallback as an HTTP attempt |
| Declared or streamed body exceeds its budget | Return `Budget` before appending excess bytes |
| Mirror fallback follows a direct denial/transport failure | Never forward the direct GitHub bearer token |

### 5. Good / Base / Bad Cases

- Good: the wizard submits the `repo` from `GitHubRepoPreview` plus
  `skills/demo`; the backend fetches a fixed-endpoint
  `skills/demo/SKILL.md` request with bounded streaming.
- Base: a remote preview submits the same repo and workspace ID; the backend
  reads from that workspace and performs no local network request.
- Bad: the renderer submits `file://`, a metadata IP, a lookalike GitHub host,
  or a crafted `downloadUrl`; such authority is absent from the IPC contract and
  cannot reach the HTTP client.

### 6. Tests Required

- Backend pure tests for repository component injection and the SSRF URL matrix:
  non-HTTPS schemes, loopback/private/link-local IPs, lookalike hosts, userinfo,
  fragments, and nonstandard ports.
- A policy test that every API/raw URL generated for every built-in endpoint
  satisfies that endpoint's declared policy.
- HTTP fixtures proving redirects are not followed, mirror fallback remains
  functional, direct PAT auth is not forwarded, and a chunked cap-plus-one body
  returns `Budget` before EOF.
- A remote workspace test asserting a mismatched repo returns
  `PreviewWorkspaceMismatch`.
- Frontend store and IPC coverage tests asserting `repo`, `sourcePath`, and
  `previewWorkspaceId` are present and `downloadUrl` is absent.
- Full gate: `just ci`.

### 7. Wrong vs Correct

#### Wrong

```rust
fetch_raw_text(&client, &renderer_supplied_download_url, auth).await
```

This lets renderer data choose the network authority and redirect surface.

#### Correct

```rust
fetch_skill_markdown(&client, &repo, &source_path, auth).await
```

The service validates structured repository identity and constructs requests
only from built-in endpoints.

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

## Scenario: Selected-Subtree TreeRaw Import

### 1. Scope / Trigger

Apply this contract when local GitHub import acquisition changes after preview.
SSH/WSL preview workspaces keep their existing remote archive flow.

### 2. Signatures

```rust
async fn try_prepare_tree_import(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    source_path: Option<&str>,
    selections: &[GitHubSkillImportSelection],
    auth: Option<&str>,
    allow_invalid_candidates: bool,
) -> Result<TreeImportOutcome, GithubImportError>;

fn plan_tree_selection(
    manifest: &RepositoryManifest,
    candidates: &[RemoteSkillCandidate],
    selections: &[GitHubSkillImportSelection],
) -> Result<TreeSelectionPlan, GithubImportError>;
```

`TreeImportOutcome` is internal and is either a fully prepared snapshot plus
fresh candidate inspection or a typed archive fallback reason. It is never
serialized or persisted.

### 3. Contracts

- Re-fetch the recursive tree at confirm time and rediscover candidates; never
  trust frontend file lists or source paths without backend validation.
- Build a stable, deduplicated union of regular files under non-skipped selected
  source paths. Root source `.` routes to archive acquisition.
- Reuse plugin manifest and candidate `SKILL.md` bytes already fetched during
  current-operation discovery. Download each remaining selected blob once with
  bounded concurrency.
- Validate every raw response against tree byte size plus shared single-file and
  aggregate resource budgets. Missing or changed files are integrity fallbacks.
- Complete tree/raw or archive acquisition before Central directory creation,
  staging, mutation locking, target swap or DB persistence. Acquisition failure
  cannot leave Central filesystem or database state behind.
- Keep full and partial import DTOs, selection resolution, progress events,
  source metadata and remote-target behavior unchanged.

Initial policy: TreeRaw is limited to non-root selections of at most 64 regular
files and 8 MiB selected bytes, downloaded with concurrency 8. Exceeding any
limit selects the archive path with `FallbackReason::Threshold`; these values
are internal policy, not frontend or persistence contracts.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| One nested skill | Download only that source subtree |
| Overlapping multi-selection | Download the file union once |
| Root skill | Use archive |
| More than 64 selected files or more than 8 MiB | Use archive |
| Candidate/plugin metadata belongs to selection | Reuse bytes; no duplicate raw request |
| Raw 404 or tree/raw byte-size mismatch | Integrity fallback before staging |
| Denial, transport or budget failure | Typed archive fallback; preserve actionable final error |
| Partial import includes invalid selection | Report existing per-skill failure and import valid selections only |

### 5. Good / Base / Bad Cases

- Good: two nested selected skills overlap in a shared subtree; the planner
  emits a stable union and each repository file is fetched once.
- Base: a root skill or a 65-file nested selection selects archive before any
  Central directory or staging path is created.
- Bad: begin staging after candidate metadata succeeds, then discover a raw 404
  halfway through selected files. This can leave partial filesystem state and
  is forbidden; all acquisition must finish first.

### 6. Tests Required

- Planner tests for nested union, overlap dedupe, root and threshold routing.
- Metadata reuse and byte-size mismatch fallback tests.
- Existing full/partial archive import, staging rollback, Central update and
  remote workspace tests must remain green.
- Full gate: `just ci`.

### 7. Wrong vs Correct

#### Wrong

```rust
for selection in selections {
    download_and_stage_subtree(selection).await?;
}
```

This trusts frontend selection order, redownloads overlaps and crosses into
Central mutation before acquisition is complete.

#### Correct

```rust
let plan = plan_tree_selection(&manifest, &fresh_candidates, &selections)?;
let snapshot = download_tree_selection(client, repo, &plan, auth, metadata).await?;
import_github_repo_skills_from_snapshot(pool, repo, &snapshot, /* ... */).await
```

The complete selected snapshot is validated first, then the existing atomic
staging/persistence pipeline runs unchanged.

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

## Scenario: Preview File Manifest

### 1. Scope / Trigger

Apply this scenario when GitHub repository import preview changes the file set
shown before confirmation. The manifest is evidence about the preview snapshot's
import boundary; it is not a post-import filesystem verification or a pinned
commit guarantee.

### 2. Signatures

```rust
#[serde(rename_all = "camelCase")]
pub struct GitHubSkillPreviewFile {
    pub path: String,
    pub byte_len: u64,
}

pub struct GitHubSkillPreview {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<GitHubSkillPreviewFile>>,
    // existing fields omitted
}
```

```ts
export interface GitHubSkillPreviewFile {
  path: string;
  byteLen: number;
}

export interface GitHubSkillPreview {
  files?: GitHubSkillPreviewFile[] | null;
}
```

Do not add file manifests to `GitHubSkillImportSelection`,
`ImportedGitHubSkillSummary`, `GitHubRepoImportResult`, database rows, or source
metadata.

### 3. Contracts

- GitHub import preview must return `Some(files)` for every candidate, with
  stable path ordering and a root-relative `SKILL.md` entry. Generic preview DTO
  consumers may keep `files = None`, which is omitted during serialization.
- Each entry is a regular file represented by a safe `/`-separated path relative
  to the final skill directory and its uncompressed byte length. Directory nodes
  and aggregate counts are derived by the frontend.
- Local preview inventories the already-downloaded `GitHubRepoSnapshot`; it must
  not download the repository again. Remote preview runs one bounded inventory
  command for the existing preview workspace, then partitions that repository
  inventory in Rust for all candidates.
- Both transports map repository files through
  `repo_file_relative_to_source`: `sourcePath = "."` keeps the complete repository,
  while a nested source keeps only that exact subtree and removes its prefix.
- The frontend may derive a read-only, virtualized tree and display the current
  rename decision as its visual root. The display field must not alter the flat
  import selection payload.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Candidate manifest contains root-relative `SKILL.md` | Return the stable manifest and allow review |
| Manifest is missing, empty, duplicated, unsafe, structurally conflicting, or lacks `SKILL.md` | Fail closed; do not present an empty tree as trustworthy or allow review |
| Remote record delimiter or byte length is malformed | Fail preview and remove the unregistered preview workspace |
| Remote inventory exceeds archive file, entry-size, or expanded-size budget | Return the existing resource-budget error |
| Generic CLI, Marketplace, or Central caller builds a preview DTO | Omit `files`; do not incur repository inventory cost |
| User renames a conflicting skill | Change only the visual root and selection rename field; keep manifest paths unchanged |

### 5. Good / Base / Bad Cases

- Good: a root skill contains `SKILL.md`, `assets/logo.png`, and
  `references/guide.md`; one preview snapshot exposes all three and the UI shows
  their complete tree.
- Base: a nested skill contains only `SKILL.md`; the preview reports one file and
  no directories, while repository siblings stay excluded.
- Bad: inventory fails but the backend serializes `files: []` or the frontend
  treats an omitted field as an empty skill package and still enables Review.

### 6. Tests Required

- Backend root and nested snapshot tests must assert exact relative paths, byte
  lengths, stable ordering, and camelCase serialization.
- Remote parser tests must cover malformed records, duplicate paths, budget
  enforcement, and one `run_script` call through the fake runner.
- Local and remote manifest attachment must use the same source-path mapping and
  reject candidates whose mapped files do not contain `SKILL.md`.
- Frontend model and wizard tests must cover totals, expansion, rename, skill
  switching, keyboard-operable directory buttons, missing-manifest blocking, and
  bounded DOM output at the 20,000-file archive limit.
- Full gate: `just ci`.

### 7. Wrong vs Correct

#### Wrong

```ts
// Re-implementing sourcePath membership in the UI can diverge from import.
const files = repositoryFiles.filter((file) => file.path.startsWith(sourcePath));
```

#### Correct

```rust
let path = repo_file_relative_to_source(&file.repo_path, &skill.source_path)?;
skill.files = Some(mapped_and_sorted_files);
```

## Scenario: Tree Manifest Acquisition Fast-Path

### 1. Scope / Trigger

Update this scenario when the GitHub import preview acquisition path
changes how it obtains repository files (tree API vs archive tarball),
the fallback matrix, or the parity contract between the two acquisition
modes.

### 2. Signatures

The fast-path owns internal acquisition types that are never serialized
to the frontend or persisted into Central:

```rust
pub(super) struct RepositoryFileMeta {
    pub repo_path: String,
    pub byte_len: u64,
    pub kind: RepositoryFileKind, // RegularBlob | SymlinkBlob | Gitlink | Other
}
pub(super) struct RepositoryManifest { /* regular_files, skipped */ }
pub(crate) enum AcquisitionMode { TreeRaw, Archive }
pub(crate) enum FallbackReason {
    Truncated, Unsupported, Denied, Transport, Budget, Integrity, Threshold,
}
```

The preview dispatcher entry point keeps its existing signature and DTO:

```rust
pub(crate) async fn preview_github_repo_import_with_auth(
    pool: &DbPool,
    repo_url: &str,
    auth: Option<&str>,
) -> Result<GitHubRepoPreview, GithubImportError>;
```

### 3. Contracts

- Preview acquisition tries the recursive Git tree API
  (`/repos/{owner}/{repo}/git/trees/{branch}?recursive=1`) first. On
  success, candidates and preview file manifests are built from the
  parsed tree without downloading the tarball.
- The tree parser classifies entries by Git mode/type: `100644` /
  `100755` blob → `RegularBlob` (candidate + raw download);
  `120000` symlink blob and `160000` gitlink → skipped (matching the
  archive `is_file()` filter); `040000` tree node → skipped; unknown
  mode/type → typed `UnsupportedMode` fallback.
- Candidate discovery, plugin manifest interpretation, frontmatter
  parsing, source-path mapping, preview skill construction, and file
  manifest attachment are shared with the archive path. The tree path
  feeds `regular_paths()` into
  `discover_skill_manifests_from_paths_with_plugin_discovery`; the
  archive path feeds `snapshot.files.keys()`. The output candidate /
  preview file sets must be equivalent for the same fixture.
- Raw blob bytes (candidate `SKILL.md`, optional `.claude-plugin/*`
  manifests) are fetched through `fetch_raw_bytes`, bounded by the
  default skill `file_bytes` budget, reusing the shared mirror/PAT
  fallback boundary.
- Acquisition failures fall back to the existing archive path. The
  fallback matrix: `TreeManifestTruncated` → `Truncated`;
  `TreeManifestUnsupportedMode` → `Unsupported`; `RateLimited` /
  `AccessDenied` → `Denied`; `Http` / `RepoNotFound` → `Transport`;
  `Budget` / `TreeManifestEntryBudgetExceeded` /
  `TreeManifestSizeOverflow` → `Budget`; `TreeManifestEntryMissingSize`
  → `Integrity`.
- Domain errors (invalid candidate, no importable skills) are NOT
  acquisition fallbacks — the archive path would surface the same
  domain failure, so the dispatcher returns them directly.
- Once acquisition produces candidates, no Central mutation begins
  until the existing staging/atomic import path takes over. The
  fast-path does not write partial state.
- SSH/WSL remote preview keeps its existing remote-workspace
  acquisition; the tree fast-path applies only to the local
  (non-remote) preview path.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Nested skill repo, tree API available | Preview built from tree manifest; no tarball request |
| Tree response `truncated: true` | Typed `Truncated` fallback → archive acquisition |
| Unknown mode/type entry | Typed `UnsupportedMode` fallback → archive acquisition |
| Regular blob missing `size` | Typed `MissingSize` fallback → archive acquisition |
| Tree entries exceed `tree_entries` budget | `Budget` fallback → archive |
| Tree response body exceeds `tree_response_bytes` | `Budget` fallback before serde allocation |
| Raw blob 404 after tree listed it | `RepoFileGone` → `Transport` fallback (integrity gap) |
| 401/403/429 on tree or raw | `Denied` fallback → archive (or final denial) |
| 5xx / mirror transport failure | `Transport` fallback → archive |
| Invalid candidate `SKILL.md` (bad frontmatter/UTF-8) | Domain error surfaced directly; no fallback |
| Plugin manifest missing in tree | Continue with no grouping (parity with archive) |
| Plugin manifest raw fetch fails | Acquisition fallback (archive reads it from tarball) |

### 5. Good / Base / Bad Cases

- Good: a nested skill repo with `skills/demo/SKILL.md` and
  `skills/demo/references/g.md`; the tree fast-path returns the same
  candidate and `GitHubSkillPreviewFile` set as the archive path,
  without downloading the tarball.
- Base: a root `SKILL.md` repo; tree fast-path and archive produce
  identical preview output.
- Bad: the tree fast-path swallows a `RateLimited` denial and returns
  `NoImportableSkills` instead of falling back to archive; or the
  tree path treats a `120000` symlink blob as a candidate while the
  archive path excludes it.

### 6. Tests Required

- Pure parser tests for `parse_tree_response` (truncated, missing size,
  unknown mode, budget, malformed JSON, unsafe path, duplicate path,
  symlink/gitlink skip).
- Parity tests proving tree vs archive produce identical
  `RemoteSkillCandidate`, `PreviewRepositoryFile`, discovery manifests,
  and `GitHubSkillPreviewFile` sets for shared fixtures (root, nested,
  multi-skill, namespaced, plugin, symlink, gitlink).
- Fallback classification matrix tests for `fallback_reason_for` and
  `map_acquisition_error_to_outcome` covering every acquisition variant.
- Plugin manifest bytes-path parity test proving
  `plugin_manifest_discovery_from_manifest_bytes` matches
  `plugin_manifest_discovery_from_snapshot` for the same fixture.
- Existing archive preview, candidate, file manifest, selection, and
  import tests must not regress.
- Full gate: `just ci`.

### 7. Wrong vs Correct

#### Wrong

```rust
// Treating a 120000 symlink blob as a candidate breaks parity with
// the archive parser's is_file() filter.
if entry_type == "blob" {
    candidates.push(entry.path);
}
```

#### Correct

```rust
let kind = classify_tree_entry(&entry.mode, &entry.entry_type);
if kind == Some(RepositoryFileKind::RegularBlob) {
    regular_files.push(/* ... */);
} else if matches!(kind, Some(SymlinkBlob) | Some(Gitlink)) {
    skipped.push(/* diagnostics only */);
} else if kind == Some(Other) {
    return Err(GithubImportError::TreeManifestUnsupportedMode { /* .. */ });
}
```
