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
marketplace skill id + enabled registry source
       │
       ▼
resolve GitHub source and pin one commit/snapshot
       │
       ▼
rebuild candidates and require one exact marketplace id match
       │
       ▼
project the complete candidate directory into CentralSkillWrite
       │
       ▼
target lock → pending recovery → durable stage/swap
       │
       ▼
skill + repository provenance + db_committed in one transaction
       │
       ▼
finalize journal → best-effort installed-cache repair
```

The cached `download_url` and frontmatter display name are never request or path
authorities. Candidate `skill_id` determines the Local/SSH/WSL target directory,
and every target receives `SKILL.md` plus its references, scripts, assets, and
other peers from the same pinned snapshot. A first install uses the existing
`central_update` journal with `hadTarget=false`; overwrite uses the same recoverable
swap with `hadTarget=true`.

`marketplace_skills.is_installed` is derived cache state. It is written only after
the Central filesystem, skill row, repository assignment, commit/digest provenance,
and journal commit succeed. A cache-marker write failure does not turn a committed
install into an error; Marketplace queries derive the live value from Central and
retry the cache repair.

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

Last reviewed: 2026-08-03
