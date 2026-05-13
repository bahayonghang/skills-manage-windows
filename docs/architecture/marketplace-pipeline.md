# Marketplace Pipeline

The Marketplace surface combines four backend services: `marketplace` (sync + cache), `github_import` (preview + import), `central_metadata` (tags + AI suggestions), and `ai_provider` (skill explanation streaming).

## Sync Loop

```text
[user clicks Sync]
       │
       ▼
commands::marketplace::sync_registry
       │
       ▼
services::marketplace::sync — GitHub API: list repo tree
       │
       ▼
parse SKILL.md frontmatter (name, description, downloadUrl)
       │
       ▼
upsert into marketplace_skills with cache_updated_at
       │
       ▼
update skill_registries.last_synced / etag / last_modified
```

Conditional fetch: the registry stores `etag` and `last_modified` from the previous response; subsequent syncs send them as conditional headers and skip parsing on `304 Not Modified`.

## Schema for Caches

| Table | Role |
| --- | --- |
| `skill_registries` | One row per source (GitHub repo or mirror). Status, last error, ETag, expiry. |
| `marketplace_skills` | Cached remote skill metadata, keyed by registry_id. |
| `skill_explanations` | AI-generated explanations keyed by `(skill_id, lang)`. |

See [Data Model](./data-model.md) for full column lists.

## Install From Marketplace

```text
install_marketplace_skill
       │
       ▼
download SKILL.md (and folder peers) into ~/.skillsmanage/skills
       │
       ▼
upsert skills row with canonical_path / is_central=true
       │
       ▼
mark marketplace_skills.is_installed=true
```

The install path reuses `installation::centralize::ensure_centralized` so once the file lands in the Central directory the rest of the pipeline behaves like any other skill.

## GitHub Import

`services::github_import/` handles bulk import from any GitHub repo:

```text
github_import/
├── source.rs           parse user input (owner/repo[@ref][:path])
├── raw_http.rs         minimal reqwest wrapper with PAT auth
├── archive.rs          fetch zipball + extract
├── preview_workspace.rs  scratch directory + cleanup
├── preview.rs          enumerate SKILL.md candidates
├── remote.rs           direct fetch of a single SKILL.md
├── import.rs           promote selected previews into Central
└── pat.rs              GitHub PAT storage
```

Preview returns a workspace id; the UI lists candidates and the user picks which to import. Unselected candidates are discarded by `discard_github_repo_preview_workspace` to keep `temp/` clean.

## AI Explanation

`services::ai_provider/` streams skill explanations:

| File | Role |
| --- | --- |
| `mod.rs` | Provider routing (Anthropic / OpenAI-compatible) |
| `claude.rs` | Anthropic API messages format |
| `stream.rs` | Server-sent events parser |
| `prompt.rs` | Prompt template + locale |
| `cache.rs` | `skill_explanations` read/write |
| `error.rs` | Error mapping to user-friendly strings |

The `explain_skill_stream` command emits `ai-explain://{job_id}` events; the UI subscribes once and renders streaming chunks. Cancellation goes through `AiTagJobRegistry::cancel`.

## Tag Suggestions

`commands::central_metadata::*` powers the tag drawer. AI suggestions are written to `skill_ai_tag_reviews` with `status='pending'`; the UI accepts or skips each one and the row moves to `accepted` or `skipped`. Accepted tags then materialize into `skill_tag_links` with `source='ai'`.

Last reviewed: 2026-05-04
