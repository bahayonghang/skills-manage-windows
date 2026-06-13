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

### 3. Contracts

- Fresh seed must create only these built-in tag rows:
  - `academic-research-writing` / `学术研究与写作`: the only ordinary built-in category.
  - `uncategorized` / `未分类`: system fallback for smart views and low-confidence AI fallback.
- Startup seed must prune obsolete rows where `skill_tags.is_builtin = 1` and `id` is not in the current built-in set.
- Pruning must delete links and pending AI reviews for pruned built-in tag ids.
- Pruning must not delete custom tags or custom tag links.
- AI prompts must list existing classifiable tag ids only: custom tags plus `academic-research-writing`; exclude `uncategorized` and retired built-ins.
- AI mapping must ignore model-returned `uncategorized` as a primary high-confidence tag and fallback to low-confidence `uncategorized` only when no usable suggestion remains.
- Frontend Central Skills surfaces must hide `uncategorized` from ordinary tag lists, while keeping special filter ids `uncategorized`, `updates`, and `ai-review` valid.

### 4. Validation & Error Matrix

| Condition | Expected behavior |
| --- | --- |
| Old built-in tag exists after app upgrade | Delete the old built-in tag row, its `skill_tag_links`, and its pending `skill_ai_tag_reviews` |
| Custom tag has the same display meaning as a retired built-in | Keep it and its links |
| AI returns an unknown tag id/name | Ignore it; if no valid suggestion remains, fallback to `uncategorized` at confidence `0.2` |
| AI returns `uncategorized` directly | Do not treat it as a primary suggestion; use only the low-confidence fallback path |
| URL or saved view contains a deleted ordinary tag id | Ignore or sanitize the stale id at runtime |
| URL or saved view contains `uncategorized`, `updates`, or `ai-review` | Preserve the special filter |

### 5. Good/Base/Bad Cases

- Good: a user-created `literature-review` tag is offered to AI before the broad built-in category.
- Base: a research-writing skill can still receive `academic-research-writing`.
- Bad: a coding skill should not be forced into `academic-research-writing` just because it is the only ordinary built-in tag.
- Bad: deleting retired built-ins must not remove a user's custom tag row.

### 6. Tests Required

- Rust DB test: fresh seed contains `academic-research-writing` and `uncategorized`, and not retired built-ins.
- Rust DB test: upgraded old built-in rows are pruned with links/reviews, while custom tags remain.
- Rust AI tests: prompt excludes `uncategorized` and retired built-ins, requires candidate ids, and allows `{"tags":[]}`.
- Rust AI tests: unknown, empty, or direct `uncategorized` model output maps to low-confidence fallback.
- Vitest: ordinary tag UI hides `uncategorized`.
- Vitest: selected tag sanitization drops stale ordinary ids and preserves special filters.

### 7. Wrong vs Correct

#### Wrong

```rust
// Adds every DB tag as an AI candidate, including system fallback tags.
let candidates = tags.iter().map(|tag| tag.id.as_str());
```

#### Correct

```rust
// AI candidates are existing classifiable tags: custom tags plus the single
// ordinary built-in category.
let candidates = tags
    .iter()
    .filter(|tag| !tag.is_builtin || tag.id == ACADEMIC_RESEARCH_WRITING_TAG_ID);
```

