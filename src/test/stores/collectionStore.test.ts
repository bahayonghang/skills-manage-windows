import { describe, it, expect, vi, beforeEach } from "vitest";
import { Collection, CollectionDetail } from "@/types";
import * as tauriBridge from "@/lib/ipc";
import { ipcFixtureError } from "@/lib/ipc/errors";

// Mock Tauri core before importing the store
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { useCollectionStore } from "@/stores/collectionStore";

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
    description: null,
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

const mockCollectionDetailB: CollectionDetail = {
  id: "col-2",
  name: "Backend",
  description: null,
  created_at: "2026-04-09T01:00:00Z",
  updated_at: "2026-04-09T01:00:00Z",
  skills: [
    {
      id: "api-designer",
      name: "api-designer",
      description: "Design REST APIs",
      file_path: "~/.skillsmanage/skills/api-designer/SKILL.md",
      is_central: true,
      scanned_at: "2026-04-09T00:00:00Z",
    },
  ],
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

function collectionIdFromArgs(args: unknown): string {
  if (args && typeof args === "object" && "collectionId" in args) {
    const collectionId = (args as { collectionId: unknown }).collectionId;
    if (typeof collectionId === "string") {
      return collectionId;
    }
  }
  throw new Error("missing collectionId");
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("collectionStore", () => {
  beforeEach(() => {
    useCollectionStore.setState({
      collections: [],
      currentDetail: null,
      isLoading: false,
      isLoadingDetail: false,
      hasLoaded: false,
      detailTargetId: null,
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
    expect(state.hasLoaded).toBe(false);
    expect(state.detailTargetId).toBeNull();
    expect(state.error).toBeNull();
  });

  // ── loadCollections ────────────────────────────────────────────────────────

  it("loadCollections populates collections", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockCollections);

    await useCollectionStore.getState().loadCollections();

    const state = useCollectionStore.getState();
    expect(state.collections).toEqual(mockCollections);
    expect(state.isLoading).toBe(false);
    expect(state.hasLoaded).toBe(true);
    expect(state.error).toBeNull();
    expect(invoke).toHaveBeenCalledWith("get_collections");
  });

  it("loadCollections treats an empty array as loaded, not never-loaded", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([]);

    await useCollectionStore.getState().loadCollections();

    const state = useCollectionStore.getState();
    expect(state.collections).toEqual([]);
    expect(state.hasLoaded).toBe(true);
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
  });

  it("loadCollections sets error on failure", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(
      ipcFixtureError("storage.unavailable", "DB error"),
    );

    await useCollectionStore.getState().loadCollections();

    const state = useCollectionStore.getState();
    expect(state.error).toBe("DB error");
    expect(state.isLoading).toBe(false);
    expect(state.hasLoaded).toBe(false);
  });

  it("returns deterministic browser fixture collections when Tauri runtime is unavailable", async () => {
    const isTauriSpy = vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(false);

    await useCollectionStore.getState().loadCollections();
    await useCollectionStore.getState().loadCollectionDetail("fixture-collection");

    expect(invoke).not.toHaveBeenCalled();
    expect(useCollectionStore.getState().collections).toEqual([
      expect.objectContaining({ id: "fixture-collection" }),
    ]);
    expect(useCollectionStore.getState().hasLoaded).toBe(true);
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
      description: null,
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
    vi.mocked(invoke).mockRejectedValueOnce(
      ipcFixtureError("resource.not_found", "Not found"),
    );

    await useCollectionStore.getState().loadCollectionDetail("invalid-id");

    const state = useCollectionStore.getState();
    expect(state.error).toBe("Not found");
    expect(state.isLoadingDetail).toBe(false);
  });

  // ── addSkillToCollection ───────────────────────────────────────────────────

  it("addSkillToCollection calls add command and reloads detail", async () => {
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

    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "add_skill_to_collection") {
        return undefined;
      }
      if (command === "get_collection_detail") {
        return updatedDetail;
      }
      throw new Error(`unexpected command ${command}`);
    });

    await useCollectionStore.getState().loadCollectionDetail("col-1");
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
    const updatedDetail: CollectionDetail = { ...mockCollectionDetail, skills: [] };

    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "remove_skill_from_collection") {
        return undefined;
      }
      if (command === "get_collection_detail") {
        return updatedDetail;
      }
      throw new Error(`unexpected command ${command}`);
    });

    await useCollectionStore.getState().loadCollectionDetail("col-1");
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

  // ── loadCollectionDetail latest-wins ───────────────────────────────────────

  it("keeps the later collection detail when B returns before A", async () => {
    const detailA = deferred<CollectionDetail>();
    const detailB = deferred<CollectionDetail>();

    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command !== "get_collection_detail") {
        throw new Error(`unexpected command ${command}`);
      }
      const collectionId = collectionIdFromArgs(args);
      if (collectionId === "col-1") return detailA.promise;
      if (collectionId === "col-2") return detailB.promise;
      throw new Error(`unexpected collection ${collectionId}`);
    });

    const loadA = useCollectionStore.getState().loadCollectionDetail("col-1");
    const loadB = useCollectionStore.getState().loadCollectionDetail("col-2");

    detailB.resolve(mockCollectionDetailB);
    await loadB;

    expect(useCollectionStore.getState().currentDetail).toEqual(mockCollectionDetailB);
    expect(useCollectionStore.getState().detailTargetId).toBe("col-2");
    expect(useCollectionStore.getState().isLoadingDetail).toBe(false);

    detailA.resolve(mockCollectionDetail);
    await loadA;

    expect(useCollectionStore.getState().currentDetail).toEqual(mockCollectionDetailB);
    expect(useCollectionStore.getState().isLoadingDetail).toBe(false);
    expect(useCollectionStore.getState().error).toBeNull();
  });

  it("does not let A's late failure overwrite B's detail error state", async () => {
    const detailA = deferred<CollectionDetail>();
    const detailB = deferred<CollectionDetail>();

    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command !== "get_collection_detail") {
        throw new Error(`unexpected command ${command}`);
      }
      const collectionId = collectionIdFromArgs(args);
      if (collectionId === "col-1") return detailA.promise;
      if (collectionId === "col-2") return detailB.promise;
      throw new Error(`unexpected collection ${collectionId}`);
    });

    const loadA = useCollectionStore.getState().loadCollectionDetail("col-1");
    const loadB = useCollectionStore.getState().loadCollectionDetail("col-2");

    detailB.resolve(mockCollectionDetailB);
    await loadB;
    expect(useCollectionStore.getState().error).toBeNull();
    expect(useCollectionStore.getState().currentDetail).toEqual(mockCollectionDetailB);

    detailA.reject(ipcFixtureError("resource.not_found", "stale A failure"));
    await loadA;

    expect(useCollectionStore.getState().currentDetail).toEqual(mockCollectionDetailB);
    expect(useCollectionStore.getState().error).toBeNull();
    expect(useCollectionStore.getState().isLoadingDetail).toBe(false);
  });

  it("does not let A's late completion end B's loading ownership", async () => {
    const detailA = deferred<CollectionDetail>();
    const detailB = deferred<CollectionDetail>();

    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command !== "get_collection_detail") {
        throw new Error(`unexpected command ${command}`);
      }
      const collectionId = collectionIdFromArgs(args);
      if (collectionId === "col-1") return detailA.promise;
      if (collectionId === "col-2") return detailB.promise;
      throw new Error(`unexpected collection ${collectionId}`);
    });

    const loadA = useCollectionStore.getState().loadCollectionDetail("col-1");
    const loadB = useCollectionStore.getState().loadCollectionDetail("col-2");

    expect(useCollectionStore.getState().isLoadingDetail).toBe(true);
    expect(useCollectionStore.getState().detailTargetId).toBe("col-2");

    detailA.resolve(mockCollectionDetail);
    await loadA;

    expect(useCollectionStore.getState().isLoadingDetail).toBe(true);
    expect(useCollectionStore.getState().currentDetail).not.toEqual(mockCollectionDetail);
    expect(useCollectionStore.getState().detailTargetId).toBe("col-2");

    detailB.resolve(mockCollectionDetailB);
    await loadB;

    expect(useCollectionStore.getState().currentDetail).toEqual(mockCollectionDetailB);
    expect(useCollectionStore.getState().isLoadingDetail).toBe(false);
  });

  it("keeps B's detail when mutation A's refresh races a switch to B", async () => {
    const addCommand = deferred<void>();
    const initialA = deferred<CollectionDetail>();
    const refreshA = deferred<CollectionDetail>();
    const col1Loads = [initialA, refreshA];
    const detailB = deferred<CollectionDetail>();
    const updatedA: CollectionDetail = {
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

    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command === "add_skill_to_collection") {
        return addCommand.promise;
      }
      if (command !== "get_collection_detail") {
        throw new Error(`unexpected command ${command}`);
      }
      const collectionId = collectionIdFromArgs(args);
      if (collectionId === "col-1") {
        const next = col1Loads.shift();
        if (!next) {
          throw new Error("unexpected extra col-1 detail request");
        }
        return next.promise;
      }
      if (collectionId === "col-2") return detailB.promise;
      throw new Error(`unexpected collection ${collectionId}`);
    });

    const loadA = useCollectionStore.getState().loadCollectionDetail("col-1");
    initialA.resolve(mockCollectionDetail);
    await loadA;

    const mutation = useCollectionStore.getState().addSkillToCollection("col-1", "code-reviewer");
    addCommand.resolve();
    await vi.waitFor(() => {
      expect(col1Loads).toHaveLength(0);
    });

    const loadB = useCollectionStore.getState().loadCollectionDetail("col-2");
    detailB.resolve(mockCollectionDetailB);
    await loadB;

    expect(useCollectionStore.getState().currentDetail).toEqual(mockCollectionDetailB);

    refreshA.resolve(updatedA);
    await mutation;

    expect(useCollectionStore.getState().currentDetail).toEqual(mockCollectionDetailB);
    expect(useCollectionStore.getState().detailTargetId).toBe("col-2");
  });

  it("does not let a stale mutation re-seize detail ownership after the target switches", async () => {
    const addCommand = deferred<void>();
    const initialA = deferred<CollectionDetail>();
    const detailB = deferred<CollectionDetail>();
    const detailIds: string[] = [];

    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command === "add_skill_to_collection") {
        return addCommand.promise;
      }
      if (command !== "get_collection_detail") {
        throw new Error(`unexpected command ${command}`);
      }
      const collectionId = collectionIdFromArgs(args);
      detailIds.push(collectionId);
      if (collectionId === "col-1") return initialA.promise;
      if (collectionId === "col-2") return detailB.promise;
      throw new Error(`unexpected collection ${collectionId}`);
    });

    const loadA = useCollectionStore.getState().loadCollectionDetail("col-1");
    initialA.resolve(mockCollectionDetail);
    await loadA;

    const mutation = useCollectionStore.getState().addSkillToCollection("col-1", "code-reviewer");

    const loadB = useCollectionStore.getState().loadCollectionDetail("col-2");
    detailB.resolve(mockCollectionDetailB);
    await loadB;

    addCommand.resolve();
    await mutation;

    expect(detailIds).toEqual(["col-1", "col-2"]);
    expect(useCollectionStore.getState().currentDetail).toEqual(mockCollectionDetailB);
    expect(useCollectionStore.getState().detailTargetId).toBe("col-2");
  });
});
