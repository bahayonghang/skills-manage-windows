/**
 * Saved Views store — Central Skills V2 / M2.
 *
 * 管理保存视图的 CRUD + reorder。`query` 字段是整段 `CentralViewState` 的
 * URL 序列化（无前导 `?`），编/解码由调用方负责（`serializeCentralViewState`/
 * `parseCentralViewStateFromUrl`），store 只做透传。
 *
 * 浏览器演示态由 src/fixtures/savedViews.ts 的内存数据集按命令名供数。
 */

import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import type { SavedView } from "@/types";

export interface SavedViewCreateInput {
  name: string;
  /** `CentralViewState` 的 URL 序列化结果（不含前导 `?`）。 */
  query: string;
  icon?: string | null;
  pinned?: boolean;
}

export interface SavedViewUpdateInput {
  name?: string;
  query?: string;
  /** `null` 表示清空 icon；`undefined` 表示不变。 */
  icon?: string | null;
  pinned?: boolean;
}

interface SavedViewsState {
  views: SavedView[];
  isLoading: boolean;
  error: string | null;

  loadSavedViews: () => Promise<void>;
  createSavedView: (input: SavedViewCreateInput) => Promise<SavedView>;
  updateSavedView: (
    id: string,
    input: SavedViewUpdateInput,
  ) => Promise<SavedView>;
  deleteSavedView: (id: string) => Promise<void>;
  reorderSavedViews: (ids: string[]) => Promise<void>;
}

// ─── Store ───────────────────────────────────────────────────────────────────

export const useSavedViewsStore = create<SavedViewsState>((set) => ({
  views: [],
  isLoading: false,
  error: null,

  loadSavedViews: async () => {
    set({ isLoading: true, error: null });
    try {
      const views = await invoke("list_saved_views");
      set({ views: views ?? [], isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  createSavedView: async (input) => {
    set({ error: null });

    try {
      const created = await invoke("create_saved_view", {
        input: {
          name: input.name,
          query: input.query,
          icon: input.icon ?? null,
          pinned: input.pinned ?? false,
        },
      });
      // 重新拉取以保证 sort_order 与 pinned-first 排序一致
      const views = await invoke("list_saved_views");
      set({ views: views ?? [] });
      return created;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  updateSavedView: async (id, input) => {
    set({ error: null });

    try {
      const updated = await invoke("update_saved_view", {
        id,
        input,
      });
      const views = await invoke("list_saved_views");
      set({ views: views ?? [] });
      return updated;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  deleteSavedView: async (id) => {
    set({ error: null });

    try {
      await invoke("delete_saved_view", { id });
      const views = await invoke("list_saved_views");
      set({ views: views ?? [] });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  reorderSavedViews: async (ids) => {
    set({ error: null });

    try {
      await invoke("reorder_saved_views", { ids });
      const views = await invoke("list_saved_views");
      set({ views: views ?? [] });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },
}));
