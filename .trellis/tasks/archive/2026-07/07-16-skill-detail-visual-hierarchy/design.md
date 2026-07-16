# Design: Skill Detail Visual Hierarchy

## Boundaries

This is a frontend-only presentation and accessibility task.

Primary files:

- `src/components/skill/SkillDetailSidebar.tsx`: section order, metadata/status composition, empty states.
- `src/components/skill/SkillDetailFileTree.tsx`: file-family presentation, disclosure behavior, accessible names and focus.
- `src/components/skill/SkillDetailViewShared.tsx`: shared inspector labels, metadata rows and platform toggle state.
- `src/test/SkillDetailFileTree.test.tsx` and `src/test/SkillDetailView.test.tsx`: behavior and semantic regression coverage.
- `src/test/themeContrast.test.ts`: label/status contrast contracts when token coverage needs to expand.
- `src/i18n/locales/en.json` and `src/i18n/locales/zh.json`: only for new accessible labels or state copy.

No store, IPC or Rust contract changes are planned. Keep classification local to the detail tree until a second real consumer needs the same mapping.

## Information Architecture

The sidebar should follow the user's decision order:

1. Source exception/status, when present.
2. Installation Status.
3. Update Status, for mutable Central skills.
4. Metadata and Classification Management.
5. Projects Using This Skill and Collections.
6. File Tree.
7. Technical details inside Metadata remain collapsed.

This preserves every section but moves the product-defining filesystem state ahead of reference material. Use spacing and hairline dividers as the default grouping mechanism. Reserve a contained surface for update controls, classification forms and exceptional read-only/error explanations.

## File Presentation Contract

Use a local pure classifier in `SkillDetailFileTree.tsx` that normalizes the basename and extension, then returns a category, Lucide icon and theme class. Match high-specificity rules first: tests before generic code, well-known names before extensions, then fallback.

| Category | Examples | Non-color cue | Theme role |
|---|---|---|---|
| directory | any `dir`, symlink with children | `Folder` / `FolderOpen` | `primary-text` |
| symlink | leaf symlink | `Link2` | `info-foreground` |
| docs | `.md`, `.mdx`, `.txt`, `README*`, `SKILL.md` | `FileText` | `primary-text` |
| structured data | `.json`, `.jsonc`, `.yaml`, `.yml`, `.toml`, `.xml` | `Braces` | `warning-foreground` |
| web/code | `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`, `.css`, `.html` | `FileCode2` | `info-foreground` |
| Python | `.py`, `.pyi` | `FileCode2` plus category test id/label | `success-foreground` |
| Rust | `.rs`, `Cargo.toml`, `Cargo.lock` | `FileCode2` / `Package` | `warning-foreground` |
| images | common raster/vector extensions | `Image` | `success-foreground` |
| config/env | `.env*`, dotfiles, config filenames | `Settings2` | `muted-foreground` or `info-foreground` |
| tests | `*.test.*`, `*.spec.*`, `tests` fixtures | `FlaskConical` | `success-foreground` |
| fallback | unknown file | `File` | `muted-foreground` |

The file name itself stays `text-foreground`; category color is secondary reinforcement. Do not use status-colored background fills on every row. Row hover/focus may use `bg-muted/40`, and the selected/open action must retain a visible `ring` focus treatment.

Top-level directories start collapsed. A directory disclosure control includes its name and `aria-expanded`; opening the directory/path remains an explicitly named secondary action so existing behavior is not removed. Loading uses `aria-busy`, and loading/empty copy remains localized.

## Metadata Presentation

- Local directory: `FolderOpen` plus a primary-tinted icon; path and open/copy action remain neutral.
- GitHub repository: `FolderGit2` or `Github` plus an info-tinted icon; repository name stays readable foreground text.
- Repository path: stays subordinate to repository identity but remains monospaced and wrap-safe.
- Technical details: keep `<details>` progressive disclosure, fix summary focus, and avoid a second nested card treatment.

Extend `MetadataRow` only with optional icon/tone props if that removes duplication across local/GitHub/technical rows. Otherwise compose icons locally in `SkillDetailSidebar`. Do not add a generic panel abstraction merely for class reuse.

## Operational State Contract

Create a small local update-status presentation map keyed by `CentralSkillUpdateStatus`, with checking handled as a transient override:

- not checked / unsupported: neutral icon and label;
- checking: spinner + info/primary treatment;
- up to date: check icon + success;
- update available: download/arrow icon + primary actionable treatment;
- remote missing: broken-link icon + warning;
- error: alert icon + destructive text.

The map changes presentation only. Button enablement and update confirmation remain owned by existing logic.

Platform toggles add an announced pressed state for installable targets. Installed targets get a persistent check/link cue; always-included targets retain a lock cue and disabled semantics. Platform icon identity, title text and existing install/uninstall callbacks remain unchanged.

## Typography And Theme Contract

- Replace literal `text-[11px]` labels with the documented `0.72rem` label scale or a shared Tailwind-compatible class.
- Remove `/70` and `/80` opacity from meaningful labels; use full `muted-foreground` or another contrast-tested foreground token.
- Reuse existing semantic tokens. Do not add hex colors in components.
- If the implementation exposes a new token combination on light surfaces, extend `themeContrast.test.ts` across all six themes or at minimum all light themes plus representative dark themes.

## Compatibility And Rollback

- No persisted data or backend migration.
- Section reordering is DOM-only; existing conditional rendering stays intact.
- File classification falls back to the current generic-file behavior for unknown extensions.
- Rollback is limited to the three skill detail components, focused tests and optional i18n/contrast assertions.

## Risks

- Reordering large JSX blocks can accidentally change guards; move complete sections without rewriting their conditions.
- Status colors can imply warning/error semantics when used categorically. Keep file-category colors on icons only and preserve foreground file names.
- Additional directory actions can create tiny or duplicate targets. Use unique accessible names and test keyboard focus.
- Global changes to `SectionLabel` affect the entire detail view. Verify all sidebar contexts and both light/dark themes.
