# GitHub import branch-selection contract excerpt

## Current evidence

- `src-tauri/src/services/github_import/source.rs:4-67` parses the source,
  inspects `/repos/{owner}/{repo}`, reads `default_branch`, and lets a tree-URL
  branch override that default.
- `src-tauri/src/services/github_import/source.rs:70-155` is the shared
  owner/repo/branch/subpath parser. `/tree/<branch>/<subpath>` is supported;
  shorthand and URL-only sources remain valid.
- `src-tauri/src/services/github_import/raw_http.rs:166-216` validates owner,
  repo, and branch before structured endpoint construction. The branch must be
  non-empty, trimmed, not `.`/`..`, and contain no control character, `/`, or
  `\\`.
- `src-tauri/src/services/github_import/snapshot.rs:268-293` parses the submitted
  source offline and binds target, owner/repo, optional URL branch, and source
  path to the registered preview snapshot.
- `src-tauri/src/commands/github_import.rs:20-90` keeps Tauri commands as thin
  shells. Preview selects Local vs SSH/WSL; import consumes `previewId` and does
  not re-resolve repository content.
- `src-tauri/src/services/github_import/tests.rs:502-512` covers tree URL parsing;
  `:2350-2400` covers target/repo/source/branch snapshot mismatch.
- `src/lib/ipc/commandMap.ts:136-154` is the typed renderer command contract.
  `src/stores/marketplaceStore.githubImportSlice.ts:49-120,122-210` owns preview
  replacement and snapshot-backed confirm state.

## Applicable executable contracts

Source: `.trellis/spec/backend/github-import-preview-contract.md`.

1. Renderer data may choose a structured repository and source path, but never
   request scheme, authority, port, IP, redirect target, or authentication
   destination.
2. Acquisition validates owner, repo, branch, and source path, then constructs
   requests only under built-in GitHub/mirror endpoints. `normalizedUrl` is
   display/reference data, not a routing authority.
3. Invalid repository components fail before issuing a request. HTTPS, exact
   host/base prefix, no redirects, PAT non-forwarding, and resource budgets stay
   unchanged.
4. Preview resolves the chosen ref to an immutable commit once. Tree/raw/archive
   acquisition uses that commit while preview display and stored provenance keep
   the user-facing branch.
5. Markdown reads and confirm import use only the registered preview snapshot.
   Missing, expired, target/repo/source mismatch, integrity mismatch, and busy
   tokens fail closed; there is no branch re-fetch fallback.
6. Import validates binding, selection, and retained-byte integrity before any
   Central filesystem/database mutation. Failure releases the lease for retry;
   success consumes the token.
7. Local and SSH/WSL flows must retain the same structured branch and snapshot
   semantics even though their acquisition/storage transports differ.
8. Per-skill provenance is written transactionally; later writers cannot erase
   known commit/digest data. This task changes no schema or persistence shape.

## Branch-selection decisions

- The new manual field is optional. Blank means the existing default/tree-URL
  resolution path; `dev` is the representative explicit value.
- The branch is a structured request field. React must not append `/tree/` or
  reproduce the Rust parser.
- URL branch plus manual branch: absent/manual or URL-only is accepted; equal
  values are accepted; unequal values fail before repository inspection.
- Manual input is normalized and validated before network IO. Slash-containing
  refs, tags, SHAs, and branch-list enumeration are outside this task.
- Existing CLI/service entry points continue to call the branch-aware core with
  no manual override, preserving URL-only behavior.
- Import sends the branch selection associated with the active preview; the
  opaque snapshot remains the content authority.

## Required proof

- Pure resolver cases: default, explicit `dev`, URL `dev`, equal URL/manual,
  conflict, blank, whitespace, slash, and invalid control input.
- Snapshot cases: matching explicit branch succeeds; mismatched branch fails and
  retains the token; moved upstream branch cannot alter imported bytes.
- Cross-transport/service cases: Local and SSH/WSL receive the same chosen
  `GitHubRepoRef`; CLI omission keeps current behavior.
- Renderer cases: typed IPC carries `branch`; blank becomes absent; confirm uses
  preview-associated selection; reset/deep-link lifecycle clears manual state.
- Security regressions: endpoint policy, PAT/mirror rules, budgets, redaction,
  typed IPC coverage, immutable preview contract, and final `just ci`.
