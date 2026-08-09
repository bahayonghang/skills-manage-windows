# GitHub import lifecycle contract snapshot

## Purpose and authority

This task-local note extracts the constraints needed by the Marketplace install and
snapshot-lifecycle children from the active
`.trellis/spec/backend/github-import-preview-contract.md` at `dev@b242ed92`.
The source spec remains authoritative and must be reread before implementation and
checking. This focused note exists because the source is larger than Trellis's
32 KiB context-injection cap and its relevant immutable-lifecycle scenario begins
after the truncation point.

## Existing immutable preview contract

- Renderer-driven GitHub preview state is session-scoped and target-bound. Preview
  pins the repository tip once and retains the bytes the user reviewed.
- Import requires the preview token, validates request binding, selection and
  digest before mutation, and never silently reacquires a moving branch.
- The import lease is single-holder. Import failure releases the lease for retry;
  success consumes the entry; discard during an active lease is deferred until
  release.
- Expired or missing state fails closed. Existing lifecycle error codes and fixed
  public summaries remain compatible and must not expose tokens, paths, digests,
  repository URLs or credentials.
- Per-skill commit/digest provenance is written in the same transaction as skill
  upsert and repository assignment. Later provenance-less writers must not erase
  known provenance.
- Local retained snapshots and remote workspaces represent the same candidate
  inventory but may have different repository-level digests because their retained
  file scopes differ.

## Constraints for snapshot lifecycle optimization

- Preserve snapshot binding, digest verification, TTL, retry-after-failure,
  consume-on-success and deferred-discard behavior while adding capacity limits.
- An active import lease is never an eviction victim.
- The current registry is process-global. A prior prune-on-lookup design was
  reverted because access from one flow could prune another test/session's state;
  capacity work must therefore use deterministic, target-scoped transitions.
- Remote workspace ownership cannot disappear from the registry until cleanup has
  succeeded through the owning target adapter. Cleanup failure must retain a
  retryable state, and stale acknowledgements must not delete a replacement entry.
- Registry mutation remains synchronous and short; remote cleanup runs outside the
  registry mutex through an immutable ticket/ack transition.

## Constraints for Marketplace reuse

- The existing preview contract is renderer-specific; Marketplace must not persist
  or counterfeit renderer preview tokens.
- Marketplace may reuse the pinned acquisition, candidate selection and shared
  import use case, but it creates its own verified snapshot/workspace for the
  requested registry entry.
- Candidate identity and source path decide the import target. Frontmatter display
  names and cached download URLs are not filesystem or request authorities.
- The final Central apply reuses the existing lock, recovery journal, complete
  directory import and provenance transaction for Local, SSH and WSL targets.

## Required regression evidence

- Existing immutable-preview lifecycle, digest, binding, provenance, retry,
  deferred-discard, redaction and cross-transport tests remain green.
- New tests cover bounded cache/registry state, cross-target cleanup ownership,
  cleanup failure/retry and Marketplace candidate/path isolation without weakening
  the existing six lifecycle error mappings.
