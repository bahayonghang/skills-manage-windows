/**
 * Tag Groups store — Central Skills V2 / M3.
 *
 * 管理标签分组的 CRUD + reorder + 把 tag 分配到 group。一级，不允许嵌套（D4）。
 * 浏览器演示态由 src/fixtures/tagGroups.ts 的内存数据集按命令名供数。
 */

import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import type { TagGroup } from "@/types";

export interface TagGroupCreateInput {
  name: string;
  color?: string | null;
}

export interface TagGroupUpdateInput {
  name?: string;
  /** `null` 清空 color；`undefined` 不变。 */
  color?: string | null;
}

interface TagGroupsState {
  groups: TagGroup[];
  isLoading: boolean;
  error: string | null;

  loadTagGroups: () => Promise<void>;
  createTagGroup: (input: TagGroupCreateInput) => Promise<TagGroup>;
  updateTagGroup: (id: string, input: TagGroupUpdateInput) => Promise<TagGroup>;
  deleteTagGroup: (id: string) => Promise<void>;
  reorderTagGroups: (ids: string[]) => Promise<void>;
  setTagGroup: (tagId: string, groupId: string | null) => Promise<void>;
}

export const useTagGroupsStore = create<TagGroupsState>((set) => ({
  groups: [],
  isLoading: false,
  error: null,

  loadTagGroups: async () => {
    set({ isLoading: true, error: null });
    try {
      const groups = await invoke("list_tag_groups");
      set({ groups: groups ?? [], isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  createTagGroup: async (input) => {
    set({ error: null });

    try {
      const created = await invoke("create_tag_group", {
        input: { name: input.name, color: input.color ?? null },
      });
      const groups = await invoke("list_tag_groups");
      set({ groups: groups ?? [] });
      return created;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  updateTagGroup: async (id, input) => {
    set({ error: null });

    try {
      const updated = await invoke("update_tag_group", { id, input });
      const groups = await invoke("list_tag_groups");
      set({ groups: groups ?? [] });
      return updated;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  deleteTagGroup: async (id) => {
    set({ error: null });

    try {
      await invoke("delete_tag_group", { id });
      const groups = await invoke("list_tag_groups");
      set({ groups: groups ?? [] });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  reorderTagGroups: async (ids) => {
    set({ error: null });

    try {
      await invoke("reorder_tag_groups", { ids });
      const groups = await invoke("list_tag_groups");
      set({ groups: groups ?? [] });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  setTagGroup: async (tagId, groupId) => {
    set({ error: null });

    try {
      await invoke("set_tag_group", { tagId, groupId });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },
}));
