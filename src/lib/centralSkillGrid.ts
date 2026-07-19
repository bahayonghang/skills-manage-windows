export const CENTRAL_SKILL_CARD_MIN_WIDTH = 220;
export const CENTRAL_SKILL_CARD_MAX_COLUMNS = 4;
export const CENTRAL_SKILL_CARD_GRID_GAP = 16;

type CentralVirtualView = "grid" | "list";
type CentralVirtualDensity = "comfortable" | "compact";

export function centralVirtualItemHeight(
  view: CentralVirtualView,
  density: CentralVirtualDensity,
  fontScale: number,
): number {
  if (view === "list") {
    if (density === "comfortable") return 196;
    return fontScale > 1
      ? Math.ceil(168 + (fontScale - 1) * 128)
      : 168;
  }

  if (density === "comfortable") {
    return fontScale > 1
      ? Math.ceil(192 + (fontScale - 1) * 320)
      : 192;
  }

  if (fontScale <= 0.875) return 172;
  if (fontScale <= 1) {
    return Math.ceil(172 + (fontScale - 0.875) * 96);
  }
  return Math.ceil(184 + (fontScale - 1) * 192);
}

export function centralSkillCardGridTemplateColumns(): string {
  const maxColumnGap =
    CENTRAL_SKILL_CARD_GRID_GAP * (CENTRAL_SKILL_CARD_MAX_COLUMNS - 1);
  const widthForMaxColumns = `calc((100% - ${maxColumnGap}px) / ${CENTRAL_SKILL_CARD_MAX_COLUMNS})`;
  const minColumnWidth = `max(${CENTRAL_SKILL_CARD_MIN_WIDTH}px, ${widthForMaxColumns})`;
  return `repeat(auto-fill, minmax(min(100%, ${minColumnWidth}), 1fr))`;
}
