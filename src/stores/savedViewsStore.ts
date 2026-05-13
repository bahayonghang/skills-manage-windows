/**
 * Saved Views store — Central Skills V2 / M2.
 *
 * 管理保存视图的 CRUD + reorder。`query` 字段是整段 `CentralViewState` 的
 * URL 序列化（无前导 `?`），编/解码由调用方负责（`serializeCentralViewState`/
 * `parseCentralViewStateFromUrl`），store 只做透传。
 *
 * 浏览器（非 Tauri）环境下不会发起 invoke，CRUD 写入内存 fixture，便于本地 dev
 * 与单元测试。
 */

import { create } from "zustand";
import { invoke, isTauriRuntime } from "@/lib/tauri";
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
  updateSavedView: (id: string, input: SavedViewUpdateInput) => Promise<SavedView>;
  deleteSavedView: (id: string) => Promise<void>;
  reorderSavedViews: (ids: string[]) => Promise<void>;
}

// ─── Browser fixture（仅在非 Tauri 环境使用） ─────────────────────────────────

let browserFixtureCounter = 0;
function nextBrowserId(): string {
  browserFixtureCounter += 1;
  return `fixture-saved-view-${browserFixtureCounter}`;
}

function nowIso(): string {
  return new Date().toISOString();
}

// ─── Store ───────────────────────────────────────────────────────────────────

export const useSavedViewsStore = create<SavedViewsState>((set, get) => ({
  views: [],
  isLoading: false,
  error: null,

  loadSavedViews: async () => {
    set({ isLoading: true, error: null });
    if (!isTauriRuntime()) {
      set({ isLoading: false });
      return;
    }
    try {
      const views = await invoke<SavedView[]>("list_saved_views");
      set({ views: views ?? [], isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  createSavedView: async (input) => {
    set({ error: null });

    if (!isTauriRuntime()) {
      // Fixture：直接构造一条插入末尾
      const next: SavedView = {
        id: nextBrowserId(),
        name: input.name,
        query: input.query,
        sort_order: get().views.length,
        icon: input.icon ?? null,
        pinned: input.pinned ?? false,
        created_at: nowIso(),
        updated_at: nowIso(),
      };
      set({ views: [...get().views, next] });
      return next;
    }

    try {
      const created = await invoke<SavedView>("create_saved_view", {
        input: {
          name: input.name,
          query: input.query,
          icon: input.icon ?? null,
          pinned: input.pinned ?? false,
        },
      });
      // 重新拉取以保证 sort_order 与 pinned-first 排序一致
      const views = await invoke<SavedView[]>("list_saved_views");
      set({ views: views ?? [] });
      return created;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  updateSavedView: async (id, input) => {
    set({ error: null });

    if (!isTauriRuntime()) {
      const next = get().views.map((view) =>
        view.id === id
          ? {
              ...view,
              ...(input.name !== undefined ? { name: input.name } : {}),
              ...(input.query !== undefined ? { query: input.query } : {}),
              ...(input.icon !== undefined ? { icon: input.icon } : {}),
              ...(input.pinned !== undefined ? { pinned: input.pinned } : {}),
              updated_at: nowIso(),
            }
          : view,
      );
      set({ views: next });
      const found = next.find((view) => view.id === id);
      if (!found) throw new Error(`Saved view '${id}' not found`);
      return found;
    }

    try {
      const updated = await invoke<SavedView>("update_saved_view", {
        id,
        input,
      });
      const views = await invoke<SavedView[]>("list_saved_views");
      set({ views: views ?? [] });
      return updated;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  deleteSavedView: async (id) => {
    set({ error: null });

    if (!isTauriRuntime()) {
      set({ views: get().views.filter((view) => view.id !== id) });
      return;
    }

    try {
      await invoke("delete_saved_view", { id });
      const views = await invoke<SavedView[]>("list_saved_views");
      set({ views: views ?? [] });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  reorderSavedViews: async (ids) => {
    set({ error: null });

    if (!isTauriRuntime()) {
      // Fixture：按 ids 顺序重排，未列出的保持原相对顺序追加在末尾
      const map = new Map(get().views.map((view) => [view.id, view]));
      const ordered: SavedView[] = [];
      ids.forEach((id, index) => {
        const view = map.get(id);
        if (view) {
          ordered.push({ ...view, sort_order: index, updated_at: nowIso() });
          map.delete(id);
        }
      });
      const trailing = Array.from(map.values()).map((view, idx) => ({
        ...view,
        sort_order: ids.length + idx,
      }));
      set({ views: [...ordered, ...trailing] });
      return;
    }

    try {
      await invoke("reorder_saved_views", { ids });
      const views = await invoke<SavedView[]>("list_saved_views");
      set({ views: views ?? [] });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },
}));
