import { describe, expect, it } from "vitest";

import {
  CENTRAL_SKILL_CARD_GRID_GAP,
  CENTRAL_SKILL_CARD_MAX_COLUMNS,
  CENTRAL_SKILL_CARD_MIN_WIDTH,
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
});
