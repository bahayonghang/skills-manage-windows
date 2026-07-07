import { describe, it, expect, vi, beforeEach } from "vitest";
import type { SavedView } from "../types";

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
import { useSavedViewsStore } from "../stores/savedViewsStore";

function makeView(id: string, overrides: Partial<SavedView> = {}): SavedView {
  return {
    id,
    name: `View ${id}`,
    query: "q=demo",
    sort_order: 0,
    icon: null,
    pinned: false,
    created_at: "2026-05-01T00:00:00Z",
    updated_at: "2026-05-01T00:00:00Z",
    ...overrides,
  };
}

describe("savedViewsStore", () => {
  beforeEach(() => {
    useSavedViewsStore.setState({ views: [], isLoading: false, error: null });
    vi.mocked(invoke).mockReset();
  });

  it("loadSavedViews populates list from backend", async () => {
    const fixture = [makeView("a", { sort_order: 0 }), makeView("b", { sort_order: 1 })];
    vi.mocked(invoke).mockResolvedValueOnce(fixture);

    await useSavedViewsStore.getState().loadSavedViews();

    expect(invoke).toHaveBeenCalledWith("list_saved_views");
    expect(useSavedViewsStore.getState().views).toEqual(fixture);
    expect(useSavedViewsStore.getState().isLoading).toBe(false);
    expect(useSavedViewsStore.getState().error).toBeNull();
  });

  it("loadSavedViews sets error on failure", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("db down"));

    await useSavedViewsStore.getState().loadSavedViews();

    expect(useSavedViewsStore.getState().error).toContain("db down");
    expect(useSavedViewsStore.getState().isLoading).toBe(false);
  });

  it("createSavedView passes input wrapper and reloads", async () => {
    const created = makeView("a");
    vi.mocked(invoke)
      .mockResolvedValueOnce(created) // create_saved_view
      .mockResolvedValueOnce([created]); // list_saved_views

    const result = await useSavedViewsStore.getState().createSavedView({
      name: "A",
      query: "q=demo",
      icon: "star",
      pinned: true,
    });

    expect(result).toEqual(created);
    expect(invoke).toHaveBeenNthCalledWith(1, "create_saved_view", {
      input: { name: "A", query: "q=demo", icon: "star", pinned: true },
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "list_saved_views");
    expect(useSavedViewsStore.getState().views).toEqual([created]);
  });

  it("createSavedView defaults icon to null and pinned to false", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(makeView("a")).mockResolvedValueOnce([]);

    await useSavedViewsStore.getState().createSavedView({ name: "A", query: "q=demo" });

    expect(invoke).toHaveBeenNthCalledWith(1, "create_saved_view", {
      input: { name: "A", query: "q=demo", icon: null, pinned: false },
    });
  });

  it("updateSavedView forwards id and input then reloads", async () => {
    const updated = makeView("a", { name: "Renamed" });
    vi.mocked(invoke).mockResolvedValueOnce(updated).mockResolvedValueOnce([updated]);

    const result = await useSavedViewsStore
      .getState()
      .updateSavedView("a", { name: "Renamed" });

    expect(result).toEqual(updated);
    expect(invoke).toHaveBeenNthCalledWith(1, "update_saved_view", {
      id: "a",
      input: { name: "Renamed" },
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "list_saved_views");
  });

  it("updateSavedView can clear icon by passing null", async () => {
    const updated = makeView("a", { icon: null });
    vi.mocked(invoke).mockResolvedValueOnce(updated).mockResolvedValueOnce([updated]);

    await useSavedViewsStore.getState().updateSavedView("a", { icon: null });

    expect(invoke).toHaveBeenNthCalledWith(1, "update_saved_view", {
      id: "a",
      input: { icon: null },
    });
  });

  it("deleteSavedView forwards id and reloads", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined).mockResolvedValueOnce([]);

    await useSavedViewsStore.getState().deleteSavedView("a");

    expect(invoke).toHaveBeenNthCalledWith(1, "delete_saved_view", { id: "a" });
    expect(invoke).toHaveBeenNthCalledWith(2, "list_saved_views");
    expect(useSavedViewsStore.getState().views).toEqual([]);
  });

  it("reorderSavedViews forwards ids array and reloads", async () => {
    const reordered = [makeView("b", { sort_order: 0 }), makeView("a", { sort_order: 1 })];
    vi.mocked(invoke).mockResolvedValueOnce(undefined).mockResolvedValueOnce(reordered);

    await useSavedViewsStore.getState().reorderSavedViews(["b", "a"]);

    expect(invoke).toHaveBeenNthCalledWith(1, "reorder_saved_views", { ids: ["b", "a"] });
    expect(invoke).toHaveBeenNthCalledWith(2, "list_saved_views");
    expect(useSavedViewsStore.getState().views).toEqual(reordered);
  });

  it("error on update propagates and stores message", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("not found"));

    await expect(
      useSavedViewsStore.getState().updateSavedView("missing", { name: "X" }),
    ).rejects.toThrow();

    expect(useSavedViewsStore.getState().error).toContain("not found");
  });
});
