# Shortcuts

Cross-platform shortcuts wired by `react-router` plus the global command palette. `Ctrl` on Windows / Linux maps to `Cmd` on macOS.

## Global

| Shortcut | Action |
| --- | --- |
| `Ctrl+K` / `Cmd+K` | Open command palette |
| `Ctrl+/` | Focus global search |
| `Ctrl+,` | Open Settings |
| `Esc` | Dismiss dialog / drawer / palette |

## Navigation

| Shortcut | Action |
| --- | --- |
| `g` then `c` | Go to Central Skills |
| `g` then `p` | Go to Platforms (last visited) |
| `g` then `m` | Go to Marketplace |
| `g` then `d` | Go to Discover |
| `g` then `s` | Go to Settings |
| `Alt+←` | Browser back |
| `Alt+→` | Browser forward |

## Skill Detail

| Shortcut | Action |
| --- | --- |
| `e` | Edit skill metadata (when permitted) |
| `i` | Open install dialog |
| `c` | Add to collection |
| `r` | Refresh skill from canonical source |
| `o` | Open in OS file manager |

## Lists

| Shortcut | Action |
| --- | --- |
| `j` / `k` | Move focus down / up in card lists |
| `Enter` | Open focused card |
| `Space` | Toggle selection (where multi-select is allowed) |

## Theme

| Shortcut | Action |
| --- | --- |
| `Ctrl+Shift+L` | Cycle Catppuccin variant |
| `Ctrl+Shift+A` | Cycle accent color |

## Notes

- `g` chord has a 600 ms timeout; press the second key within that window or it resets.
- Keyboard handling is paused while a text input is focused, so typing `g` in a search box does not trigger navigation.

Last reviewed: 2026-05-04
