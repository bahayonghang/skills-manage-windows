# Marketplace/Import/CLI 覆盖设计

## Ownership

Own `marketplace.rs`, `github_import.rs`, `local_archive_import.rs`, `portable_state.rs`, `skills_cli.rs` and their
operation-record construction seams. Domain behavior remains in existing modules.

## Safe Envelope

Allowed: controlled action/code/category/phase, target kind, requested/succeeded/failed/skipped counts, bounded logical
skill/agent IDs only where existing validators already prove safety, duration, retryable, operation/batch ID.

Banned: URL/ref/SHA/digest, archive/state path or bytes, manifests, PAT/API key, request/response body, AI prompt/response,
CLI command/args/env/output, snapshot tokens and source paths.

## Lifecycle and Delegation

- terminal-only: credential set/clear/test, registry CRUD, reveal/export after result;
- started-then-terminal: registry sync, install/import, explanation generation, portability jobs, Skills CLI mutations;
- preview/search/read/doctor/list: runtime-only on success, backend Runtime on failure;
- outer command owns operation; nested central/install/import implementation returns typed safe outcome only.

## Stable Errors

Reuse existing GitHub/import/Skills CLI code registries. Fill known gaps in the owning domain, never parse Display or HTTP/
process output. Provider availability remains external evidence and is not inferred from fixture success.
