# Central Skills Screenshot Interface Audit

Source: user-provided screenshot of the Central Skills page on 2026-06-24.

## Confirmed From Screenshot

#### Concentric Border Radius
| Before | After |
| --- | --- |
| Header action controls mix pill-like buttons, rounded selects, and a compact ellipsis control with similar radii but different padding, so the row reads as a set of unrelated widgets. | Normalize header action radii by role: primary actions, secondary actions, select trigger, and icon-only menu should use compatible radii and padding so the controls feel like one toolbar. |
| The left filter panel contains a rounded "Collapse all groups" control inside a rounded panel with tight padding, but both surfaces visually compete. | Recalculate nested radii for the sidebar hero control and its icon slot so the outer surface radius is larger than the inner slot by the visible padding. |
| Skill cards use rounded card shells with small action buttons and badges that are close to the card edge; the relationship between card radius and inner controls is visually loose. | Keep card shell radius, but tune inner badge/action radii and spacing so nested surfaces follow the card padding instead of reading as scattered pills. |

#### Shadows Over Borders
| Before | After |
| --- | --- |
| Cards rely on a visible ring and many low-contrast separator lines; on the dark surface, repeated borders make the grid look busy and flat. | Use a tokenized dark-surface shadow/ring recipe for cards and hover states, reducing hard ring dominance while preserving separation. |
| Sidebar, top filter row, search toolbar, and list area are separated primarily by hard horizontal/vertical borders. | Keep structural dividers where needed, but soften non-essential container borders with background contrast and subtle shadow/ring layers. |
| Dropdown/popover surfaces such as the top-filter "More" menu use basic border + shadow styling. | Align popover depth with the rest of the dark UI using the same shadow-border hover/lift language. |

#### Optical Alignment
| Before | After |
| --- | --- |
| Text-plus-icon buttons in the header use uniform gap/padding, and asymmetric icons make some labels feel slightly off-center. | Apply consistent button icon-side padding and icon sizing so GitHub import, Update Center, Check all repositories, and menu buttons align optically. |
| The sidebar collapsed/expanded toggle and group-collapse icon slot have different visual weights from adjacent labels and counts. | Center icon affordances optically and give icon-only controls a stable square hit target. |
| Card action icons sit close together in the upper-right, making scan targets feel small despite enough visual whitespace around the card. | Give card action buttons consistent square targets and align them to the title cap-height instead of just the flex row center. |

#### Typography
| Before | After |
| --- | --- |
| Skill descriptions are clamped, but long English and Chinese descriptions often end abruptly and create uneven text rhythm across cards. | Add `text-pretty` to card summary/body text where supported and keep clamps stable. |
| Page title, card titles, sidebar labels, counts, and toolbar labels share a monospaced visual feel, reducing hierarchy on a dense operational screen. | Preserve the chosen font system, but sharpen hierarchy with clearer title weights, body opacity, and balanced wrapping for short headings. |
| Dynamic counts already use tabular numbers in some places, but sidebar badges, result count, update count, and card usage counts should be audited as one set. | Ensure all changing counters in the Central Skills screen use tabular numerals consistently. |

#### Minimum Hit Area
| Before | After |
| --- | --- |
| Card selection checkboxes, card icon buttons, filter chip remove buttons, and small sidebar row actions visually appear below the 40px hit-area target. | Expand small controls to at least 40x40 hit area where space allows, or use pseudo-elements/negative margins without overlapping adjacent controls. |
| Top filter pills and repository rows are dense; hover/click regions are not equally clear across source pills, tag pills, and repo rows. | Normalize interactive row and pill hit areas while keeping the compact information density. |

#### Scale On Press
| Before | After |
| --- | --- |
| Header buttons, card action buttons, filter pills, sidebar items, and chip remove buttons mostly change color only. | Add interruptible `active:scale-[0.96]` only to button-like controls where it improves tactile feedback and does not destabilize dense rows. |

#### Transition Specificity
| Before | After |
| --- | --- |
| Existing Central components use many `transition-colors` and some scoped transitions, but the screen should be audited for accidental `transition-all` or overly broad motion before adding polish. | Use explicit transition properties for scale, color, opacity, filter, box-shadow, and width; avoid `transition-all`. |

#### Contextual Icon Animation
| Before | After |
| --- | --- |
| Hover-only actions in compact cards appear/disappear through opacity only, and menu/filter state icons mostly switch statically. | Where icons change state, use CSS cross-fades with opacity/scale/filter values from the interface polish skill; do not add a motion dependency. |

## Confirmed From Code Inspection

- `src/components/central/CentralSkillsShell.tsx` owns the header toolbar, search toolbar, top-level body layout, and CentralSidebar/List composition.
- `src/components/central/CentralSidebar.tsx` owns pinned/overlay sidebar behavior, the collapse-all control, repository search, smart views, and collapsed rail.
- `src/components/central/CentralTopFilters.tsx` owns the source pills, tag pills, and top-filter "More" menu.
- `src/components/central/CentralSearchBar.tsx` owns search input, command palette button, filter chips, invalid-token hint, and chip remove controls.
- `src/components/central/CentralSkillListContent.tsx` owns list/grid padding and virtualized grid/list selection of `UnifiedSkillCard`.
- `src/components/skill/UnifiedSkillCard.tsx` is the only skill card implementation and must remain the card surface target.
- `src/components/ui/button-variants.ts` defines the shared Button primitive classes, currently including `active:not-aria-[haspopup]:translate-y-px`.
- `package.json` does not include `motion` or `framer-motion`; any icon state polish should use CSS-only cross-fades.
- Frontend spec requires Central card grid sizing to keep using `src/lib/centralSkillGrid.ts`.

## Out Of Scope For This Task

- Reworking Central Skills information architecture, filters, or repository sync behavior.
- Changing persisted view-state semantics, URL query shape, store behavior, or Tauri IPC.
- Replacing `UnifiedSkillCard` or adding a second card component.
- Introducing a new animation library.
- Redesigning the global app theme or changing the default font settings.
