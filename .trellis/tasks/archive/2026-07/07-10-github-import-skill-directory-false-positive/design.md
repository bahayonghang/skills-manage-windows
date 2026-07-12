# Design

## Scope

This task fixes two deterministic failures in one GitHub preview workflow:

1. A valid repository-level `skill/SKILL.md` is discovered and parsed but then discarded as a generic candidate.
2. The resulting non-authentication error contains `subpaths`, whose `pat` substring incorrectly triggers GitHub PAT guidance.

The deliverables stay in one task because they share the same user-visible reproduction and can be validated together without introducing independent rollout or migration work.

## Current Data Flow

```text
repo URL
  -> resolve_repo_source
  -> download repository snapshot / create remote workspace
  -> discover skill/SKILL.md
  -> build_remote_skill_candidate
       source_path = "skill"
       skill_id = "skill"
  -> is_generic_remote_skill_candidate == true
  -> candidate silently dropped
  -> preview skills empty
  -> NoImportableSkills string
  -> looksLikeGitHubAuthGuidance
       "subpaths" contains "pat"
  -> unrelated PAT guidance rendered
```

## Backend Design

### Repository-Level Singular Skill Container

Treat exactly `skill/SKILL.md` at repository depth one as a repository-level skill container. It is analogous to a root `SKILL.md` for candidate identity while retaining `source_path = "skill"` as the content and update boundary.

Candidate fields for `yetone/kill-ai-slop`:

```text
source_path          = "skill"
skill_id             = "kill-ai-slop"
skill_name           = frontmatter.name ("kill-ai-slop")
root_directory       = "/"
skill_directory_name = "skill"
```

The skill ID uses the existing repository-ID normalization already applied to root `SKILL.md`, including the existing `-skill` suffix behavior. The rule must live in the shared candidate builder so local snapshots and SSH/WSL workspaces cannot diverge.

The source path must not be rewritten to `.`:

- import staging must copy only the `skill/` subtree, not the website or other repository contents;
- source/update metadata must continue to point at `skill`;
- root directory and download URL semantics remain repository-relative;
- import selections remain keyed by the stable repository source path.

### Preserve Generic Deep-Directory Filtering

Keep `is_generic_remote_skill_candidate` and its security/quality role for deeper paths such as:

```text
agent_reach/skill/SKILL.md
packages/example/skill/SKILL.md
```

Those candidates continue to derive `skill_id = "skill"` and remain filtered. Only the exact repository-level path `skill` receives repository identity, so the original regression test remains valid.

### Explicit Source Subpath

For `.../tree/main/skill`, discovery still emits the repository-relative manifest path `skill`. The same shared identity rule therefore returns the same candidate as the repository-root URL without special URL parsing or duplicated branches.

### Compatibility

No DTO, IPC payload, database schema, or import-selection changes are required. `pluginName`, conflict detection, rename/overwrite behavior, and imported summary shapes remain unchanged.

The ID change only affects a candidate that was previously unimportable, so there is no migration for existing Central rows.

## Frontend Design

### Authentication Guidance Classification

Keep classification in `looksLikeGitHubAuthGuidance`, but replace the broad substring expression with explicit signals that match backend denial messages:

- rate-limit wording;
- `Personal Access Token` or standalone `PAT`;
- `GitHub denied access`;
- `requires authentication`;
- configured GitHub token wording.

Do not treat bare `github`, bare `settings`, or embedded `pat` substrings as sufficient. This prevents invalid URLs, unsupported paths, parse errors, archive failures, and `subpaths` from showing authentication guidance.

`looksLikeConfiguredGitHubTokenFailure` remains the selector for the configured-token variant of the existing localized hint. No new user-visible text is planned.

### Test Seam

Cover the pure classifier with representative backend strings and cover the rendered input-step error in the real `GitHubRepoImportWizard`:

- `No importable ... subpaths ...` renders the error but no PAT hint;
- rate-limit denial still renders the generic PAT settings hint;
- configured-token denial still selects the configured-token hint.

Where marketplace view tests use a mocked wizard, reuse the production helper instead of retaining a second regex if that support code must change.

## Contract And Spec Updates

Extend `.trellis/spec/backend/github-import-preview-contract.md` with a repository-level singular `skill/` scenario and an error-guidance matrix. The durable contract is:

- top-level `skill/` uses repository identity but retains `sourcePath=skill`;
- deeper generic `.../skill/` remains filtered;
- auth guidance is not inferred from arbitrary substrings.

No frontend-wide async error contract change is needed because the dialog already shows and clears the inline error correctly; this task changes classification, not lifecycle behavior.

## Risks And Mitigations

- **Accidentally copy the whole repository.** Keep `source_path = "skill"`; test imported files or source metadata against that boundary.
- **Re-admit the original generic wrapper candidate.** Keep the deep-path test and crafted-selection rejection test green.
- **Change IDs for normal nested skills.** Limit repository identity to exact `source_path == "skill"`; add neighboring negative tests.
- **Local/remote drift.** Change only `build_remote_skill_candidate`, which both flows use, and retain remote discovery tests.
- **Hide real auth help.** Test actual `GitHubAccessDenial` display phrases for rate-limit, unauthenticated, and configured-token cases.
- **Test mocks diverge from production.** Prefer importing the production classifier in test support instead of duplicating its expression.

## Rollback

Rollback is code-only and requires no data migration:

1. Revert the repository-level ID branch to restore the old generic filter behavior.
2. Revert the classifier phrase list to restore the previous hint behavior.
3. Remove the paired regression cases and spec additions.

Because previously successful imports and persisted data are unchanged, rollback does not require database cleanup.
