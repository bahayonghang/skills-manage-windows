# Central Skill Tags

## Scenario: Built-In Tag Taxonomy And AI Tagging

### 1. Scope / Trigger

- Trigger: Changing Central Skills taxonomy, skill tag seed data, AI tag suggestions, or frontend tag-filter compatibility.
- Owner files: `src-tauri/src/db/seed.rs`, `src-tauri/src/db/types.rs`, `src-tauri/src/services/ai_tagging/*`, and frontend Central Skills tag consumers.
- Principle: treat `uncategorized` as a system fallback and smart-view filter, not as an ordinary user-facing category.

### 2. Signatures

- DB tables: `skill_tags`, `skill_tag_links`, `skill_ai_tag_reviews`.
- Built-in tag constants:
  - `ACADEMIC_RESEARCH_WRITING_TAG_ID = "academic-research-writing"`
  - `UNCATEGORIZED_TAG_ID = "uncategorized"`
- Backend command surface stays unchanged: `get_skill_tags`, `assign_skill_tags`, `suggest_skill_tags`, and `bulk_suggest_skill_tags`.
- AI response envelope: `{"tags": [...], "new_tag": {"name", "description", "confidence", "reason"}}`; `new_tag` is optional and legacy bare arrays remain valid.
- Proposal review rows use `PendingAiTagReviewInput { tag_id, confidence, reason, proposed_name, proposed_description }`.
- `SkillAiTagReview.is_proposal` is serialized to the frontend; result/progress payloads carry `proposals` alongside ordinary `suggestions`.

### 3. Contracts

- Fresh seed must create these stable built-in tag ids:
  - `academic-research-writing` / `学术研究与写作`
  - `frontend-development` / `前端开发`
  - `backend-development` / `后端开发`
  - `devops-deployment` / `DevOps 与部署`
  - `testing-quality` / `测试与质量`
  - `docs-writing` / `文档与写作`
  - `data-analysis` / `数据与分析`
  - `design-ui` / `设计与 UI`
  - `ai-prompt-engineering` / `AI 与提示工程`
  - `productivity-tools` / `效率与工具`
  - `office-documents` / `办公文档`
  - `uncategorized` / `未分类`: system fallback for smart views and low-confidence AI fallback.
- Published built-in ids are stable data contracts. Removing one is a destructive migration because startup pruning deletes that row's links and pending reviews.
- Startup seed is custom-first:
  - If a custom row already owns the built-in id, keep the custom row and its links unchanged and skip that built-in.
  - If a different custom row already owns the built-in name, keep the custom row and skip inserting that built-in.
  - If the built-in id already exists as built-in, refresh its metadata. When the desired name is owned by another row, keep the existing name and refresh only non-name metadata so startup cannot fail on `UNIQUE(name)`.
- Startup seed must prune obsolete rows where `skill_tags.is_builtin = 1` and `id` is not in the current built-in set.
- Pruning must delete links and pending AI reviews for pruned built-in tag ids.
- Pruning must not delete custom tags or custom tag links.
- AI prompts must list every existing classifiable tag id, built-in or custom, and exclude only `uncategorized`.
- AI mapping must ignore model-returned `uncategorized` as a primary high-confidence tag and fallback to low-confidence `uncategorized` only when no usable suggestion remains.
- AI must prefer an existing specific tag and may propose at most one new category only when no candidate covers the skill. A valid proposal suppresses the `uncategorized` fallback and always enters review regardless of confidence.
- Proposal generation writes only `skill_ai_tag_reviews`. It must not create a `skill_tags` or `skill_tag_links` row until acceptance; skipping changes review status only.
- Proposal rows store normalized name/description in nullable review columns and are read through a `LEFT JOIN`. Pending rows with neither a matching tag nor non-empty proposal metadata are filtered out.
- Tag IDs have one shared derivation contract: use the normalized ASCII slug when non-empty; otherwise use `tag-` plus the first 16 lowercase hex characters of SHA-256 over the trimmed name. This keeps Chinese proposal identity stable across skills and process runs.
- A proposal whose trimmed name or derived ID matches an existing tag downgrades to ordinary reuse. If a derived ID belongs to a different name during tag creation, creation falls back to a UUID.
- Custom tag creation is atomic: insert with conflict handling, then read by unique name. Concurrent same-name creates and accepting the same proposal for multiple skills reuse one row.
- Accepting a proposal runs tag creation, skill-link upsert, and review status changes in one transaction. The link uses the actual tag ID returned by name-idempotent creation.
- Manual/AI assignment and pending-review replacement validate every referenced tag and skill before mutation. Each public call uses one transaction and bounded inserts; AI replacement deletes only `source='ai'`, so validation or insert failure restores the old AI/pending set and preserves manual links.
- Frontend ordinary filter surfaces must hide unused built-in tags and always hide `uncategorized`; custom tags remain visible even at zero usage. Tag management and assignment surfaces keep the full non-system taxonomy so an unused built-in can receive its first assignment.
- Selected tag sanitization is based on known ids rather than current visibility. Special filter ids `uncategorized`, `updates`, and `ai-review` remain valid.

### 4. Validation & Error Matrix

| Condition | Expected behavior |
| --- | --- |
| Fresh database or repeated startup | Seed all 12 current built-ins idempotently |
| Custom tag already owns a current built-in id | Keep the custom row, metadata, and links; do not convert it to built-in |
| Custom tag with a different id already owns a current built-in name | Startup succeeds; keep the custom row and skip that built-in insert |
| Existing built-in metadata is stale | Refresh name, description, and color unless another row owns the desired name |
| Old built-in tag exists after app upgrade | Delete the old built-in tag row, its `skill_tag_links`, and its pending `skill_ai_tag_reviews` |
| Custom tag has the same display meaning as a retired built-in | Keep it and its links |
| AI returns an unknown tag id/name | Ignore it; if no valid suggestion remains, fallback to `uncategorized` at confidence `0.2` |
| AI returns `uncategorized` directly | Do not treat it as a primary suggestion; use only the low-confidence fallback path |
| AI returns a valid new-tag proposal only | Persist one pending proposal; create no tag and no fallback link |
| Proposal name or derived ID matches an existing tag | Process it as an ordinary suggestion for the existing tag |
| Two skills propose and accept the same new name | Create one custom tag and link both skills to it |
| Proposal is skipped | Mark the review skipped; create no tag or link |
| Pending review points to no tag and has no proposal metadata | Omit it from the review result |
| Normalized tag ID is already owned by another name | Create the new tag with a UUID while preserving name uniqueness |
| URL or saved view contains a deleted ordinary tag id | Ignore or sanitize the stale id at runtime |
| URL or saved view contains `uncategorized`, `updates`, or `ai-review` | Preserve the special filter |

### 5. Good/Base/Bad Cases

- Good: a user-created `literature-review` tag and relevant built-in categories are all offered to AI; prompt guidance prefers the most specific match.
- Good: two skills propose `安全审计`; both reviews share the stable digest ID, and accepting either creates/reuses one custom tag.
- Base: a research-writing skill can receive `academic-research-writing`, while a frontend skill can receive `frontend-development`.
- Base: legacy `[{"tag":"backend-development"}]` output parses with no proposal.
- Bad: creating a custom tag as soon as the model proposes it, which leaves taxonomy residue after skip.
- Bad: using a random proposal ID for a Chinese name, which prevents cross-skill proposal convergence.
- Bad: an unused built-in category occupies ordinary filter space before any skill is linked to it.
- Bad: deleting retired built-ins must not remove a user's custom tag row.
- Bad: startup converts a same-id custom tag into a built-in or fails because a custom tag already owns a built-in name.

### 6. Tests Required

- Rust DB test: fresh and repeated seed contain all 12 current built-ins and not retired built-ins.
- Rust DB tests: same-id and same-name custom conflicts survive startup without metadata, link, or ownership changes.
- Rust DB test: upgraded old built-in rows are pruned with links/reviews, while custom tags remain.
- Rust AI tests: prompt includes ordinary built-in and custom ids, excludes `uncategorized`, requires candidate ids, and allows `{"tags":[]}`.
- Rust AI tests: unknown, empty, or direct `uncategorized` model output maps to low-confidence fallback.
- Rust AI tests: reuse-only, proposal-only, mixed, empty-envelope, and legacy-array formats parse; proposal collisions downgrade to reuse.
- Rust DB tests: legacy review schema gains both proposal columns; proposal rows round-trip without creating tags; orphan rows are filtered.
- Rust DB tests: concurrent same-name creation is idempotent; normalized-ID collisions fall back; accept reuses one tag across skills; skip leaves no tag/link.
- Rust DB tests: mixed valid/missing assignment writes no links; AI and pending-review replacement survive a second-insert trigger with their old sets intact, then succeed on retry.
- Rust service test: a proposal-only bulk result reports one proposal and one review count with no `uncategorized` link.
- Vitest: the review drawer shows the localized proposal badge and description, and accept/skip receive the proposal tag ID.
- Vitest: ordinary filter UI hides `uncategorized` and unused built-ins, shows used built-ins, and always shows custom tags.
- Vitest: selected tag sanitization drops stale ordinary ids and preserves special filters.

### 7. Wrong vs Correct

#### Wrong

```rust
// Treats only custom tags and one historical built-in as classifiable.
let candidates = tags
    .iter()
    .filter(|tag| !tag.is_builtin || tag.id == ACADEMIC_RESEARCH_WRITING_TAG_ID);
```

#### Correct

```rust
// Every existing category is classifiable except the system fallback.
let candidates = tags
    .iter()
    .filter(|tag| tag.id != UNCATEGORIZED_TAG_ID);
```

#### Wrong

```rust
// Creates taxonomy residue before explicit review.
let tag = create_skill_tag(pool, &proposal.name, proposal.description.as_deref(), None).await?;
replace_skill_ai_tags(pool, skill_id, &[(tag.id, proposal.confidence, proposal.reason)]).await?;
```

#### Correct

```rust
// Persist proposal metadata only; accept_ai_tag_reviews creates and links later.
replace_pending_ai_tag_reviews(pool, skill_id, &[PendingAiTagReviewInput {
    tag_id: derive_skill_tag_id(&proposal.name),
    proposed_name: Some(proposal.name),
    proposed_description: proposal.description,
    confidence: proposal.confidence,
    reason: proposal.reason,
}]).await?;
```

