# Bug Analysis: root skill repositories lose descendant files

## Summary

The repository parser finds root-level skills correctly, but two snapshot consumers interpret the root source path `.` as "top-level files only" instead of "the complete repository tree". The same faulty filter is used for initial import and Central update hashing/writes, so root skill repositories are truncated on disk and the update center cannot detect the missing descendants.

## Reference Repository Evidence

Evidence was collected from GitHub on 2026-07-15. `agent-browser` browser reads failed in this environment, so the authoritative repository metadata and tree endpoints were queried through `gh api`.

| Local skill | GitHub source | Default branch / observed HEAD | Manifest layout | Upstream tree | Local result |
| --- | --- | --- | --- | --- | --- |
| `agent-browser` | https://github.com/vercel-labs/agent-browser | `main` / `3591f0f4b719c94bcb9aec83ebe811c5dd7f587a` | `skills/agent-browser/SKILL.md` | Selected subtree contains only `SKILL.md` | Correct single-file control; source path is `skills/agent-browser` |
| `huashu-design` | https://github.com/alchaincyf/huashu-design | `master` / `0e7ec8aca0058184c1a9e06e57697e84f68a3f0f` | Root `SKILL.md` | 177 files; top dirs `assets`, `demos`, `references`, `scripts` | Only 9 root files; all four directories missing |
| `yao-meta` | https://github.com/yaojingang/yao-meta-skill | `main` / `4eb11f923dc71173736ebf541a7eebfff942d10e` | Root `SKILL.md` | 650 files; 20 top-level directories | Only 9 root files; all directories missing |

The local `SKILL.md` blobs exactly match their intended upstream paths:

```text
agent-browser  local == skills/agent-browser/SKILL.md  bdd73cc60a51261b0d18e3d3d646cba9e6280bc2
huashu-design  local == SKILL.md                       e86780d280ea57bb8e67cc2d5a5dbe32e913e056
yao-meta       local == SKILL.md                       a2c7a6e8974d698fd8716daa642d3e0036727820
```

This rules out stale manifests or wrong GitHub repositories. The importer selected the right manifest and then dropped descendant files.

## Layout Classification

### Root package

`huashu-design` and `yao-meta-skill` use the repository root as the skill package:

```text
repo/
  SKILL.md
  references/**
  scripts/**
  assets/**
  ... other tracked repository files
```

For this layout:

- candidate identity is repository-derived;
- `sourcePath` is `.`;
- content boundary is the complete repository snapshot;
- GitHub archive budgets and safe relative-path checks remain the safety boundary.

`yao-meta-skill` also contains example and fixture `SKILL.md` files. Current discovery already handles this correctly: `direct_skill_manifest` adds the root manifest, priority roots are checked, and recursive fallback runs only when no direct/priority candidates exist (`source.rs:516-531`). The examples are therefore content inside the root package, not extra import candidates.

### Nested package

`agent-browser` is a control case, not a root package:

```text
repo/
  skills/
    agent-browser/
      SKILL.md
  skill-data/**
  cli/**
  packages/**
```

The database records `source_path = "skills/agent-browser"`; the selected subtree currently contains exactly one file. Importing the whole repository would be a regression because it would include unrelated product source and runtime data.

## Database Evidence

The live local database confirms candidate identity and source assignment are already correct:

```text
agent-browser  -> vercel-labs/agent-browser  source_path=skills/agent-browser
huashu-design  -> alchaincyf/huashu-design   source_path=.
yao-meta       -> yaojingang/yao-meta-skill  source_path=.
```

Both root packages have `skill_update_states.status = up_to_date` despite missing every descendant directory. This is expected from the faulty hash input, not evidence that the local package is complete.

## Code-Level Root Cause

### Candidate discovery is correct

`src-tauri/src/services/github_import/source.rs:357-388` already gives root manifests repository identity and preserves `source_path = "."`. `manifest_from_skill_md_path` maps root `SKILL.md` to `.` and `/` as expected.

### Initial local import truncates the root

`src-tauri/src/services/github_import/progress.rs:18-54` collects files for staging. Its root branch is:

```rust
if source_path == "." {
    if path.contains('/') {
        return None;
    }
    path.clone()
}
```

Every descendant such as `references/guide.md` contains `/` and is discarded before progress totals and staging writes are built. The nested branch correctly strips the selected source prefix and keeps descendants.

### Updates repeat the same truncation

`src-tauri/src/services/central_updates/fs.rs:115-151` duplicates the same root branch in `collect_remote_skill_files`. The resulting file list feeds:

- remote hash generation in `core.rs:503-513`;
- normal and force update plans;
- atomic local/remote Central writes;
- copy-install refreshes.

Consequences:

1. descendant-only upstream changes do not affect `remote_hash`;
2. incomplete root packages can compare equal and remain `up_to_date`;
3. applying an update still writes only root-level files;
4. replacing the target directory removes any locally restored descendants again.

### Remote direct import already has the intended behavior

`src-tauri/src/services/github_import/remote.rs:408-450` maps `source_path = "."` to the remote repository directory and runs:

```sh
cp -a "$source_dir/." "$stage_dir/"
```

This recursively copies the full root package. The bug is therefore a local snapshot collector defect plus a shared Central update collector defect, with a transport consistency regression.

## Root Cause Classification

- **Primary: implicit path assumption.** `.` was interpreted as "files with no directory separator" instead of the source root containing every descendant.
- **Secondary: duplicated contract.** Import and update copied the same incorrect path-remapping logic into separate modules.
- **Contributing: test gap.** Root tests stop at candidate preview/conflict behavior; the only content-boundary test uses a nested source path.
- **Observed impact: false integrity state.** The update hash consumes the same truncated set, hiding the corruption as `up_to_date`.

## Required Prevention

1. Define one pure repository-file-to-source-relative-path function and reuse it in both collectors.
2. Pin `.` as identity mapping for every safe repository file path, not only direct children.
3. Add neighboring root and nested fixtures to prevent scope broadening.
4. Test the full round trip: collect -> hash -> atomic write -> refresh, not candidate discovery alone.
5. Extend `.trellis/spec/backend/github-import-preview-contract.md` with a root content-boundary scenario, including existing-install repair behavior.

## Evidence Commands

```powershell
gh api repos/<owner>/<repo>
gh api repos/<owner>/<repo>/contents
gh api "repos/<owner>/<repo>/git/trees/<branch>?recursive=1"
git hash-object C:\Users\lyh\.skillsmanage\skills\<skill>\SKILL.md
python -  # sqlite3 stdlib used read-only because sqlite3 CLI is unavailable
```
