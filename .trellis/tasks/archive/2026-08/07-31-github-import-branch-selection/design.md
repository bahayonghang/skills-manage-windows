# Design: GitHub import manual branch selection

## 1. Invariants and boundaries

The optional branch input selects repository identity; it does not select
content after preview. Preview resolves the chosen branch tip once, registers an
immutable snapshot, and confirmation consumes that snapshot. No renderer value
may choose a network authority, bypass the shared GitHub source parser, or cause
confirmation to re-fetch a moved branch.

This task adds a manual single-segment branch value. It does not add branch-list
discovery or relax the current ref validator to allow `/`.

## 2. Frontend state and UX

Add controlled `branch` and `onBranchChange` props beside the existing
`repoUrl` props. The shared import-intent store owns `githubBranch` because the
same wizard is launched from Central, Marketplace, and deep-link intent flows.
The value remains presentation state until preview submission.

The input block renders a compact optional branch field with localized label,
placeholder, and a hint that blank uses the repository default. It uses the
existing `GitBranch` icon and responsive input layout; it does not render
hard-coded branch pills or a fake dropdown. Preview is disabled only by the
existing URL/loading rules; backend validation owns branch correctness so the
same rule applies to every caller.

Session cleanup clears `githubBranch` together with `githubSource`. Opening or
consuming a deep-link source also clears `githubBranch`, because a branch already
encoded in `/tree/...` is authoritative for that new intent and must not inherit
manual state from the previous session. Dirty-session detection includes a
non-empty branch even when the URL is blank.

## 3. Structured request contract

Thread an optional `branch` field through the wizard callbacks, marketplace
store actions, and the typed command map:

```ts
preview_github_repo_import: {
  repoUrl: string;
  branch?: string | null;
}

import_github_repo_skills: {
  previewId: string;
  repoUrl: string;
  branch?: string | null;
  selections: GitHubSkillImportSelection[];
}
```

The store normalizes blank/whitespace-only UI input to `null` for transport and
records the branch request associated with the active preview. Confirm sends
that preview-associated value, not an unrelated later input value. Existing
callers that omit `branch` retain default/tree-URL behavior.

The renderer never appends `/tree/` or parses a GitHub source. `repoUrl` remains
the display and source-path input; `branch` is a structured selection hint.

## 4. Shared Rust resolution

Introduce a small service-layer source request/helper around the existing
parser. Resolution order is:

1. Parse and validate owner, repo, URL branch, and source subpath with
   `parse_github_source`.
2. Normalize the optional explicit branch: trim; blank becomes absent; invalid
   single-segment values return a branch-specific typed error before network IO.
3. If URL and explicit branches are both present, require exact equality. A
   mismatch returns a branch-conflict typed error before repository inspection.
4. If either explicit or URL branch is present, use it. Otherwise inspect the
   repository and use GitHub's `default_branch` as today.
5. Build and validate one `GitHubRepoRef`, then enter the existing commit pin,
   tree/archive acquisition, preview registry, import, provenance, and update
   paths unchanged.

The Tauri preview command passes the optional field into both Local and SSH/WSL
service entry points. Existing service/CLI helpers remain source-compatible by
calling the new branch-aware core with `None`; CLI users can continue selecting a
branch through an existing tree URL.

Snapshot binding gains the same optional branch evidence. The token remains the
content authority, but an explicit branch or URL branch must match the snapshot's
display repo. Import still fails before mutation on any binding mismatch.

## 5. Errors and localization

Add stable, non-sensitive GitHub import error codes for invalid manual branch and
URL/manual conflict. Missing or inaccessible branches continue through the
existing commit-resolution failure path. English and Chinese resources explain:

- branch must be a safe single-segment name;
- URL branch and branch field disagree;
- inaccessible/missing branch requires correcting the branch and previewing
  again.

Messages do not include PATs, snapshot IDs, workspace paths, or response bodies.

## 6. Compatibility and rollback

No database migration is needed: repository and per-skill provenance already
persist the resolved `GitHubRepoRef.branch`, and Central update identity already
uses it. No production dependency or packaging change is required.

Rollback removes the branch input/state and optional IPC field, then routes the
service wrappers back to the existing URL-only resolver. Existing tree URLs,
CLI import, stored provenance, and imported skills remain valid.

## 7. Verification strategy

- Pure Rust tests cover default, explicit, same-URL, conflicting, whitespace,
  slash-containing, and missing branch cases, including failure before request
  or Central mutation where test seams allow.
- Snapshot tests cover branch binding and retained-byte import behavior.
- Store/IPC tests assert `branch: "dev"` on preview and confirm, `null` for blank,
  and preview-associated branch reuse.
- Import-intent tests cover dirty detection and branch reset on close, new import,
  target reset, and queued deep-link consumption.
- Wizard tests cover bilingual labels/hints, user entry, submission, and existing
  `/tree/dev` compatibility while preserving stable mocked selector state.
- Contract tests keep the typed command map and immutable snapshot boundary in
  sync; final acceptance uses `just ci`.
