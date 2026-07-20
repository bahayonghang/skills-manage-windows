import { describe, expect, it } from "vitest";

import {
  getVisibleSkillTags,
  getVisibleSkillTagsWithUsage,
  isSpecialTagFilterId,
  isSystemTagId,
  sanitizeSelectedTagIds,
} from "@/lib/centralTags";
import type { SkillTag } from "@/types";

function tag(id: string, isBuiltin = false): SkillTag {
  return {
    id,
    name: id,
    is_builtin: isBuiltin,
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

  it("hides unused built-ins while keeping used built-ins and custom tags", () => {
    const custom = tag("custom");
    const unusedBuiltin = tag("frontend-development", true);
    const usedBuiltin = tag("backend-development", true);

    expect(
      getVisibleSkillTagsWithUsage(
        [custom, unusedBuiltin, usedBuiltin, tag("uncategorized", true)],
        {
          custom: 0,
          "frontend-development": 0,
          "backend-development": 2,
          uncategorized: 4,
        },
      ),
    ).toEqual([custom, usedBuiltin]);
  });
});
