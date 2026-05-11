/**
 * Tag Groups store — Central Skills V2 / M3.
 *
 * 管理标签分组的 CRUD + reorder + 把 tag 分配到 group。一级，不允许嵌套（D4）。
 * 浏览器 / 非 Tauri 环境写入内存 fixture，便于本地 dev 与单元测试。
 */

import { create } from "zustand";
import { invoke, isTauriRuntime } from "@/lib/tauri";
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

let browserFixtureCounter = 0;
function nextBrowserId(): string {
  browserFixtureCounter += 1;
  return `fixture-tag-group-${browserFixtureCounter}`;
}

function nowIso(): string {
  return new Date().toISOString();
}

export const useTagGroupsStore = create<TagGroupsState>((set, get) => ({
  groups: [],
  isLoading: false,
  error: null,

  loadTagGroups: async () => {
    set({ isLoading: true, error: null });
    if (!isTauriRuntime()) {
      set({ isLoading: false });
      return;
    }
    try {
      const groups = await invoke<TagGroup[]>("list_tag_groups");
      set({ groups: groups ?? [], isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  createTagGroup: async (input) => {
    set({ error: null });

    if (!isTauriRuntime()) {
      const next: TagGroup = {
        id: nextBrowserId(),
        name: input.name,
        color: input.color ?? null,
        sort_order: get().groups.length,
        is_builtin: false,
        created_at: nowIso(),
        updated_at: nowIso(),
      };
      set({ groups: [...get().groups, next] });
      return next;
    }

    try {
      const created = await invoke<TagGroup>("create_tag_group", {
        input: { name: input.name, color: input.color ?? null },
      });
      const groups = await invoke<TagGroup[]>("list_tag_groups");
      set({ groups: groups ?? [] });
      return created;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  updateTagGroup: async (id, input) => {
    set({ error: null });

    if (!isTauriRuntime()) {
      const next = get().groups.map((g) =>
        g.id === id
          ? {
              ...g,
              ...(input.name !== undefined ? { name: input.name } : {}),
              ...(input.color !== undefined ? { color: input.color } : {}),
              updated_at: nowIso(),
            }
          : g,
      );
      set({ groups: next });
      const found = next.find((g) => g.id === id);
      if (!found) throw new Error(`Tag group '${id}' not found`);
      return found;
    }

    try {
      const updated = await invoke<TagGroup>("update_tag_group", { id, input });
      const groups = await invoke<TagGroup[]>("list_tag_groups");
      set({ groups: groups ?? [] });
      return updated;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  deleteTagGroup: async (id) => {
    set({ error: null });

    if (!isTauriRuntime()) {
      set({ groups: get().groups.filter((g) => g.id !== id) });
      return;
    }

    try {
      await invoke("delete_tag_group", { id });
      const groups = await invoke<TagGroup[]>("list_tag_groups");
      set({ groups: groups ?? [] });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  reorderTagGroups: async (ids) => {
    set({ error: null });

    if (!isTauriRuntime()) {
      const map = new Map(get().groups.map((g) => [g.id, g]));
      const ordered: TagGroup[] = [];
      ids.forEach((id, index) => {
        const g = map.get(id);
        if (g) {
          ordered.push({ ...g, sort_order: index, updated_at: nowIso() });
          map.delete(id);
        }
      });
      const trailing = Array.from(map.values()).map((g, idx) => ({
        ...g,
        sort_order: ids.length + idx,
      }));
      set({ groups: [...ordered, ...trailing] });
      return;
    }

    try {
      await invoke("reorder_tag_groups", { ids });
      const groups = await invoke<TagGroup[]>("list_tag_groups");
      set({ groups: groups ?? [] });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  setTagGroup: async (tagId, groupId) => {
    set({ error: null });

    if (!isTauriRuntime()) {
      // 浏览器 fixture 中 tag 数据在 centralSkillsStore，这里只做 noop（保留接口）。
      return;
    }

    try {
      await invoke("set_tag_group", { tagId, groupId });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },
}));
