import { describe, it, expect, vi, beforeEach } from "vitest";
import { Collection, CollectionDetail } from "../types";
import * as tauriBridge from "@/lib/tauri";

// Mock Tauri core before importing the store
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { useCollectionStore } from "../stores/collectionStore";

// ─── Fixtures ─────────────────────────────────────────────────────────────────

const mockCollections: Collection[] = [
  {
    id: "col-1",
    name: "Frontend",
    description: "Frontend skills",
    created_at: "2026-04-09T00:00:00Z",
    updated_at: "2026-04-09T00:00:00Z",
  },
  {
    id: "col-2",
    name: "Backend",
    description: undefined,
    created_at: "2026-04-09T01:00:00Z",
    updated_at: "2026-04-09T01:00:00Z",
  },
];

const mockCollectionDetail: CollectionDetail = {
  id: "col-1",
  name: "Frontend",
  description: "Frontend skills",
  created_at: "2026-04-09T00:00:00Z",
  updated_at: "2026-04-09T00:00:00Z",
  skills: [
    {
      id: "frontend-design",
      name: "frontend-design",
      description: "Build distinctive frontend UIs",
      file_path: "~/.skillsmanage/skills/frontend-design/SKILL.md",
      is_central: true,
      scanned_at: "2026-04-09T00:00:00Z",
    },
  ],
};

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("collectionStore", () => {
  beforeEach(() => {
    useCollectionStore.setState({
      collections: [],
      currentDetail: null,
      isLoading: false,
      isLoadingDetail: false,
      error: null,
    });
    vi.clearAllMocks();
  });

  // ── Initial State ──────────────────────────────────────────────────────────

  it("has correct initial state", () => {
    const state = useCollectionStore.getState();
    expect(state.collections).toEqual([]);
    expect(state.currentDetail).toBeNull();
    expect(state.isLoading).toBe(false);
    expect(state.isLoadingDetail).toBe(false);
    expect(state.error).toBeNull();
  });

  // ── loadCollections ────────────────────────────────────────────────────────

  it("loadCollections populates collections", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockCollections);

    await useCollectionStore.getState().loadCollections();

    const state = useCollectionStore.getState();
    expect(state.collections).toEqual(mockCollections);
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
    expect(invoke).toHaveBeenCalledWith("get_collections");
  });

  it("loadCollections sets error on failure", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("DB error"));

    await useCollectionStore.getState().loadCollections();

    const state = useCollectionStore.getState();
    expect(state.error).toContain("DB error");
    expect(state.isLoading).toBe(false);
  });

  it("returns deterministic browser fixture collections when Tauri runtime is unavailable", async () => {
    const isTauriSpy = vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(false);

    await useCollectionStore.getState().loadCollections();
    await useCollectionStore.getState().loadCollectionDetail("fixture-collection");

    expect(invoke).not.toHaveBeenCalled();
    expect(useCollectionStore.getState().collections).toEqual([
      expect.objectContaining({ id: "fixture-collection" }),
    ]);
    expect(useCollectionStore.getState().currentDetail).toEqual(
      expect.objectContaining({
        id: "fixture-collection",
        skills: [expect.objectContaining({ id: "fixture-central-skill" })],
      })
    );

    isTauriSpy.mockRestore();
  });

  // ── createCollection ───────────────────────────────────────────────────────

  it("createCollection adds new collection and reloads", async () => {
    const newCollection: Collection = {
      id: "col-3",
      name: "Test",
      description: "Test desc",
      created_at: "2026-04-10T00:00:00Z",
      updated_at: "2026-04-10T00:00:00Z",
    };

    vi.mocked(invoke)
      .mockResolvedValueOnce(newCollection) // create_collection
      .mockResolvedValueOnce([...mockCollections, newCollection]); // get_collections

    const result = await useCollectionStore.getState().createCollection("Test", "Test desc");

    expect(result).toEqual(newCollection);
    expect(invoke).toHaveBeenCalledWith("create_collection", { name: "Test", description: "Test desc" });
    expect(invoke).toHaveBeenCalledWith("get_collections");
    const state = useCollectionStore.getState();
    expect(state.collections).toHaveLength(3);
  });

  it("createCollection with no description passes undefined", async () => {
    const newCollection: Collection = {
      id: "col-3",
      name: "Test",
      created_at: "2026-04-10T00:00:00Z",
      updated_at: "2026-04-10T00:00:00Z",
    };

    vi.mocked(invoke)
      .mockResolvedValueOnce(newCollection)
      .mockResolvedValueOnce([newCollection]);

    await useCollectionStore.getState().createCollection("Test");

    expect(invoke).toHaveBeenCalledWith("create_collection", { name: "Test", description: undefined });
  });

  // ── updateCollection ───────────────────────────────────────────────────────

  it("updateCollection calls update and reloads", async () => {
    const updated: Collection = { ...mockCollections[0], name: "Updated", description: "new desc" };

    vi.mocked(invoke)
      .mockResolvedValueOnce(updated) // update_collection
      .mockResolvedValueOnce([updated, mockCollections[1]]); // get_collections

    const result = await useCollectionStore.getState().updateCollection("col-1", "Updated", "new desc");

    expect(result).toEqual(updated);
    expect(invoke).toHaveBeenCalledWith("update_collection", {
      collectionId: "col-1",
      name: "Updated",
      description: "new desc",
    });
    const state = useCollectionStore.getState();
    expect(state.collections[0].name).toBe("Updated");
  });

  // ── deleteCollection ───────────────────────────────────────────────────────

  it("deleteCollection removes collection from state", async () => {
    useCollectionStore.setState({ collections: mockCollections });

    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined) // delete_collection
      .mockResolvedValueOnce([mockCollections[1]]); // get_collections

    await useCollectionStore.getState().deleteCollection("col-1");

    expect(invoke).toHaveBeenCalledWith("delete_collection", { collectionId: "col-1" });
    const state = useCollectionStore.getState();
    expect(state.collections).toHaveLength(1);
    expect(state.collections[0].id).toBe("col-2");
  });

  // ── loadCollectionDetail ───────────────────────────────────────────────────

  it("loadCollectionDetail populates currentDetail", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockCollectionDetail);

    await useCollectionStore.getState().loadCollectionDetail("col-1");

    const state = useCollectionStore.getState();
    expect(state.currentDetail).toEqual(mockCollectionDetail);
    expect(state.isLoadingDetail).toBe(false);
    expect(invoke).toHaveBeenCalledWith("get_collection_detail", { collectionId: "col-1" });
  });

  it("loadCollectionDetail sets error on failure", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("Not found"));

    await useCollectionStore.getState().loadCollectionDetail("invalid-id");

    const state = useCollectionStore.getState();
    expect(state.error).toContain("Not found");
    expect(state.isLoadingDetail).toBe(false);
  });

  // ── addSkillToCollection ───────────────────────────────────────────────────

  it("addSkillToCollection calls add command and reloads detail", async () => {
    useCollectionStore.setState({ currentDetail: mockCollectionDetail });

    const updatedDetail: CollectionDetail = {
      ...mockCollectionDetail,
      skills: [
        ...mockCollectionDetail.skills,
        {
          id: "code-reviewer",
          name: "code-reviewer",
          description: "Review code",
          file_path: "~/.skillsmanage/skills/code-reviewer/SKILL.md",
          is_central: true,
          scanned_at: "2026-04-09T00:00:00Z",
        },
      ],
    };

    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined) // add_skill_to_collection
      .mockResolvedValueOnce(updatedDetail); // get_collection_detail

    await useCollectionStore.getState().addSkillToCollection("col-1", "code-reviewer");

    expect(invoke).toHaveBeenCalledWith("add_skill_to_collection", {
      collectionId: "col-1",
      skillId: "code-reviewer",
    });
    expect(invoke).toHaveBeenCalledWith("get_collection_detail", { collectionId: "col-1" });
    const state = useCollectionStore.getState();
    expect(state.currentDetail?.skills).toHaveLength(2);
  });

  // ── removeSkillFromCollection ──────────────────────────────────────────────

  it("removeSkillFromCollection calls remove command and reloads detail", async () => {
    useCollectionStore.setState({ currentDetail: mockCollectionDetail });

    const updatedDetail: CollectionDetail = { ...mockCollectionDetail, skills: [] };

    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined) // remove_skill_from_collection
      .mockResolvedValueOnce(updatedDetail); // get_collection_detail

    await useCollectionStore.getState().removeSkillFromCollection("col-1", "frontend-design");

    expect(invoke).toHaveBeenCalledWith("remove_skill_from_collection", {
      collectionId: "col-1",
      skillId: "frontend-design",
    });
    const state = useCollectionStore.getState();
    expect(state.currentDetail?.skills).toHaveLength(0);
  });

  // ── exportCollection ───────────────────────────────────────────────────────

  it("exportCollection returns JSON string from backend", async () => {
    const jsonStr = JSON.stringify({
      version: 1,
      name: "Frontend",
      description: "Frontend skills",
      skills: ["frontend-design"],
      createdAt: "2026-04-09T00:00:00Z",
      exportedFrom: "SkillPort",
    });

    vi.mocked(invoke).mockResolvedValueOnce(jsonStr);

    const result = await useCollectionStore.getState().exportCollection("col-1");

    expect(result).toBe(jsonStr);
    expect(invoke).toHaveBeenCalledWith("export_collection", { collectionId: "col-1" });
  });

  // ── importCollection ───────────────────────────────────────────────────────

  it("importCollection calls import command and reloads collections", async () => {
    const jsonStr = JSON.stringify({
      version: 1,
      name: "Imported",
      description: "Imported collection",
      skills: ["frontend-design"],
      createdAt: "2026-04-09T00:00:00Z",
      exportedFrom: "SkillPort",
    });

    const importedCollection: Collection = {
      id: "col-new",
      name: "Imported",
      description: "Imported collection",
      created_at: "2026-04-10T00:00:00Z",
      updated_at: "2026-04-10T00:00:00Z",
    };

    vi.mocked(invoke)
      .mockResolvedValueOnce(importedCollection) // import_collection
      .mockResolvedValueOnce([...mockCollections, importedCollection]); // get_collections

    const result = await useCollectionStore.getState().importCollection(jsonStr);

    expect(result).toEqual(importedCollection);
    expect(invoke).toHaveBeenCalledWith("import_collection", { json: jsonStr });
    const state = useCollectionStore.getState();
    expect(state.collections).toHaveLength(3);
  });
});
