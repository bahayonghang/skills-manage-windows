# Dense typography visual and virtualization evidence

Date: 2026-07-18

Browser harness: Vite `http://127.0.0.1:24202` + `agent-browser`

Desktop harness: `pnpm tauri dev` + Windows WebView2 inspection

## Computed font sizes

All values came from rendered `.text-ui-meta` / `.text-ui-micro` nodes after changing the real Settings Density control.

| Scale | Root | UI meta | UI micro | Body profile |
| ---: | ---: | ---: | ---: | --- |
| 0.875 | 14px | 9.625px | 8.75px | JetBrains Mono |
| 1 | 16px | 11px | 10px | JetBrains Mono |
| 1.125 | 18px | 12.375px | 11.25px | JetBrains Mono and system |

System profile resolved to `ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif`. No viewport-driven font size or inline `fontSize` remains.

## Central virtualization

Browser fixture count is 72, so both thresholds are crossed (>60 list and >40 grid). Data mixes Chinese descriptions, deliberately long English names/descriptions, and long Windows paths.

Before correction:

- compact list allocated 148px while every visible card was at least 168px: 19/19 to 20/20 visible cards crossed the next row at all three scales.
- comfortable grid at 1.125 allocated 192px while cards reached 228.813px: 36/36 visible cards crossed row bounds.
- compact grid allocated 172px; cards reached 184px at Scale 1 and 206.875px at 1.125.
- `<60` non-virtual Chinese (47 cards) and long-English (60 cards) compact-list groups reached 180.875px at system/1.125, revealing clipping that a fixed parent's `scrollHeight` alone would miss.

After `centralVirtualItemHeight()` correction:

| Scale | Compact list | Compact grid | Comfortable list | Comfortable grid | Result |
| ---: | ---: | ---: | ---: | ---: | --- |
| 0.875 | 168px | 172px | 196px | 192px | 0 overlap / 0 scroll overflow |
| 1 | 168px | 184px | 196px | 192px | 0 overlap / 0 scroll overflow |
| 1.125 | 184px | 208px | 196px | 232px | 0 overlap / 0 scroll overflow |

At system/1.125, comfortable grid also passed at 900x600 (2 columns), 1200x800 (4 columns), and 1440x900 (4 columns), with no root document overflow. List scroll reached `scrollTop=max`; first item at top and final `中文高密度排版验证技能-69` at bottom were both rendered and reachable. Thresholds and overscan were unchanged.

## Surface checks

- Central: mixed Chinese/long English at 900x600 list compact and 1440x900 grid; 1200x800 was measured live.
- Settings Connections: 900x600 system/1.125 long Windows path wrapped with `scrollWidth ~= clientWidth`; page switch now leaves local main `scrollTop=0` and the heading is not hidden.
- Marketplace: 1200x800 system/1.125 rendered categories, search, and repeated cards without overlap.
- Projects: 900x600 system/1.125 rendered the browser empty state without overlap.
- Usage: 1440x900 system/1.125 rendered table, recent calls, and 16-week heatmap without overlap.
- GitHub import preview: browser harness correctly kept desktop-only preview disabled. Real Tauri preview of `https://github.com/openai/skills` completed read-only with 43 discovered/selected skills; list, long descriptions, repo path, SKILL.md pane, tabs, and footer controls did not overlap. Review/Confirm was not invoked.

## Theme and accent evidence

`themeContrast.test.ts` covers all 6 themes x 14 accents across background/card/popover/sidebar, plus semantic state surfaces. Six default-Scale screenshots show the Appearance page with all 14 accent swatches. Claude Dark mauve/red/maroon use readable text-only overrides; fill and ring colors remain the original accent.

## Screenshot index

All images are under `visual-evidence/`:

- `central-900x600-system-spacious-list-compact.png`
- `central-1440x900-system-spacious-grid.png`
- `settings-connections-900x600-system-long-windows-path.png`
- `marketplace-1200x800-system-spacious.png`
- `projects-900x600-system-spacious.png`
- `usage-1440x900-system-spacious.png`
- `tauri-github-preview-openai-skills.png`
- `theme-{mocha,macchiato,frappe,latte,claude-light,claude-dark}-14-accents-1440x900-default.png`

No unrun viewport/theme/accent is described here as visually passed: the full 6x14 claim is contrast-matrix evidence; screenshot evidence is one Appearance capture per theme with all 14 swatches visible.

## Verification results

- Typography/font/contrast/display focused Vitest: 4 files, 51 passed.
- Unified card/Central GitHub preview/GitHub wizard/skill detail/Usage focused Vitest: 5 files, 116 passed.
- `pnpm build`: passed; 2711 modules transformed.
- `git diff --check`: passed (CRLF conversion notices only, no whitespace error).
- Final `just ci`: passed; web 125 files / 1385 passed + 1 skipped, Rust library 851 passed + 4 ignored, CLI envelope 3 passed, project E2E 5 passed; typecheck, lint, sizecheck, entrypoint check, and clippy passed.
