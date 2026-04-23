# Claude Platform Multi-Source Scan

Mission-specific implementation notes for Claude source handling.

## Desired Behavior
- `claude-code` stays one platform
- Rows from `~/.claude/skills` and `~/.claude/plugins/marketplaces/*` appear together under that platform
- Marketplace rows are read-only/display-only
- If the same logical skill exists in both roots, show both rows and mark the conflict

## Design Constraints
- Do not reuse canonical install semantics for marketplace observation rows
- Do not mutate or centralize marketplace rows
- Do not deduplicate duplicate-source rows away from the user surface
- Preserve row-specific identity from list to detail and through rescans

## Hotspots
- root discovery for Claude
- stable row identity when two rows share the same logical skill id/name
- query payload shape for list/detail surfaces
- suppression of list/detail management affordances for marketplace rows
- rescan cleanup without source swaps or ghost rows
