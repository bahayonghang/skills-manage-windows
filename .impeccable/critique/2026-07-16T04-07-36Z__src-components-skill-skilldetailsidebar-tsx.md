---
target: skill detail inspector sidebar and file tree
total_score: 24
p0_count: 0
p1_count: 4
timestamp: 2026-07-16T04-07-36Z
slug: src-components-skill-skilldetailsidebar-tsx
---
## Design Health Score

| # | Heuristic | Score | Key issue |
|---|---|---:|---|
| 1 | Visibility of System Status | 3 | Async/update states exist; install confirmation remains icon-only. |
| 2 | Match System / Real World | 3 | Familiar file vocabulary; directory expand/open behavior is split. |
| 3 | User Control and Freedom | 3 | Confirmations and disclosures are solid. |
| 4 | Consistency and Standards | 2 | Tree and hand-styled controls diverge from shared conventions. |
| 5 | Error Prevention | 3 | Invalid/destructive actions are guarded. |
| 6 | Recognition Rather Than Recall | 2 | Platform state requires icon and color knowledge. |
| 7 | Flexibility and Efficiency | 2 | Direct toggles help; expanded tree adds many tab stops. |
| 8 | Aesthetic and Minimalist Design | 2 | Repeated slabs flatten hierarchy and bury install state. |
| 9 | Error Recovery | 2 | Update errors appear; some open-path failures stay silent. |
| 10 | Help and Documentation | 2 | Read-only explanations help; tree/platform state is under-explained. |
| **Total** | | **24/40** | **Acceptable; significant hierarchy and accessibility work remains.** |

## Anti-Patterns Verdict

The page is not broadly AI-generated: real paths, honest filesystem states and a restrained theme keep it recognizably SkillPort. The inspector rail does show localized AI-slop drift through repeated rounded slabs, pill status, uppercase micro-labels and equal visual weight across unrelated sections.

The deterministic detector reported 10 advisory `design-system-font-size` findings: `SkillDetailSidebar.tsx` lines 136, 194, 203, 212, 418, 493, 630, 636 and 666, plus `SkillDetailFileTree.tsx` line 87. All use literal `text-[11px]`, while DESIGN.md defines labels as `0.72rem`. No false positive was confirmed; treat this as one repeated type-ramp defect rather than ten separate issues.

Browser inspection reached a fixture skill drawer at `http://127.0.0.1:24200/central`, but the populated body was replaced by the explicit “Skill detail needs the desktop runtime” fallback. Mutable injection could not be proven, so no overlay was claimed. The three user-supplied screenshots are the faithful visual evidence.

## Overall Impression

The two-pane shell and real filesystem content are strong. The largest opportunity is to make the sidebar follow the product decision order: installation and update first, organization second, files and technical metadata last.

## What's Working

- Read-only sources are explained and mutation controls are honestly withheld.
- Technical metadata uses appropriate progressive disclosure.
- Loading, empty, disabled, update and error states already exist in product code.

## Priority Issues

### P1: Installation state relies on color and opacity

`SkillDetailViewShared.tsx:112-131` renders icon-only toggles without `aria-pressed`; installed, uninstalled and locked states depend heavily on accent/opacity. Add announced state plus persistent check/link/lock cues.

### P1: File-tree controls are inefficient and ambiguously named

`SkillDetailFileTree.tsx:39-67` splits expansion and opening into repeated generic controls without directory-specific accessible names or `aria-expanded`. Use named disclosures, visible focus and an explicit secondary open action while preserving path behavior.

### P1: Meaningful labels lose contrast

`SkillDetailViewShared.tsx:13-34` and `SkillDetailFileTree.tsx:87` combine literal 11px text with `/70` or `/80` opacity. Use the documented label scale and a contrast-tested foreground across all themes.

### P1: Sidebar order contradicts the product promise

`SkillDetailSidebar.tsx:327-570` places Metadata, File Tree, Update and Classification before Installation Status. Move installation/update ahead of reference material and default-collapse top-level directories.

### P2: Repeated bordered slabs flatten hierarchy

`SkillDetailViewShared.tsx:195` and `SkillDetailFileTree.tsx:90` repeat nearly identical contained surfaces. Use spacing and hairline dividers for metadata; reserve containment for stateful controls and exceptional states.

## Persona Red Flags

- **Alex, power user:** the expanded tree creates many tab stops and pushes core install controls below secondary information.
- **Sam, accessibility-dependent:** platform state lacks announced pressed state; directory controls have duplicate names; essential labels fall below AA in some themes.
- **Mina, multi-agent skill curator:** platform state requires memorized icons and hover/color inference, so distribution cannot be audited confidently at a glance.

## Minor Observations

- `sortedEntries` in `SkillDetailFileTree.tsx:83` does not sort.
- Collapsed directories still use `FolderOpen`.
- File-tree loading does not expose `aria-busy`.
- The GitHub action has a weaker hand-written focus treatment than shared buttons.

## Questions to Consider

- If “where is this installed?” is SkillPort's defining promise, why is Installation Status the fifth major section?
- Which bordered rectangles communicate real containment, and which only add texture?
- Can every platform and file-tree state be explained without color, hover or icon memorization?
