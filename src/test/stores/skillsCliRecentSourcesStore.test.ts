import { beforeEach, describe, expect, it } from "vitest";

import { ipcFixtureError } from "@/lib/ipc/errors";
import {
  SKILLS_CLI_RECENT_SOURCES_PARSE_ERROR,
  SKILLS_CLI_RECENT_SOURCES_SETTING_KEY,
} from "@/pages/skillsCliInstallViewModel";
import { useSkillsCliRecentSourcesStore } from "@/stores/skillsCliRecentSourcesStore";
import { ipcInvokeCalls, mockIpcCommand } from "@/test/support/ipcMock";

function resetStore() {
  useSkillsCliRecentSourcesStore.getState().reset();
}

describe("skillsCliRecentSourcesStore", () => {
  beforeEach(resetStore);

  it("round-trips latest-first sources through the exact settings key", async () => {
    mockIpcCommand(
      "get_setting",
      JSON.stringify(["owner/repo", "https://github.com/a/b"]),
    );
    await useSkillsCliRecentSourcesStore.getState().load();
    expect(ipcInvokeCalls("get_setting")).toEqual([
      {
        command: "get_setting",
        args: { key: SKILLS_CLI_RECENT_SOURCES_SETTING_KEY },
      },
    ]);
    expect(useSkillsCliRecentSourcesStore.getState().sources).toEqual([
      "owner/repo",
      "https://github.com/a/b",
    ]);
    expect(useSkillsCliRecentSourcesStore.getState().error).toBeNull();

    mockIpcCommand("set_setting", undefined);
    await useSkillsCliRecentSourcesStore.getState().push("https://github.com/c/d");
    expect(ipcInvokeCalls("set_setting")).toEqual([
      {
        command: "set_setting",
        args: {
          key: SKILLS_CLI_RECENT_SOURCES_SETTING_KEY,
          value: JSON.stringify([
            "https://github.com/c/d",
            "owner/repo",
            "https://github.com/a/b",
          ]),
        },
      },
    ]);
    expect(useSkillsCliRecentSourcesStore.getState().sources[0]).toBe(
      "https://github.com/c/d",
    );
  });

  it("dedupes by moving the existing source to the front and truncates to 8", async () => {
    useSkillsCliRecentSourcesStore.setState({
      sources: ["a", "b", "c", "d", "e", "f", "g", "h"],
      loaded: true,
    });
    mockIpcCommand("set_setting", undefined);
    await useSkillsCliRecentSourcesStore.getState().push("c");
    expect(useSkillsCliRecentSourcesStore.getState().sources).toEqual([
      "c",
      "a",
      "b",
      "d",
      "e",
      "f",
      "g",
      "h",
    ]);

    await useSkillsCliRecentSourcesStore.getState().push("z");
    expect(useSkillsCliRecentSourcesStore.getState().sources).toEqual([
      "z",
      "c",
      "a",
      "b",
      "d",
      "e",
      "f",
      "g",
    ]);
    expect(useSkillsCliRecentSourcesStore.getState().sources).toHaveLength(8);
  });

  it("fail-closes invalid persisted JSON without updating sources for preview", async () => {
    mockIpcCommand("get_setting", '{"source":"owner/repo"}');
    await useSkillsCliRecentSourcesStore.getState().load();
    expect(useSkillsCliRecentSourcesStore.getState().sources).toEqual([]);
    expect(useSkillsCliRecentSourcesStore.getState().error).toBe(
      SKILLS_CLI_RECENT_SOURCES_PARSE_ERROR,
    );
    expect(useSkillsCliRecentSourcesStore.getState().loaded).toBe(true);
  });

  it("treats a missing setting as empty without error", async () => {
    mockIpcCommand("get_setting", null);
    await useSkillsCliRecentSourcesStore.getState().load();
    expect(useSkillsCliRecentSourcesStore.getState().sources).toEqual([]);
    expect(useSkillsCliRecentSourcesStore.getState().error).toBeNull();
  });

  it("does not block later manual use when load fails", async () => {
    mockIpcCommand("get_setting", () => {
      throw ipcFixtureError(
        "internal.unexpected",
        "settings unavailable",
      );
    });
    await useSkillsCliRecentSourcesStore.getState().load();
    expect(useSkillsCliRecentSourcesStore.getState().sources).toEqual([]);
    expect(useSkillsCliRecentSourcesStore.getState().loaded).toBe(true);
    expect(useSkillsCliRecentSourcesStore.getState().error).not.toBeNull();

    mockIpcCommand("set_setting", undefined);
    await useSkillsCliRecentSourcesStore.getState().push("owner/repo");
    expect(useSkillsCliRecentSourcesStore.getState().sources).toEqual([
      "owner/repo",
    ]);
    expect(useSkillsCliRecentSourcesStore.getState().error).toBeNull();
  });

  it("keeps previous sources when push persistence fails", async () => {
    useSkillsCliRecentSourcesStore.setState({
      sources: ["owner/repo"],
      loaded: true,
    });
    mockIpcCommand("set_setting", () => {
      throw new Error("setting_value_invalid: The setting value is invalid.");
    });
    await expect(
      useSkillsCliRecentSourcesStore.getState().push("https://example.com/x"),
    ).rejects.toThrow();
    expect(useSkillsCliRecentSourcesStore.getState().sources).toEqual([
      "owner/repo",
    ]);
    expect(ipcInvokeCalls("set_setting")).toHaveLength(1);
  });
});
