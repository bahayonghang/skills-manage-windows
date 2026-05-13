# Skill Protocol

A skill is a directory containing a `SKILL.md` file with YAML frontmatter and a markdown body. SkillPort follows the [Agent Skills](https://github.com/anthropics/agent-skills) open pattern.

## Directory Layout

```text
my-skill/
├── SKILL.md              required — frontmatter + markdown body
├── reference/            optional — supporting docs the agent may read
│   └── api.md
├── scripts/              optional — runnable helpers
│   └── lint.sh
└── assets/               optional — images, fixtures, prompts
    └── prompt.txt
```

The skill ID is the directory name. The display name and description live in the frontmatter.

## SKILL.md

```markdown
---
name: my-skill
description: One-sentence summary that explains when to use this skill.
version: 1.0.0
---

# My Skill

Markdown body that the agent reads as instructions.
```

### Required Frontmatter Fields

| Field | Type | Notes |
| --- | --- | --- |
| `name` | string | Display name; should match the directory name when possible |
| `description` | string | Short, action-oriented summary used in cards and search |

### Optional Frontmatter Fields

| Field | Type | Notes |
| --- | --- | --- |
| `version` | string | Semver-style; surfaced in detail view |
| `tags` | string[] | Free-form tags; merged with SkillPort's local tag system |
| `author` | string | Display only |
| `homepage` | string | Display only |

Other fields are preserved when SkillPort writes a skill back to disk; only the fields above are interpreted by the UI.

## Body

The markdown body is rendered in the Skill Detail page through `react-markdown` with GFM extensions. Code blocks honor language fences for syntax highlighting.

## Validation

| Failure | Behavior |
| --- | --- |
| Missing `SKILL.md` | Directory ignored; no row written |
| Malformed YAML | Skill listed with raw filename as `name`; description falls back to first paragraph |
| Missing `name` / `description` | Substituted with directory name; warning surfaces in Settings → Diagnostics |

## Update Identity

When an installed skill is later updated remotely (Marketplace / GitHub import), SkillPort matches by `(repository_id, source_path)` so renaming the directory does not produce a duplicate row.

## Deletion Safety

Deleting a Central skill walks the install table and removes every symlink / copy first; a database delete only happens after every filesystem-side install row succeeds.

Last reviewed: 2026-05-04
