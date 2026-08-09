import { describe, expect, it } from "vitest";

import {
  CENTRAL_SKILL_CARD_GRID_GAP,
  CENTRAL_SKILL_CARD_MAX_COLUMNS,
  CENTRAL_SKILL_CARD_MIN_WIDTH,
  centralVirtualItemHeight,
  centralSkillCardGridTemplateColumns,
} from "@/lib/centralSkillGrid";

describe("Central skill card grid sizing", () => {
  it("allows four columns only after the content region is wide enough", () => {
    expect(CENTRAL_SKILL_CARD_MIN_WIDTH).toBe(220);
    expect(CENTRAL_SKILL_CARD_GRID_GAP).toBe(16);
    expect(CENTRAL_SKILL_CARD_MAX_COLUMNS).toBe(4);
    expect(centralSkillCardGridTemplateColumns()).toBe(
      "repeat(auto-fill, minmax(min(100%, max(220px, calc((100% - 48px) / 4))), 1fr))",
    );
  });

  it("keeps fixed virtual rows tall enough at every supported font scale", () => {
    // 契约值 = 基准高度 × fontScale（全站 rem 随 --font-scale 线性缩放），
    // 基准值见 centralSkillGrid.ts 中 CENTRAL_GRID_CARD_HEIGHT / CENTRAL_LIST_CARD_HEIGHT。
    expect(centralVirtualItemHeight("list", "comfortable", 0.875)).toBe(172);
    expect(centralVirtualItemHeight("list", "comfortable", 1)).toBe(196);
    expect(centralVirtualItemHeight("list", "comfortable", 1.125)).toBe(221);
    expect(centralVirtualItemHeight("list", "compact", 0.875)).toBe(147);
    expect(centralVirtualItemHeight("list", "compact", 1)).toBe(168);
    expect(centralVirtualItemHeight("list", "compact", 1.125)).toBe(189);
    expect(centralVirtualItemHeight("grid", "comfortable", 0.875)).toBe(210);
    expect(centralVirtualItemHeight("grid", "comfortable", 1)).toBe(240);
    expect(centralVirtualItemHeight("grid", "comfortable", 1.125)).toBe(270);
    expect(centralVirtualItemHeight("grid", "compact", 0.875)).toBe(189);
    expect(centralVirtualItemHeight("grid", "compact", 1)).toBe(216);
    expect(centralVirtualItemHeight("grid", "compact", 1.125)).toBe(243);
  });
});
