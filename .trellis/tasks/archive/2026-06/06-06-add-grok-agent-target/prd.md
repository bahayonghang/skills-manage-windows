# Add Grok agent target

## Goal

Add Grok as a first-class built-in skills target in this fork so users can install, scan, view, and document Grok skills with the paths introduced by upstream Skills Manager v1.23.0.

## User Value

Users who use Grok can manage Grok skills from SkillPort instead of manually copying skills into Grok-specific directories.

## Source Evidence

- Upstream release: https://github.com/xingkongliang/skills-manager/releases/tag/v1.23.0
- Upstream comparison: https://github.com/xingkongliang/skills-manager/compare/v1.22.5...v1.23.0
- Release notes state Grok is a built-in agent with global path `~/.grok/skills`, project path `<repo>/.grok/skills`, canonical ordering immediately after Codex, Settings agent grouping, and a dedicated icon.
- Upstream implementation adds a `grok` tool adapter with `relative_skills_dir = ".grok/skills"` and `relative_detect_dir = ".grok"`, adds `grok` after `codex` in `DEFAULT_PRIORITY_ORDER`, adds merge-order tests, adds `grok` to the Settings mainstream group, and adds icon mapping for `grok.svg`.

## Confirmed Local Facts

- This fork does not currently seed a `grok` agent target in `src-tauri/src/db/seed.rs`; `rg` found Grok only in Skill Usage provider code.
- Skill Usage already has a separate Grok usage provider under `src-tauri/src/services/usage/providers/grok.rs`; the new task should not duplicate or refactor usage ingestion.
- Built-in platform targets are seeded through `src-tauri/src/db/seed.rs`, and repeated startup updates existing built-in rows while preserving only the Central store path.
- Frontend default platform visibility mirrors backend defaults in `src/lib/platformVisibility.ts`.
- Frontend target grouping and install-agent ordering live in `src/lib/platformTargetGroups.ts`.
- Platform icons are rendered by `src/components/platform/PlatformIcon.tsx`.
- Documentation lists supported Coding targets and paths in both `README.md` and `README_CN.md`.
- Project guidance requires user-visible text to go through i18n where applicable, and requires `just ci` before completion.

## Requirements

- Add a built-in `grok` coding agent target.
- Use the Grok global skills directory `~/.grok/skills` on local Windows/macOS/Linux paths and `/home/.../.grok/skills` for remote POSIX-home seeding.
- Support Grok project skills under `.grok/skills`.
- Place Grok immediately after Codex in default-enabled platform ordering and user-facing platform sorting.
- Implement Grok strictly as an independent upstream-compatible target, not as a member of this fork's Universal Agents virtual target.
- Show Grok with a recognizable platform icon rather than the generic fallback.
- Update tests that assert built-in agent seed data, remote-home paths, project install paths, platform visibility ordering, and icon coverage.
- Update README and README_CN path tables and explanatory text so the documented Grok path matches the implementation.
- Keep Skill Usage Grok provider behavior unchanged except for any display/icon reuse that is directly required.

## Acceptance Criteria

- [ ] A fresh database contains a built-in `grok` agent with display name `Grok`, category `coding`, global skills dir ending in `.grok/skills`, project skills dir `.grok/skills`, an icon name, and default enabled state consistent with the approved scope.
- [ ] Remote-home database initialization resolves Grok global skills to `<remote-home>/.grok/skills`.
- [ ] Installing a central skill into Grok writes to Grok's own configured target path.
- [ ] Installing a project skill into Grok writes to `<project>/.grok/skills/<skill>`.
- [ ] Platform visibility / sorting shows Grok immediately after Codex when it is enabled.
- [ ] Platform icon tests cover Grok and do not fall back to the generic unknown icon.
- [ ] README.md and README_CN.md include Grok in the Coding target table with the same paths used by code.
- [ ] Focused frontend/Rust tests for the changed surfaces pass.
- [ ] `just ci` passes before the task is declared complete.

## Out of Scope

- Do not implement or change Grok API/model-provider settings.
- Do not change the existing Grok Skill Usage log parser.
- Do not adopt upstream v1.23.0 preset export or active-preset install behavior unless the Grok implementation reveals a direct dependency.
- Do not rename existing Codex, Antigravity, Gemini, or Universal Agents behavior.
- Do not add Grok to Universal Agents membership, install-agent order, or representative-agent constants.

## Decisions

- Grok must follow upstream exactly as an independent target using `~/.grok/skills` and `.grok/skills`.

## Open Questions

- None blocking.
