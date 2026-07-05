import { describe, it, expect, vi, beforeEach } from "vitest";
import type { TagGroup } from "../types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@/lib/ipc", async () => {
  const actual = await vi.importActual<typeof import("@/lib/ipc")>("@/lib/ipc");
  return {
    ...actual,
    invoke: (await import("@tauri-apps/api/core")).invoke,
    isTauriRuntime: () => true,
  };
});

import { invoke } from "@tauri-apps/api/core";
import { useTagGroupsStore } from "../stores/tagGroupsStore";

function makeGroup(id: string, overrides: Partial<TagGroup> = {}): TagGroup {
  return {
    id,
    name: `Group ${id}`,
    color: null,
    sort_order: 0,
    is_builtin: false,
    created_at: "2026-05-01T00:00:00Z",
    updated_at: "2026-05-01T00:00:00Z",
    ...overrides,
  };
}

describe("tagGroupsStore", () => {
  beforeEach(() => {
    useTagGroupsStore.setState({ groups: [], isLoading: false, error: null });
    vi.mocked(invoke).mockReset();
  });

  it("loadTagGroups populates list from backend", async () => {
    const fixture = [makeGroup("a"), makeGroup("b", { sort_order: 1 })];
    vi.mocked(invoke).mockResolvedValueOnce(fixture);

    await useTagGroupsStore.getState().loadTagGroups();

    expect(invoke).toHaveBeenCalledWith("list_tag_groups");
    expect(useTagGroupsStore.getState().groups).toEqual(fixture);
  });

  it("loadTagGroups sets error on failure", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("boom"));
    await useTagGroupsStore.getState().loadTagGroups();
    expect(useTagGroupsStore.getState().error).toContain("boom");
  });

  it("createTagGroup wraps input and reloads", async () => {
    const created = makeGroup("a", { color: "#ff0000" });
    vi.mocked(invoke).mockResolvedValueOnce(created).mockResolvedValueOnce([created]);

    const result = await useTagGroupsStore
      .getState()
      .createTagGroup({ name: "A", color: "#ff0000" });

    expect(result).toEqual(created);
    expect(invoke).toHaveBeenNthCalledWith(1, "create_tag_group", {
      input: { name: "A", color: "#ff0000" },
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "list_tag_groups");
  });

  it("createTagGroup defaults color to null", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(makeGroup("a")).mockResolvedValueOnce([]);
    await useTagGroupsStore.getState().createTagGroup({ name: "A" });
    expect(invoke).toHaveBeenNthCalledWith(1, "create_tag_group", {
      input: { name: "A", color: null },
    });
  });

  it("updateTagGroup forwards id and input then reloads", async () => {
    const updated = makeGroup("a", { name: "Renamed" });
    vi.mocked(invoke).mockResolvedValueOnce(updated).mockResolvedValueOnce([updated]);

    await useTagGroupsStore.getState().updateTagGroup("a", { name: "Renamed" });

    expect(invoke).toHaveBeenNthCalledWith(1, "update_tag_group", {
      id: "a",
      input: { name: "Renamed" },
    });
  });

  it("updateTagGroup can clear color via null", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(makeGroup("a")).mockResolvedValueOnce([]);
    await useTagGroupsStore.getState().updateTagGroup("a", { color: null });
    expect(invoke).toHaveBeenNthCalledWith(1, "update_tag_group", {
      id: "a",
      input: { color: null },
    });
  });

  it("deleteTagGroup forwards id and reloads", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined).mockResolvedValueOnce([]);
    await useTagGroupsStore.getState().deleteTagGroup("a");
    expect(invoke).toHaveBeenNthCalledWith(1, "delete_tag_group", { id: "a" });
    expect(useTagGroupsStore.getState().groups).toEqual([]);
  });

  it("reorderTagGroups forwards ids array and reloads", async () => {
    const reordered = [makeGroup("b", { sort_order: 0 }), makeGroup("a", { sort_order: 1 })];
    vi.mocked(invoke).mockResolvedValueOnce(undefined).mockResolvedValueOnce(reordered);

    await useTagGroupsStore.getState().reorderTagGroups(["b", "a"]);

    expect(invoke).toHaveBeenNthCalledWith(1, "reorder_tag_groups", { ids: ["b", "a"] });
    expect(useTagGroupsStore.getState().groups).toEqual(reordered);
  });

  it("setTagGroup forwards tagId and groupId", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useTagGroupsStore.getState().setTagGroup("tag-1", "group-1");

    expect(invoke).toHaveBeenCalledWith("set_tag_group", {
      tagId: "tag-1",
      groupId: "group-1",
    });
  });

  it("setTagGroup with null groupId clears assignment", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);
    await useTagGroupsStore.getState().setTagGroup("tag-1", null);
    expect(invoke).toHaveBeenCalledWith("set_tag_group", {
      tagId: "tag-1",
      groupId: null,
    });
  });

  it("error on update propagates and stores message", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("not found"));
    await expect(
      useTagGroupsStore.getState().updateTagGroup("missing", { name: "X" }),
    ).rejects.toThrow();
    expect(useTagGroupsStore.getState().error).toContain("not found");
  });
});
