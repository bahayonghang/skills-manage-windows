import { create } from "zustand";
import { invoke, isTauriRuntime } from "@/lib/ipc";
import { Collection, CollectionDetail, CollectionBatchInstallResult } from "@/types";
import { usePlatformStore } from "@/stores/platformStore";
import {
  BROWSER_PLATFORM_PATHS,
  getPlatformSkillDir,
  getPlatformSkillFilePath,
} from "@/lib/platformPathPolicy";

const BROWSER_FIXTURE_COLLECTIONS: Collection[] = [
  {
    id: "fixture-collection",
    name: "Fixture Collection",
    description: "Browser validation fixture collection.",
    created_at: "2026-04-17T00:00:00.000Z",
    updated_at: "2026-04-17T00:00:00.000Z",
  },
];

const BROWSER_FIXTURE_COLLECTION_DETAIL: CollectionDetail = {
  id: "fixture-collection",
  name: "Fixture Collection",
  description: "Browser validation fixture collection.",
  created_at: "2026-04-17T00:00:00.000Z",
  updated_at: "2026-04-17T00:00:00.000Z",
  skills: [
    {
      id: "fixture-central-skill",
      name: "fixture-central-skill",
      description: "Browser validation fixture for Collection drawer entry flows.",
      file_path: getPlatformSkillFilePath(
        BROWSER_PLATFORM_PATHS,
        "central",
        "fixture-central-skill"
      ),
      canonical_path: getPlatformSkillDir(
        BROWSER_PLATFORM_PATHS,
        "central",
        "fixture-central-skill"
      ),
      is_central: true,
      source: "browser-fixture",
      scanned_at: "2026-04-17T00:00:00.000Z",
    },
  ],
};

// ─── State ────────────────────────────────────────────────────────────────────

interface CollectionState {
  collections: Collection[];
  currentDetail: CollectionDetail | null;
  isLoading: boolean;
  isLoadingDetail: boolean;
  /**
   * True after at least one successful list load, including an empty array.
   * Array length must not be used as a substitute for this fact.
   */
  hasLoaded: boolean;
  /**
   * Collection id currently owning in-flight or latest detail request.
   * Mutations refresh only when this still matches the mutated collection.
   */
  detailTargetId: string | null;
  error: string | null;

  // Actions
  loadCollections: () => Promise<void>;
  createCollection: (name: string, description?: string) => Promise<Collection>;
  updateCollection: (id: string, name: string, description?: string) => Promise<Collection>;
  deleteCollection: (id: string) => Promise<void>;
  loadCollectionDetail: (id: string) => Promise<void>;
  addSkillToCollection: (collectionId: string, skillId: string) => Promise<void>;
  removeSkillFromCollection: (collectionId: string, skillId: string) => Promise<void>;
  batchInstallCollection: (collectionId: string, agentIds: string[]) => Promise<CollectionBatchInstallResult>;
  exportCollection: (collectionId: string) => Promise<string>;
  importCollection: (json: string) => Promise<Collection>;
  refreshCounts: () => Promise<void>;
}

/** Monotonic owner for loadCollectionDetail writes. Module-level so overlapping awaits share one counter. */
let detailRequestId = 0;

// ─── Store ────────────────────────────────────────────────────────────────────

export const useCollectionStore = create<CollectionState>((set, get) => ({
  collections: [],
  currentDetail: null,
  isLoading: false,
  isLoadingDetail: false,
  hasLoaded: false,
  detailTargetId: null,
  error: null,

  /**
   * Load all collections from the backend.
   */
  loadCollections: async () => {
    set({ isLoading: true, error: null });
    if (!isTauriRuntime()) {
      set({
        collections: BROWSER_FIXTURE_COLLECTIONS,
        isLoading: false,
        hasLoaded: true,
      });
      usePlatformStore.getState().setCollectionCount(BROWSER_FIXTURE_COLLECTIONS.length);
      return;
    }
    try {
      const collections = await invoke("get_collections");
      set({ collections: collections ?? [], isLoading: false, hasLoaded: true });
      usePlatformStore.getState().setCollectionCount(collections?.length ?? 0);
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  /**
   * Create a new collection and refresh the list.
   */
  createCollection: async (name: string, description?: string) => {
    set({ error: null });
    try {
      const collection = await invoke("create_collection", {
        name,
        description: description ?? null,
      });
      // Refresh collections list.
      const collections = await invoke("get_collections");
      set({ collections: collections ?? [], hasLoaded: true });
      usePlatformStore.getState().setCollectionCount(collections?.length ?? 0);
      return collection;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  /**
   * Update an existing collection's name/description and refresh the list.
   */
  updateCollection: async (id: string, name: string, description?: string) => {
    set({ error: null });
    try {
      const collection = await invoke("update_collection", {
        collectionId: id,
        name,
        description: description ?? null,
      });
      // Refresh collections list.
      const collections = await invoke("get_collections");
      set({ collections: collections ?? [], hasLoaded: true });
      usePlatformStore.getState().setCollectionCount(collections?.length ?? 0);
      // Also update currentDetail if it's for this collection.
      const { currentDetail } = get();
      if (currentDetail?.id === id) {
        set({
          currentDetail: {
            ...currentDetail,
            name: collection.name,
            description: collection.description,
            updated_at: collection.updated_at,
          },
        });
      }
      return collection;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  /**
   * Delete a collection and refresh the list.
   */
  deleteCollection: async (id: string) => {
    set({ error: null });
    try {
      await invoke("delete_collection", { collectionId: id });
      // Refresh collections list.
      const collections = await invoke("get_collections");
      set({ collections: collections ?? [], hasLoaded: true });
      usePlatformStore.getState().setCollectionCount(collections?.length ?? 0);
      // Clear currentDetail if it was for this collection.
      const { currentDetail } = get();
      if (currentDetail?.id === id) {
        set({ currentDetail: null });
      }
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  /**
   * Load a collection's detail (including member skills).
   * Only the current request (matching both request id and target id) may write
   * currentDetail, detail error, or finish isLoadingDetail.
   */
  loadCollectionDetail: async (id: string) => {
    set({ detailTargetId: id, isLoadingDetail: true, error: null });
    const requestId = ++detailRequestId;
    const isCurrentRequest = () =>
      requestId === detailRequestId && get().detailTargetId === id;

    if (!isTauriRuntime()) {
      if (isCurrentRequest()) {
        set({
          currentDetail:
            id === BROWSER_FIXTURE_COLLECTION_DETAIL.id
              ? BROWSER_FIXTURE_COLLECTION_DETAIL
              : null,
          isLoadingDetail: false,
        });
      }
      return;
    }
    try {
      const detail = await invoke("get_collection_detail", {
        collectionId: id,
      });
      if (isCurrentRequest()) {
        set({ currentDetail: detail, isLoadingDetail: false });
      }
    } catch (err) {
      if (isCurrentRequest()) {
        set({ error: String(err), isLoadingDetail: false });
      }
    }
  },

  /**
   * Add a skill to a collection and reload the detail.
   */
  addSkillToCollection: async (collectionId: string, skillId: string) => {
    set({ error: null });
    try {
      await invoke("add_skill_to_collection", { collectionId, skillId });
      if (get().detailTargetId === collectionId) {
        await get().loadCollectionDetail(collectionId);
      }
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  /**
   * Remove a skill from a collection and reload the detail.
   */
  removeSkillFromCollection: async (collectionId: string, skillId: string) => {
    set({ error: null });
    try {
      await invoke("remove_skill_from_collection", { collectionId, skillId });
      if (get().detailTargetId === collectionId) {
        await get().loadCollectionDetail(collectionId);
      }
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  /**
   * Batch install all skills in a collection to the given agents.
   */
  batchInstallCollection: async (collectionId: string, agentIds: string[]) => {
    set({ error: null });
    try {
      const result = await invoke("batch_install_collection", {
        collectionId,
        agentIds,
      });
      return result;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  /**
   * Export a collection as a JSON string.
   */
  exportCollection: async (collectionId: string) => {
    try {
      return await invoke("export_collection", { collectionId });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  /**
   * Import a collection from a JSON string and refresh the list.
   */
  importCollection: async (json: string) => {
    set({ error: null });
    try {
      const collection = await invoke("import_collection", { json });
      // Refresh collections list.
      const collections = await invoke("get_collections");
      set({ collections: collections ?? [], hasLoaded: true });
      usePlatformStore.getState().setCollectionCount(collections?.length ?? 0);
      return collection;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  refreshCounts: async () => {
    if (!isTauriRuntime()) {
      set({ collections: BROWSER_FIXTURE_COLLECTIONS, hasLoaded: true });
      usePlatformStore.getState().setCollectionCount(BROWSER_FIXTURE_COLLECTIONS.length);
      return;
    }
    try {
      const collections = await invoke("get_collections");
      set({ collections: collections ?? [], hasLoaded: true });
      usePlatformStore.getState().setCollectionCount(collections?.length ?? 0);
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },
}));
