# Archive acquisition baseline & tree fast-path fixture plan

> Status: Commit 1 research gate. Records the deterministic evidence the PRD
> requires before changing the production preview/import acquisition path.

## 1. Current archive acquisition (the baseline we must beat)

The local GitHub import acquisition path downloads the full repository tarball
twice per complete preview→confirm→import cycle:

| Phase | Function | Acquisition call | Bytes |
| --- | --- | --- | --- |
| Preview | `preview_github_repo_import_with_auth` | `download_repo_snapshot` ([`preview.rs:136`](../../../../src-tauri/src/services/github_import/preview.rs)) | full tarball (gz) + full expanded tree in memory |
| Import | `import_github_repo_skills_with_auth` | `download_repo_snapshot` ([`import.rs:34`](../../../../src-tauri/src/services/github_import/import.rs)) | full tarball again |
| Partial import | `import_github_repo_skills_partially_with_auth` | `download_repo_snapshot` ([`import.rs:162`](../../../../src-tauri/src/services/github_import/import.rs)) | full tarball a third time |

Each `download_repo_snapshot` call:

1. `download_repository_archive` → `send_github_request_with_fallback` against
   `/repos/{owner}/{repo}/tarball/{branch}` with PAT only on the direct
   `github` endpoint, mirror fallback on transport/5xx, typed denial on
   401/403/429 ([`raw_http.rs:112`](../../../../src-tauri/src/services/github_import/raw_http.rs)).
2. `snapshot_from_repository_archive_with_budget` — gzip-decodes, iterates tar
   entries, rejects non-file types (symlink/dir/other), enforces
   `archive_bytes` / `archive_files` / `archive_expanded_bytes` /
   `archive_entry_bytes` budgets, then builds `GitHubRepoSnapshot { files:
   HashMap<String, Vec<u8>> }` ([`archive.rs:91`](../../../../src-tauri/src/services/github_import/archive.rs)).

Deterministic facts (no network measurement needed):

- Two full archive downloads per complete cycle (preview + import).
- Preview holds the **complete expanded tree** in `GitHubRepoSnapshot.files`
  even though only candidate `SKILL.md` + plugin manifests are actually read.
- Import re-downloads and re-expands the full archive even when the user
  selected a single nested skill whose subtree is a fraction of the repo.
- No cache between preview and import: `GitHubPreviewWorkspace` is SSH/WSL-only;
  the local path has no preview→confirm workspace reuse.

## 2. Shared discovery the tree fast-path must reuse

`discover_skill_manifests_from_paths_with_plugin_discovery` is already a pure
path-based entry point
([`source.rs:497`](../../../../src-tauri/src/services/github_import/source.rs)):

```rust
pub(super) fn discover_skill_manifests_from_paths_with_plugin_discovery<'a, I>(
    paths: I,
    source_path: Option<&str>,
    plugin_discovery: &PluginManifestDiscovery,
) -> Result<Vec<SnapshotSkillManifest>, GithubImportError>
where I: IntoIterator<Item = &'a str>
```

It takes a path set + source_path + plugin discovery, and returns
`SnapshotSkillManifest` entries (source_path, skill_md_path, plugin_name,
root_directory, skill_directory_name, from_manifest_hint). The archive flow
feeds it `snapshot.files.keys()`; the tree flow must feed it the regular-blob
paths from `RepositoryManifest`. No second scanning implementation.

The remaining shared helpers the parity strategy pins:

- `repo_file_relative_to_source` ([`source.rs:680`](../../../../src-tauri/src/services/github_import/source.rs)) — same source-path mapping for preview file manifests.
- `build_preview_skills` ([`import.rs:637`](../../../../src-tauri/src/services/github_import/import.rs)) — Central conflict lookup + DTO construction.
- `attach_preview_file_manifests` ([`preview.rs:99`](../../../../src-tauri/src/services/github_import/preview.rs)) — turns repository files into per-skill `GitHubSkillPreviewFile`.
- `parse_frontmatter` ([`source.rs:737`](../../../../src-tauri/src/services/github_import/source.rs)) — single frontmatter entry point via `scanner::extract_frontmatter_block`.
- `plugin_manifest_discovery_from_manifest_bytes` ([`plugin_manifest.rs:75`](../../../../src-tauri/src/services/github_import/plugin_manifest.rs)) — bytes-in discovery; tree flow will pass raw bytes of `.claude-plugin/*.json` fetched via raw.

## 3. Git tree API shape & parity-relevant entry modes

`GET /repos/{owner}/{repo}/git/trees/{branch}?recursive=1` returns:

```json
{
  "sha": "<sha>",
  "url": "<api url>",
  "tree": [
    { "path": "README.md", "mode": "100644", "type": "blob", "sha": "...", "size": 123 },
    { "path": "subdir", "mode": "040000", "type": "tree", "sha": "...", "url": "..." },
    { "path": "link.txt", "mode": "120000", "type": "blob", "sha": "...", "size": 11 },
    { "path": "vendor/submod", "mode": "160000", "type": "commit", "sha": "...", "url": "..." }
  ],
  "truncated": false
}
```

Mode/type parity matrix vs the existing archive parser (`archive.rs:112`
skips every non-`is_file()` entry):

| Git mode | Git type | Tree classifier | Archive behavior | Parity |
| --- | --- | --- | --- | --- |
| `100644` | `blob` | `RegularBlob` | regular file → snapshot | ✅ both include |
| `100755` | `blob` | `RegularBlob` | regular file → snapshot | ✅ both include |
| `120000` | `blob` | `SymlinkBlob` | tar symlink entry → skipped by `is_file()` | ✅ both exclude |
| `160000` | `commit` | `Gitlink` | tar dir/gitlink absence → skipped | ✅ both exclude |
| `040000` | `tree` | (tree node, no size) | tar directory entry → skipped by `is_file()` | ✅ both exclude |
| unknown mode/type | unknown | `Other` | n/a | ⚠️ tree-only; triggers typed fallback |

The tree parser must **not** treat `120000` symlink blobs or `160000` gitlinks
as skill files, and must not download their raw bytes. Unknown mode/type
combinations return a typed fallback error so the dispatcher can switch to
archive (where the equivalent entries are simply absent from the tar regular
file set).

## 4. Fixture catalogue (parity & fallback matrix)

The PRD requires ≥10 fixture classes. Catalogue (used by Commit 2 parity tests
once mock HTTP lands; shapes fixed here so the parser tests in Commit 1 can
reference them):

| # | Fixture | Tree entries | Archive entries | Asserts |
| --- | --- | --- | --- | --- |
| F1 | root skill | `SKILL.md`, `README.md` | same | parity candidate/preview |
| F2 | nested single skill | `skills/demo/SKILL.md`, `skills/demo/references/g.md` | same | subtree selection downloads only `skills/demo/*` |
| F3 | multi-skill repo | 3× `skills/<name>/SKILL.md` | same | multi-select union + dedupe |
| F4 | plugin manifest | `.claude-plugin/plugin.json` + listed skills | same | pluginName parity, additive hints |
| F5 | private/PAT | 401/403 on direct | archive 401/403 | denial classification parity |
| F6 | rate-limited | 429 + x-ratelimit on direct | archive 429 | rate-limit fallback parity |
| F7 | tree truncated | `truncated: true` | full archive ok | tree → fallback Archive |
| F8 | mirror failure | all mirrors 5xx | archive mirrors 5xx | typed transport error |
| F9 | symlink blob `120000` | `link` mode 120000 | tar symlink entry | neither becomes candidate/raw download |
| F10 | gitlink `160000` | `vendor/sub` mode 160000 | tar absent | neither becomes candidate/raw download |
| F11 | missing size | regular blob w/o `size` | n/a | typed `MissingSize` fallback |
| F12 | unknown mode | `100000` blob | n/a | typed `Unsupported` fallback |
| F13 | over entry budget | >2048 entries | archive >20000 | tree `Budget`, archive ok |
| F14 | over byte budget | entries summing >limit | archive ok | tree `Budget` |

F9 and F10 **must** construct real tar symlink/gitlink entries and real tree
mode values — not just regular-file helpers — so the two acquisition paths are
proven to exclude them with the same semantics.

## 5. Performance decision model (not a hardcoded 300-file threshold)

Per design §7, the acquisition mode decision uses a cost model, not a constant:

```text
tree_raw_cost   = request_overhead * file_count + selected_bytes
archive_cost    = archive_bytes + extraction_cost
```

- `request_overhead` ≈ one round-trip per raw blob (bounded concurrency, host
  rate limiter shared with archive).
- `file_count` = number of regular blobs in the selected subtree union.
- `selected_bytes` = sum of selected regular blob sizes.
- `archive_bytes` = tarball content-length (or expanded bytes if no
  content-length).
- `extraction_cost` ≈ expanded_bytes (proportional).

The dispatcher (Commit 2/3) computes both and picks the cheaper; root-scope
(`sourcePath = "."`) or large selections bias toward Archive; small nested
subtrees bias toward TreeRaw. CI asserts deterministic properties (no archive
request on supported nested fixtures, selected-file set equals selection union,
bounded concurrency), not flaky wall-time.

## 6. Cache decision

Per PRD R5, Commit 1 does **not** add a metadata cache. The baseline above
already proves the dominant cost is the double archive download. If, after the
tree fast-path lands, profiling shows repeated tree API calls between preview
and import remain a bottleneck, Commit 4 (conditional) will add a ≤4-entry,
10-min TTL, process-local LRU keyed by normalized owner/repo/branch/sourcePath
(no PAT/bytes). The research decision recorded here: **no cache in Commit 1**.

## 7. Commit 1 scope (this commit)

- [x] This baseline doc.
- [x] `tree_manifest.rs`: `RepositoryFileMeta`, `RepositoryFileKind`,
      `RepositoryManifest`, `AcquisitionMode`, `FallbackReason` types.
- [x] `parse_tree_response` bounded parser: classifies modes, skips non-blob,
      enforces `tree_entries` + tree-response byte budget, detects
      `truncated`, rejects unsafe paths, returns typed fallback errors.
- [x] `ResourceBudget::reject_tree_entries` + `reject_tree_response_size`
      helpers.
- [x] Parser table tests (F1, F7, F9, F10, F11, F12, F13, F14 + safe-path
      rejection + malformed JSON).
- [ ] Mock HTTP infrastructure + parity/fallback tests (F2-F8) — Commit 2.
- [ ] Preview dispatcher integration — Commit 2.
- [ ] Selected subtree import + bounded raw downloader — Commit 3.
- [ ] Acquisition diagnostics + before/after metrics — Commit 3.
- [ ] Spec update (`github-import-preview-contract.md` acquisition scenario) — Commit 3.
