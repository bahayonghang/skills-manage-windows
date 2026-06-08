export const CENTRAL_SKILL_CARD_MIN_WIDTH = 220;
export const CENTRAL_SKILL_CARD_MAX_COLUMNS = 4;
export const CENTRAL_SKILL_CARD_GRID_GAP = 16;

export function centralSkillCardGridTemplateColumns(): string {
  const maxColumnGap =
    CENTRAL_SKILL_CARD_GRID_GAP * (CENTRAL_SKILL_CARD_MAX_COLUMNS - 1);
  const widthForMaxColumns = `calc((100% - ${maxColumnGap}px) / ${CENTRAL_SKILL_CARD_MAX_COLUMNS})`;
  const minColumnWidth = `max(${CENTRAL_SKILL_CARD_MIN_WIDTH}px, ${widthForMaxColumns})`;
  return `repeat(auto-fill, minmax(min(100%, ${minColumnWidth}), 1fr))`;
}
