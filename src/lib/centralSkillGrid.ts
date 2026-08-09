export const CENTRAL_SKILL_CARD_MIN_WIDTH = 220;
export const CENTRAL_SKILL_CARD_MAX_COLUMNS = 4;
export const CENTRAL_SKILL_CARD_GRID_GAP = 16;

type CentralVirtualView = "grid" | "list";
type CentralVirtualDensity = "comfortable" | "compact";

/**
 * 卡片高度契约（fontScale = 1 时的像素值）。
 *
 * 虚拟化网格/列表按固定行高做绝对定位，该值必须是卡片真实内容的**上限**，
 * 而不是估算：卡片内部各行已收敛为定值（标题行 min-h-7、描述 line-clamp、
 * meta 单行 nowrap、标签行 h-6、footer size-8 + border-t），且全部尺寸走 rem，
 * 随根字号 `--font-scale` 线性缩放，因此运行时行高 = 基准值 × fontScale。
 *
 * 组成（grid / compact ≈ 215px）：padding 28 + 标题 28 + 描述 2 行 ≈ 40
 * + meta 22 + 标签行 24 + footer 41 + 4×gap 8；comfortable 描述 3 行 ≈ 62。
 * list 视图卡片无 footer/平台图标行，自然更矮。
 */
export const CENTRAL_GRID_CARD_HEIGHT: Record<CentralVirtualDensity, number> = {
  comfortable: 240,
  compact: 216,
};

export const CENTRAL_LIST_CARD_HEIGHT: Record<CentralVirtualDensity, number> = {
  comfortable: 196,
  compact: 168,
};

export function centralVirtualItemHeight(
  view: CentralVirtualView,
  density: CentralVirtualDensity,
  fontScale: number,
): number {
  const base =
    view === "list"
      ? CENTRAL_LIST_CARD_HEIGHT[density]
      : CENTRAL_GRID_CARD_HEIGHT[density];
  return Math.ceil(base * fontScale);
}

export function centralSkillCardGridTemplateColumns(): string {
  const maxColumnGap =
    CENTRAL_SKILL_CARD_GRID_GAP * (CENTRAL_SKILL_CARD_MAX_COLUMNS - 1);
  const widthForMaxColumns = `calc((100% - ${maxColumnGap}px) / ${CENTRAL_SKILL_CARD_MAX_COLUMNS})`;
  const minColumnWidth = `max(${CENTRAL_SKILL_CARD_MIN_WIDTH}px, ${widthForMaxColumns})`;
  return `repeat(auto-fill, minmax(min(100%, ${minColumnWidth}), 1fr))`;
}
