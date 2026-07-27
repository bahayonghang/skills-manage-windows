# Live GitHub Preview Snapshot Evidence

Date: 2026-07-27

## Current data flow

- Local preview downloads/extracts a bounded `GitHubRepoSnapshot`, builds preview files, then drops the snapshot. Local markdown and import resolve from repo/branch again.
- SSH/WSL preview creates a remote temporary workspace and registers a 30-minute in-memory token. Markdown reads use that workspace.
- Remote import accepts an optional workspace ID. Missing, expired, or unknown IDs silently create a fresh workspace from the current branch, so the current token is a performance hint rather than an immutable content authority.
- A successful remote import consumes and removes the workspace. Failed imports leave it available for retry.

## Reusable contracts

- `GitHubRepoSnapshot` and remote workspace already represent the two storage variants.
- `GITHUB_PREVIEW_WORKSPACES`, target/repo/source matching, TTL pruning, discard, and target-reset cleanup can evolve into one enum-backed registry.
- Local archive import provides stable SHA-256 + byte-length fingerprinting and fail-before-mutation behavior.
- `github-import-preview-contract.md` already owns bounded fetch, typed failures, and candidate/file-manifest rules.

## Migration/provenance placement

- The live migration runner has immutable descriptors v1-v3; the next schema change must be v4 with checksum coverage.
- `skill_repositories` is shared by all skills from one repo. Repo-level commit/digest would overwrite provenance when skills were imported from different snapshots.
- `skill_repository_members` is per skill and already owns `source_path`; nullable `resolved_commit_sha` and `content_digest` belong there.
- Existing rows remain NULL (`provenance unknown`). New import persistence must write skill and membership provenance in one transaction.

## Resolved product choice

The user selected consume-on-success. Markdown reads and failed imports may reuse an unexpired token; a successful import consumes it immediately. Expiry, explicit discard, target reset, or application restart also invalidates the token. An exclusive import lease is required so two concurrent imports cannot both mutate before the success-time consume step.
