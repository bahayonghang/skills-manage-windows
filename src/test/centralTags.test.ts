import { describe, expect, it } from "vitest";

import {
  getVisibleSkillTags,
  isSpecialTagFilterId,
  isSystemTagId,
  sanitizeSelectedTagIds,
} from "@/lib/centralTags";
import type { SkillTag } from "@/types";

function tag(id: string): SkillTag {
  return {
    id,
    name: id,
    is_builtin: false,
    created_at: "",
    updated_at: "",
  };
}

describe("centralTags", () => {
  it("hides uncategorized from ordinary visible tag lists", () => {
    expect(
      getVisibleSkillTags([
        tag("academic-research-writing"),
        tag("uncategorized"),
      ]),
    ).toEqual([tag("academic-research-writing")]);
    expect(isSystemTagId("uncategorized")).toBe(true);
  });

  it("keeps smart-view tag filters while dropping stale normal tag ids", () => {
    expect(
      sanitizeSelectedTagIds(
        ["uncategorized", "updates", "ai-review", "deleted-default", "custom"],
        [tag("custom")],
      ),
    ).toEqual(["uncategorized", "updates", "ai-review", "custom"]);
    expect(isSpecialTagFilterId("ai-review")).toBe(true);
  });
});
