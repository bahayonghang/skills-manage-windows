# Design

## Scope

Fix the source-path content-boundary contract shared by GitHub snapshot import and Central updates. Candidate discovery, persisted repository assignment, IPC DTOs and frontend behavior remain unchanged.

This stays one task because the import and update symptoms come from the same path remapping rule. Splitting them would allow a root skill to import correctly but become truncated again on its first update.

## Layout Contract

| Layout | Candidate source path | Candidate identity | Content boundary |
| --- | --- | --- | --- |
| Root `SKILL.md` | `.` | normalized repository name | every file in the repository snapshot, path unchanged |
| Top-level `skill/SKILL.md` | `skill` | normalized repository name | only `skill/**`, prefix removed |
| Named nested skill | e.g. `skills/agent-browser` | normalized skill directory name | only selected subtree, prefix removed |
| Deep generic `.../skill/SKILL.md` | filtered | none | none |

The layout is derived from the selected manifest path, never from a repository allowlist or frontmatter name.

## Current Data Flow

```text
GitHub archive
  -> GitHubRepoSnapshot { repo_path -> bytes }
  -> discover root SKILL.md
  -> candidate sourcePath = "."                 (correct)
  -> collect files
       sourcePath "." => reject paths with "/" (bug)
  -> import staging / remote hash
       only root files survive
  -> Central directory and update state
       descendants missing, hash still reports up_to_date
```

Remote direct import bypasses the faulty snapshot collector and already recursively copies the selected directory.

## Shared Path Mapping

Add one small pure helper in the GitHub import source/path ownership boundary and export it crate-locally for Central updates. Suggested contract:

```rust
pub(crate) fn repo_file_relative_to_source(
    repo_path: &str,
    source_path: &str,
) -> Option<String>;
```

Behavior:

```text
repo_path="references/a.md", source_path="."
  -> "references/a.md"

repo_path="skills/demo/references/a.md", source_path="skills/demo"
  -> "references/a.md"

repo_path="skills/other/SKILL.md", source_path="skills/demo"
  -> None
```

The helper only maps membership and relative paths. Existing collectors continue to own their typed output, empty-source error, byte length/bytes and ordering. Existing write functions continue to perform safe-relative-path validation before filesystem writes.

This is intentionally smaller than a new snapshot abstraction. Two consumers already duplicate a non-trivial contract and the duplication caused this defect; a pure helper removes that failure mode without restructuring the import or update domains.

## Import Behavior

`collect_snapshot_source_files` uses the shared helper for every snapshot entry:

- root source returns all repository paths unchanged;
- nested source strips exactly one normalized source prefix;
- unrelated paths remain excluded;
- sorting, resource totals, progress payloads and staging writes remain unchanged.

The end-to-end import regression must use a root snapshot with nested runtime resources and assert the actual Central target tree, not just the intermediate file list.

Full and partial import already share `collect_snapshot_source_files`, so one production change covers both. Rename and overwrite continue through `import_single_staged_skill` and its existing backup/restore transaction behavior.

## Update Behavior

`collect_remote_skill_files` uses the same helper, so the same root descendants feed remote hashing and atomic writes.

After the change:

1. a fresh inventory for an incomplete root skill computes the local hash over its actual directory and the remote hash over the complete root file set;
2. the state becomes `update_available`;
3. normal update or force mirror writes the complete file set to a temporary directory;
4. the existing atomic swap removes stale files and installs the full root tree;
5. managed copy installations are refreshed from the repaired Central directory.

No one-off migration or direct filesystem repair is needed. Existing repository assignments already contain `source_path = "."`; the next non-cached/fresh comparison naturally detects the missing files. Tests must not rely on a stale cached inventory.

## Local / SSH / WSL Consistency

- Local direct GitHub import becomes recursive for root packages.
- SSH/WSL direct GitHub import remains recursive through `cp -a`.
- Central updates for local and remote targets both consume the corrected snapshot file set before transport-specific atomic writes, so their content boundary becomes identical.
- No new remote process or per-file SSH loop is introduced; Central update batching remains unchanged.

## Safety And Resource Boundaries

- GitHub archive extraction remains the source of snapshot files and continues to enforce archive, entry, expanded-size and file-count budgets.
- The GitHub archive naturally excludes `.git`; tracked dotfiles and directories such as `.github` remain part of a root package, matching the requested whole-repository boundary.
- Existing `is_safe_repo_relative_path` / Central update safe-relative-path checks still run before writing.
- No symlink following, path traversal, absolute path or backslash behavior changes.
- There is no repository-name special case and no new ignore list.

## Compatibility

- `agent-browser` remains scoped to `skills/agent-browser`; `skill-data/`, `cli/`, `packages/` and root files stay excluded.
- `skill/SKILL.md` keeps `sourcePath = "skill"` and copies only that subtree.
- Multi-skill and agent-specific roots remain subtree-scoped.
- Root repositories containing example/fixture `SKILL.md` files continue to prefer the direct root candidate; recursive fallback does not run after a direct candidate is found.
- Repository rows, member assignments, update-state schema, preview DTOs and import selections do not change.

## Test Strategy

1. Pure mapping table for root, nested, unrelated and unsafe-looking paths.
2. Import collector test proving root descendants are included and nested scope remains narrow.
3. End-to-end root import test proving nested files are written and repository assignment stays `.`.
4. Update filesystem test proving root descendants affect hash and atomic output.
5. Inventory/apply regression proving an incomplete root installation becomes updateable and is repaired, including stale-file removal.
6. Existing remote import script test plus an assertion that root maps to the remote repository directory.
7. Existing root candidate, `skill/` container, generic filter, plugin grouping and nested import regressions remain green.

## Rollback

The production change is limited to a pure path mapping helper and two collector call sites. If a regression appears, revert those call sites and helper together; no schema or data rollback is required. Existing incomplete installs remain in their pre-task state until a corrected update is applied.
