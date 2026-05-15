import { describe, expect, it } from "vitest";
import {
  findDuplicatePlatformSkillGroups,
  isPluginReadOnlySkillRow,
  isWritablePlatformSkillRow,
} from "@/lib/platformDuplicateSkills";
import type { ScannedSkill } from "@/types";

function skill(overrides: Partial<ScannedSkill>): ScannedSkill {
  return {
    id: "demo",
    name: "Demo",
    description: "Demo skill",
    file_path: "/tmp/demo/SKILL.md",
    dir_path: "/tmp/demo",
    link_type: "copy",
    is_central: false,
    ...overrides,
  };
}

describe("platformDuplicateSkills", () => {
  it("flags only groups that contain a writable platform row and a plugin/read-only row", () => {
    const groups = findDuplicatePlatformSkillGroups([
      skill({
        id: "shared",
        name: "Shared",
        dir_path: "/tmp/.agents/skills/shared",
      }),
      skill({
        id: "shared",
        name: "Shared",
        dir_path: "/tmp/.codex/plugins/cache/example/skills/shared",
        source_kind: "plugin",
        is_read_only: true,
      }),
      skill({
        id: "plugin-only",
        name: "Plugin only",
        dir_path: "/tmp/.codex/plugins/cache/example/skills/plugin-only",
        source_kind: "plugin",
        is_read_only: true,
      }),
      skill({
        id: "ordinary",
        name: "Ordinary",
        dir_path: "/tmp/.agents/skills/ordinary",
      }),
    ]);

    expect(groups).toHaveLength(1);
    expect(groups[0].skillId).toBe("shared");
    expect(groups[0].writableRows).toHaveLength(1);
    expect(groups[0].pluginRows).toHaveLength(1);
  });

  it("treats Claude user rows as writable and plugin rows as read-only", () => {
    const userRow = skill({
      source_kind: "user",
      is_read_only: false,
    });
    const pluginRow = skill({
      source_kind: "plugin",
      is_read_only: true,
    });

    expect(isWritablePlatformSkillRow(userRow)).toBe(true);
    expect(isPluginReadOnlySkillRow(userRow)).toBe(false);
    expect(isWritablePlatformSkillRow(pluginRow)).toBe(false);
    expect(isPluginReadOnlySkillRow(pluginRow)).toBe(true);
  });
});
