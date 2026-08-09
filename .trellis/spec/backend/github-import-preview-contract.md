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
    preview_id: String,
    repo: GitHubRepoRef,
    source_path: String,
) -> Result<String, String>;

pub(crate) async fn fetch_github_skill_markdown_from_snapshot(
    active_target: &ResolvedTarget,
    preview_id: &str,
    repo: &GitHubRepoRef,
    source_path: &str,
) -> Result<String, GithubImportError>;
```

```ts
fetchGitHubSkillMarkdown(repo: GitHubRepoRef, sourcePath: string): Promise<void>;
```

### 3. Contracts

- The IPC payload contains `previewId`, `repo`, and `sourcePath`; it never
  accepts `downloadUrl` as request authority. `previewId` is required — there is
  no unauthenticated Markdown read path.
- Markdown is served **only** from the registered preview snapshot. No transport
  re-downloads `SKILL.md` at read time; the raw-HTTP `fetch_skill_markdown`
  helper was deleted so this cannot regress. See
  _Immutable Preview Snapshot Lifecycle_ for binding and digest rules.
- Acquisition-time raw/API requests (issued during preview, not during read)
  still validate `owner`, `repo`, `branch`, and `sourcePath` and construct
  `<sourcePath>/SKILL.md` under the fixed `GITHUB_MIRROR_ENDPOINTS`.
  `normalizedUrl` is display/reference data and is not used for HTTP routing.
- Every production API/raw request is HTTPS, uses the endpoint's exact host and
  base-path prefix, has no userinfo or fragment, and uses the standard HTTPS
  port. Bearer auth is sent only to the direct GitHub endpoint.
- The shared client has a 5-second connect timeout, a 30-second total timeout,
  and `redirect::Policy::none()`. API/raw requests never follow a 3xx. Archive
  acquisition alone may execute one of the finite, explicitly validated redirect
  chains described under _Archive Canonical Redirect Boundary_ below.
- Raw response bodies are checked against `content_length` when present and are
  then accumulated with checked arithmetic through `bytes_stream()`. The budget
  is checked before each chunk is appended.

### 4. Validation & Error Matrix

| Condition                                                            | Required behavior                                   |
| -------------------------------------------------------------------- | --------------------------------------------------- |
| Invalid owner, repo, or branch component                             | Return `InvalidRepoComponent`; issue no request     |
| Unsafe or empty repository-relative path                             | Return `UnsupportedRepoPath`; issue no request      |
| URL falls outside a built-in endpoint's scheme/host/port/path policy | Return `InvalidUrl`; issue no request               |
| Snapshot repo/source/target differs from submitted values            | Return `PreviewWorkspaceMismatch`; read no file     |
| `previewId` unknown, expired, or target changed                      | Preserve the typed snapshot lifecycle error         |
| Snapshot file bytes no longer match the registered `sha256`          | Return `PreviewSnapshotIntegrity`; return no bytes  |
| API/raw 3xx points to any destination                                | Do not follow; classify/fallback as an HTTP attempt |
| Archive 302 target violates the codeload policy                      | Return `ArchiveRedirectRejected`; issue no second request |
| Declared or streamed body exceeds its budget                         | Return `Budget` before appending excess bytes       |
| Mirror fallback follows a direct denial/transport failure            | Never forward the direct GitHub bearer token        |

### 5. Good / Base / Bad Cases

- Good: the wizard submits its `previewId`, the `repo` from `GitHubRepoPreview`,
  and `skills/demo`; the backend returns the `skills/demo/SKILL.md` bytes held
  in the snapshot and issues no network request.
- Base: the user expands the same candidate twice; both reads return byte-identical
  content and neither consumes the token.
- Bad: the renderer submits `file://`, a metadata IP, a lookalike GitHub host,
  or a crafted `downloadUrl`; such authority is absent from the IPC contract and
  cannot reach the HTTP client.
- Bad: a read falls back to raw HTTP because the snapshot expired — the branch may
  have moved, so the user would preview bytes that import will not write.

### 6. Tests Required

- Backend pure tests for repository component injection and the SSRF URL matrix:
  non-HTTPS schemes, loopback/private/link-local IPs, lookalike hosts, userinfo,
  fragments, and nonstandard ports.
- A policy test that every API/raw URL generated for every built-in endpoint
  satisfies that endpoint's declared policy.
- HTTP fixtures proving API/raw redirects are not followed, archive redirects
  follow only one validated codeload hop without Bearer auth, mirror fallback
  remains functional, and a chunked cap-plus-one body returns `Budget` before EOF.
- A snapshot binding test asserting a mismatched repo returns
  `PreviewWorkspaceMismatch`.
- A test proving repeated reads return the preview bytes and never consume the
  token (`snapshot_reads_return_preview_bytes_and_never_consume_the_token`).
- Frontend store and IPC coverage tests asserting `previewId`, `repo`, and
  `sourcePath` are present and `downloadUrl` is absent.
- Full gate: `just ci`.

### 7. Wrong vs Correct

#### Wrong

```rust
fetch_raw_text(&client, &renderer_supplied_download_url, auth).await
```

This lets renderer data choose the network authority and redirect surface.

#### Correct

```rust
fetch_github_skill_markdown_from_snapshot(active_target, &preview_id, &repo, &source_path).await
```

The service validates the snapshot binding and returns bytes the user already
confirmed. Acquisition-time requests, issued during preview only, are built from
structured repository identity and built-in endpoints — never from renderer URLs.

## Scenario: Archive Canonical Redirect Boundary

### 1. Scope / Trigger

Apply this scenario when GitHub archive acquisition, the shared request fallback
helper, archive URL validation, or archive download errors change. It covers
preview, import, Marketplace, and Central Update consumers of
`download_repo_snapshot`; it does not grant redirect authority to API/raw reads.

### 2. Signatures

```rust
struct GitHubArchiveInitialResponse {
    response: reqwest::Response,
    provenance: GitHubEndpointProvenance,
}

enum GitHubEndpointProvenance {
    TrustedDirect,
    Mirror,
}

async fn finish_repository_archive_response(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    initial: GitHubArchiveInitialResponse,
    auth_token: Option<&str>,
    budget: ResourceBudget,
    redirect_policy: &ArchiveRedirectPolicy,
) -> Result<Vec<u8>, GithubImportError>;
```

### 3. Contracts

- The shared reqwest client remains `redirect::Policy::none()`. A dedicated
  archive wrapper may return initial `301 Moved Permanently` or `302 Found`
  responses plus their endpoint provenance; every standard API/raw wrapper
  remains success-only.
- `TrustedDirect` is derived from the distinct built-in direct endpoint that
  issued the response, never from a renderer URL or from the `Location` host.
  A mirror response can authorize a direct codeload 302 only; it cannot authorize
  numeric repository canonicalization.
- An initial 302 `Location` must occur exactly once and parse as an absolute
  HTTPS URL whose ASCII host is exactly `codeload.github.com`, effective port is
  443, and userinfo, query, and fragment are absent. Owner and repo compare
  ASCII-case-insensitively with the structured input.
- An initial 301 is accepted only from `TrustedDirect`. Its single `Location`
  must be exactly `https://api.github.com/repositories/{positive_u64}/tarball/{same_ref}`
  under the production authority policy. That API request may be rebuilt with
  Bearer auth and must return exactly one 302 to codeload.
- After the trusted numeric hop, codeload may use a renamed canonical owner/repo;
  both components still pass the repository component validators and the ref
  remains byte-for-byte equal to the structured input. Repository identity and
  provenance rows are not rewritten as a side effect of update checking.
- Before URL normalization, raw backslashes, encoded `/` or `\`, empty or
  non-empty userinfo, and literal or percent-encoded `.` / `..` path segments
  are rejected. For an ordinary branch, parsed codeload segments must equal
  `{owner}/{repo}/legacy.tar.gz/refs/heads/{branch}` exactly. When the structured
  ref is exactly 40 ASCII hexadecimal characters (the pinned preview ref), the
  only accepted path is `{owner}/{repo}/legacy.tar.gz/{same_sha}`. The two shapes
  are not interchangeable and neither accepts a suffix or encoded separator.
- Every hop is a new request. Bearer is permitted only on the original trusted
  API request and its validated numeric API hop. Codeload and mirror requests
  never receive Bearer. Codeload must return a terminal non-3xx response, so a
  chain uses at most three requests.
- A successful terminal response uses the same bounded archive reader and
  extraction budgets as a direct 2xx archive. Existing 404, denial, transport,
  mirror-auth isolation, and resource-budget semantics remain unchanged.
- Archive acquisition classifies timeout, request/connect, response-body read,
  and exhausted retryable server status as typed variants. These variants keep
  the public `github_import.transport_failed` code while exposing distinct
  static diagnostic categories; no classifier parses `Display` text.
- `ArchiveRedirectRejected` has no dynamic fields and maps to
  `github_import.archive_redirect_rejected`; URLs, headers, repository paths,
  response bodies, and credentials never enter the error value.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Direct or mirror 302 to a case-equivalent codeload identity | Fetch once without Bearer; continue bounded archive handling |
| Trusted direct 301 to exact numeric API, then canonical codeload 302 | API hop may use Bearer; codeload does not; continue bounded handling |
| Mirror 301, non-numeric/zero/overflow ID, changed ref, or extra API segment | `ArchiveRedirectRejected`; do not authorize canonical identity |
| Missing, duplicate, relative, or malformed `Location` | `ArchiveRedirectRejected`; issue no next request |
| HTTP, userinfo, query, fragment, non-443 port, lookalike/IP host | `ArchiveRedirectRejected`; no second request |
| Raw backslash, encoded separator, dot segment, branch, path, or suffix differs | `ArchiveRedirectRejected`; issue no next request |
| Direct 302 changes owner/repo beyond ASCII case | `ArchiveRedirectRejected`; renamed identity needs the trusted numeric proof chain |
| Pinned 40-hex SHA uses `refs/heads`, or ordinary branch uses direct suffix | `ArchiveRedirectRejected`; no second request |
| Numeric API response is not 302, or codeload response is 3xx | `ArchiveRedirectRejected`; do not continue the chain |
| API/raw endpoint returns 3xx | Preserve the global no-redirect behavior |
| Archive timeout/request/body read or exhausted retryable server status | Preserve a typed static family for downstream retry and diagnostics |

### 5. Good / Base / Bad Cases

- Good: a stale owner/repo receives direct `301 -> /repositories/123/tarball/main`,
  then `302 -> codeload/{renamed_owner}/{renamed_repo}/.../main`; only the two API
  requests contain Bearer and the bounded archive is accepted.
- Base: direct or mirror returns a 302 whose owner/repo differs only by ASCII
  case; the unauthenticated codeload response is accepted.
- Bad: a mirror returns the numeric 301, or a direct 302 changes repository
  identity beyond case. Neither response proves GitHub canonical ownership.
- Bad: `Url::parse` would normalize `other\..\repo` into an accepted path. The
  raw-syntax guard rejects it before parsing.

### 6. Tests Required

- Pure production-policy matrices for direct codeload and numeric API targets,
  including scheme/authority/port/path/query/fragment/userinfo, raw backslash,
  `%2f`, `%5c`, dot segment, ref, numeric ID, ordinary branch, and pinned SHA.
- Header tests for zero, one, and multiple `Location` values.
- Test-only local transport fixture proving both branch and pinned-SHA
  `302 -> 200 tar.gz -> snapshot` paths, first-hop Bearer presence, second-hop
  Bearer absence, and second-hop 3xx rejection.
- A three-request fixture proving trusted direct
  `301 numeric API -> 302 canonical codeload -> 200 archive`, Bearer on only the
  API hops, and safe renamed owner/repo acceptance.
- Hostile transport fixtures proving mirror numeric 301, non-302 numeric
  response, codeload redirect, and additional hops fail without another request.
- Existing no-redirect, mirror auth isolation, archive budget, and unsafe-entry tests.
- Full gate: `just ci`.

### 7. Wrong vs Correct

#### Wrong

```rust
if response.status().is_redirection() {
    client.get(response.headers()[LOCATION].to_str()?).send().await?
}
```

This follows an unproven chain, lets mirrors authorize identity changes, and can
copy credentials across authorities.

#### Correct

```rust
match (initial.provenance, initial.response.status()) {
    (_, StatusCode::FOUND) => validate_same_repository_codeload(...),
    (GitHubEndpointProvenance::TrustedDirect, StatusCode::MOVED_PERMANENTLY) => {
        validate_numeric_api_target(...)
    }
    _ => return Err(GithubImportError::ArchiveRedirectRejected),
}
```

The archive-only finite state machine carries provenance explicitly, validates
each target before rebuilding the next request, and scopes Bearer to trusted API
authorities.

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

| Condition                                                                           | Required behavior                                                                   |
| ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Missing or invalid manifest JSON                                                    | Continue legacy discovery with no grouping                                          |
| Manifest path has traversal, absolute path, backslash, URL, or remote object source | Ignore that path or entry                                                           |
| Manifest hint has no `SKILL.md`                                                     | Drop hint, keep preview/import working                                              |
| Manifest hint has invalid `SKILL.md`                                                | Drop hint, keep preview/import working                                              |
| Same invalid skill is discovered by legacy heuristics                               | Preserve existing invalid-candidate failure                                         |
| At least one preview skill has `pluginName`                                         | Frontend renders grouped sections and puts ungrouped skills under localized `Other` |
| No preview skill has `pluginName`                                                   | Frontend renders the existing flat preview list                                     |
| User imports grouped preview                                                        | Import selection payload stays flat and keyed by `sourcePath`                       |

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

| Condition                                      | Required behavior                                                  |
| ---------------------------------------------- | ------------------------------------------------------------------ |
| One nested skill                               | Download only that source subtree                                  |
| Overlapping multi-selection                    | Download the file union once                                       |
| Root skill                                     | Use archive                                                        |
| More than 64 selected files or more than 8 MiB | Use archive                                                        |
| Candidate/plugin metadata belongs to selection | Reuse bytes; no duplicate raw request                              |
| Raw 404 or tree/raw byte-size mismatch         | Integrity fallback before staging                                  |
| Denial, transport or budget failure            | Typed archive fallback; preserve actionable final error            |
| Partial import includes invalid selection      | Report existing per-skill failure and import valid selections only |

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

| Condition                                                      | Required behavior                                        |
| -------------------------------------------------------------- | -------------------------------------------------------- |
| Repository contains valid top-level `skill/SKILL.md`           | Preview one candidate using the normalized repository ID |
| Same repository is addressed through `tree/<branch>/skill`     | Return the same candidate identity and source path       |
| Repository contains only deep `.../skill/SKILL.md` wrappers    | Preserve generic-candidate filtering                     |
| `NoImportableSkills` text contains `subpaths`                  | Render the error without PAT guidance                    |
| URL validation text contains `github.com`                      | Render the error without PAT guidance                    |
| Backend reports rate limiting or unauthenticated access denial | Render the generic PAT settings guidance                 |
| Backend reports a configured token access denial               | Render the configured-token guidance                     |

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

| Condition                                                                             | Required behavior                                                      |
| ------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Root snapshot has `SKILL.md` plus `assets/`, `references/`, or `scripts/` descendants | Import every file with its original relative path                      |
| Existing root package lacks descendants                                               | Fresh inventory reports an update and update restores descendants      |
| Root upstream deletes a file                                                          | Atomic replacement removes the stale local file                        |
| Source is `skills/agent-browser`                                                      | Import only that subtree, never repository root or sibling directories |
| Update write fails                                                                    | Restore the previous directory and leak no staging/backup directory    |

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

| Condition                                                                                     | Required behavior                                                                     |
| --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Candidate manifest contains root-relative `SKILL.md`                                          | Return the stable manifest and allow review                                           |
| Manifest is missing, empty, duplicated, unsafe, structurally conflicting, or lacks `SKILL.md` | Fail closed; do not present an empty tree as trustworthy or allow review              |
| Remote record delimiter or byte length is malformed                                           | Fail preview and remove the unregistered preview workspace                            |
| Remote inventory exceeds archive file, entry-size, or expanded-size budget                    | Return the existing resource-budget error                                             |
| Generic CLI, Marketplace, or Central caller builds a preview DTO                              | Omit `files`; do not incur repository inventory cost                                  |
| User renames a conflicting skill                                                              | Change only the visual root and selection rename field; keep manifest paths unchanged |

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
const files = repositoryFiles.filter((file) =>
  file.path.startsWith(sourcePath),
);
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

| Condition                                            | Required behavior                                      |
| ---------------------------------------------------- | ------------------------------------------------------ |
| Nested skill repo, tree API available                | Preview built from tree manifest; no tarball request   |
| Tree response `truncated: true`                      | Typed `Truncated` fallback → archive acquisition       |
| Unknown mode/type entry                              | Typed `UnsupportedMode` fallback → archive acquisition |
| Regular blob missing `size`                          | Typed `MissingSize` fallback → archive acquisition     |
| Tree entries exceed `tree_entries` budget            | `Budget` fallback → archive                            |
| Tree response body exceeds `tree_response_bytes`     | `Budget` fallback before serde allocation              |
| Raw blob 404 after tree listed it                    | `RepoFileGone` → `Transport` fallback (integrity gap)  |
| 401/403/429 on tree or raw                           | `Denied` fallback → archive (or final denial)          |
| 5xx / mirror transport failure                       | `Transport` fallback → archive                         |
| Invalid candidate `SKILL.md` (bad frontmatter/UTF-8) | Domain error surfaced directly; no fallback            |
| Plugin manifest missing in tree                      | Continue with no grouping (parity with archive)        |
| Plugin manifest raw fetch fails                      | Acquisition fallback (archive reads it from tarball)   |

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

## Scenario: Immutable Preview Snapshot Lifecycle

### 1. Scope / Trigger

Apply this contract whenever GitHub preview acquisition, the preview snapshot
registry, `import_github_repo_skills`, snapshot Markdown reads, or per-skill
import provenance changes. It exists because the upstream branch can move between
preview and import: without a pinned immutable snapshot the user confirms one set
of bytes and Central receives another.

Scope is the renderer-driven GitHub repo import wizard only (Local, SSH, and WSL
targets). Central update sync, portable state, and `skills.sh` build their own
workspace from their own verified inventory and are explicitly out of scope.

### 2. Signatures

```rust
#[tauri::command]
pub async fn import_github_repo_skills(
    app: AppHandle,
    state: State<AppState>,
    preview_id: String,
    repo_url: String,
    selections: Vec<GitHubSkillImportSelection>,
) -> Result<GitHubRepoImportResult, String>;

#[tauri::command]
pub async fn discard_github_repo_preview_snapshot(
    state: State<AppState>,
    preview_id: String,
) -> Result<(), String>;
```

Registry (module-private, session-scoped, never persisted across restarts):

```rust
fn register_preview_snapshot(snapshot: PreviewSnapshot) -> Result<(), GithubImportError>;
fn reserve_remote_preview_snapshot(target_id: &str, target_kind: TargetKind, now: DateTime<Utc>)
    -> Result<RemoteReservationAttempt<'static>, GithubImportError>;
fn lookup_preview_snapshot(preview_id: &str, now: DateTime<Utc>)
    -> Result<Arc<PreviewSnapshot>, GithubImportError>;
fn acquire_import_lease(preview_id: &str, now: DateTime<Utc>)
    -> Result<Arc<PreviewSnapshot>, GithubImportError>;
fn sweep_preview_snapshots_for_target(target_id: &str, now: DateTime<Utc>)
    -> Vec<CleanupTicket>;
fn ack_preview_snapshot_cleanup(ticket: &CleanupTicket) -> bool;
```

Digest v1 (`services/github_import/digest.rs`):

```rust
fn aggregate_digest(domain: &str, entries: &[DigestFileEntry]) -> String;
// domains: skillport.github.repository-snapshot.v1 | skillport.github.skill-content.v1
// framing: domain_len:u64be | domain | count:u64be
//          | repeat(path_len:u64be | path | byte_len:u64be | sha256_raw[32])
// output:  sha256-v1:<lowercase hex>
```

### 3. Contracts

DTO fields, required on both sides — none of these are optional:

| Field | Type | Meaning |
| --- | --- | --- |
| `GitHubRepoPreview.previewId` | `String` | Snapshot token; required by import and Markdown read |
| `GitHubRepoPreview.resolvedCommitSha` | `String` | Commit the snapshot was acquired at |
| `GitHubRepoPreview.snapshotDigest` | `String` | `sha256-v1:<hex>` over the retained repository files |
| `GitHubRepoPreview.expiresAt` | `String` | RFC 3339; TTL is `REMOTE_PREVIEW_WORKSPACE_TTL_MINUTES` |
| `GitHubSkillPreviewFile.sha256` | `String` | Per-file `sha256-v1:<hex>`; display/evidence only |

- Preview pins the tip once via `/repos/{owner}/{repo}/commits/{ref}`. Tree, raw
  blob, and tarball requests (local and remote) all use that resolved SHA.
  Candidate display data and `downloadUrl` keep the user-facing branch —
  `pinned_repo_ref` replaces only the acquisition ref, never display metadata.
- Preview retains real bytes. The tree fast-path downloads all candidate subtrees
  (`TreeSelectionScope::AllCandidates`); root candidates and over-threshold
  selections fall back to archive. Per-file SHA-256 and "no second acquisition"
  cannot both hold without retained bytes.
- `aggregate_digest` sorts entries by UTF-8 byte order inside the function and
  frames every field with a `u64` big-endian length, so the value cannot depend on
  `HashMap` iteration order and `a/b` + `c` cannot collide with `a` + `b/c`.
- Import is the only mutating consumer and takes a required `preview_id`.
  `import_github_repo_skills_from_preview` performs lease, then binding, then
  selection, then digest verification before any FS or DB mutation. There is no
  branch re-fetch, no fresh-workspace fallback, and no local HTTP re-download:
  `resolve_remote_import_workspace` and `fetch_skill_markdown` were deleted, and a
  structural test forbids acquisition symbols inside `snapshot_import.rs`.
- The lease is single-holder. `Ready` + acquire becomes `Importing`; failure
  releases back to `Ready` so the same token can be retried; success consumes the
  entry atomically; a discard requested during a lease is deferred to release.
- Registry production policy is deterministic and bounded: at most four `Ready`
  previews per target, at most 256 MiB of Local retained bytes per target, and at
  most 64 total entries. Active imports count toward retained bytes and global
  ownership but are never eviction victims. LRU uses a monotonic access sequence,
  not wall-clock tie ordering.
- Expiry is enforced on every `lookup_preview_snapshot` and
  `acquire_import_lease`, so a stale token can never reach storage. Local expired
  entries are reclaimed synchronously on target-scoped register/sweep. Remote
  expiry, LRU eviction, discard, and import success transition to
  `CleanupPending`; lookup/import then fail closed until the owning target removes
  the workspace and acknowledges the generation-tagged cleanup ticket.
- Remote preview reserves per-target/global admission before creating a workspace.
  Cancellation before workspace ownership drops the reservation. A returned
  workspace is synchronously claimed into that reservation before any further
  await; cancellation after the claim transitions the same slot to
  `CleanupPending`. If cleanup of a newly created but unusable workspace fails,
  the same reservation remains `CleanupPending`, so ownership stays retryable
  without ever creating entry 65.
- Sweeps accept one `target_id` and return tickets only for that owner. A connection
  for target A never removes or acknowledges target B. Remote deletion runs outside
  the registry mutex; failed deletion leaves the ticket pending for the next owning
  connection, preview, discard, or import attempt.
- The renderer must discard explicitly on reset, on replacement by a new preview,
  on target change, and on wizard close.
- Per-skill provenance is written in the same transaction as the skill upsert and
  repository assignment, onto `skill_repository_members` — not
  `skill_repositories`, whose rows are shared by every skill from one repo. Writes
  use `COALESCE(excluded.…, existing)`, so a later provenance-less writer (Central
  update, CLI, portable state) cannot erase a known commit/digest. `NULL` means
  "unknown", including for every pre-v4 row. See
  [Versioned SQLite Migration Contract](./database-migrations.md).
- `snapshot_digest` is only ever compared against itself. The local tree fast-path
  retains candidate subtrees while a remote workspace holds the whole repo, so the
  value differs by transport for the same repo; per-candidate manifests stay
  identical.
- `to_ipc_error()` emits `github_import.<code>:<fixed English summary>` for the eight
  lifecycle variants (`preview_missing`, `preview_expired`,
  `preview_mismatch`, `preview_integrity`, `preview_busy`,
  `preview_capacity`, `preview_cleanup_pending`, `preview_commit_unresolved`).
  Every other variant keeps its historical Display
  text so PAT guidance and existing toasts are unchanged. No code path logs or
  serializes a token, workspace path, digest, or file content.

### 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| `previewId` absent from the payload | Reject at deserialization; there is no optional fallback |
| Token unknown / registry lost it | `PreviewSnapshotMissing` -> `github_import.preview_missing` |
| Token past `expiresAt` | `PreviewWorkspaceExpired` -> `github_import.preview_expired` |
| Snapshot repo, source root, or target differs from the request | `PreviewWorkspaceMismatch` / `PreviewTargetChanged` -> `github_import.preview_mismatch` |
| Retained bytes no longer match the registered per-file `sha256` | `PreviewSnapshotIntegrity` -> `github_import.preview_integrity`; no mutation |
| A second import already leases the same token | `PreviewSnapshotBusy` -> `github_import.preview_busy`; fail closed |
| Per-target/global admission has no safe victim | `PreviewCapacity` -> `github_import.preview_capacity`; create no remote workspace |
| Remote workspace deletion failed | Keep `CleanupPending`; lookup/import -> `github_import.preview_cleanup_pending` until owning-target retry ack |
| Tip commit cannot be resolved during preview | `PreviewCommitUnresolved` -> `github_import.preview_commit_unresolved` |
| Selection names a `sourcePath` absent from the snapshot | Fail before mutation |
| Import fails after the lease is taken | Release lease, keep the snapshot, allow retry with the same token |
| Import succeeds | Consume the token atomically and release storage |

Every one of these fails before FS or DB mutation. All eight coded variants map to a
bilingual "preview again" state in the renderer.

### 5. Good / Base / Bad Cases

- Good: preview a repo, the branch receives a new commit, then import — Central
  receives the previewed bytes, provenance records the previewed commit, and no
  second download occurs.
- Base: preview then import immediately; the token is consumed and a subsequent
  import with the same token returns `preview_missing`.
- Good: import fails on a disk error; the same token still imports on retry.
- Bad: import silently re-resolves the branch because the token expired. The user
  confirmed a diff they will not receive.
- Bad: putting `resolved_commit_sha` on `skill_repositories`; two skills imported
  from different snapshots of the same repo would overwrite each other.
- Bad: digesting a `HashMap` in iteration order, or concatenating
  `path + len + hash` without length framing.

### 6. Tests Required

- Digest purity: `digest_is_stable_and_independent_of_input_order`,
  `repository_digest_ignores_hashmap_insertion_order`,
  `digest_framing_prevents_path_boundary_collisions`.
- Registry lifecycle: unknown/expired rejection,
  `import_lease_is_exclusive_and_released_for_retry`, deferred discard under
  lease, `expiry_pruning_removes_only_unleased_snapshots`, deterministic
  per-target/byte/global limits, active-lease protection, concurrent reservation,
  cleanup-pending retry, and stale generation acknowledgement.
- Immutability: `import_uses_preview_bytes_and_persists_per_skill_provenance`
  (branch bytes change after preview) plus the structural guard
  `preview_import_module_cannot_acquire_repository_content`.
- Binding: `import_fails_closed_on_binding_mismatch_and_keeps_the_token`.
- Pinning: `pinned_repo_ref_only_replaces_the_acquisition_ref`.
- Provenance: `renamed_import_records_provenance_and_skip_writes_nothing` and
  `test_github_provenance_is_written_once_and_preserved_by_later_writers` (the
  `COALESCE` contract).
- Redaction:
  `snapshot_lifecycle_errors_use_stable_ipc_codes_without_leaking_details`
  asserting no `github-preview-`, `/tmp/`, `sha256-v1:`, or `ghp_` substring.
- Cross-transport parity:
  `remote_inventory_digest_matches_the_local_snapshot_digest` and
  `tree_selection_repository_files_match_archive_for_candidate_subtrees`.
- Remote ownership: FakeRunner tests must prove failed `remove_tree` leaves lookup
  and import closed until retry ack, and target A cannot execute target B's ticket.
- Renderer contract: `src/test/contracts/githubPreviewSnapshotContract.test.ts`
  proving zero `previewWorkspaceId` references and a single
  `invoke("import_github_repo_skills")` call site.
- Full gate: `just ci`.

> **Known gap**: `connect_remote_target` has no injection seam, so real SSH/WSL
> snapshot read/import is not covered end to end. FakeRunner covers the remote
> inventory protocol and digest parity instead. Adding a transport seam for
> `connect_remote_target` is a separate refactor — see
> [Transport Seam](./transport-seam.md).

### 7. Wrong vs Correct

#### Wrong

```rust
// Optional token with a "helpful" fallback: when the token is gone we quietly
// re-resolve the branch, so import writes bytes the user never confirmed.
let workspace = match preview_workspace_id {
    Some(id) => take_preview_workspace(&id).unwrap_or(create_fresh_workspace(&repo).await?),
    None => create_fresh_workspace(&repo).await?,
};
```

#### Correct

```rust
// Required token, fail closed, verify before mutating.
let snapshot = acquire_import_lease(preview_id, Utc::now())?;
snapshot.validate_binding(active_target, &repo, &source)?;
snapshot.verify_integrity(&selections)?;
// ... only now may FS/DB mutation begin
```

A lost or moved snapshot is a user-visible "preview again", never a silent
re-fetch.

## Scenario: Manual Single-Segment Branch Selection

### 1. Scope / Trigger

Apply this contract when the GitHub import branch input, preview/import command
arguments, repository source parsing, or snapshot binding changes. The renderer
may select a branch, but shared Rust service code remains the only authority for
parsing, validation, URL/manual reconciliation, and default-branch resolution.

### 2. Signatures

```rust
preview_github_repo_import(repo_url: String, branch: Option<String>)
import_github_repo_skills(
    preview_id: String,
    repo_url: String,
    branch: Option<String>,
    selections: Vec<GitHubSkillImportSelection>,
)

resolve_repo_source_with_branch(
    repo_url: &str,
    selected_branch: Option<&str>,
    auth_token: Option<&str>,
) -> Result<ResolvedGitHubRepoSource, GithubImportError>
```

```ts
preview_github_repo_import: { repoUrl: string; branch?: string | null };
import_github_repo_skills: {
  previewId: string;
  repoUrl: string;
  branch?: string | null;
  selections: GitHubSkillImportSelection[];
};
```

### 3. Contracts

- Trim the manual value; missing, empty, or whitespace-only means no manual
  selection. A non-empty value uses the existing safe single-segment branch
  validator. Slash/backslash, controls, `.` and `..` remain invalid.
- Parse the repository source first, then reconcile its optional `/tree/<branch>`
  branch with the manual field before constructing a client or issuing a
  request. Equal values succeed; unequal values fail closed. Neither wins
  silently.
- Use manual or URL branch when present. Only when both are absent may the
  repository inspection response supply `default_branch`.
- Local, SSH, and WSL preview entry points call the same branch-aware resolver.
  Existing CLI/service helpers remain source-compatible wrappers that pass
  `None`; CLI tree URLs retain their historical behavior.
- Confirmation validates the same optional branch evidence against the
  registered snapshot and imports retained bytes only. It never resolves a new
  branch tip or reacquires repository content.
- The renderer passes branch as structured data. It must not append `/tree/`,
  parse GitHub source paths, or choose request authority.

### 4. Validation & Error Matrix

| Input / condition | Required result |
| --- | --- |
| root URL + missing/blank branch | inspect and use repository default branch |
| root URL + `dev` | validate and resolve `dev` |
| `/tree/dev/<path>` + missing or `dev` | use `dev` and preserve source path |
| `/tree/dev/<path>` + `main` | `BranchSelectionConflict` -> `github_import.branch_conflict`; no request/mutation |
| manual `feature/foo`, backslash, or control | `InvalidBranchSelection` -> `github_import.branch_invalid`; no request/mutation |
| selected branch is absent/inaccessible | preserve existing commit-resolution/access failure; no Central mutation |
| confirmation branch differs from snapshot | fail binding before Central mutation; retain snapshot for retry |

The two branch error envelopes use fixed public summaries and never include the
branch value, PAT, preview token, workspace path, or response body.

### 5. Good / Base / Bad Cases

- Good: root URL plus `dev` previews and imports retained `dev` bytes; preview,
  result, per-skill provenance, and Central update identity all record `dev`.
- Base: blank manual input preserves default-branch behavior byte-for-byte.
- Good: `/tree/dev/skills` plus manual `dev` succeeds without renderer URL
  rewriting.
- Bad: manual `main` silently overrides `/tree/dev`, or confirmation reads the
  current input instead of the branch associated with its preview.
- Bad: relaxing this scenario to slash-containing refs without reviewing every
  API/raw/archive URL builder and endpoint policy.

### 6. Tests Required

- Pure Rust reconciliation tests: none/blank, trimmed `dev`, URL-only, equal,
  conflict, slash/backslash, and control characters.
- Snapshot binding tests: explicit equal branch succeeds; explicit mismatch and
  URL/manual conflict fail before mutation.
- IPC tests: `branch_invalid` and `branch_conflict` map to fixed public messages
  without dynamic detail.
- Store/contract tests: blank sends `null`, explicit input is trimmed, confirm
  reuses `previewedBranch`, command map contains both optional branch fields,
  and renderer production code builds no `/tree/` URL.
- Intent/wizard/page tests: branch-only dirty detection, deep-link/reset
  clearing, controlled Central/Marketplace inputs, and bilingual error text.
- Full gate: `just ci`.

### 7. Wrong vs Correct

#### Wrong

```ts
await previewGitHubRepoImport(`${repoUrl}/tree/${branch}`);
```

This duplicates parser behavior, breaks source subpaths, and can let display
state change repository identity outside the shared service boundary.

#### Correct

```ts
await invoke("preview_github_repo_import", {
  repoUrl,
  branch: branch.trim() || null,
});
```

Rust reconciles one structured hint with the URL and binds the resolved branch
to the immutable preview snapshot.
